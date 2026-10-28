//! The distortion and the delay against the original (task-026): the diagnostic songs with no
//! PCF and no compressor routed (mixer compressor and PCF ids 0, in the chunk and in every event
//! of the mixer track; derived from the `.rbs` files with `tools/rbs.py`, task-026), 123 of the
//! 145, render byte-identical to their corpus references (`tests/refs/manifest.json`, the WAV's
//! sha256 at the entry's `idle_blocks`, 9088 for every diag song) with the engine's own delay
//! ([`rb_engine::delay::Delay`]) and distortion ([`rb_engine::dist::Distortion`]) and the
//! built-in samples (rb's embedded kit). Every one has the delay enabled. The reference
//! comparison of every corpus song is the golden test's (`corpus_golden.rs`, task-029); the
//! device tests keep the controls, which prove the golden test hears each device.
//!
//! They are split, without overlap, among the device tests: the 36 TB-303 songs of
//! `corpus_tb303.rs` (`delay_303` among them), the 45 TR-808 songs of `corpus_tr808.rs` (the
//! other `delay_*` songs and `mixer_delay_send_sweep`), the 33 TR-909 songs of `corpus_tr909.rs`,
//! and the 9 distortion songs here (task-049). The other 22, which route the PCF or the
//! compressor (`comp_*`, `pcf_*`, `mixer_pcf_switch` and `full_mix`), are in
//! `corpus_pcf_comp.rs` (task-027).

mod common;

use common::{differing, samples};
use rb_engine::frame::{Bypass, Devices, IdleSend};

/// The songs that route the distortion: `dist_*` and `mixer_dist_switch` (`dist_disabled`
/// routes it with the device off).
const DIST_SONGS: &[&str] = &[
    "dist_a0_s0",
    "dist_a100_s0",
    "dist_a100_s127",
    "dist_a100_s64",
    "dist_a127_s0",
    "dist_a20_s0",
    "dist_disabled",
    "dist_drums",
    "mixer_dist_switch",
];

/// The songs that set the delay's own parameters (in `corpus_tb303.rs` and `corpus_tr808.rs`).
const DELAY_SONGS: &[&str] = &[
    "delay_303",
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
];

/// The controls: with the distortion bypassed every enabled distortion song differs from its
/// reference (both the shaper and the clip of shape 0), and with the idle send in place of the
/// delay every delay song does, so the golden test hears the distortion and the delay.
#[test]
fn the_effect_songs_differ_without_the_effects() {
    let set = samples();
    assert_eq!(DIST_SONGS.len(), 9);
    let dist: Vec<&str> = DIST_SONGS
        .iter()
        .copied()
        .filter(|n| *n != "dist_disabled")
        .collect();
    let no_dist = || {
        let mut d = Devices::implemented(Some(set.clone()));
        d.dist = Box::new(Bypass);
        d
    };
    let no_delay = || {
        let mut d = Devices::implemented(Some(set.clone()));
        d.delay = Box::new(IdleSend);
        d
    };
    assert_eq!(
        differing(&dist, no_dist),
        dist,
        "equal without the distortion"
    );
    assert_eq!(
        differing(DELAY_SONGS, no_delay),
        DELAY_SONGS,
        "equal without the delay"
    );
}
