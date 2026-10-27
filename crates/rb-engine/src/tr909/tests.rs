use super::*;
use crate::process::DrumCounters;
use crate::samples::{COUNT, FIRST_ID, Retired};

/// A sample set whose resource `id` holds `id - 900` frames of the value `id - 1000` (the 909
/// kit's slot number), so every voice's sample can be told apart.
fn set() -> Arc<SampleSet> {
    Arc::new(SampleSet::from_samples(
        (0..COUNT)
            .map(|i| {
                let id = FIRST_ID as usize + i;
                Sample::from_audio(&vec![id as i16 - 1000; id - 900])
            })
            .collect(),
    ))
}

/// The slot a voice's sample came from.
fn slot_of(s: &Option<Arc<Sample>>) -> i16 {
    s.as_ref().unwrap().audio()[0]
}

/// A msg 73 payload (RBS order) with pattern 0 = `steps` (length 16, no shuffle).
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

/// The defaults in RBS order (all three permuted bytes are 100, so the order does not matter).
fn rbs_defaults() -> Params {
    DEFAULT_PARAMS
}

fn device_with(process: ProcessState, params: Params, steps: &[[u8; 12]]) -> Tr909 {
    let mut d = Tr909::new(set(), process);
    d.data(&payload(params, steps), false);
    d.relocate(0, 0);
    d.start(0);
    d
}

fn device(params: Params, steps: &[[u8; 12]]) -> Tr909 {
    device_with(ProcessState::new(), params, steps)
}

/// A step with column `c` at value `v` for each `(c, v)`.
fn row(cols: &[(usize, u8)]) -> [u8; 12] {
    let mut r = [0; 12];
    for &(c, v) in cols {
        r[c] = v;
    }
    r
}

#[test]
fn ctor_state() {
    let d = Tr909::new(set(), ProcessState::new());
    assert_eq!((d.live, d.params), (DEFAULT_PARAMS, DEFAULT_PARAMS));
    assert_eq!(
        (d.cur_pattern, d.enabled, d.wrapped, d.last_pos),
        (0, 1, false, 0)
    );
    assert!(!d.flam_pending);
    let p = d.pattern(31);
    assert_eq!((p[0], p[1], p[2], p[193]), (0, 16, 0, 0));
    // T909[64] = 256 (unit speed), DT[15] = 1
    assert_eq!(d.tom_step[64], 256);
    assert_eq!(d.decay[15], 1.0);
}

#[test]
fn msg_73_permutes_bytes_20_to_22_and_set_param_follows() {
    let mut p = rbs_defaults();
    // RBS order: 20 rim shot, 21 clap, 22 hi-hat
    p[20] = 11;
    p[21] = 22;
    p[22] = 33;
    p[FLAM] = 5;
    let mut d = device(p, &[]);
    assert_eq!(
        (d.live[RS_LEVEL], d.live[CP_LEVEL], d.live[HH_LEVEL]),
        (11, 22, 33)
    );
    assert_eq!(d.params_rbs_order()[20..23], [11, 22, 33]);
    // the controllers are in RBS order too, 29 included
    d.set_param(20, 1);
    d.set_param(21, 2);
    d.set_param(22, 3);
    d.set_param(29, 4);
    d.set_param(30, 9);
    assert_eq!(
        (
            d.live[RS_LEVEL],
            d.live[CP_LEVEL],
            d.live[HH_LEVEL],
            d.live[FLAM]
        ),
        (1, 2, 3, 4)
    );
    assert_eq!(d.live[LAST_STEP], 0);
    // slot 8: the tick-0 events of controllers 0..29, in RBS order
    let mut t = Track::new(crate::song::Device::Tr909);
    d.init_track(&mut t);
    let ev = t.events();
    assert_eq!(ev.len(), 30);
    let v = |c: u8| ev.iter().find(|e| e.ctrl == c).unwrap().value;
    assert_eq!((v(20), v(21), v(22), v(29)), (11, 22, 33, 5));
}

