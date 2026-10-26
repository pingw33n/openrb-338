use super::*;
use crate::frame::BLOCK;
use crate::samples::{COUNT, FIRST_ID};
use crate::song::Device;

/// A sample set whose resource `id` holds `id - 900` frames of the value `id - 999` (the
/// built-in kit's slot number), so every voice's sample can be told apart.
fn set() -> Arc<SampleSet> {
    Arc::new(SampleSet::from_samples(
        (0..COUNT)
            .map(|i| {
                let id = FIRST_ID as usize + i;
                Sample::from_audio(&vec![(id - 999) as i16; id - 900])
            })
            .collect(),
    ))
}

/// The slot a voice's sample came from.
fn slot_of(s: &Option<Arc<Sample>>) -> i16 {
    s.as_ref().unwrap().audio()[0]
}

/// A msg 56 payload with pattern 0 = `steps` (length 16, no shuffle), the rest empty.
fn payload(params: Params, steps: &[[u8; 12]]) -> Vec<u8> {
    let mut b = params.to_vec();
    for p in 0..PATTERNS {
        b.extend([0, 16]);
        for s in 0..16 {
            let st = if p == 0 { steps.get(s).copied() } else { None };
            b.extend(st.unwrap_or([0; 12]));
        }
    }
    b
}

fn device(params: Params, steps: &[[u8; 12]]) -> Tr808 {
    let mut d = Tr808::new(set(), ProcessState::new());
    d.data(&payload(params, steps), false);
    d.relocate(0, 0);
    d.start(0);
    d
}

fn row(cols: &[usize]) -> [u8; 12] {
    let mut r = [0; 12];
    for &c in cols {
        r[c] = 1;
    }
    r
}

const K64: f64 = K as f64;

#[test]
fn ctor_state() {
    let d = Tr808::new(set(), ProcessState::new());
    assert_eq!((d.live, d.params), (DEFAULT_PARAMS, DEFAULT_PARAMS));
    assert_eq!(
        (d.cur_pattern, d.enabled, d.wrapped, d.last_pos),
        (0, 1, false, 0)
    );
    let p = d.pattern(31);
    assert_eq!((p[0], p[1], p[2], p[193]), (0, 16, 0, 0));
    assert!(!d.simple[0].v.active && !d.dual.v.active);
    // T808[64] = 256 (unit speed)
    assert_eq!(d.tom_step[64], 256);
}

#[test]
fn accent_and_the_simple_instruments() {
    let mut p = DEFAULT_PARAMS;
    p[ACCENT] = 127;
    p[RS_LEVEL] = 100;
    p[CB_LEVEL] = 50;
    let mut d = device(p, &[row(&[AC, RS, CP, CB]), row(&[RS])]);
    d.step(0, 0, 0);
    // f32(1 * 127 * 0.03125 + 1)
    assert_eq!(d.accent, 4.96875);
    let lev = |l: f64| ((((K64 * l) * l) * LEVEL_SCALE as f64) * 4.96875) as f32;
    // the trigger folds in 2^-15
    let g = |l: f64| (lev(l) as f64 / 32768.0) as f32;
    assert_eq!(
        (d.simple[0].v.g, d.simple[1].v.g, d.simple[2].v.g),
        (g(100.0), g(100.0), g(50.0))
    );
    // the selectors are 0: CL and MA
    assert_eq!(slot_of(&d.simple[0].v.sample), slot::CL as i16);
    assert_eq!(slot_of(&d.simple[1].v.sample), slot::MA as i16);
    assert_eq!(slot_of(&d.simple[2].v.sample), slot::CB as i16);
    assert_eq!((d.simple[0].v.m, d.simple[0].v.hold), (1.0, 0));
    // the same step again does not retrigger; the next one does, without the accent
    d.simple[0].v.g = 0.0;
    d.step(24, 24, 0);
    assert_eq!(d.simple[0].v.g, 0.0);
    d.live[RS_SELECT] = 1;
    d.step(48, 48, 0);
    assert_eq!(d.accent, 1.0);
    assert_eq!(slot_of(&d.simple[0].v.sample), slot::RS as i16);
}

