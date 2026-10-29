use super::*;
use crate::frame::BLOCK;
use crate::tables::Tables;

fn ctx(t: &Tables) -> BlockContext<'_> {
    BlockContext {
        tables: t,
        meter_index: 0,
        tempo: 120_000,
        tick: 0,
        stack_word: None,
    }
}

#[test]
fn constants_decode() {
    assert_eq!(ATTACK, 0.01f32);
    assert_eq!(RELEASE, 0.9984f32);
    assert_eq!(OFF_SCALE, 0.99);
    assert_eq!(C127, (1.0f64 / 127.0) as f32);
    assert_eq!(FLOOR_SCALE2, f64::from_bits(0x3F00_0000_0000_0000));
    assert_eq!(FLOOR_OFFSET, f64::from_bits(0x3F89_0000_0000_0000));
    assert_eq!(X_SCALE, f64::from_bits(0x3FD7_7000_0000_0000));
    assert_eq!(GUARD, 1e-6f32);
}

#[test]
fn the_update_parameters() {
    // amount 0: no gain factor, offset 1; threshold 0: the floor is the offset alone
    assert_eq!(params(0, 0), (0.0, 1.0, 25.0 / 2048.0));
    // threshold 32: floor = ((32 * f32(1/127)) * 80000) / 32768 + 0.01220703125
    let th = 32.0 * C127 as f64;
    assert_eq!(
        params(0, 32).2,
        (th * 80000.0 / 32768.0 + 0.01220703125) as f32
    );
    // amount 127, threshold 32: af = 127 * f32(1/127), X = af * 2 * 0.3662109375,
    // off = 1 - af * 0.99
    let af = 127.0 * C127 as f64;
    let (x, off, _) = params(127, 32);
    assert_eq!(x, (af * 2.0 * 0.3662109375) as f32);
    assert_eq!(off, (1.0 - af * 0.99) as f32);
}

#[test]
fn attack_hold_release_floor() {
    // attack: s > env moves env by (s - env)*0.01 and reloads the hold
    let mut hold = 0;
    assert_eq!(envelope(1.0, 2.0, &mut hold, 0.0), 1.0 + 0.01f32 as f64);
    assert_eq!(hold, 300);
    // hold: below env, the counter runs down and env stands
    let mut hold = 5;
    assert_eq!(envelope(1.0, 0.5, &mut hold, 0.0), 1.0);
    assert_eq!(hold, 4);
    // release once the hold is 0: env*0.9984f + 1e-6f
    let mut hold = 0;
    assert_eq!(
        envelope(1.0, 0.5, &mut hold, 0.0),
        RELEASE as f64 + GUARD as f64
    );
    assert_eq!(hold, 0);
    // equal is not above: no attack
    let mut hold = 0;
    assert_eq!(
        envelope(0.5, 0.5, &mut hold, 0.0),
        0.5 * RELEASE as f64 + GUARD as f64
    );
    // never below the floor
    let mut hold = 3;
    assert_eq!(envelope(0.001, 0.0, &mut hold, 0.0122), 0.0122f32 as f64);
}

#[test]
fn the_detector() {
    assert_eq!(detect(-2.0), 2.0);
    assert_eq!(detect(2.0), 2.0);
}

/// A compressor past its update (counter 1) with X 1, offset 0 and a floor of 4.
fn ramping() -> Compressor {
    let mut c = Compressor::new();
    c.counter = 1;
    c.x = 1.0;
    c.off = 0.0;
    c.floor = 4.0;
    c
}

#[test]
fn the_gain_ramps_linearly_over_a_group() {
    let t = Tables::new();
    let mut c = ramping();
    let mut bus = [1.0; BLOCK];
    c.process(&mut bus, &ctx(&t));
    // env attacks from 1 towards |1 + 1| = 2 and is floored at 4: the target is 1/4, the
    // step (1/4 - 0)/4; sample k of the first group gets the gain after k+1 steps
    assert_eq!(bus[..5], [0.0625, 0.125, 0.1875, 0.25, 0.25]);
    assert!(bus[4..].iter().all(|&x| x == 0.25));
    assert_eq!(c.gain, 0.25);
    assert_eq!(c.env, 4.0);
    // the first group attacked (hold 300); the other 15 found 2 below the floored 4
    assert_eq!(c.hold, 285);
    assert_eq!(c.counter, 2);
}

#[test]
fn stereo_is_linked() {
    let t = Tables::new();
    let mut c = ramping();
    let mut l = [1.0; BLOCK];
    let mut r = [0.0; BLOCK];
    c.process_stereo(&mut l, &mut r, &ctx(&t));
    // one detector, one gain for both buses
    assert_eq!(l[..4], [0.0625, 0.125, 0.1875, 0.25]);
    assert_eq!(r, [0.0; BLOCK]);
    // the detector sums samples 1 and 3 of both: |1 + 1 + 1 + 1| = 4 > env
    let mut c = ramping();
    c.floor = 0.0;
    let mut l = [1.0; BLOCK];
    let mut r = [1.0; BLOCK];
    c.process_stereo(&mut l, &mut r, &ctx(&t));
    assert_eq!(c.hold, 300);
    let mut env = 1.0f64;
    let mut hold = 0;
    for _ in 0..16 {
        env = envelope(env, 4.0, &mut hold, 0.0);
    }
    assert_eq!(c.env, env as f32);
}

#[test]
fn disabled_passes_and_stands() {
    let t = Tables::new();
    let mut c = Compressor::new();
    c.live[ENABLED] = 0;
    let mut bus = [0.5; BLOCK];
    c.process(&mut bus, &ctx(&t));
    assert_eq!(bus, [0.5; BLOCK]);
    assert_eq!(c.counter, 0);
}

#[test]
fn the_first_call_runs_the_update_and_fades_in() {
    let t = Tables::new();
    let mut c = Compressor::new();
    let mut bus = [0.0; BLOCK];
    c.process(&mut bus, &ctx(&t));
    assert_eq!((c.x, c.off, c.floor), params(70, 64));
    // the ctor's byte 3 (127) is the gain meter: G = 0 gives 0
    assert_eq!(c.live[GAIN_METER], 0);
    assert_eq!(c.counter, 1);
}

#[test]
fn the_meters() {
    let mut c = Compressor::new();
    c.gain = 0.5;
    c.peak = 0.5;
    c.param_update();
    assert_eq!(c.live[GAIN_METER], 9);
    assert_eq!(c.live[PEAK_METER], 63);
    c.gain = 7.5;
    c.peak = 2.0;
    c.param_update();
    assert_eq!(c.live[GAIN_METER], 127);
    assert_eq!(c.live[PEAK_METER], 0x7F);
    let t = Tables::new();
    c.update(&ctx(&t));
    assert_eq!(c.live[GAIN_METER..], [0, 0]);
}

#[test]
fn messages_and_controllers() {
    let mut c = Compressor::new();
    let block = [1, 100, 10, 127, 0];
    c.data(&block, true);
    assert_eq!(c.pattern, block);
    assert_eq!(c.live, DEFAULTS);
    c.data(&block, false);
    assert_eq!(c.live, block);
    c.set_param(THRESHOLD as u8, 5);
    c.set_param(GAIN_METER as u8, 9);
    assert_eq!(c.live, [1, 100, 5, 127, 0]);
}

#[test]
fn the_mono_block_leaves_the_release_factor() {
    let mut c = Compressor::new();
    assert_eq!(c.stack_word(Some(5)), Some(0x3F7F_9724));
    c.set_param(ENABLED as u8, 0);
    assert_eq!(c.stack_word(Some(5)), Some(5));
}