#[test]
fn step_values_and_the_accent() {
    let mut p = rbs_defaults();
    p[ACCENT] = 127;
    // value 1, 2 and 3 on three SIMPLE instruments; 4 never comes from a file; 5 plays nothing
    let mut d = device(p, &[row(&[(AC, 2), (RS, 1), (CP, 2), (CH, 3), (OH, 5)])]);
    d.step(0, 0, 120_000);
    // f32(2 * 127 * 0.03125 + 1)
    assert_eq!(d.accent, 8.9375);
    // the last instrument triggered (descending) is RS at value 1: f32(8.9375 * 0.8)
    assert_eq!(d.gain, 7.15f32);
    let l = 100.0f64;
    let g = |c: f32, gain: f32| {
        let x = ((((LEVEL_SCALE as f64 * l) * l) * c as f64) * gain as f64) as f32;
        (x as f64 / 32768.0) as f32
    };
    assert_eq!(d.simple[SIMPLE_RS].v.g, g(C_RS_OH, 7.15));
    // value 2: 8.9375 * 2
    assert_eq!(d.simple[SIMPLE_CP].v.g, g(C_CP, 17.875));
    // value 3 plays at the value-1 gain
    assert_eq!(d.simple[SIMPLE_CH].v.g, g(C_CH, 7.15));
    assert!(!d.simple[SIMPLE_OH].v.active);
    assert_eq!(slot_of(&d.simple[SIMPLE_RS].v.sample), slot::RS as i16);
    assert_eq!(slot_of(&d.simple[SIMPLE_CP].v.sample), slot::CP as i16);
}

#[test]
fn descending_order_lets_the_closed_hihat_choke_the_open_one() {
    let mut p = rbs_defaults();
    p[23] = 127; // closed hi-hat decay: DT[15] = 1, so its m = 1^8
    p[24] = 0; // open hi-hat decay: DT[0]
    let mut d = device(p, &[row(&[(CH, 1), (OH, 1)])]);
    d.step(0, 0, 120_000);
    // OH (column 9) triggers first with m = DT[0]; CH (8) then sets it to 0.996f
    assert!(d.simple[SIMPLE_OH].v.active);
    assert_eq!(d.simple[SIMPLE_OH].v.m.to_bits(), 0x3F7E_F9DB);
    assert_eq!(d.simple[SIMPLE_CH].v.m, 1.0);
    // OH alone keeps DT[0]: 1 - 15 * 2.5e-5 accumulated in f64
    let mut d = device(p, &[row(&[(OH, 1)])]);
    d.step(0, 0, 120_000);
    assert_eq!(d.simple[SIMPLE_OH].v.m, d.decay[0]);
    assert!((d.decay[0] as f64 - (1.0 - 15.0 * 2.5e-5)).abs() < 1e-7);
}

#[test]
fn closed_hihat_decay_is_dt_to_the_eighth() {
    let mut p = rbs_defaults();
    p[23] = 64; // DT[64 >> 3] = DT[8]
    let mut d = device(p, &[row(&[(CH, 1)])]);
    d.step(0, 0, 120_000);
    let x = d.decay[8] as f64;
    assert_eq!(
        d.simple[SIMPLE_CH].v.m,
        ((x * x) * (x * x) * ((x * x) * (x * x))) as f32
    );
    // DT[8] = 1 - 7 * 2.5e-5, so m is about 1 - 56 * 2.5e-5
    assert!((d.simple[SIMPLE_CH].v.m as f64 - (1.0 - 56.0 * 2.5e-5)).abs() < 1e-6);
}

