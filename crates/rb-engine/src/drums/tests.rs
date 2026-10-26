//! The voice classes against block outputs computed by hand (task-024, AC #2). The gains are
//! powers of two and the samples small integers, so every expected value is exact.

use super::*;
use crate::frame::BLOCK;

fn sample(audio: &[i16]) -> Option<Arc<Sample>> {
    Some(Arc::new(Sample::from_audio(audio)))
}

/// The trigger gain that gives the voice gain `g` (`g = f32(gain * 2^-15)`).
fn gain(g: f32) -> f32 {
    g * 32768.0
}

#[test]
fn simple_decays_per_pair_and_ends_below_3e_8() {
    let mut v = Simple::default();
    v.trigger(sample(&[1024; 200]), gain(1.0), 0.5, 0);
    assert_eq!((v.v.g, v.v.left, v.v.active), (1.0, 200, true));
    let mut out = [0.0; BLOCK];
    v.process(&mut out);
    for k in 0..32 {
        // pair k plays at 0.5^k
        let want = 1024.0 * 0.5f32.powi(k as i32);
        assert_eq!((out[2 * k], out[2 * k + 1]), (want, want), "pair {k}");
    }
    // g = 2^-32 after the block, below 3e-8: the voice ends
    assert_eq!(v.v.g, 2.0f32.powi(-32));
    assert!(!v.v.active);
}

#[test]
fn simple_adds_into_the_bus_and_holds() {
    let mut v = Simple::default();
    v.trigger(sample(&[100, -200].repeat(200)), gain(0.5), 0.5, 2);
    let mut out = [1.0; BLOCK];
    // two held blocks at 0.5, then the decay
    for b in 0..2 {
        out.fill(1.0);
        v.process(&mut out);
        assert_eq!((out[0], out[1], out[63]), (51.0, -99.0, -99.0), "block {b}");
        assert_eq!(v.v.hold, 1 - b as u32);
    }
    out.fill(1.0);
    v.process(&mut out);
    assert_eq!((out[0], out[1], out[2], out[3]), (51.0, -99.0, 26.0, -49.0));
    assert_eq!(v.v.g, 0.5 * 0.5f32.powi(32));
}

#[test]
fn simple_ends_with_its_sample_and_reads_the_padding() {
    let mut v = Simple::default();
    v.trigger(sample(&[8; 100]), gain(1.0), 1.0, 0);
    let mut out = [0.0; BLOCK];
    v.process(&mut out);
    assert_eq!((v.v.left, v.v.active, v.v.pos), (36, true, 64));
    out.fill(0.0);
    v.process(&mut out);
    // frames 64..100 play, 100..128 are the zero padding; then the voice ends
    assert_eq!((out[35], out[36], out[63]), (8.0, 0.0, 0.0));
    assert_eq!((v.v.left, v.v.active), (-28, false));
    // an inactive voice adds nothing
    out.fill(3.0);
    v.process(&mut out);
    assert_eq!(out, [3.0; BLOCK]);
}

#[test]
fn simple_retrigger_restarts_and_silence_clears_the_gain() {
    let mut v = Simple::default();
    let s = sample(&[1; 300]);
    v.trigger(s.clone(), gain(1.0), 1.0, 0);
    let mut out = [0.0; BLOCK];
    v.process(&mut out);
    // a trigger without a sample keeps the last one
    v.trigger(None, gain(2.0), 1.0, 0);
    assert_eq!((v.v.pos, v.v.left, v.v.g), (0, 300, 2.0));
    v.silence();
    assert_eq!((v.v.active, v.v.g), (false, 0.0));
}

#[test]
fn filter_is_a_one_pole_low_pass() {
    let mut v = Filter::default();
    let mut imp = vec![0i16; 200];
    imp[0] = 1024;
    v.y = 5.0;
    v.trigger(sample(&imp), gain(1.0), 1.0, 0, 0.5);
    // the trigger clears y
    assert_eq!((v.y, v.c), (0.0, 0.5));
    let mut out = [0.0; BLOCK];
    v.process(&mut out);
    for (i, &o) in out.iter().enumerate() {
        assert_eq!(o, 1024.0 * 0.5f32.powi(i as i32), "sample {i}");
    }
    // y after the block: 1024 * 2^-63, stored as f32
    assert_eq!(v.y, 2.0f32.powi(-53));
}

#[test]
fn filter_keeps_y_unrounded_within_the_block() {
    // c = 0.5, g = 1, m = 1 and the input 1 at sample 0, 1024 at 30, -512 at 31: y_n = 2^-n
    // except y_30 = 1024 + 2^-30 (41 binary digits, output as f32 1024) and y_31 = 2^-31. A y
    // rounded to f32 at sample 30 would make y_31 = 0.
    let mut s = vec![0i16; 200];
    s[0] = 1;
    s[30] = 1024;
    s[31] = -512;
    let mut v = Filter::default();
    v.trigger(sample(&s), gain(1.0), 1.0, 0, 0.5);
    let mut out = [0.0; BLOCK];
    v.process(&mut out);
    for (n, &o) in out.iter().enumerate() {
        let want = if n == 30 {
            1024.0
        } else {
            2f32.powi(-(n as i32))
        };
        assert_eq!(o, want, "sample {n}");
    }
    assert_eq!((v.y, v.v.g), (2f32.powi(-63), 1.0));
}

