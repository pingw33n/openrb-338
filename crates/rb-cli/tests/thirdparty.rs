//! The check of a third-party collection of mods and songs (task-058, task-121): the user's own
//! files, read in place and never in the repository. Two variables name them, and each test
//! skips with a message when its input is not set:
//!
//! - `RB_THIRDPARTY_MODS_DIR`: a folder of the RECRYPTED `.rbm` files (`rb mod recrypt`; task-063),
//!   for the `mods` test, and for the `songs` test's mod resolution when it is set;
//! - `RB_THIRDPARTY_SONGS_DIR`: a folder of songs (`.rbs` files, any name), for the `songs` test.
//!
//! Run it in release, with the summary shown:
//!
//! ```text
//! RB_THIRDPARTY_MODS_DIR=~/rb-mods/mods RB_THIRDPARTY_SONGS_DIR=~/rb-songs/songs \
//!     cargo test --release -p rb-cli --test thirdparty -- --nocapture --test-threads 1
//! ```
//!
//! - `mods`: every recrypted `.rbm` through `crates/rbm` (parse, the embedded key bytes, every
//!   embedded file, the 46 slots with the built-in fallback), compared with the committed manifest
//!   `tests/refs/thirdparty_mods.json`: the SHA-256 of the recrypted file, of every embedded file
//!   and of every slot's channel-0 PCM, as `tools/rbm.py` decoded them from the ORIGINAL mods with
//!   the CD's RB20FUL.DAT. The manifest holds only hashes and names. To regenerate it (only when
//!   the collection changes), the originals and the key file are arguments of the tool, not
//!   variables of the test: `tools/thirdparty_mods_manifest.py --orig MODS.ORIG --mods MODS --dat
//!   RB20FUL.DAT`.
//! - `songs`: every distinct song (by SHA-256): an RBS 4.2 song is read and written back
//!   (byte-identical) and rendered with `rb render --mod-dir DIR --standard-kit` (DIR is
//!   `RB_THIRDPARTY_MODS_DIR` when it is set; without it no mod is found and the test says so), its
//!   mod resolved as the original does; a ReBirth 1.x song (a MIDI file, `MThd`, task-060) is
//!   rendered too, and its import's message stream (`rb_engine::load::load_1x`) must equal
//!   `tools/rb1x.py`'s. No song may panic.

mod common;
#[path = "../../../tests/common/python.rs"]
mod python;

use std::collections::{BTreeMap, HashMap};
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::Mutex;

use common::root;
use rb_engine::samples::{self, Sample, SampleSet};
use serde_json::Value;
use sha2::{Digest, Sha256};

/// The inputs of the `songs` test.
struct Env {
    /// The recrypted mods: what rb loads, when the folder is given.
    mods: Option<PathBuf>,
    songs: PathBuf,
}

fn env() -> Option<Env> {
    let songs = rb_testkit::thirdparty_songs()?;
    let mods = common::env_path("RB_THIRDPARTY_MODS_DIR");
    if mods.is_none() {
        eprintln!(
            "note: RB_THIRDPARTY_MODS_DIR is not set: the songs' mods are not resolved \
             (the built-in kit plays instead)"
        );
    }
    Some(Env { mods, songs })
}

fn hex(b: &[u8]) -> String {
    Sha256::digest(b)
        .iter()
        .map(|x| format!("{x:02x}"))
        .collect()
}

/// A stand-in built-in kit: the comparison only needs to tell the mod's slots from the others.
fn builtin() -> SampleSet {
    SampleSet::from_samples(
        (0..samples::COUNT)
            .map(|i| Sample::from_audio(&[i as i16 + 1; 3]))
            .collect(),
    )
}

