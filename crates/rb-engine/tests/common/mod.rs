//! Helpers of the corpus tests (`corpus_golden.rs`, `corpus_tb303.rs`, `corpus_tr808.rs`,
//! `corpus_tr909.rs`, `corpus_effects.rs`, `corpus_pcf_comp.rs`): the manifest, the songs, the samples and the
//! render.
//!
//! Every song of the diagnostic corpus has the delay enabled, and an enabled delay adds its
//! output (about 1e-6 at idle, plus 1e-6 x the channels D samples later) into the master, so the
//! corpus references depend on the delay: the tests render with the engine's own
//! ([`rb_engine::delay::Delay`], in [`Devices::implemented`]).

#![allow(dead_code)]

use std::path::{Path, PathBuf};
use std::sync::Arc;

use rb_engine::frame::export;
use rb_engine::frame::{Devices, Engine};
use rb_engine::samples::SampleSet;
use rb_engine::song::{Delay, Device, Song};
use rb_engine::tables::Tables;
use serde_json::Value;
use sha2::{Digest, Sha256};

pub fn root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../..")
}

pub fn json(p: &Path) -> Value {
    serde_json::from_slice(&std::fs::read(p).unwrap()).unwrap()
}

pub fn manifest() -> Value {
    json(&root().join("tests/refs/manifest.json"))
}

/// The song of a manifest entry, converted to the engine's model.
pub fn song(entry: &Value) -> Song {
    let p = root().join(entry["song"].as_str().unwrap());
    let data = std::fs::read(&p).unwrap_or_else(|err| panic!("{}: {err}", p.display()));
    rb_engine::load::convert(&rbs::Song::parse(&data).unwrap()).unwrap()
}

/// The built-in samples of `original/ReBirth.dll`.
pub fn samples() -> Arc<SampleSet> {
    let p = root().join("original/ReBirth.dll");
    let dll = std::fs::read(&p).unwrap_or_else(|err| panic!("{}: {err}", p.display()));
    Arc::new(SampleSet::from_dll(&dll).unwrap())
}

pub fn sha256(b: &[u8]) -> String {
    Sha256::digest(b)
        .iter()
        .map(|b| format!("{b:02x}"))
        .collect()
}

/// The WAV file of the export of `s` after `idle` idle blocks, with `devices`.
pub fn render_with(tables: &Arc<Tables>, devices: Devices, s: &Song, idle: u32) -> Vec<u8> {
    let mut eng = Engine::new(tables.clone(), devices);
    export::wav_bytes(&export::export_song(&mut eng, s, idle))
}

/// The diag songs of `names` whose render with `devices()` differs from their corpus reference
/// (the WAV's sha256 at the entry's `idle_blocks`); every render must have the entry's frames.
pub fn differing<'a>(names: &[&'a str], devices: impl Fn() -> Devices) -> Vec<&'a str> {
    let m = manifest();
    let tables = Arc::new(Tables::new());
    let mut bad = Vec::new();
    for name in names {
        let e = &m["diag"][*name];
        let idle = e["idle_blocks"].as_u64().unwrap() as u32;
        let wav = render_with(&tables, devices(), &song(e), idle);
        assert_eq!(wav.len(), 44 + 4 * e["frames"].as_u64().unwrap() as usize);
        if sha256(&wav) != e["wav_sha256"].as_str().unwrap() {
            bad.push(*name);
        }
    }
    bad
}

/// The song with the delay disabled: the chunk's enabled byte and every `enabled` event of the
/// delay's track set to 0, the edit `tools/nodelay.py` makes on the `.rbs` for rbhost.
pub fn without_delay(mut s: Song) -> Song {
    s.delay.params[Delay::ENABLED] = 0;
    for e in &mut s.tracks[Device::Delay.id() as usize] {
        if e.ctrl as usize == Delay::ENABLED {
            e.value = 0;
        }
    }
    s
}