#[test]
fn flam_retriggers_values_above_2_once_past_the_flam_tick() {
    let mut p = rbs_defaults();
    p[FLAM] = 64;
    let ps = ProcessState::new();
    // SD flam, BD plain on step 0
    let mut d = device_with(ps.clone(), p, &[row(&[(BD, 1), (SD, 3)])]);
    d.step(0, 0, 120_000);
    // 0 + (64 * 120000) / 700000 + 1 = 10 + 1
    assert_eq!(d.flam_tick, 11);
    assert!(d.flam_pending);
    // accent 1: SD at 0.8, BD at 0.8
    assert_eq!(d.gain, 0.8);
    let c = ps.drum_counters();
    assert_eq!((c.snare_909, c.bd_909), (1, 1));
    // not past the flam tick yet
    for t in [1, 11] {
        d.step(t, t, 120_000);
        assert_eq!(ps.drum_counters().snare_909, 1);
    }
    // tick 12 > 11: SD alone retriggers at 1.5
    d.step(12, 12, 120_000);
    assert!(!d.flam_pending);
    assert_eq!(d.gain, 1.5);
    let c = ps.drum_counters();
    assert_eq!((c.snare_909, c.bd_909), (2, 1));
    // once only
    d.step(13, 13, 120_000);
    assert_eq!(ps.drum_counters().snare_909, 2);
    // the next step arms it again
    d.step(48, 48, 120_000);
    assert!(d.flam_pending);
    assert_eq!(d.flam_tick, 59);
}

#[test]
fn flam_interval_0_retriggers_two_ticks_later() {
    let mut p = rbs_defaults();
    p[FLAM] = 0;
    let ps = ProcessState::new();
    // step 2 (tick 96)
    let mut d = device_with(ps.clone(), p, &[[0; 12], [0; 12], row(&[(SD, 3)])]);
    d.step(96, 96, 140_000);
    assert_eq!(d.flam_tick, 97);
    d.step(97, 97, 140_000);
    assert_eq!(ps.drum_counters().snare_909, 1);
    d.step(98, 98, 140_000);
    assert_eq!(ps.drum_counters().snare_909, 2);
}

#[test]
fn bass_drum_alternates_its_voices_and_picks_the_sample_by_tune() {
    let mut p = rbs_defaults();
    p[BD_TUNE] = 127;
    p[BD_ATTACK] = 0;
    let ps = ProcessState::new();
    let mut d = device_with(ps.clone(), p, &[row(&[(BD, 1)]), row(&[(BD, 1)])]);
    d.step(0, 0, 120_000);
    // flag 0: voice A plays, B fades with 0.99, the flag becomes 1
    assert!(d.filter[FILTER_BD_A].v.active && !d.filter[FILTER_BD_B].v.active);
    assert_eq!(d.filter[FILTER_BD_B].v.m.to_bits(), 0x3F7D_70A4);
    assert_eq!(ps.drum_counters().bd_909, 1);
    // tune 127 >> 5 = 3: bd4; attack 0: c = C8[0] = 0.92
    assert_eq!(slot_of(&d.filter[FILTER_BD_A].v.sample), 28);
    assert_eq!(d.filter[FILTER_BD_A].c, 0.92);
    d.step(48, 48, 120_000);
    assert!(d.filter[FILTER_BD_B].v.active);
    assert_eq!(d.filter[FILTER_BD_A].v.m.to_bits(), 0x3F7D_70A4);
    assert_eq!(ps.drum_counters().bd_909, 0);
}

#[test]
fn snare_follows_seq909_from_the_global_counter() {
    let mut p = rbs_defaults();
    p[SD_TUNE] = 64; // (64*5) >> 7 = 2: SD909[2] = 6, slot 31 (sd3)
    p[SD_TONE] = 10;
    p[SD_SNAPPY] = 100;
    let ps = ProcessState::new();
    ps.set_drum_counters(DrumCounters {
        snare_909: 31,
        ..Default::default()
    });
    let mut d = device_with(ps.clone(), p, &[row(&[(SD, 1)]), row(&[(SD, 1)])]);
    d.step(0, 0, 120_000);
    // 31 + 1 wraps to 0: SEQ909[0] = 0, SDN[0] = 9, slot 34 (sdn1)
    assert_eq!(ps.drum_counters().snare_909, 0);
    assert_eq!(slot_of(&d.dual.v.sample), 31);
    assert_eq!(slot_of(&d.dual.b), 34);
    // hold tone + 30, m 0.998, g2 = f32(100 * 0.01f) = 1
    assert_eq!(d.dual.v.hold, 40);
    assert_eq!(d.dual.v.m.to_bits(), 0x3F7F_7CEE);
    assert_eq!(d.dual.g2, 1.0);
    d.step(48, 48, 120_000);
    // SEQ909[1] = 1: SDN[1] = 10, slot 35 (sdn2)
    assert_eq!(slot_of(&d.dual.b), 35);
}

