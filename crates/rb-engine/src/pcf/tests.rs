use super::*;
use crate::tables::Tables;

fn ctx(t: &Tables, tick: i32) -> BlockContext<'_> {
    BlockContext {
        tables: t,
        meter_index: 0,
        tempo: 120_000,
        tick,
        stack_word: None,
    }
}

#[test]
fn constants_decode() {
    assert_eq!(FREQ_SCALE2, 1.0 / 128.0);
    assert_eq!(TOP, 511.0);
    assert_eq!(SLOW_KEEP, 0.995f32);
    // the slow take is 1 - 0.995f exactly, not f32(0.005)
    assert_eq!(SLOW_TAKE as f64, 1.0 - SLOW_KEEP as f64);
    assert_eq!(METER_DECAY, 0.92f32);
    assert_eq!(DEFAULTS, [1, 70, 64, 40, 24, 90, 1, 0]);
}

#[test]
fn the_pattern_data_and_past_it() {
    // pattern 0: 16 steps at 16 per bar, one trigger of 120 (even: instant) on step 0
    assert_eq!(pattern(0), (16, 16));
    assert_eq!(velocity(0, 0), 120);
    assert_eq!(velocity(0, 1), 0);
    // pattern 13: 32 steps; its odd velocities attack slowly
    assert_eq!(pattern(13), (32, 16));
    assert_eq!(velocity(13, 0), 1);
    // pattern 53: 8 steps per bar
    assert_eq!(pattern(53).1, 8);
    // past the 55 patterns: wave 64 reads a resolution of 0 and a length of 16, whose 16 steps
    // are all 0; wave 127 reads a length of 0
    assert_eq!(pattern(64), (16, 0));
    assert!((0..16).all(|s| velocity(64, s) == 0));
    assert_eq!(pattern(127), (0, 0));
    // the 17 waves of length 0, which fault: 55, 111 and 113..127 (112 reads a length of 120)
    let zero: Vec<u8> = (0..128).filter(|&w| pattern(w).0 == 0).collect();
    let want: Vec<u8> = [55, 111].into_iter().chain(113..128).collect();
    assert_eq!(zero, want);
    assert_eq!(zero.len(), 17);
    assert_eq!(pattern(55).0, 0);
    assert_eq!(pattern(111).0, 0);
    assert_eq!(pattern(112).0, 120);
    // 73 waves of 55..127 have a resolution other than 8, 16 or 32 (the stale quotient)
    let stale = (55..128)
        .filter(|&w| quotient(0, pattern(w).1).is_none())
        .count();
    assert_eq!(stale, 73);
    // and every wave 0..54 has a valid resolution and length
    assert!((0..55).all(|w| quotient(0, pattern(w).1).is_some() && pattern(w).0 > 0));
    // a wave above 127 is clamped
    assert_eq!(pattern(200), pattern(127));
    // a negative step reads no trigger
    assert_eq!(velocity(0, -1), 0);
}

#[test]
fn the_step_quotient() {
    assert_eq!(quotient(100, 32), Some(4));
    assert_eq!(quotient(100, 16), Some(2));
    assert_eq!(quotient(100, 8), Some(1));
    assert_eq!(quotient(100, 0), None);
    assert_eq!(quotient(100, 120), None);
    // `idiv` truncates toward zero
    assert_eq!(quotient(-30, 32), Some(-1));
}

#[test]
fn the_cutoff_smoother() {
    let k = Smoother {
        b2: 0.5,
        a: 1.5,
        g: 0.25,
    };
    // f = ((10*512)*(1/128))*0.25 = 10; (1.5*2 + 10) - 0.5*1 = 12.5
    assert_eq!(smooth_cutoff(&k, 10, 2.0, 1.0), 12.5);
    assert_eq!(smooth_cutoff(&k, 0, 0.0, 0.0), 0.0);
}

#[test]
fn the_trigger_level() {
    // (120*40) >> 5 = 150
    assert_eq!(trigger_level(120, 40, 0.0), 150.0);
    // 150 + 400 > 511: 511 - 400
    assert_eq!(trigger_level(120, 40, 400.0), 111.0);
    // exactly 511 stays
    assert_eq!(trigger_level(120, 40, 361.0), 150.0);
    // (1*127) >> 5 = 3
    assert_eq!(trigger_level(1, 127, 0.0), 3.0);
}

