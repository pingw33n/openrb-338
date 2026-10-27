//! The golden test of the mod songs (task-033): the 8 entries of the manifest's `mod` set (rbhost
//! with the mod loaded) and the 3 of `modapp` (the original app's own exports, each at its own
//! N), rendered with the mod's samples and compared by the pass criterion of task-013, the
//! sha256 of the whole WAV.
//!
//! The mods are Propellerhead material and are not in the repository: the test needs
//! `RB_MOD_DIR` (the folder of the .rbm files) and `RB20FUL.DAT` in `RB_RB20FUL`, and skips with
//! a message when either is unset. Each mod is checked against the file name, size and sha256
//! the entry records. The renders are long (41 minutes of audio for the 8), so the test is
//! ignored in debug builds: `cargo test --release` runs it.
//!
//! This test lives in rb-cli because it needs both the engine and the `rbm` crate (the engine's
//! tests cannot depend on `rbm`, which depends on the engine).

#[path = "../../rb-engine/tests/common/mod.rs"]
mod common;

use std::path::PathBuf;
use std::sync::Arc;

use common::{manifest, samples, sha256, song};
use rb_engine::frame::export;
use rb_engine::frame::{Devices, Engine};
use rb_engine::tables::Tables;
use serde_json::Value;

fn env() -> Option<(PathBuf, PathBuf)> {
    let (Some(dir), Some(dat)) = (
        std::env::var_os("RB_MOD_DIR"),
        std::env::var_os("RB_RB20FUL"),
    ) else {
        eprintln!("skipped: RB_MOD_DIR and RB_RB20FUL are not both set");
        return None;
    };
    Some((PathBuf::from(dir), PathBuf::from(dat)))
}

/// Renders the entry with its mod's samples; `None` when equal to the reference. With
/// `recrypted`, the mod is first rewritten by `rbm::recrypt` (task-059) and its samples are
/// loaded with the embedded key bytes alone, RB20FUL.DAT unused.
fn check(
    label: &str,
    e: &Value,
    dir: &std::path::Path,
    dat: &std::path::Path,
    recrypted: bool,
) -> Option<String> {
    let m = &e["mod"];
    let file = dir.join(m["file"].as_str().unwrap());
    let bytes = std::fs::read(&file).unwrap_or_else(|err| panic!("{}: {err}", file.display()));
    assert_eq!(
        bytes.len() as u64,
        m["size"].as_u64().unwrap(),
        "{label}: mod size"
    );
    assert_eq!(
        sha256(&bytes),
        m["sha256"].as_str().unwrap(),
        "{label}: mod sha256"
    );
    let rbm = rbm::Rbm::parse(bytes).unwrap();
    // the name in the song is the mod's INFO name, which is how the CLI finds the file
    let s = song(e);
    assert_eq!(rbm.info().unwrap().name, s.mod_name, "{label}");
    assert_eq!(s.mod_name, m["name"].as_str().unwrap(), "{label}");
    let mut dat = std::fs::File::open(dat).unwrap();
    let table = rbm::KeyTable::read(&rbm, &mut dat).unwrap();
    let set = if recrypted {
        let back = rbm::Rbm::parse(rbm::recrypt(&rbm, &table).unwrap()).unwrap();
        let t = rbm::KeyTable::embedded(&back).expect("a recrypted mod selects the window");
        assert!(back.slots_covered(&t), "{label}");
        back.load_samples(&t, &samples()).unwrap().set
    } else {
        rbm.load_samples(&table, &samples()).unwrap().set
    };
    let idle = e["idle_blocks"].as_u64().unwrap() as u32;
    let mut eng = Engine::new(
        Arc::new(Tables::new()),
        Devices::implemented(Some(Arc::new(set))),
    );
    let got = export::export_song(&mut eng, &s, idle);
    let wav = export::wav_bytes(&got);
    (sha256(&wav) != e["wav_sha256"].as_str().unwrap()).then(|| {
        format!(
            "{label} (N {idle}): sha256 differs ({} frames, reference {}); `rb compare --manifest \
             tests/refs/manifest.json --set {} --mod-dir ... --rb20ful ... \"{label}\"` for the \
             rest",
            got.len() / 2,
            e["frames"],
            label.split('/').next().unwrap()
        )
    })
}

fn check_set(set: &str, expect: usize, recrypted: bool) {
    let Some((dir, dat)) = env() else { return };
    let m = manifest();
    let entries: Vec<(&String, &Value)> = m[set].as_object().unwrap().iter().collect();
    assert_eq!(entries.len(), expect);
    let failures: Vec<String> = std::thread::scope(|sc| {
        let hs: Vec<_> = entries
            .iter()
            .map(|(name, e)| {
                let (dir, dat) = (&dir, &dat);
                sc.spawn(move || check(&format!("{set}/{name}"), e, dir, dat, recrypted))
            })
            .collect();
        hs.into_iter().filter_map(|h| h.join().unwrap()).collect()
    });
    assert!(
        failures.is_empty(),
        "{} of {expect} {set} songs differ:\n{}",
        failures.len(),
        failures.join("\n")
    );
}

/// The 8 shipped mod songs, rbhost references at 9088 idle blocks.
#[test]
#[cfg_attr(
    debug_assertions,
    ignore = "41 minutes of audio; runs in cargo test --release"
)]
fn mod_songs_match_their_references() {
    check_set("mod", 8, false);
}

/// The same 8 songs with the recrypted mods, loaded without RB20FUL.DAT (task-059).
#[test]
#[cfg_attr(
    debug_assertions,
    ignore = "41 minutes of audio; runs in cargo test --release"
)]
fn mod_songs_match_with_recrypted_mods() {
    check_set("mod", 8, true);
}

/// The 3 app exports with the recrypted mods, loaded without RB20FUL.DAT (task-059).
#[test]
#[cfg_attr(
    debug_assertions,
    ignore = "22 million frames; runs in cargo test --release"
)]
fn modapp_exports_match_with_recrypted_mods() {
    check_set("modapp", 3, true);
}

/// The 3 exports of the original app with a mod, each at its own fitted N.
#[test]
#[cfg_attr(
    debug_assertions,
    ignore = "22 million frames; runs in cargo test --release"
)]
fn modapp_exports_match_at_their_fitted_n() {
    check_set("modapp", 3, false);
}
