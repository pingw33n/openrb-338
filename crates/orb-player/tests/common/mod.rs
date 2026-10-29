//! The player tests' helpers: the corpus manifest, its songs and the built-in samples.

#![allow(dead_code)]

use std::path::{Path, PathBuf};
use std::sync::{Arc, OnceLock};

use orb_engine::samples::SampleSet;
use orb_engine::song::Song;
use orb_engine::tables::Tables;
use orb_player::{Audio, CALL_FRAMES, Command, Player};
use serde_json::Value;

pub fn root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../..")
}

/// A path from the environment (`None` when unset or empty); a relative one is relative to the
/// workspace root, as cargo runs the tests in the crate's directory.
pub fn env_path(name: &str) -> Option<PathBuf> {
    std::env::var_os(name)
        .filter(|v| !v.is_empty())
        .map(|v| root().join(v))
}

pub fn manifest() -> Value {
    let p = root().join("tests/refs/manifest.json");
    serde_json::from_slice(&std::fs::read(p).unwrap()).unwrap()
}

/// The song of a manifest entry, converted to the engine's model.
pub fn song(entry: &Value) -> Song {
    let p = root().join(entry["song"].as_str().unwrap());
    let data = std::fs::read(&p).unwrap_or_else(|err| panic!("{}: {err}", p.display()));
    orb_engine::load::convert(&rbs::Song::parse(&data).unwrap()).unwrap()
}

pub fn tables() -> Arc<Tables> {
    static T: OnceLock<Arc<Tables>> = OnceLock::new();
    T.get_or_init(|| Arc::new(Tables::new())).clone()
}

/// The built-in samples: the kit orb embeds (`assets/Standard.rbm`, the samples of the
/// original's ReBirth.dll byte for byte).
pub fn samples() -> Arc<SampleSet> {
    rbm::standard::builtin_samples()
}

/// A kit of fresh samples (shared with nothing else): the built-in ones with `f` applied to
/// every value.
pub fn kit(f: impl Fn(i16) -> i16) -> Arc<SampleSet> {
    use orb_engine::samples::{FIRST_ID, LAST_ID, Sample};
    let s = samples();
    Arc::new(SampleSet::from_samples(
        (FIRST_ID..=LAST_ID)
            .map(|id| {
                let a: Vec<i16> = s.get(id).audio().iter().map(|&x| f(x)).collect();
                Sample::from_audio(&a)
            })
            .collect(),
    ))
}

/// Sends a command that must fit.
pub fn send(p: &mut Player, c: Command) {
    assert!(p.send(c).is_ok(), "the command queue is full");
}

/// Runs `calls` engine calls (one 1024-frame callback each) and returns their output.
pub fn run(a: &mut Audio, calls: usize) -> Vec<i16> {
    let mut out = vec![0i16; 2 * CALL_FRAMES * calls];
    for c in out.chunks_mut(2 * CALL_FRAMES) {
        a.render_i16(c);
    }
    out
}