#[test]
fn the_envelope_index() {
    assert_eq!(env_index(-1.0), 0);
    assert_eq!(env_index(f64::NAN), 0);
    assert_eq!(env_index(3.9), 3);
    assert_eq!(env_index(511.0), 511);
    assert_eq!(env_index(600.0), 511);
}

#[test]
fn the_envelope_step() {
    let mut e = Envelope {
        e: 8.0,
        env: 1.0,
        keep: 0.5,
        take: 0.25,
    };
    // E = 8*0.5 = 4; env = 1*0.5 + (4 + 2)*0.25 = 2
    assert_eq!(e.step(0.5, 2.0), 2.0);
    assert_eq!(e.e, 4.0);
    // E = 2; env = 2*0.5 + (2 + 2)*0.25 = 2
    assert_eq!(e.step(0.5, 2.0), 2.0);
}

#[test]
fn the_filter_step() {
    let mut f = Filter { y1: 1.0, y2: 0.5 };
    // (1*2 + 0.5*1) - 0.25*0.5 = 2.375
    assert_eq!(f.step(1.0, &[0.5, 0.25, 2.0]), 2.375);
    assert_eq!(f, Filter { y1: 2.375, y2: 1.0 });
}

#[test]
fn low_pass_and_band_pass_outputs() {
    // c = (0, 0, 1): y = x, so the low-pass passes the input and the band-pass outputs its
    // difference from the previous sample
    let tab = [[0.0, 0.0, 1.0]; 512];
    let input: Block = std::array::from_fn(|i| i as f32);
    let env = Envelope {
        e: 0.0,
        env: 0.0,
        keep: 1.0,
        take: 0.0,
    };
    let mut lp = input;
    let (mut e, mut f) = (env, Filter { y1: 0.0, y2: 0.0 });
    run(&mut lp, &tab, false, &mut e, &mut f, 1.0, 0.0);
    assert_eq!(lp, input);
    assert_eq!(f, Filter { y1: 63.0, y2: 62.0 });
    let mut bp = input;
    let (mut e, mut f) = (env, Filter { y1: -5.0, y2: 0.0 });
    run(&mut bp, &tab, true, &mut e, &mut f, 1.0, 0.0);
    assert_eq!(bp[0], 5.0);
    assert!(bp[1..].iter().all(|&x| x == 1.0));
}

#[test]
fn disabled_passes_and_stands() {
    let t = Tables::new();
    let mut p = Pcf::new();
    p.live[ENABLED] = 0;
    let mut bus = [0.5; BLOCK];
    p.process(&mut bus, &ctx(&t, 0));
    assert_eq!(bus, [0.5; BLOCK]);
    assert_eq!(p.counter, 0);
    assert_eq!(p.faulted(), None);
}

#[test]
fn a_zero_length_faults_once_per_block() {
    let t = Tables::new();
    let mut p = Pcf::new();
    p.live[WAVE] = 55;
    let mut bus = [0.5; BLOCK];
    p.process(&mut bus, &ctx(&t, 0));
    assert_eq!(p.faulted(), Some(Fault::PcfZeroLength { wave: 55 }));
    let mut p = Pcf::new();
    p.live[WAVE] = 127;
    p.smooth = [3.0, 2.0];
    let mut bus = [0.5; BLOCK];
    p.process(&mut bus, &ctx(&t, 0));
    assert_eq!(p.faulted(), Some(Fault::PcfZeroLength { wave: 127 }));
    assert_eq!(p.faulted(), None);
    // the bus is untouched; the counter and `+0x84` were stored before the division
    assert_eq!(bus, [0.5; BLOCK]);
    assert_eq!(p.counter, 1);
    assert_eq!(p.smooth, [3.0, 3.0]);
    assert_eq!(p.base, 0.0);
}

