use super::*;
use crate::frame::BLOCK;
use crate::song::Device;

/// A pattern payload (msg 39) with pattern 0 = `steps` (len 16, no shuffle) and pattern 1 all
/// rests of length 16.
fn payload(params: Params, steps: &[[u8; 6]]) -> Vec<u8> {
    let mut b = params.to_vec();
    for p in 0..PATTERNS {
        b.extend([0, 16]);
        for s in 0..16 {
            let st = if p == 0 { steps.get(s).copied() } else { None };
            b.extend(st.unwrap_or([0, 0, 0, 0, 0, 1]));
        }
    }
    b
}

const NOTE: [u8; 6] = [0, 0, 0, 0, 0, 0];
const REST: [u8; 6] = [0, 0, 0, 0, 0, 1];
const SLIDE: [u8; 6] = [0, 1, 0, 0, 0, 0];

fn device(steps: &[[u8; 6]]) -> Tb303 {
    let mut d = Tb303::new();
    d.data(&payload(DEFAULT_PARAMS, steps), false);
    d.relocate(0, 0);
    d
}

#[test]
fn ctor_state() {
    let d = Tb303::new();
    assert_eq!(d.live, DEFAULT_PARAMS);
    assert_eq!(d.params, DEFAULT_PARAMS);
    assert_eq!((d.cs1, d.cs2), (0.5, 0.5));
    assert_eq!((d.gate_off, d.enabled, d.cur_pattern), (-1, 1, 0));
    assert_eq!((d.amp_in, d.amp_fb, d.amp, d.env), (0.0, 0.0, 0.0, 0.0));
    let p = d.pattern(31);
    assert_eq!((p[0], p[1], p[2 + 6 * 15 + 5]), (0, 16, 1));
}

#[test]
fn pitch_of_a_note() {
    let mut d = Tb303::new();
    // tune 64 is table entry 16 = 0 semitones: index 3072, times 16
    d.note_on([0, 0, 0, 0, 0, 0]);
    assert_eq!((d.pitch_target, d.pitch), (49152, 49152));
    // note 3, octave up, tune 0 (-12): (3 + 12 - 12)*64 + 3072
    d.live[TUNE] = 0;
    d.note_on([3, 0, 0, 1, 0, 0]);
    assert_eq!(d.pitch_target, ((3 * 64 + 3072) * 16) as u32);
    // up and down cancel; tune 127 is +12
    d.live[TUNE] = 127;
    d.note_on([12, 0, 0, 1, 1, 0]);
    assert_eq!(d.pitch_target, (((12 + 12) * 64 + 3072) * 16) as u32);
}

#[test]
fn note_on_coefficients_by_sound_model() {
    let mut d = Tb303::new();
    d.live[RESONANCE] = 64;
    d.live[DECAY] = 100;
    d.note_on(NOTE);
    let c = st((64.0 * ACC_RESO_SCALE + ACC_RESO_BASE) * ACC_RESO_MUL);
    let mut v = c as f64;
    for _ in 0..6 {
        v = v * v;
    }
    assert_eq!(d.acc_coef, v as f32);
    assert_eq!((d.amp_in, d.amp_fb, d.env, d.retrig), (1.0, 0.0, 1.0, true));
    let mut w = Tb303::new();
    w.vintage = true;
    w.live = d.live;
    w.note_on(NOTE);
    let mut v = c as f64;
    v = v * v;
    v = v * v;
    assert_eq!(w.acc_coef, v as f32);
    assert_eq!((w.amp_in, w.amp_fb), AMP_VINTAGE);
    // the accent replaces the decay and sets the amount
    d.note_on([0, 0, 1, 0, 0, 0]);
    assert_eq!(d.env_decay, square_n(ACC_DECAY, 6));
    assert_eq!(d.acc_amount, st(64.0 * ACC_AMOUNT_SCALE));
}

#[test]
fn rest_sets_pitch_but_not_the_gate() {
    let mut d = Tb303::new();
    d.note_on([5, 0, 0, 0, 0, 1]);
    assert_eq!(d.gate, 0.0);
    assert!(!d.retrig);
    assert_eq!(d.pitch_target, ((5 * 64 + 3072) * 16) as u32);
}

#[test]
fn gate_off_after_half_a_step() {
    let mut d = device(&[NOTE, NOTE]);
    d.start(0);
    d.step(0, 0, 120_000);
    assert_eq!((d.gate, d.gate_off), (1.0, 24));
    d.step(23, 23, 120_000);
    assert_eq!(d.gate, 1.0);
    d.step(24, 24, 120_000);
    assert_eq!((d.gate, d.gate_off), (0.0, -1));
    assert_eq!((d.amp_in, d.amp_fb), AMP_OFF_20);
    // the next step triggers again
    d.step(48, 48, 120_000);
    assert_eq!((d.gate, d.gate_off, d.live[LAST_STEP]), (1.0, 72, 1));
}

#[test]
fn slide_ties_into_the_next_step() {
    let mut d = device(&[SLIDE, NOTE, REST]);
    d.start(0);
    d.step(0, 0, 120_000);
    d.step(30, 30, 120_000);
    // the note-off is skipped: the gate stays open across the slide
    assert_eq!(d.gate, 1.0);
    d.env = 0.5;
    d.step(48, 48, 120_000);
    // the slid-into note does not restart the envelope or the pitch
    assert_eq!((d.env, d.slide), (0.5, false));
    d.step(72, 72, 120_000);
    assert_eq!(d.gate, 0.0);
}

