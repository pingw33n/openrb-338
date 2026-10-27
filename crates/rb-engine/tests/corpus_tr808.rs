//! The TR-808 against the original (task-024): every diagnostic song whose sound is the 808
//! (with or without the 303s) and the mixer only renders byte-identical to its corpus reference
//! (`tests/refs/manifest.json`, the WAV's sha256 at the entry's `idle_blocks`), with the built-in
//! samples of `original/ReBirth.dll` and the engine's delay (task-026, task-049). The references
//! were rendered by rbhost in a fresh process per song, so the process-global drum counters start
//! at 0, as in a new [`rb_engine::process::ProcessState`]. The comparison itself is the golden
//! test's (`corpus_golden.rs`, task-029); this file keeps the control and the counter test.

mod common;

use std::sync::Arc;

use common::{differing, manifest, samples, song};
use rb_engine::frame::{Devices, Engine, export};
use rb_engine::tables::Tables;

/// The diagnostic songs whose sound comes from the TR-808 (some with the TB-303s) and the mixer
/// only: an 808 pattern holds a step within its length, no 909 pattern holds a step, and no
/// insert is routed (mixer compressor and PCF ids 0, no channel's distortion on, in the chunk or
/// the mixer track). Derived from the `.rbs` files with `tools/rbs.py` (task-024). All of them
/// have the delay enabled.
const TR808_SONGS: &[&str] = &[
    // every instrument and knob
    "808_accent_level0",
    "808_accent_level127",
    "808_bass",
    "808_bass_knobs_hi",
    "808_bass_knobs_lo",
    "808_clap",
    "808_clap_knobs_hi",
    "808_clap_knobs_lo",
    "808_closed_hihat",
    "808_cow_bell",
    "808_cymbal",
    "808_cymbal_knobs_hi",
    "808_cymbal_knobs_lo",
    "808_hi_tom",
    "808_low_tom",
    "808_lowtom_knobs_hi",
    "808_lowtom_knobs_lo",
    "808_mid_tom",
    "808_open_hihat",
    "808_openhat_knobs_hi",
    "808_openhat_knobs_lo",
    "808_rim_shot",
    "808_snare",
    "808_snare_knobs_hi",
    "808_snare_knobs_lo",
    // the delay on an 808 source
    "delay_automated",
    "delay_pan0",
    "delay_pan127",
    "delay_straight_fb0",
    "delay_straight_fb127",
    "delay_straight_fb64",
    "delay_straight_steps1",
    "delay_straight_steps8",
    "delay_triplet_fb0",
    "delay_triplet_fb127",
    "delay_triplet_fb64",
    // the sequencer and the mixer around an 808 (some with a 303)
    "loop_chase",
    "mixer_delay_send_sweep",
    "mixer_level_sweep_808",
    "mixer_master_full",
    "mixer_pan_sweep_808",
    "tempo_143_21",
    "tempo_200_003",
    "tempo_66_667",
    "tempo_97_3",
];

/// The control: without the 808 (no samples, the silent placeholder) every 808 song differs
/// from its reference, so the golden test (`corpus_golden.rs`, which compares every song with
/// its corpus reference, task-029) hears the 808 in each.
#[test]
fn tr808_songs_differ_without_the_808() {
    assert_eq!(TR808_SONGS.len(), 45);
    assert_eq!(
        differing(TR808_SONGS, || Devices::implemented(None)),
        TR808_SONGS
    );
}

/// An export of `808_snare` (8 snare hits in its loop) moves the 808 snare counter by 9, the
/// step measured from app exports (`docs/export_state.md` section 11.2). In this model the
/// ninth hit is the loop start's step, triggered in the block the export renders past the loop
/// end and drops; the references cannot show that (the frames are dropped), only the count.
#[test]
fn an_808_snare_export_moves_the_snare_counter_by_9() {
    let m = manifest();
    let tables = Arc::new(Tables::new());
    let e = &m["diag"]["808_snare"];
    let s = song(e);
    let mut eng = Engine::new(tables, Devices::implemented(Some(samples())));
    export::export_song(&mut eng, &s, e["idle_blocks"].as_u64().unwrap() as u32).unwrap();
    let c = eng.process_state().drum_counters();
    assert_eq!((c.snare_808, c.bd_808), (9, 0));
}
