use super::*;
use crate::tables::Tables;

fn ctx(t: &Tables, tempo: u32) -> BlockContext<'_> {
    BlockContext {
        tables: t,
        meter_index: 0,
        tempo,
    }
}

#[test]
fn the_length_formula() {
    // 120 BPM, 3 sixteenths: q = 4*120000/100 = 4800, 79380000 / 4800 = 16537.5 → 16537 (the
    // corpus export-start dump's D)
    assert_eq!(length(120_000, 3, 1), 16537);
    // any non-zero mode is sixteenths
    assert_eq!(length(120_000, 3, 7), 16537);
    // mode 0, triplets: q = 3600, 79380000 / 3600 = 22050
    assert_eq!(length(120_000, 3, 0), 22050);
    // 66.667 BPM, 1 sixteenth: q = 266668/100 = 2666, 26460000 / 2666 = 9924.98 → 9924
    assert_eq!(length(66_667, 1, 1), 9924);
    // 32 triplets at 120 BPM: 846720000 / 3600 = 235200, clamped to 0x1FFFF
    assert_eq!(length(120_000, 32, 0), 0x1FFFF);
    // 26460000 * 200 wraps: 5292000000 - 2^32 = 997032704, / 4800 = 207715 → clamped
    assert_eq!(length(120_000, 200, 1), 0x1FFFF);
    // 26460000 * 170 = 4498200000 wraps to 203232704, / 48000 (1200 BPM) = 4234
    assert_eq!(length(1_200_000, 170, 1), 4234);
}

#[test]
fn the_gains_smooth_by_0_9_and_0_1_unrounded() {
    // 0 * 0.9 + 1 * 0.1 is the f64 0.1, which the block uses as it is
    assert_eq!(smooth(0.0, 1.0), 0.1);
    assert_eq!(smooth(0.0, 2.0), 0.2);
    assert_eq!(smooth(1.0, 1.0), 1.0);
    assert_eq!(smooth(10.0, 0.0), 9.0);
}

#[test]
fn an_impulse_comes_back_after_the_length() {
    let t = Tables::new();
    let c = ctx(&t, 120_000);
    let p = ProcessState::new();
    p.refill_delay_line();
    let mut d = Delay::new(p.clone());
    // steps 1, sixteenths: 26460000 / 4800 = 5512 samples; feedback 0; pan 64
    d.live = [1, 1, 1, 0, 64, 0];
    let gate = |p: &ProcessState| p.delay_line().gate;
    let mut out = Vec::new();
    for b in 0..100 {
        let mut send = [GUARD; BLOCK];
        if b == 0 {
            send[0] = 0.5;
        }
        let (mut l, mut r) = ([0.0f32; BLOCK], [0.0f32; BLOCK]);
        d.process(&mut send, &mut l, &mut r, &c);
        assert!(send.iter().all(|&x| x == GUARD));
        assert!(!gate(&p));
        out.extend(l.iter().zip(r.iter()).map(|(&a, &b)| (a, b)));
    }
    assert_eq!(d.length, 5512);
    assert_eq!(d.write, (100 * BLOCK) as u32);
    assert_eq!(d.counter, 100);
    // the line held 1e-6 everywhere, so the left output is 1e-6 x the gain until 5512, where
    // the 0.5 written at sample 0 comes back
    let pg = t.pan[127][64];
    assert_eq!(d.target, [(GUARD as f64 + pg[0] as f64) as f32; 2]);
    assert!(out[5511].0 < 1e-5 && out[5511].0 > 0.0);
    assert!(out[5512].0 > 0.4 && out[5512].1 > 0.4);
    assert!(out[5513].0 < 1e-5);
}

#[test]
fn a_disabled_delay_refills_the_line_once() {
    let t = Tables::new();
    let c = ctx(&t, 120_000);
    let p = ProcessState::new();
    let mut d = Delay::new(p.clone());
    d.live[ENABLED] = 0;
    let mut send = [0.5f32; BLOCK];
    let (mut l, mut r) = ([0.25f32; BLOCK], [0.25f32; BLOCK]);
    d.process(&mut send, &mut l, &mut r, &c);
    assert!(send.iter().all(|&x| x == GUARD));
    assert!(l.iter().chain(r.iter()).all(|&x| x == 0.25));
    let line = p.delay_line();
    assert!(line.gate && line.buf.iter().all(|&x| x == GUARD));
    // with the gate set, a later disabled block leaves the line alone
    let mut mark = p.delay_line();
    mark.buf[0] = 3.0;
    p.set_delay_line(mark);
    d.process(&mut send, &mut l, &mut r, &c);
    assert_eq!(p.delay_line().buf[0], 3.0);
    assert_eq!((d.counter, d.write, d.peak), (0, 0, GUARD));
}

#[test]
fn msg_86_goes_live_only_in_pattern_mode_and_slot_6_refills() {
    let p = ProcessState::new();
    let mut d = Delay::new(p.clone());
    d.data(&[0, 5, 0, 10, 20, 0], true);
    assert_eq!((d.pattern, d.live), ([0, 5, 0, 10, 20, 0], DEFAULTS));
    d.data(&[1, 6, 1, 11, 21, 0], false);
    assert_eq!(d.live, [1, 6, 1, 11, 21, 0]);
    d.set_param(4, 99);
    d.set_param(5, 1);
    assert_eq!(d.live, [1, 6, 1, 11, 99, 0]);
    d.reset();
    assert!(p.delay_line().gate);
}

#[test]
fn a_pan_above_127_is_clamped() {
    let t = Tables::new();
    let mut d = Delay::new(ProcessState::new());
    d.set_param(PAN as u8, 200);
    assert_eq!(d.live[PAN], 127);
    d.data(&[1, 3, 1, 0, 255, 0], false);
    assert_eq!(d.live[PAN], 127);
    let mut send = [GUARD; BLOCK];
    let (mut l, mut r) = ([0.0f32; BLOCK], [0.0f32; BLOCK]);
    d.process(&mut send, &mut l, &mut r, &ctx(&t, 120_000));
    let pg = t.pan[127][127];
    assert_eq!(d.target[1], (GUARD as f64 + pg[1] as f64) as f32);
}