#[test]
fn open_hihat_holds_and_the_closed_hihat_chokes_it() {
    let mut p = DEFAULT_PARAMS;
    p[OH_LEVEL] = 101;
    p[OH_DECAY] = 10;
    p[CH_LEVEL] = 100;
    let mut d = device(p, &[row(&[OH]), row(&[OH, CH])]);
    d.step(0, 0, 0);
    let oh = &d.simple[3].v;
    assert_eq!(
        (oh.hold, oh.m, slot_of(&oh.sample)),
        (40, OH_DECAY_M, slot::OH as i16)
    );
    // level 101 >> 1 = 50
    let g = (((K64 * 50.0) * 50.0) * LEVEL_SCALE as f64) as f32;
    assert_eq!(oh.g, (g as f64 / 32768.0) as f32);
    // OH then CH on one step: CH clears every SIMPLE voice's hold, OH's included
    d.step(48, 48, 0);
    assert_eq!((d.simple[3].v.hold, d.simple[3].v.active), (0, true));
    let ch = &d.simple[4].v;
    // (2 * 100) / 3 = 66
    let g = (((K64 * 66.0) * 66.0) * LEVEL_SCALE as f64) as f32;
    assert_eq!(
        (ch.g, ch.m, slot_of(&ch.sample)),
        ((g as f64 / 32768.0) as f32, 1.0, 12)
    );
}

#[test]
fn bass_drum_alternates_voices_through_the_global_flag() {
    let mut p = DEFAULT_PARAMS;
    p[BD_LEVEL] = 100;
    p[BD_TONE] = 64;
    p[BD_DECAY] = 64;
    let mut d = device(p, &[row(&[BD]), row(&[BD]), row(&[BD])]);
    let pr = d.process.clone();
    d.step(0, 0, 0);
    let c = TR808_TONE[4];
    let x = ((K64 * 100.0) * 100.0) * LEVEL_SCALE as f64;
    let g = ((1.0 - c as f64) * x) as f32;
    let a = &d.filter[0];
    assert_eq!((a.v.active, a.c, a.v.m), (true, c, TR808_DECAY[8]));
    assert_eq!(a.v.g, (g as f64 / 32768.0) as f32);
    assert_eq!((d.filter[1].v.m, pr.bd_808()), (BD_FADE, 1));
    d.step(48, 48, 0);
    assert_eq!(
        (d.filter[1].v.active, d.filter[0].v.m, pr.bd_808()),
        (true, BD_FADE, 0)
    );
    // a preset flag picks voice B first
    let mut d = device(p, &[row(&[BD])]);
    d.process.set_bd_808(1);
    d.step(0, 0, 0);
    assert!(d.filter[1].v.active && !d.filter[0].v.active);
}

#[test]
fn snare_noise_follows_seq808_at_the_global_counter() {
    let mut p = DEFAULT_PARAMS;
    p[SD_LEVEL] = 127;
    p[SD_TONE] = 127;
    p[SD_SNAPPY] = 64;
    let steps: Vec<[u8; 12]> = (0..16).map(|_| row(&[SD])).collect();
    let mut d = device(p, &steps);
    let mut noise = Vec::new();
    for s in 0..5 {
        d.step(48 * s, 48 * s, 0);
        noise.push(slot_of(&d.dual.b));
    }
    // counter 1..5: SEQ808 = 1, 2, 3, 1, 0 -> noise slots 8, 9, 10, 8, 7
    assert_eq!(noise, [8, 9, 10, 8, 7]);
    assert_eq!(d.process.drum_counters().snare_808, 5);
    // tone 127: (127*5) >> 7 = 4 -> SD5 (slot 6)
    assert_eq!(slot_of(&d.dual.v.sample), 6);
    let g1 = (((K64 * 127.0) * 127.0) * SD_SCALE as f64) as f32;
    let sk = 64.0 * K64;
    assert_eq!(
        (d.dual.v.g, d.dual.g2),
        ((g1 as f64 / 32768.0) as f32, (sk * sk) as f32)
    );
}

