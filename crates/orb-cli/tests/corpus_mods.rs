//! The golden test of the mod songs (task-033): the 8 entries of the manifest's `mod` set (rbhost
//! with the mod loaded) and the 3 of `modapp` (the original app's own exports, each at its own
//! N), rendered with the mod's samples and compared by the pass criterion of task-013, the
//! sha256 of the whole WAV.
//!
//! The mods are Propellerhead material and are not in the repository. Since task-063 orb loads
//! only RECRYPTED mods (`orb mod recrypt`, task-059) and the song tests read no RB20FUL.DAT: they
//! need the four recrypted shipped mods, which `orb-testkit` makes from the CD image
//! (`ORB_CD_ISO`; task-121) and checks against their pinned SHA-256s, and skip with a message
//! when it is unset. A recrypted mod is the original plus a 1024-byte pad, so each file is
//! checked against the name and the size the entry records (+ 1032 bytes: the pad chunk), and
//! its INFO name against the song's.
//!
//! One test still runs the recrypt: with the original, non-recrypted mods and the key file of
//! the same image it recrypts each original in process, requires the bytes to equal the
//! recrypted file, checks the original against the sha256 the entry records, and renders the
//! songs with the result.
//!
//! The renders are long (41 minutes of audio for the 8), so the tests are ignored in debug
//! builds: `cargo test --release` runs them.
//!
//! This test lives in orb-cli because it needs both the engine and the `rbm` crate (the engine's
//! tests cannot depend on `rbm`, which depends on the engine).

#[path = "../../orb-engine/tests/common/mod.rs"]
mod common;

use std::path::{Path, PathBuf};
use std::sync::Arc;

use common::{manifest, samples, sha256, song};
use orb_engine::frame::export;
use orb_engine::frame::{Devices, Engine};
use orb_engine::tables::Tables;
use serde_json::Value;

/// A recrypted mod is the original plus the pad chunk: 8 bytes of header, 1024 of payload.
const PAD_CHUNK: u64 = 1032;

/// The recrypted shipped mods, from the CD image (`ORB_CD_ISO`).
fn mod_dir() -> Option<PathBuf> {
    orb_testkit::recrypted_mods().map(Path::to_path_buf)
}

/// The originals and the key file of the recrypt test, from the CD image.
fn orig_env() -> Option<(PathBuf, PathBuf)> {
    let cd = orb_testkit::cd_files()?;
    Some((cd.orig_mods.clone(), cd.dat.clone()))
}

/// Renders the entry with its recrypted mod's samples (loaded with the embedded key bytes
/// alone); `None` when equal to the reference. With `orig`, the original mod is first recrypted
/// in process and must give the bytes of the file in `dir`.
fn check(label: &str, e: &Value, dir: &Path, orig: Option<&(PathBuf, PathBuf)>) -> Option<String> {
    let m = &e["mod"];
    let name = m["file"].as_str().unwrap();
    let file = dir.join(name);
    let bytes = std::fs::read(&file).unwrap_or_else(|err| panic!("{}: {err}", file.display()));
    let mut rbm = rbm::Rbm::parse(bytes.clone()).unwrap();
    if let Some((odir, dat)) = orig {
        let obytes = std::fs::read(odir.join(name)).unwrap();
        assert_eq!(
            obytes.len() as u64,
            m["size"].as_u64().unwrap(),
            "{label}: original mod size"
        );
        assert_eq!(
            sha256(&obytes),
            m["sha256"].as_str().unwrap(),
            "{label}: original mod sha256"
        );
        let orbm = rbm::Rbm::parse(obytes).unwrap();
        let table = rbm::KeyTable::read(&orbm, &mut std::fs::File::open(dat).unwrap()).unwrap();
        let out = rbm::recrypt(&orbm, &table).unwrap();
        assert!(out == bytes, "{label}: rbm::recrypt differs from {name}");
        rbm = rbm::Rbm::parse(out).unwrap();
    } else {
        assert_eq!(
            bytes.len() as u64,
            m["size"].as_u64().unwrap() + PAD_CHUNK,
            "{label}: recrypted mod size"
        );
    }
    // the name in the song is the mod's INFO name, which is how the CLI finds the file
    let s = song(e);
    assert_eq!(rbm.info().unwrap().name, s.mod_name, "{label}");
    assert_eq!(s.mod_name, m["name"].as_str().unwrap(), "{label}");
    let t = rbm::KeyTable::embedded(&rbm).expect("a recrypted mod selects the window");
    assert!(rbm.slots_covered(&t), "{label}");
    let set = rbm.load_samples(&t, &samples()).unwrap().set;
    let idle = e["idle_blocks"].as_u64().unwrap() as u32;
    let mut eng = Engine::new(
        Arc::new(Tables::new()),
        Devices::implemented(Some(Arc::new(set))),
    );
    let got = export::export_song(&mut eng, &s, idle).unwrap();
    let wav = export::wav_bytes(&got);
    (sha256(&wav) != e["wav_sha256"].as_str().unwrap()).then(|| {
        format!(
            "{label} (N {idle}): sha256 differs ({} frames, reference {}); `orb compare --manifest \
             tests/refs/manifest.json --set {} --mod-dir ... \"{label}\"` for the rest",
            got.len() / 2,
            e["frames"],
            label.split('/').next().unwrap()
        )
    })
}

fn check_set(set: &str, expect: usize, with_orig: bool) {
    let Some(dir) = mod_dir() else { return };
    let orig = if with_orig {
        let Some(o) = orig_env() else { return };
        Some(o)
    } else {
        None
    };
    let m = manifest();
    let entries: Vec<(&String, &Value)> = m[set].as_object().unwrap().iter().collect();
    assert_eq!(entries.len(), expect);
    let failures: Vec<String> = std::thread::scope(|sc| {
        let hs: Vec<_> = entries
            .iter()
            .map(|(name, e)| {
                let (dir, orig) = (&dir, orig.as_ref());
                sc.spawn(move || check(&format!("{set}/{name}"), e, dir, orig))
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

/// The 8 shipped mod songs, rbhost references at 9088 idle blocks, with the recrypted mods and
/// no RB20FUL.DAT.
#[test]
#[cfg_attr(
    debug_assertions,
    ignore = "41 minutes of audio; runs in cargo test --release"
)]
fn mod_songs_match_their_references() {
    check_set("mod", 8, false);
}

/// The 3 exports of the original app with a mod, each at its own fitted N, with the recrypted
/// mods and no RB20FUL.DAT.
#[test]
#[cfg_attr(
    debug_assertions,
    ignore = "22 million frames; runs in cargo test --release"
)]
fn modapp_exports_match_at_their_fitted_n() {
    check_set("modapp", 3, false);
}

/// `rbm::recrypt` of the original mods (from the CD image) gives the recrypted files, and the 8
/// mod songs render equal with them.
#[test]
#[cfg_attr(
    debug_assertions,
    ignore = "41 minutes of audio; runs in cargo test --release"
)]
fn recrypt_of_the_originals_matches_the_references() {
    check_set("mod", 8, true);
}
