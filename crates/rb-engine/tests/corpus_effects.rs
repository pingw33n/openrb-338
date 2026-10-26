//! The distortion and the delay against the original (task-026): every diagnostic song whose
//! sound comes from devices the engine implements (the TB-303s, the TR-808, the TR-909, the
//! distortion, the delay and the mixer) renders byte-identical to its corpus reference
//! (`tests/refs/manifest.json`, the WAV's sha256 at the entry's `idle_blocks`, 9088 for every
//! diag song), with the engine's own delay ([`rb_engine::delay::Delay`]) and distortion
//! ([`rb_engine::dist::Distortion`]) and the built-in samples of `original/ReBirth.dll`.
//!
//! This includes every song of the TB-303, TR-808 and TR-909 corpus tests, which still check
//! against the test-only delay model ([`common::TestDelay`]) until task-049 switches them.

mod common;

use std::sync::Arc;

use common::{manifest, render_with, samples, sha256, song};
use rb_engine::frame::Devices;
use rb_engine::tables::Tables;

/// The diagnostic songs with no PCF and no compressor routed (mixer compressor and PCF ids 0,
/// in the chunk and in every event of the mixer track), derived from the `.rbs` files with
/// `tools/rbs.py` (task-026): 123 of the 145. Every one has the delay enabled. The distortion is
/// routed in `dist_*` and `mixer_dist_switch` (`dist_disabled` routes it with the device off).
/// Left out (the PCF or the compressor, task-027): `comp_*`, `pcf_*`, `mixer_pcf_switch` and
/// `full_mix`.
const SONGS: &[&str] = &[
    // the TB-303s (`corpus_tb303.rs` and its delay songs)
    "303_accent0",
    "303_accent127",
    "303_env0",
    "303_env127_decay0",
    "303_env127_decay127",
    "303_res0",
    "303_res127",
    "303_res64",
    "303_saw_highcut",
    "303_saw_lowcut",
    "303_slide",
    "303_slide_accent",
    "303_square_highcut",
    "303_square_lowcut",
    "303_tune_hi",
    "303_tune_lo",
    "303_two_voices",
    "303_vintage_accent",
    "303_vintage_highcut",
    "303_vintage_lowcut",
    "303_vintage_res127",
    "303_vintage_saw",
    "303_vintage_slide",
    "303_vintage_square",
    // the TR-808 (`corpus_tr808.rs`)
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
    // the TR-909 (`corpus_tr909.rs`)
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
    // the delay
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
    "mixer_delay_send_sweep",
    // the distortion
    "dist_a0_s0",
    "dist_a100_s0",
    "dist_a100_s127",
    "dist_a100_s64",
    "dist_a127_s0",
    "dist_a20_s0",
    "dist_disabled",
    "dist_drums",
    "mixer_dist_switch",
    // the sequencer, the mixer and the tempo around them
    "drums_kit",
    "loop_bar1",
    "loop_bar2",
    "loop_chase",
    "loop_events_inside",
    "mixer_level_sweep_303",
    "mixer_level_sweep_808",
    "mixer_level_sweep_909",
    "mixer_master_full",
    "mixer_master_low",
    "mixer_pan_sweep_303",
    "mixer_pan_sweep_808",
    "mixer_pan_sweep_909",
    "shuffle_303_g0",
    "shuffle_303_g127",
    "shuffle_303_g64",
    "shuffle_drums_g127",
    "shuffle_drums_g64",
    "shuffle_flag_off",
    "silence",
    "tempo_143_21",
    "tempo_200_003",
    "tempo_66_667",
    "tempo_97_3",
];

/// AC #1: every song above equals its corpus reference with the engine's devices.
#[test]
fn songs_with_the_engine_delay_and_distortion_match_their_references() {
    let m = manifest();
    let tables = Arc::new(Tables::new());
    let set = samples();
    let mut bad = Vec::new();
    for name in SONGS {
        let e = &m["diag"][*name];
        let idle = e["idle_blocks"].as_u64().unwrap() as u32;
        let d = Devices::implemented(Some(set.clone()));
        let wav = render_with(&tables, d, &song(e), idle);
        assert_eq!(wav.len(), 44 + 4 * e["frames"].as_u64().unwrap() as usize);
        if sha256(&wav) != e["wav_sha256"].as_str().unwrap() {
            bad.push(*name);
        }
    }
    assert_eq!(SONGS.len(), 123);
    assert!(
        bad.is_empty(),
        "{} differ from their references: {bad:?}",
        bad.len()
    );
}

/// The controls: with the distortion bypassed every enabled distortion song differs from its
/// reference (both the shaper and the clip of shape 0), and with the idle send in place of the
/// delay every delay song does, so the test above hears both effects.
#[test]
fn the_effect_songs_differ_without_the_effects() {
    let m = manifest();
    let tables = Arc::new(Tables::new());
    let set = samples();
    let dist = [
        "dist_a0_s0",
        "dist_a100_s0",
        "dist_a100_s127",
        "dist_a100_s64",
        "dist_a127_s0",
        "dist_a20_s0",
        "dist_drums",
        "mixer_dist_switch",
    ];
    let delay = SONGS.iter().filter(|n| n.starts_with("delay_"));
    let mut same = Vec::new();
    for (name, is_dist) in dist
        .iter()
        .map(|n| (n, true))
        .chain(delay.map(|n| (n, false)))
    {
        let e = &m["diag"][*name];
        let idle = e["idle_blocks"].as_u64().unwrap() as u32;
        let mut d = Devices::implemented(Some(set.clone()));
        if is_dist {
            d.dist = Box::new(rb_engine::frame::Bypass);
        } else {
            d.delay = Box::new(rb_engine::frame::IdleSend);
        }
        let wav = render_with(&tables, d, &song(e), idle);
        if sha256(&wav) == e["wav_sha256"].as_str().unwrap() {
            same.push(*name);
        }
    }
    assert!(same.is_empty(), "equal without their effect: {same:?}");
}