#[test]
fn mods() {
    let Some(dir) = rb_testkit::thirdparty_mods() else {
        return;
    };
    let manifest: Value = serde_json::from_slice(
        &std::fs::read(root().join("tests/refs/thirdparty_mods.json")).unwrap(),
    )
    .expect("tests/refs/thirdparty_mods.json");
    let reference = manifest["mods"].as_array().unwrap();
    assert!(!reference.is_empty(), "the manifest has no mod");

    let bi = builtin();
    let (mut ok, mut failed, mut partial, mut files_checked, mut slots_checked) = (0, 0, 0, 0, 0);
    let mut problems = Vec::new();
    for m in reference {
        let file = m["file"].as_str().unwrap();
        let path = dir.join(file);
        let Ok(bytes) = std::fs::read(&path) else {
            problems.push(format!("{file}: not in {}", dir.display()));
            continue;
        };
        if hex(&bytes) != m["sha256"].as_str().unwrap() {
            problems.push(format!("{file}: differs from the manifest's SHA-256"));
            continue;
        }
        let parsed = rbm::Rbm::parse(bytes);
        let py_err = m["error"].as_str();
        let rbm = match (parsed, py_err) {
            (Err(err), Some(pe)) => {
                println!("FAIL {file}: rb: {err}; rbm.py: {pe}");
                failed += 1;
                continue;
            }
            (Err(err), None) => {
                problems.push(format!("{file}: rb fails ({err}), rbm.py loads it"));
                continue;
            }
            (Ok(r), _) => r,
        };
        let name = rbm.info().map(|i| i.name.clone());
        if let Some(pe) = py_err {
            if name.as_deref().is_some_and(|n| !n.is_empty()) {
                problems.push(format!("{file}: rbm.py fails ({pe}), rb loads it"));
            } else {
                println!("FAIL {file}: {pe}");
                failed += 1;
            }
            continue;
        }
        assert_eq!(name.as_deref(), m["name"].as_str(), "{file}: INFO name");
        // the recrypted mod loads with the embedded key bytes alone
        let table = rbm::KeyTable::embedded(&rbm)
            .filter(|t| rbm.all_covered(t))
            .unwrap_or_else(|| panic!("{file}: not recrypted"));

        // Every embedded file, in file order (the recrypted mod has one more: the pad).
        let py_files = m["files"].as_array().unwrap();
        let ours: Vec<_> = rbm
            .embedded()
            .filter(|c| c.name.as_deref() != Some(rbm::PAD_NAME))
            .cloned()
            .collect();
        assert_eq!(ours.len(), py_files.len(), "{file}: EMBF count");
        for (c, p) in ours.iter().zip(py_files) {
            let b = rbm.file(c, &table).unwrap();
            if c.name.as_deref() != p[0].as_str() || hex(&b) != p[1] {
                problems.push(format!("{file}: {:?} differs from rbm.py", c.name));
            }
            files_checked += 1;
        }

        // The 46 slots: the mod's where rbm.py found one, the built-in sample elsewhere.
        let slots = m["slots"].as_object().unwrap();
        let slot_errors = m["slot_errors"].as_object().unwrap();
        match rbm.load_samples(&table, &bi) {
            Err(err) => problems.push(format!("{file}: load_samples: {err}")),
            Ok(l) => {
                for id in samples::FIRST_ID..=samples::LAST_ID {
                    let sname = rbm::slot_name(id).unwrap();
                    match (slots.get(sname), slot_errors.get(sname)) {
                        (Some(sha), _) => {
                            let pcm: Vec<u8> = l
                                .set
                                .get(id)
                                .audio()
                                .iter()
                                .flat_map(|x| x.to_be_bytes())
                                .collect();
                            if !l.is_from_mod(id) || hex(&pcm) != *sha {
                                problems.push(format!("{file}: slot {sname} differs from rbm.py"));
                            }
                            slots_checked += 1;
                        }
                        (None, Some(why)) => {
                            problems.push(format!("{file}: rb loads {sname}, rbm.py says {why}"))
                        }
                        (None, None) => {
                            if l.is_from_mod(id) || l.set.get(id) != bi.get(id) {
                                problems.push(format!("{file}: slot {sname} should be built in"));
                            }
                        }
                    }
                }
            }
        }
        if slots.len() + slot_errors.len() < samples::COUNT {
            partial += 1;
        }
        println!(
            "OK   {file}: {:?}, {} files, {}/46 slots",
            name.unwrap_or_default(),
            ours.len(),
            slots.len() + slot_errors.len(),
        );
        ok += 1;
    }
    // a mod of the folder the manifest does not know: the collection grew, regenerate the
    // manifest (tools/thirdparty_mods_manifest.py)
    let known: std::collections::HashSet<&str> = reference
        .iter()
        .map(|m| m["file"].as_str().unwrap())
        .collect();
    let mut extra: Vec<String> = std::fs::read_dir(&dir)
        .unwrap()
        .filter_map(|e| e.ok())
        .map(|e| e.file_name().to_string_lossy().into_owned())
        .filter(|f| f.to_ascii_lowercase().ends_with(".rbm") && !known.contains(f.as_str()))
        .collect();
    extra.sort();
    println!(
        "mods: {} checked, {ok} load, {failed} fail; {partial} keep built-in slots; \
         {files_checked} embedded files and {slots_checked} slots equal the manifest's",
        reference.len()
    );
    if !extra.is_empty() {
        problems.push(format!(
            "{} .rbm in the folder are not in tests/refs/thirdparty_mods.json (regenerate it \
             with tools/thirdparty_mods_manifest.py): {}",
            extra.len(),
            extra.join(", ")
        ));
    }
    assert!(problems.is_empty(), "{}", problems.join("\n"));
}

