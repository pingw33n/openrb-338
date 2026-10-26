//! The engine frame against the reference corpus (`tests/refs/manifest.json`, task-021).

use std::path::{Path, PathBuf};
use std::sync::Arc;

use rb_engine::frame::export::{self, ExportRange};
use rb_engine::frame::{Devices, Engine, Idle};
use rb_engine::tables::Tables;
use serde_json::Value;
use sha2::{Digest, Sha256};

fn root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../..")
}

fn manifest() -> Value {
    let p = root().join("tests/refs/manifest.json");
    serde_json::from_slice(&std::fs::read(p).unwrap()).unwrap()
}

fn song(entry: &Value) -> rb_engine::song::Song {
    let p = root().join(entry["song"].as_str().unwrap());
    let data = std::fs::read(&p).unwrap_or_else(|e| panic!("{}: {e}", p.display()));
    rb_engine::load::convert(&rbs::Song::parse(&data).unwrap()).unwrap()
}

/// AC #2: the export driver's frame count equals the reference's for every diagnostic and
/// shipped song of the corpus.
#[test]
fn frame_counts_match_the_corpus() {
    let m = manifest();
    let mut checked = 0;
    for set in ["diag", "shipped"] {
        for (name, e) in m[set].as_object().unwrap() {
            let s = song(e);
            let got = export::frame_count(ExportRange::from(&s.globals));
            let want = e["frames"].as_u64().unwrap() as usize;
            assert_eq!(got, want, "{set}/{name}");
            checked += 1;
        }
    }
    assert_eq!(checked, 145 + 8);
}

/// AC #3: the silence song renders exactly equal to its reference (the WAV's sha256, header
/// included), at the manifest's idle count.
#[test]
fn silence_matches_its_reference() {
    let m = manifest();
    let e = &m["diag"]["silence"];
    let s = song(e);
    let mut eng = Engine::new(
        Arc::new(Tables::new()),
        Devices::placeholders(),
        Box::new(Idle),
    );
    eng.load(&s);
    let idle = e["idle_blocks"].as_u64().unwrap() as u32;
    let samples = export::export(&mut eng, ExportRange::from(&s.globals), idle);
    assert_eq!(samples.len() / 2, e["frames"].as_u64().unwrap() as usize);
    let hash: String = Sha256::digest(export::wav_bytes(&samples))
        .iter()
        .map(|b| format!("{b:02x}"))
        .collect();
    assert_eq!(hash, e["wav_sha256"].as_str().unwrap());
}