#[test]
fn toms_pick_the_tom_or_the_conga_and_the_tune_step() {
    let mut p = DEFAULT_PARAMS;
    p[LT_LEVEL + 1] = 0;
    p[MT_LEVEL + 2] = 1;
    p[HT_LEVEL] = 80;
    p[HT_LEVEL + 1] = 127;
    let mut d = device(p, &[row(&[LT, MT, HT])]);
    d.step(0, 0, 0);
    let t = crate::tables::tr808_tom_step();
    let slots: Vec<i16> = d.pitched.iter().map(|v| slot_of(&v.v.sample)).collect();
    assert_eq!(slots, [slot::LC as i16, slot::MT as i16, slot::HC as i16]);
    assert_eq!(
        (d.pitched[0].step, d.pitched[2].step),
        (t[0] as u32, t[127] as u32)
    );
    let g = (((K64 * 80.0) * 80.0) * K64) as f32;
    assert_eq!(d.pitched[2].v.g, (g as f64 / 32768.0) as f32);
}

#[test]
fn disabled_or_pattern_change_at_the_bar_line() {
    let mut d = device(DEFAULT_PARAMS, &[row(&[CB])]);
    d.live[ENABLED] = 0;
    // the enable is latched only at the bar line
    d.step(0, 0, 0);
    assert!(!d.simple[2].v.active);
    d.live[ENABLED] = 1;
    d.live[PATTERN] = 1;
    // step 15, then the wrap to step 0 at the next bar line
    d.step(720, 720, 0);
    assert_eq!(d.cur_pattern, 0);
    d.step(768, 768, 0);
    assert!(!d.wrapped);
    assert_eq!((d.cur_pattern, d.pat_start, d.enabled), (1, 768, 1));
    // pattern 1 is empty
    assert!(!d.simple[2].v.active);
}

#[test]
fn audio_zeroes_the_bus_and_sums_in_order() {
    let mut p = DEFAULT_PARAMS;
    p[CB_LEVEL] = 127;
    let mut d = device(p, &[row(&[CB])]);
    let t = crate::tables::Tables::new();
    let ctx = BlockContext {
        tables: &t,
        meter_index: 0,
        tempo: 120_000,
    };
    let mut bus = [5.0; BLOCK];
    d.process(&mut bus, &ctx);
    assert_eq!(bus, [0.0; BLOCK]);
    d.step(0, 0, 0);
    d.process(&mut bus, &ctx);
    let g = d.simple[2].v.g;
    assert_eq!(bus[0], (g as f64 * slot::CB as f64) as f32);
    d.silence();
    d.process(&mut bus, &ctx);
    assert_eq!(bus, [0.0; BLOCK]);
}

#[test]
fn init_track_inserts_the_29_controllers() {
    let d = Tr808::new(set(), ProcessState::new());
    let mut d = d;
    let mut tr = Track::new(Device::Tr808);
    d.init_track(&mut tr);
    assert_eq!(tr.events().len(), 29);
    assert!(tr.events().iter().all(|e| e.tick == 0));
    let mut vals = [0u8; 29];
    for e in tr.events() {
        vals[e.ctrl as usize] = e.value;
    }
    assert_eq!(vals[..], DEFAULT_PARAMS[..29]);
    // controllers 0 and 1 are quantised to a bar
    assert!(tr.insert(24, 1, 5));
    assert!(tr.events().iter().any(|e| e.tick == 768 && e.ctrl == 1));
}

#[test]
fn song_mode_and_data_copy_all_32_bytes() {
    let mut d = Tr808::new(set(), ProcessState::new());
    let mut p = DEFAULT_PARAMS;
    p[29] = 7;
    d.data(&payload(p, &[]), true);
    // song mode: the block stays in the pattern-mode copy
    assert_eq!((d.params[29], d.live[29]), (7, 0));
    d.set_song_mode(false);
    assert_eq!(d.live, p);
    d.live[CH_LEVEL] = 1;
    d.set_song_mode(true);
    assert_eq!(d.params[CH_LEVEL], 1);
    d.set_param(28, 9);
    d.set_param(29, 9);
    assert_eq!((d.live[28], d.live[29]), (9, 7));
}