#[test]
fn toms_and_cymbals_use_their_kv_and_t909() {
    let mut p = rbs_defaults();
    p[LT_LEVEL] = 64;
    p[LT_LEVEL + 1] = 64; // tune 64: step 256
    p[LT_LEVEL + 2] = 127; // DT[15] = 1
    p[CC_LEVEL] = 64;
    let mut d = device(p, &[row(&[(LT, 2), (CC, 2)])]);
    d.step(0, 0, 120_000);
    // gain 2.0: ((64 * (64 * 6.2e-5f)) * kv) * 2
    let g = |kv: f32| {
        let x = (((64.0 * (64.0 * LEVEL_SCALE as f64)) * kv as f64) * 2.0) as f32;
        (x as f64 / 32768.0) as f32
    };
    assert_eq!(d.pitched[0].v.g, g(KV_LT));
    assert_eq!(d.pitched[3].v.g, g(KV_CY));
    assert_eq!((d.pitched[0].step, d.pitched[0].v.m), (256, 1.0));
    assert_eq!(slot_of(&d.pitched[0].v.sample), slot::LT as i16);
    assert_eq!(slot_of(&d.pitched[3].v.sample), slot::CC as i16);
}

/// The ctor's tick-0 events are the defaults of `sub_100195A0` as read in the disassembly, in
/// RBS order (`docs/song_load.md` section 4.9, task-034), controllers 0..29.
#[test]
fn init_track_holds_the_ctor_defaults() {
    let mut d = Tr909::new(set(), ProcessState::new());
    let mut t = Track::new(crate::song::Device::Tr909);
    d.init_track(&mut t);
    assert!(t.events().iter().all(|e| e.tick == 0));
    let mut vals = [0xFFu8; 30];
    for e in t.events() {
        vals[e.ctrl as usize] = e.value;
    }
    assert_eq!(
        vals,
        [
            1, 0, 32, 100, 64, 64, 127, 100, 64, 64, 64, 100, 64, 127, 100, 64, 127, 100, 64, 127,
            100, 100, 100, 32, 127, 100, 64, 100, 64, 64,
        ]
    );
}

/// Song msg 19 on the 909 (`sub_1002B8C0`): slot 6, then the table; the voices' samples and
/// the old table go to `Retired`; the next trigger takes its sample from the new table.
#[test]
fn set_samples_silences_then_replaces_the_table() {
    let mut p = rbs_defaults();
    p[RS_LEVEL] = 100;
    p[CP_LEVEL] = 100;
    let mut d = device(p, &[row(&[(RS, 1), (CP, 1)]), row(&[(RS, 1)])]);
    d.step(0, 0, 0);
    assert_eq!(d.simple.iter().filter(|v| v.v.active).count(), 2);
    let old = d.samples.clone();
    // resource 1000 + n holds -(n + 1): minus the 909 slot number, minus 1
    let new = Arc::new(SampleSet::from_samples(
        (0..COUNT)
            .map(|i| Sample::from_audio(&vec![-(i as i16) - 1; 300]))
            .collect(),
    ));
    let mut r = Retired::default();
    d.set_samples(&new, &mut r);
    assert!(!d.simple.iter().any(|v| v.v.active));
    assert!(Arc::ptr_eq(&d.samples, &new));
    assert!(Arc::ptr_eq(r.sets().next().unwrap(), &old));
    assert_eq!(r.samples().count(), 2);
    d.step(48, 48, 0);
    let rs = d.simple.iter().find(|v| v.v.active).unwrap();
    assert_eq!(slot_of(&rs.v.sample), -(slot::RS as i16) - 1);
}