#[test]
fn pattern_changes_at_the_bar_line() {
    let mut d = device(&[NOTE; 16]);
    d.song_mode = true;
    d.start(0);
    d.step(0, 0, 120_000);
    d.set_param(PATTERN as u8, 1);
    d.step(48, 48, 120_000);
    assert_eq!(d.cur_pattern, 0);
    d.step(768, 768, 120_000);
    assert_eq!((d.cur_pattern, d.pat_start), (1, 768));
    // pattern 1 is all rests: the gate stays closed
    d.step(800, 800, 120_000);
    d.step(816, 816, 120_000);
    assert_eq!(d.gate, 0.0);
}

#[test]
fn shuffle_flag_selects_the_shuffled_tick() {
    let mut d = device(&[NOTE, NOTE]);
    d.patterns[0] = 1;
    d.start(0);
    d.step(0, 0, 120_000);
    // tick 48 shuffled to 40 is still step 0
    d.step(48, 40, 120_000);
    assert_eq!(d.live[LAST_STEP], 0);
    d.step(60, 48, 120_000);
    assert_eq!(d.live[LAST_STEP], 1);
}

#[test]
fn relocate_reads_the_position_not_its_argument() {
    let mut d = device(&[]);
    d.live[PATTERN] = 3;
    d.live[ENABLED] = 0;
    d.relocate(0, 1600);
    assert_eq!((d.pat_start, d.cur_pattern, d.enabled), (1536, 3, 0));
    d.start(800);
    assert_eq!((d.pat_start, d.wrapped), (768, true));
}

#[test]
fn song_mode_keeps_the_live_parameters() {
    let mut d = Tb303::new();
    d.set_song_mode(true);
    let mut p = DEFAULT_PARAMS;
    p[CUTOFF] = 99;
    d.data(&payload(p, &[]), true);
    assert_eq!((d.live[CUTOFF], d.params[CUTOFF]), (20, 99));
    d.set_param(CUTOFF as u8, 50);
    d.set_song_mode(false);
    assert_eq!(d.live[CUTOFF], 99);
}

#[test]
fn reset_keeps_the_coefficients() {
    let mut d = Tb303::new();
    d.coefs = [1.0, 2.0, 3.0, 4.0];
    d.retrig = true;
    d.phase = 0.25;
    d.cs1 = 0.1;
    d.slide = true;
    d.reset();
    assert_eq!((d.coefs, d.retrig), ([1.0, 2.0, 3.0, 4.0], true));
    assert_eq!((d.phase, d.cs1, d.cs2, d.gate_off), (0.0, 0.5, 0.5, -1));
    // slot 6 after the reset: the release constants
    assert_eq!(
        (d.slide, d.amp_in, d.amp_fb),
        (false, AMP_OFF_20.0, AMP_OFF_20.1)
    );
}

#[test]
fn init_track_quantises_the_pattern_controllers() {
    let mut d = Tb303::new();
    let mut tr = Track::new(Device::Tb303_1);
    d.init_track(&mut tr);
    let ev: Vec<_> = tr
        .events()
        .iter()
        .map(|e| (e.tick, e.ctrl, e.value))
        .collect();
    assert_eq!(ev.len(), 9);
    assert!(
        ev.iter()
            .all(|&(t, c, v)| t == 0 && v == DEFAULT_PARAMS[c as usize])
    );
    tr.insert(24, ENABLED as u8, 0);
    tr.insert(24, CUTOFF as u8, 5);
    let late: Vec<_> = tr.events()[9..].iter().map(|e| (e.tick, e.ctrl)).collect();
    assert_eq!(late, [(24, CUTOFF as u8), (768, ENABLED as u8)]);
}

#[test]
fn idle_output_is_zero_and_the_phase_runs() {
    let t = Tables::new();
    let ctx = BlockContext {
        tables: &t,
        meter_index: 0,
        tempo: 120_000,
        tick: 0,
        stack_word: None,
    };
    for vintage in [false, true] {
        let mut d = Tb303::new();
        d.vintage = vintage;
        let mut bus = [1.0f32; BLOCK];
        d.process(&mut bus, &ctx);
        assert!(bus.iter().all(|&x| x == 0.0));
        let inc = t.tb303_pitch[0] as f64;
        let mut ph = 0.0f64;
        for _ in 0..BLOCK {
            ph += inc;
        }
        assert_eq!(d.phase, ph as f32);
    }
}

#[test]
fn a_note_sounds_in_both_models() {
    let t = Tables::new();
    let ctx = BlockContext {
        tables: &t,
        meter_index: 0,
        tempo: 120_000,
        tick: 0,
        stack_word: None,
    };
    for vintage in [false, true] {
        let mut d = device(&[NOTE]);
        d.vintage = vintage;
        d.start(0);
        d.step(0, 0, 120_000);
        let mut peak = 0.0f32;
        for _ in 0..20 {
            let mut bus = [0.0f32; BLOCK];
            d.process(&mut bus, &ctx);
            peak = bus.iter().fold(peak, |m, &x| m.max(x.abs()));
        }
        assert!(peak > 0.05, "vintage {vintage}: peak {peak}");
        // the 2.0 engine started its coefficients without a ramp and cleared the flag; the
        // Vintage loop does not read it
        assert_eq!(d.retrig, vintage);
    }
}