/// The INFO menu name of every mod of `dir`, in the original's folder order (NTFS, upcased).
fn info_names(dir: &Path) -> Vec<(String, Option<String>)> {
    let mut files: Vec<String> = std::fs::read_dir(dir)
        .unwrap()
        .map(|e| e.unwrap().file_name().to_string_lossy().into_owned())
        .filter(|f| f.to_ascii_lowercase().ends_with(".rbm"))
        .collect();
    files.sort_by_key(|f| (f.to_uppercase(), f.clone()));
    files
        .into_iter()
        .map(|f| {
            let name = std::fs::read(dir.join(&f))
                .ok()
                .and_then(|b| rbm::Rbm::parse(b).ok())
                .and_then(|r| r.info().map(|i| i.name.clone()));
            (f, name)
        })
        .collect()
}

/// How the song's mod name resolves against the folder.
fn resolution(mods: &[(String, Option<String>)], name: &str) -> String {
    if name == "Standard ReBirth" {
        return "standard".into();
    }
    if let Some((f, _)) = mods.iter().find(|(_, n)| n.as_deref() == Some(name)) {
        return format!("INFO name = {f}");
    }
    let low = name.to_lowercase();
    if let Some((f, _)) = mods.iter().find(|(f, _)| {
        let f = f.to_lowercase();
        f == low || f.strip_suffix(".rbm") == Some(low.as_str())
    }) {
        return format!("file name only = {f}");
    }
    if let Some((f, n)) = mods
        .iter()
        .find(|(_, n)| n.as_deref().is_some_and(|n| n.to_lowercase() == low))
    {
        return format!(
            "INFO name differs in case = {f} ({:?})",
            n.as_deref().unwrap()
        );
    }
    "missing".into()
}

#[test]
fn songs() {
    let Some(e) = env() else { return };
    let mods = e.mods.as_deref().map(info_names).unwrap_or_default();
    let mut paths: Vec<PathBuf> = std::fs::read_dir(&e.songs)
        .unwrap()
        .map(|x| x.unwrap().path())
        .filter(|p| {
            p.is_file()
                && p.file_name()
                    .is_some_and(|f| f.to_string_lossy().to_lowercase().contains(".rbs"))
        })
        .collect();
    paths.sort();
    let mut seen: HashMap<String, PathBuf> = HashMap::new();
    let mut unique = Vec::new();
    let mut dups = 0;
    for p in paths {
        let h = hex(&std::fs::read(&p).unwrap());
        if seen.contains_key(&h) {
            dups += 1;
        } else {
            seen.insert(h.clone(), p.clone());
            unique.push((h, p));
        }
    }

    let tmp = tempfile::tempdir().unwrap();
    let results: Mutex<BTreeMap<String, (String, String)>> = Mutex::new(BTreeMap::new());
    let next = Mutex::new(0usize);
    let threads = std::thread::available_parallelism().map_or(4, |n| n.get());
    std::thread::scope(|s| {
        for _ in 0..threads {
            s.spawn(|| {
                loop {
                    let i = {
                        let mut n = next.lock().unwrap();
                        *n += 1;
                        *n - 1
                    };
                    let Some((h, p)) = unique.get(i) else { break };
                    let r = check_song(&e, &mods, p, &tmp.path().join(format!("{h}.wav")));
                    let label = p.file_name().unwrap().to_string_lossy().into_owned();
                    results.lock().unwrap().insert(label, r);
                }
            });
        }
    });

    let results = results.into_inner().unwrap();
    let mut classes: BTreeMap<String, usize> = BTreeMap::new();
    let mut problems = Vec::new();
    for (label, (class, detail)) in &results {
        println!("{class:<10} {label}: {detail}");
        *classes.entry(class.clone()).or_default() += 1;
        if class.starts_with("FAIL") {
            problems.push(format!("{label}: {detail}"));
        }
    }
    println!(
        "songs: {} files with .rbs in the name, {dups} duplicates, {} distinct: {classes:?}",
        results.len() + dups,
        results.len()
    );
    assert!(problems.is_empty(), "{}", problems.join("\n"));
}

