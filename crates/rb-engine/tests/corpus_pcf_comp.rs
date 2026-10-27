//! The PCF and the compressor against the original (task-027): every diagnostic song that
//! routes either renders byte-identical to its corpus reference (`tests/refs/manifest.json`,
//! the WAV's sha256 at the entry's `idle_blocks`, 9088) with the engine's devices
//! ([`Devices::implemented`]) and the built-in samples of `original/ReBirth.dll`.
//!
//! The shipped songs (manifest `shipped`, the eight that use the built-in kit) are checked in
//! `shipped_songs_match_their_references`, which renders about 37 minutes of audio (4 s in a
//! release build, about a minute in a debug one) and is `#[ignore]`d: run it with
//! `cargo test --release -p rb-engine --test corpus_pcf_comp -- --ignored`.

mod common;

use std::sync::Arc;

use common::{differing, manifest, render_with, samples, sha256, song};
use rb_engine::frame::{Bypass, Devices};
use rb_engine::song::{Device, Mixer, Song};
use rb_engine::tables::Tables;

/// Whether the song routes the PCF or the compressor: a non-zero id in the mixer chunk or in
/// any mixer-track event of that id (an insert that is never routed is never heard).
fn routes(s: &Song, ctrl: u8) -> bool {
    let chunk = if ctrl == Mixer::CTRL_PCF_ID {
        s.mixer.pcf_id
    } else {
        s.mixer.comp_id
    };
    chunk != 0
        || s.tracks[Device::Mixer.id() as usize]
            .iter()
            .any(|e| e.ctrl == ctrl && e.value != 0)
}

/// The diagnostic songs that route the PCF (`pcf_*`, `mixer_pcf_switch`, `full_mix`).
const PCF_SONGS: &[&str] = &[
    "full_mix",
    "mixer_pcf_switch",
    "pcf_bp_freq10_res100",
    "pcf_bp_freq120_res20",
    "pcf_bp_wave0",
    "pcf_bp_wave127",
    "pcf_bp_wave64",
    "pcf_lp_amount0_decay64",
    "pcf_lp_amount127_decay0",
    "pcf_lp_amount127_decay127",
    "pcf_lp_freq10_res100",
    "pcf_lp_freq120_res20",
    "pcf_lp_wave0",
    "pcf_lp_wave127",
    "pcf_lp_wave64",
    "pcf_on_909",
];

/// The diagnostic songs that route the compressor (`comp_*`, `full_mix`).
const COMP_SONGS: &[&str] = &[
    "comp_ch_808",
    "comp_ch_a100_t10",
    "comp_ch_a127_t0",
    "comp_ch_a30_t64",
    "comp_master_a100_t10",
    "comp_master_a127_t0",
    "full_mix",
];

/// The two lists are exactly the diag songs that route the PCF, resp. the compressor.
#[test]
fn the_lists_are_the_routing_songs() {
    let m = manifest();
    let diag = m["diag"].as_object().unwrap();
    let derive = |ctrl: u8| -> Vec<String> {
        let mut v: Vec<String> = diag
            .iter()
            .filter(|(_, e)| routes(&song(e), ctrl))
            .map(|(n, _)| n.clone())
            .collect();
        v.sort();
        v
    };
    assert_eq!(derive(Mixer::CTRL_PCF_ID), PCF_SONGS);
    assert_eq!(derive(Mixer::CTRL_COMP_ID), COMP_SONGS);
}

/// AC #1: every PCF and compressor song equals its corpus reference with the engine's devices.
#[test]
fn pcf_and_comp_songs_match_their_references() {
    let set = samples();
    let mut names: Vec<&str> = PCF_SONGS.iter().chain(COMP_SONGS).copied().collect();
    names.sort();
    names.dedup();
    assert_eq!(names.len(), 22);
    let bad = differing(&names, || Devices::implemented(Some(set.clone())));
    assert!(bad.is_empty(), "differ from their references: {bad:?}");
}

/// The controls: with the PCF bypassed every PCF song differs from its reference, and with
/// the compressor bypassed every compressor song does, so the test above hears both.
#[test]
fn the_songs_differ_without_the_effects() {
    let set = samples();
    let without = |pcf: bool| {
        let set = set.clone();
        move || {
            let mut d = Devices::implemented(Some(set.clone()));
            if pcf {
                d.pcf = Box::new(Bypass);
            } else {
                d.comp = Box::new(Bypass);
            }
            d
        }
    };
    assert_eq!(
        differing(PCF_SONGS, without(true)),
        PCF_SONGS,
        "equal without the PCF"
    );
    assert_eq!(
        differing(COMP_SONGS, without(false)),
        COMP_SONGS,
        "equal without the compressor"
    );
}

/// The shipped songs that use the built-in kit against their references (rbhost at 9088,
/// sha256 and frame count only, no audio committed).
#[test]
#[ignore = "about a minute in a debug build; run with --release -- --ignored"]
fn shipped_songs_match_their_references() {
    let m = manifest();
    let set = samples();
    let tables = Arc::new(Tables::new());
    let shipped = m["shipped"].as_object().unwrap();
    assert_eq!(shipped.len(), 8);
    let mut bad = Vec::new();
    for (name, e) in shipped {
        let idle = e["idle_blocks"].as_u64().unwrap() as u32;
        let wav = render_with(
            &tables,
            Devices::implemented(Some(set.clone())),
            &song(e),
            idle,
        );
        let frames = e["frames"].as_u64().unwrap() as usize;
        if wav.len() != 44 + 4 * frames || sha256(&wav) != e["wav_sha256"].as_str().unwrap() {
            bad.push(name.clone());
        }
    }
    assert!(bad.is_empty(), "differ from their references: {bad:?}");
}
