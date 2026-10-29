//! The TR-909 against the original (task-025): every diagnostic song whose sound is the 909
//! (with or without the 808) and the mixer only renders byte-identical to its corpus reference
//! (`tests/refs/manifest.json`, the WAV's sha256 at the entry's `idle_blocks`), with the built-in
//! samples (orb's embedded kit) and the engine's delay (task-026, task-049). The references
//! were rendered by rbhost in a fresh process per song, so the process-global drum counters start
//! at 0, as in a new [`orb_engine::process::ProcessState`]. The comparison itself is the golden
//! test's (`corpus_golden.rs`, task-029); this file keeps the control.

mod common;

use common::{differing, samples};
use orb_engine::frame::Devices;

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

/// The control: without the 909 (its channel the silent placeholder, the 808 kept) every 909
/// song differs from its reference, so the golden test (`corpus_golden.rs`, which compares every
/// song with its corpus reference, task-029) hears the 909 in each.
#[test]
fn tr909_songs_differ_without_the_909() {
    let set = samples();
    let no_909 = || {
        let mut d = Devices::implemented(Some(set.clone()));
        d.channels[3] = Box::new(orb_engine::frame::Silent);
        d
    };
    assert_eq!(TR909_SONGS.len(), 33);
    assert_eq!(differing(TR909_SONGS, no_909), TR909_SONGS);
}