/// (class, detail) of one song: `RBS42`, `RBS1X` (rendered, and its import messages equal
/// `tools/rb1x.py`'s), or `FAIL...`.
fn check_song(
    e: &Env,
    mods: &[(String, Option<String>)],
    path: &Path,
    out: &Path,
) -> (String, String) {
    let data = std::fs::read(path).unwrap();
    let mut detail = String::new();
    let rbs42 = data.starts_with(b"CAT ");
    if rbs42 {
        let song = match rbs::Song::parse(&data) {
            Ok(s) => s,
            Err(err) => return ("FAIL-read".into(), format!("{err}")),
        };
        match song.write() {
            Ok(w) if w == data => {}
            Ok(w) => match rbs::Song::parse(&w) {
                Ok(s2) if s2 == song => detail.push_str("not canonical (re-reads equal); "),
                _ => return ("FAIL-write".into(), "the written song differs".into()),
            },
            Err(err) => return ("FAIL-write".into(), format!("{err}")),
        }
        detail.push_str(&format!(
            "mod {:?}: {}; ",
            song.glob.mod_name,
            resolution(mods, &song.glob.mod_name)
        ));
    } else if !data.starts_with(b"MThd") {
        return ("OTHER".into(), "neither RBS 4.2 nor a MIDI file".into());
    }
    let mut cmd = Command::new(env!("CARGO_BIN_EXE_rb"));
    cmd.arg("render").arg(path).arg("-o").arg(out);
    match &e.mods {
        Some(mods) => cmd.arg("--mod-dir").arg(mods),
        // no mod folder: the mod of the child's own environment must not matter
        None => cmd.env_remove("RB_MOD_DIR").env_remove("RB_RBM_DIR"),
    };
    let o = cmd.arg("--standard-kit").output().unwrap();
    let stderr = String::from_utf8_lossy(&o.stderr);
    let warnings: Vec<&str> = stderr
        .lines()
        .filter(|l| l.contains("warning"))
        .map(|l| l.split(": warning: ").last().unwrap_or(l))
        .collect();
    let _ = std::fs::remove_file(out);
    if stderr.contains("panicked") || o.status.code().is_none_or(|c| c > 2) {
        return ("FAIL-panic".into(), stderr.into_owned());
    }
    if !o.status.success() {
        return ("FAIL-render".into(), stderr.trim().to_string());
    }
    detail.push_str("rendered");
    for w in warnings {
        detail.push_str("; ");
        detail.push_str(w.chars().take(100).collect::<String>().as_str());
    }
    if rbs42 {
        return ("RBS42".into(), detail);
    }
    // a ReBirth 1.x song: the import's messages equal tools/rb1x.py's
    let ours = rbs::rb1x::Song1x::parse(&data)
        .map_err(|e| e.to_string())
        .and_then(|s| rb_engine::load::load_1x(&s).map_err(|e| e.to_string()))
        .map(|l| l.messages(0));
    match (ours, py_1x_messages(path)) {
        (Ok(a), Ok(b)) if a == b => {
            detail.push_str(&format!("; {} messages equal tools/rb1x.py's", a.len()));
            ("RBS1X".into(), detail)
        }
        (Ok(a), Ok(b)) => (
            "FAIL-msgs".into(),
            format!(
                "{} messages, tools/rb1x.py {}; first difference at {}",
                a.len(),
                b.len(),
                a.iter()
                    .zip(&b)
                    .position(|(x, y)| x != y)
                    .unwrap_or(a.len().min(b.len()))
            ),
        ),
        (a, b) => (
            "FAIL-msgs".into(),
            format!("rb {:?}, rb1x.py {:?}", a.err(), b.err()),
        ),
    }
}

/// The load messages of `tools/rb1x.py` for a 1.x song: u32 count, then u32 length + bytes.
fn py_1x_messages(path: &Path) -> Result<Vec<Vec<u8>>, String> {
    const DUMP: &str = r#"
import os, struct, sys
sys.path.insert(0, os.path.join(sys.argv[1], "tools"))
import rb1x
msgs = rb1x.read_song(sys.argv[2]).load_messages()
out = struct.pack("<I", len(msgs))
out += b"".join(struct.pack("<I", len(m)) + m for m in msgs)
sys.stdout.buffer.write(out)
"#;
    let o = python::python()
        .arg("-c")
        .arg(DUMP)
        .arg(root())
        .arg(path)
        .output()
        .map_err(|e| e.to_string())?;
    if !o.status.success() {
        return Err(String::from_utf8_lossy(&o.stderr).into_owned());
    }
    let b = o.stdout;
    let u32le = |p: usize| u32::from_le_bytes(b[p..p + 4].try_into().unwrap()) as usize;
    let mut p = 4;
    let mut out = Vec::new();
    for _ in 0..u32le(0) {
        let n = u32le(p);
        out.push(b[p + 4..p + 4 + n].to_vec());
        p += 4 + n;
    }
    Ok(out)
}