#[test]
fn filter_with_a_decaying_gain_matches_its_closed_form() {
    // c = 0.5, s = 1, g = 1 halving per pair (m = 0.5): sample j enters at 2^-(j/2) and
    // halves per sample, so y_n = sum over j = 0..=n of 2^-((n - j) + j/2), exact in f64.
    let mut v = Filter::default();
    v.trigger(sample(&[1; 200]), gain(1.0), 0.5, 0, 0.5);
    let mut out = [0.0; BLOCK];
    v.process(&mut out);
    let y = |n: i32| -> f64 { (0..=n).map(|j| 2f64.powi(-((n - j) + j / 2))).sum() };
    // the first ones by hand: 1, 1.5, then at g = 0.5: 0.75 + 0.5, 0.625 + 0.5
    assert_eq!((y(0), y(1), y(2), y(3)), (1.0, 1.5, 1.25, 1.125));
    for (n, &o) in out.iter().enumerate() {
        assert_eq!(o, y(n as i32) as f32, "sample {n}");
    }
    assert_eq!((v.y, v.v.g), (y(63) as f32, 2f32.powi(-32)));
}

#[test]
fn dual_mixes_two_samples_and_decays_only_the_second() {
    let mut v = Dual::default();
    v.trigger(
        sample(&[1000; 100]),
        gain(0.5),
        0.5,
        0,
        sample(&[2000; 70]),
        0.25,
    );
    // the shorter sample's length
    assert_eq!((v.v.left, v.v.g, v.g2), (70, 0.5, 0.25));
    let mut out = [0.0; BLOCK];
    v.process(&mut out);
    // 0.5 * (1000 + 2000 * 0.25 * 0.5^k)
    assert_eq!((out[0], out[1]), (750.0, 750.0));
    assert_eq!((out[2], out[3]), (625.0, 625.0));
    assert_eq!(out[63], (0.5 * (1000.0 + 500.0 * 0.5f64.powi(31))) as f32);
    assert_eq!((v.v.g, v.g2), (0.5, 0.25 * 0.5f32.powi(32)));
    assert_eq!((v.v.left, v.v.active, v.b_pos), (6, true, 64));
    out.fill(0.0);
    v.process(&mut out);
    // B (70 frames) ends at 6; A plays on to 36; both padded with zeros
    assert_eq!(
        (out[5], out[6], out[35], out[36]),
        (500.0, 500.0, 500.0, 0.0)
    );
    assert!(!v.v.active);
}

#[test]
fn dual_ends_at_once_when_g1_is_below_3e_8() {
    let mut v = Dual::default();
    v.trigger(
        sample(&[1; 1000]),
        gain(1e-8),
        1.0,
        0,
        sample(&[1; 1000]),
        1.0,
    );
    // g1 = 1e-8f exactly (the trigger's scaling by 2^-15 undoes gain()'s 2^15)
    assert_eq!(v.v.g, 1e-8f32);
    let mut out = [0.0; BLOCK];
    v.process(&mut out);
    // the block still plays in full: (1 + 1*1.0) * g1, exact in f32
    assert_eq!(out, [2.0 * 1e-8f32; BLOCK]);
    assert!(!v.v.active);
}

#[test]
fn pitched_interpolates_and_ends_past_its_last_frame() {
    // s[i] = 256 i at half speed: the interpolated value is the phase itself
    let ramp: Vec<i16> = (0..64).map(|i| (256 * i) as i16).collect();
    let mut v = Pitched::default();
    v.trigger(sample(&ramp), gain(1.0), 1.0, 0, 128);
    assert_eq!((v.end, v.phase), (64 << 8, 0));
    let mut out = [0.0; BLOCK];
    v.process(&mut out);
    for (n, &o) in out.iter().enumerate() {
        assert_eq!(o, (128 * n) as f32, "sample {n}");
    }
    assert_eq!((v.phase, v.v.active), (8192, true));
    v.process(&mut out);
    // the phase reaches the end exactly: not past it
    assert_eq!((v.phase, v.v.active), (16384, true));
    v.process(&mut out);
    assert!(!v.v.active);
}

#[test]
fn pitched_shifts_arithmetically_and_decays_per_pair() {
    // a = 0, b = -255, f = 128: (128 * -255) >> 8 = -127.5 floored to -128
    let mut v = Pitched::default();
    v.trigger(sample(&[0, -255, 0, 0]), gain(1.0), 0.5, 0, 128);
    let mut out = [0.0; BLOCK];
    v.process(&mut out);
    assert_eq!((out[0], out[1], out[2]), (0.0, -128.0, -127.5));
    // pair 1 at gain 0.5: sample 2 is s[1] = -255
    assert!(!v.v.active);
    assert_eq!(v.v.g, 0.5f32.powi(32));
}

#[test]
fn pitched_has_no_gain_test() {
    let mut v = Pitched::default();
    v.trigger(sample(&[1; 1000]), gain(1e-9), 0.0, 0, 256);
    let mut out = [0.0; BLOCK];
    v.process(&mut out);
    assert!(v.v.active);
}
