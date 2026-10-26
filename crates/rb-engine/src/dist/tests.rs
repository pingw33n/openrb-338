use super::*;
use crate::frame::GUARD;

fn ctx(t: &Tables) -> BlockContext<'_> {
    BlockContext {
        tables: t,
        meter_index: 0,
        tempo: 120_000,
    }
}

#[test]
fn the_shaper_interpolates_and_clamps_to_w0_and_w254() {
    let mut w = [0.0f32; 256];
    for (i, x) in w.iter_mut().enumerate() {
        *x = i as f32;
    }
    w[3] = 1.0;
    w[4] = 3.0;
    // a linear row gives v back
    assert_eq!(shape(&w, 10.25), 10.25);
    // between W[3] = 1 and W[4] = 3: 0.5 * 2 + 1
    assert_eq!(shape(&w, 3.5), 2.0);
    // the ends: below 0 → W[0], above 254 → W[254] (not W[255])
    assert_eq!(shape(&w, -0.5), 0.0);
    assert_eq!(shape(&w, 254.0), 254.0);
    assert_eq!(shape(&w, 254.5), 254.0);
    assert_eq!(shape(&w, 1e9), 254.0);
}

#[test]
fn the_grid_interpolates_bilinearly() {
    // coordinate 0 on both axes: the corner itself
    assert_eq!(grid(0, 0), GRID[0][0] as f64);
    // coord(127) = 127 * f32(1/127) * 3.999 = 3.99899998..: row/column 3, fraction 0.999
    // shape 0, amount 127: 0.2 + 0.999 * (0.15 - 0.2) = 0.15005
    assert!((grid(0, 127) - 0.15005).abs() < 1e-6, "{}", grid(0, 127));
    // shape 127, amount 127: rows 3 and 4 at columns 3/4 are flat (0.1, 0.08):
    // 0.1 + 0.999 * (0.08 - 0.1) = 0.08002
    assert!(
        (grid(127, 127) - 0.08002).abs() < 1e-6,
        "{}",
        grid(127, 127)
    );
    // shape 64, amount 0: coord = 2.0152441, 0.2 + 0.0152441 * (0.15 - 0.2) = 0.1992378
    assert!((grid(64, 0) - 0.199_237_8).abs() < 1e-6, "{}", grid(64, 0));
}

#[test]
fn the_gains_smooth_by_0_9_and_0_1() {
    // 0 * 0.9 + 2 * 0.1 = 0.2, stored as f32
    assert_eq!(smooth(0.0, 2.0), 0.2f32);
    // a gain at its target stays there: 0.9 + 0.1 rounds to exactly 1
    assert_eq!(smooth(1.0, 1.0), 1.0);
    // 10 * 0.9 + 0 = 9
    assert_eq!(smooth(10.0, 0.0), 9.0);
}

#[test]
fn shape_0_is_a_hard_clip() {
    let t = Tables::new();
    let c = ctx(&t);
    let mut d = Distortion::new();
    d.live = [1, 0, 0, 0];
    // amount 0: T[0] = 0, so g = 1 and th = 1
    let mut bus = [0.0f32; BLOCK];
    bus[0] = 0.5;
    bus[1] = 3.0;
    bus[2] = -2.0;
    d.process(&mut bus, &c);
    assert!(d.clip);
    assert_eq!(&bus[..3], &[0.5, 1.0, -1.0]);
    // amount 127: g = 16 * T[127] + 1 = 17, th = 1/(16 * T[90] + 1)
    d.live[AMOUNT] = 127;
    d.counter = 0;
    let th = 1.0 / (16.0 * t.level[90] as f64 + 1.0);
    let mut bus = [0.0f32; BLOCK];
    bus[0] = 0.001;
    bus[1] = 1.0;
    bus[2] = -1.0;
    d.process(&mut bus, &c);
    assert_eq!(bus[0], (17.0 * 0.001f32 as f64) as f32);
    assert_eq!(bus[1], th as f32);
    assert_eq!(bus[2], -th as f32);
}

#[test]
fn the_shaper_path_reads_the_rounded_gains() {
    let t = Tables::new();
    let c = ctx(&t);
    let mut d = Distortion::new();
    d.live = [1, 100, 64, 0];
    let mut bus = [GUARD; BLOCK];
    d.process(&mut bus, &c);
    assert!(!d.clip);
    assert_eq!(d.row, 16);
    assert_eq!(d.counter, 1);
    // the first call's gains are 0.1 x their targets
    assert_eq!(d.gain_in, smooth(0.0, d.target_in));
    assert_eq!(
        d.target_in,
        ((t.level[100] as f64 + 0.0625) * 2048.0) as f32
    );
    assert_eq!(d.target_out, (grid(64, 100) * 5.0 / 256.0) as f32);
    let v = d.gain_in as f64 * GUARD as f64 + 128.0;
    let y = shape(&t.dist_shaper[16], v) * d.gain_out as f64;
    assert!(bus.iter().all(|&x| x == y as f32));
}

#[test]
fn a_disabled_distortion_leaves_the_bus_and_its_counter() {
    let t = Tables::new();
    let mut d = Distortion::new();
    d.live[ENABLED] = 0;
    let mut bus = [0.25f32; BLOCK];
    d.process(&mut bus, &ctx(&t));
    assert!(bus.iter().all(|&x| x == 0.25));
    assert_eq!(d.counter, 0);
}

#[test]
fn msg_94_goes_live_only_in_pattern_mode() {
    let mut d = Distortion::new();
    d.data(&[1, 20, 30, 0], true);
    assert_eq!((d.pattern, d.live), ([1, 20, 30, 0], DEFAULTS));
    d.data(&[0, 40, 50, 0], false);
    assert_eq!(d.live, [0, 40, 50, 0]);
    d.set_param(2, 7);
    d.set_param(3, 9);
    assert_eq!(d.live, [0, 40, 7, 0]);
}

#[test]
fn amount_and_shape_above_127_are_clamped() {
    let t = Tables::new();
    for v in [128u8, 255] {
        let mut d = Distortion::new();
        d.set_param(AMOUNT as u8, v);
        d.set_param(SHAPE as u8, v);
        assert_eq!(d.live, [1, 127, 127, 0]);
        d.data(&[1, v, v, 0], false);
        assert_eq!((d.pattern, d.live), ([1, 127, 127, 0], [1, 127, 127, 0]));
        // the update and the block index the grid, T and W without a panic
        let mut bus = [0.5f32; BLOCK];
        d.process(&mut bus, &ctx(&t));
        assert_eq!(d.row, 31);
        assert_eq!(d.target_out, (grid(127, 127) * 5.0 / 256.0) as f32);
    }
}
