//! The TR-909 against the original (task-025): every diagnostic song whose sound is the 909
//! (with or without the 808) and the mixer only renders byte-identical to its corpus reference
//! (`tests/refs/manifest.json`, the WAV's sha256 at the entry's `idle_blocks`), with the built-in
//! samples of `original/ReBirth.dll` and the test-only delay model in place of the engine's delay
//! (task-026). The references were rendered by rbhost in a fresh process per song, so the
//! process-global drum counters start at 0, as in a new [`rb_engine::process::ProcessState`].

mod common;

use std::sync::Arc;

use common::{manifest, render_with_test_delay, samples, sha256, song};
use rb_engine::frame::Devices;
use rb_engine::tables::Tables;

/// The diagnostic songs whose sound comes from the TR-909 (three with the 808) and the mixer
/// only: a 909 pattern holds a step within its length and no insert is routed (mixer
/// compressor and PCF ids 0, no channel's distortion on, in the chunk or the mixer track).
/// Derived from the `.rbs` files with `tools/rbs.py` (task-025). All of them have the delay
/// enabled. Left out: `pcf_on_909` (the PCF) and `full_mix` (every device).
const TR909_SONGS: &[&str] = &[
    // every instrument and knob
    "909_accent_level0",
    "909_accent_level127",
    "909_bass",
    "909_bass_knobs_hi",
    "909_bass_knobs_lo",
    "909_clap",
    "909_closed_hihat",
    "909_closedhat_knobs_hi",
    "909_closedhat_knobs_lo",
    "909_crash",
    "909_crash_ride_knobs_hi",
    "909_crash_ride_knobs_lo",
    "909_flam_interval0",
    "909_flam_interval127",
    "909_flam_interval64",
    "909_hi_tom",
    "909_hightom_knobs_hi",
    "909_hightom_knobs_lo",
    "909_low_tom",
    "909_mid_tom",
    "909_open_hihat",
    "909_openhat_knobs_hi",
    "909_openhat_knobs_lo",
    "909_ride",
    "909_rim_shot",
    "909_snare",
    "909_snare_knobs_hi",
    "909_snare_knobs_lo",
    // the mixer around a 909, and both drum machines together
    "drums_kit",
    "mixer_level_sweep_909",
    "mixer_pan_sweep_909",
    "shuffle_drums_g127",
    "shuffle_drums_g64",
];

/// AC #1: every 909 song equals its corpus reference.
#[test]
fn tr909_songs_with_a_test_delay_match_their_references() {
    let m = manifest();
    let tables = Arc::new(Tables::new());
    let set = samples();
    let mut bad = Vec::new();
    for name in TR909_SONGS {
        let e = &m["diag"][*name];
        let idle = e["idle_blocks"].as_u64().unwrap() as u32;
        let d = Devices::implemented(Some(set.clone()));
        let wav = render_with_test_delay(&tables, d, &song(e), idle);
        assert_eq!(wav.len(), 44 + 4 * e["frames"].as_u64().unwrap() as usize);
        if sha256(&wav) != e["wav_sha256"].as_str().unwrap() {
            bad.push(*name);
        }
    }
    assert_eq!(TR909_SONGS.len(), 33);
    assert!(bad.is_empty(), "differ from their references: {bad:?}");
}

/// The control: without the 909 (the silent placeholder) a 909 song differs from its
/// reference, so the test above sees the 909's output.
#[test]
fn tr909_songs_differ_without_the_909() {
    let m = manifest();
    let tables = Arc::new(Tables::new());
    let e = &m["diag"]["909_snare"];
    let idle = e["idle_blocks"].as_u64().unwrap() as u32;
    let wav = render_with_test_delay(&tables, Devices::implemented(None), &song(e), idle);
    assert_ne!(sha256(&wav), e["wav_sha256"].as_str().unwrap());
}