#[test]
fn an_instant_trigger() {
    let t = Tables::new();
    let mut p = Pcf::new();
    // wave 0: step 0 at tick 0 has velocity 120 (even); amount 40 gives E = 150
    p.live[WAVE] = 0;
    p.live[AMOUNT] = 40;
    p.live[FREQUENCY] = 0;
    let mut bus = [0.0; BLOCK];
    p.process(&mut bus, &ctx(&t, 0));
    let dec = PCF_DECAY[(DEFAULTS[DECAY] >> 3) as usize];
    assert_eq!(p.step, 0);
    assert_eq!(p.decay, dec);
    assert_eq!(p.attack, 0.0);
    assert_eq!(p.base, 0.0);
    // keep 0, take 1: the envelope is E (decayed once per group) plus the base
    let mut e = 150.0f64;
    for _ in 0..16 {
        e *= dec as f64;
    }
    assert_eq!(p.trigger, e as f32);
    assert_eq!(p.env, e as f32);
    // a zero input stays zero through the filter
    assert_eq!(bus, [0.0; BLOCK]);
    assert_eq!(p.lp, [0.0; 2]);
    // the same step again does not retrigger: the slow factors of r = 0 keep E decaying
    let mut e2 = p.trigger as f64;
    p.process(&mut bus, &ctx(&t, 47));
    for _ in 0..16 {
        e2 *= dec as f64;
    }
    assert_eq!(p.trigger, e2 as f32);
}

#[test]
fn a_stale_quotient_is_the_channel_s_stack_word() {
    let t = Tables::new();
    // wave 30 (16 steps at 16 per bar) at tick 1000: step 20 % 16 = 4, velocity 65
    let mut p = Pcf::new();
    p.live[WAVE] = 30;
    let mut bus = [0.0; BLOCK];
    p.process(&mut bus, &ctx(&t, 1000));
    assert_eq!((p.quotient, p.step), (20, 4));
    // wave 64 (resolution 0) after the 909: the slot holds 0x1002B8B4, step 0x1002B8B4 % 16
    p.live[WAVE] = 64;
    let after_909 = BlockContext {
        stack_word: Some(0x1002_B8B4),
        ..ctx(&t, 1500)
    };
    p.process(&mut bus, &after_909);
    assert_eq!((p.quotient, p.step), (0x1002_B8B4, 4));
    // back to wave 30 at a tick of step 12 (velocity 58): a new step, which triggers
    p.live[WAVE] = 30;
    let e = p.trigger;
    p.process(&mut bus, &ctx(&t, 12 * 48));
    assert_eq!(p.step, 12);
    assert_ne!(p.trigger, e);
    // without a stack word (a 303 channel) the last quotient stands: 12 % 16
    p.live[WAVE] = 64;
    p.process(&mut bus, &ctx(&t, 3000));
    assert_eq!((p.quotient, p.step), (12, 12));
    // the 808's word
    p.process(
        &mut bus,
        &BlockContext {
            stack_word: Some(0x1002_8278),
            ..ctx(&t, 3000)
        },
    );
    assert_eq!(p.step, 0x1002_8278 % 16);
}

#[test]
fn the_meter_update_and_slot_16() {
    let mut p = Pcf::new();
    p.peak = 0.5;
    p.meter_update();
    assert_eq!(p.live[METER], 63);
    assert_eq!(p.peak, (0.5f64 * 0.92f32 as f64) as f32);
    p.peak = 2.0;
    p.meter_update();
    assert_eq!(p.live[METER], 0x7F);
    let t = Tables::new();
    p.update(&ctx(&t, 0));
    assert_eq!(p.live[METER], 0);
}

#[test]
fn messages_and_controllers() {
    let mut p = Pcf::new();
    let block = [1, 2, 3, 4, 5, 6, 0, 0];
    p.data(&block, true);
    assert_eq!(p.pattern, block);
    assert_eq!(p.live, DEFAULTS);
    p.data(&block, false);
    assert_eq!(p.live, block);
    p.set_param(MODE as u8, 1);
    p.set_param(METER as u8, 9);
    assert_eq!(p.live[MODE], 1);
    assert_eq!(p.live[METER], 0);
    p.set_song_mode(false);
    assert_eq!(p.live, block);
}
