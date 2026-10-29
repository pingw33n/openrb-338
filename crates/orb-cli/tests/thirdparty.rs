//! The check of a third-party collection of mods and songs (task-058, task-121): the user's own
//! files, read in place and never in the repository. Two variables name them, and each test
//! skips with a message when its input is not set:
//!
//! - `ORB_THIRDPARTY_MODS_DIR`: a folder of the RECRYPTED `.rbm` files (`orb mod recrypt`;
//!   task-063), for the `mods` test, and for the `songs` test's mod resolution (the `songs` test
//!   fails without it when its manifest lists a song with a third-party mod);
//! - `ORB_THIRDPARTY_SONGS_DIR`: a folder of songs (every file under it, recursively, whose name
//!   contains `.rbs` in any case), for the `songs` test.
//!
//! Run it in release, with the summary shown (`songs` renders over 1200 songs on all cores: about
//! 2 minutes):
//!
//! ```text
//! ORB_THIRDPARTY_MODS_DIR=~/rb-mods/mods ORB_THIRDPARTY_SONGS_DIR=~/rb-songs/songs \
//!     cargo test --release -p orb-cli --test thirdparty -- --nocapture --test-threads 1
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
//! - `songs`: every distinct song (by SHA-256) against the committed manifest
//!   `tests/refs/thirdparty_songs.json` (task-129): the SHA-256 of rbhost's render of each song,
//!   the original ReBirth.dll on the test machine, with the mod the original resolves. A song file
//!   the manifest does not list (by its SHA-256 and its path under the folder) fails the test. An
//!   RBS song is read and written back (byte-identical, or re-read equal) and rendered with `orb
//!   render --idle-blocks N --mod-dir DIR --standard-kit` (N from the manifest, 9088; DIR is
//!   `ORB_THIRDPARTY_MODS_DIR`; without it every listed song with a third-party mod is a failure
//!   naming the variable, so a green run has compared every listed song); a ReBirth 1.x song (a
//!   MIDI file, `MThd`, task-060) is rendered at idle 0, and its import's message stream
//!   (`orb_engine::load::load_1x`) must equal `tools/rb1x.py`'s. The render's SHA-256 must equal
//!   the manifest's. A song the manifest lists as skipped (one the original refuses) is only
//!   listed. No song may panic. To regenerate the manifest when the collection changes (it needs
//!   the test machine): `tools/rbhost/thirdparty_songs.py --songs SONGS --mods MODS --rb20ful
//!   RB20FUL.DAT` (resumable; the paths are its arguments, not variables of the test).

mod common;
#[path = "../../../tests/common/python.rs"]
mod python;

use std::collections::{BTreeMap, HashMap};
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::Mutex;

use common::root;
use orb_engine::samples::{self, Sample, SampleSet};
use serde_json::Value;
use sha2::{Digest, Sha256};

/// The inputs of the `songs` test.
struct Env {
    /// The recrypted mods: what orb loads, when the folder is given.
    mods: Option<PathBuf>,
    songs: PathBuf,
}

fn env() -> Option<Env> {
    let songs = orb_testkit::thirdparty_songs()?;
    let mods = common::env_path("ORB_THIRDPARTY_MODS_DIR");
    if mods.is_none() {
        eprintln!(
            "note: ORB_THIRDPARTY_MODS_DIR is not set: the songs' mods are not resolved, so \
             the songs test fails on every song of the manifest that plays one"
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
    let Some(dir) = orb_testkit::thirdparty_mods() else {
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
                println!("FAIL {file}: orb: {err}; rbm.py: {pe}");
                failed += 1;
                continue;
            }
            (Err(err), None) => {
                problems.push(format!("{file}: orb fails ({err}), rbm.py loads it"));
                continue;
            }
            (Ok(r), _) => r,
        };
        let name = rbm.info().map(|i| i.name.clone());
        if let Some(pe) = py_err {
            if name.as_deref().is_some_and(|n| !n.is_empty()) {
                problems.push(format!("{file}: rbm.py fails ({pe}), orb loads it"));
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
                            problems.push(format!("{file}: orb loads {sname}, rbm.py says {why}"))
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
    // the same order and names as orb itself uses (`mods::scan`, folder_order)
    orb_cli::mods::scan(dir)
        .unwrap_or_else(|_| panic!("cannot read {}", dir.display()))
        .into_iter()
        .map(|(p, name)| (p.file_name().unwrap().to_string_lossy().into_owned(), name))
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
    // orb (task-150): the INFO name ignoring ASCII case, never a file name
    if let Some((f, n)) = mods
        .iter()
        .find(|(_, n)| n.as_deref().is_some_and(|n| n.eq_ignore_ascii_case(name)))
    {
        return format!(
            "INFO name differs in case = {f} ({:?})",
            n.as_deref().unwrap()
        );
    }
    "missing".into()
}

/// The song files under `dir`, recursively (a file whose name contains `.rbs`, any case), as
/// paths relative to `dir` with `/` separators, sorted; as `tools/rbhost/thirdparty_songs.py`.
fn song_files(dir: &Path) -> Vec<String> {
    fn walk(dir: &Path, rel: &str, out: &mut Vec<String>) {
        for e in std::fs::read_dir(dir).unwrap() {
            let e = e.unwrap();
            let name = e.file_name().to_string_lossy().into_owned();
            let r = if rel.is_empty() {
                name.clone()
            } else {
                format!("{rel}/{name}")
            };
            let ft = e.file_type().unwrap();
            if ft.is_dir() {
                walk(&e.path(), &r, out);
            } else if e.path().is_file() && name.to_lowercase().contains(".rbs") {
                out.push(r);
            }
        }
    }
    let mut out = Vec::new();
    walk(dir, "", &mut out);
    out.sort();
    out
}

/// The rbhost manifest `tests/refs/thirdparty_songs.json` (task-129): SHA-256 to the entry of
/// `songs` (rendered) or `skipped` (with the reason), and whether it was skipped.
fn songs_manifest() -> HashMap<String, (Value, bool)> {
    let m: Value = serde_json::from_slice(
        &std::fs::read(root().join("tests/refs/thirdparty_songs.json")).unwrap(),
    )
    .expect("tests/refs/thirdparty_songs.json");
    let mut out = HashMap::new();
    for (key, skipped) in [("songs", false), ("skipped", true)] {
        for e in m[key].as_array().unwrap() {
            out.insert(
                e["sha256"].as_str().unwrap().to_string(),
                (e.clone(), skipped),
            );
        }
    }
    out
}

#[test]
fn songs() {
    let Some(e) = env() else { return };
    let mods = e.mods.as_deref().map(info_names).unwrap_or_default();
    let manifest = songs_manifest();
    let mut seen: HashMap<String, PathBuf> = HashMap::new();
    let mut unique = Vec::new();
    let (mut dups, mut files) = (0, 0);
    let mut unlisted = Vec::new();
    for rel in song_files(&e.songs) {
        files += 1;
        let p = e.songs.join(&rel);
        let h = hex(&std::fs::read(&p).unwrap());
        let listed = manifest.get(&h).is_some_and(|(m, _)| {
            m["files"]
                .as_array()
                .unwrap()
                .iter()
                .any(|f| f.as_str() == Some(rel.as_str()))
        });
        if !listed {
            unlisted.push(rel.clone());
        }
        if seen.contains_key(&h) {
            dups += 1;
        } else {
            seen.insert(h.clone(), p.clone());
            unique.push((h, rel, p));
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
                    let Some((h, rel, p)) = unique.get(i) else {
                        break;
                    };
                    let r = match manifest.get(h) {
                        Some((m, true)) => (
                            "SKIPPED".to_string(),
                            format!("rbhost: {}", m["reason"].as_str().unwrap_or("?")),
                        ),
                        entry => check_song(
                            &e,
                            &mods,
                            p,
                            &tmp.path().join(format!("{h}.wav")),
                            entry.map(|(m, _)| m),
                        ),
                    };
                    results.lock().unwrap().insert(rel.clone(), r);
                }
            });
        }
    });

    let results = results.into_inner().unwrap();
    let mut classes: BTreeMap<String, usize> = BTreeMap::new();
    let mut problems = Vec::new();
    let (mut equal, mut not_compared, mut need_mods) = (0, 0, 0);
    for (label, (class, detail)) in &results {
        println!("{class:<10} {label}: {detail}");
        *classes.entry(class.clone()).or_default() += 1;
        if class == "FAIL-mods" {
            need_mods += 1;
        } else if class.starts_with("FAIL") {
            problems.push(format!("{label}: {detail}"));
        }
        if detail.contains(EQUAL) {
            equal += 1;
        }
        if detail.contains(NOT_COMPARED) {
            not_compared += 1;
        }
    }
    println!(
        "songs: {files} files with .rbs in the name, {dups} duplicates, {} distinct: \
         {classes:?}; {equal} renders equal rbhost's (tests/refs/thirdparty_songs.json), \
         {not_compared} not compared",
        results.len()
    );
    if need_mods > 0 {
        problems.push(format!(
            "{need_mods} songs of tests/refs/thirdparty_songs.json play a third-party mod and \
             were not compared with rbhost's render: set ORB_THIRDPARTY_MODS_DIR to the folder \
             of the recrypted third-party mods"
        ));
    }
    if !unlisted.is_empty() {
        problems.push(format!(
            "{} song files of the folder are not in tests/refs/thirdparty_songs.json (regenerate \
             it with tools/rbhost/thirdparty_songs.py): {}",
            unlisted.len(),
            unlisted.join(", ")
        ));
    }
    assert!(problems.is_empty(), "{}", problems.join("\n"));
}

const EQUAL: &str = "equals rbhost's render";
const NOT_COMPARED: &str = "not compared with rbhost's render";

/// (class, detail) of one song: `RBS42`, `RBS1X` (rendered, and its import messages equal
/// `tools/rb1x.py`'s), or `FAIL...`; with its manifest `entry`, rendered with the entry's
/// settings and `FAIL-hash` unless the WAV's SHA-256 equals rbhost's.
fn check_song(
    e: &Env,
    mods: &[(String, Option<String>)],
    path: &Path,
    out: &Path,
    entry: Option<&Value>,
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
    let mut cmd = Command::new(env!("CARGO_BIN_EXE_orb"));
    cmd.arg("render").arg(path).arg("-o").arg(out);
    if let Some(n) = entry.and_then(|m| m["idle_blocks"].as_u64()) {
        cmd.arg("--idle-blocks").arg(n.to_string());
    }
    // A song whose mod no INFO name of the folder has exactly plays the built-in kit in the
    // original (the Standard choice of its dialog), but orb's --mod-dir ignores ASCII case
    // ("omega" finds Omega.rbm, INFO name "Omega"; docs/rbm.md 10, task-150): such a song is
    // rendered without the folder, as the manifest's rbhost render was.
    let missing = entry
        .is_some_and(|m| m["mod"].is_null() && m["mod_name"].as_str() != Some("Standard ReBirth"));
    if let (true, true, Some(mods)) = (missing, rbs42, &e.mods) {
        // and with the folder orb does load the mod that differs in case only
        let o = Command::new(env!("CARGO_BIN_EXE_orb"))
            .args(["info", "--json", "--mod-dir"])
            .arg(mods)
            .arg(path)
            .output()
            .unwrap();
        let j: Value = serde_json::from_slice(&o.stdout).unwrap();
        let name = j["mod_name"].as_str().unwrap_or_default();
        let want = info_names(mods)
            .into_iter()
            .find(|(_, n)| n.as_deref().is_some_and(|n| n.eq_ignore_ascii_case(name)));
        let file = j["mod_file"]
            .as_str()
            .map(|f| f.rsplit(['/', '\\']).next().unwrap());
        if want.as_ref().map(|(f, _)| f.as_str()) != file {
            return (
                "FAIL-mod-lookup".into(),
                format!("orb info resolves {name:?} to {file:?}, expected {want:?}"),
            );
        }
    }
    match &e.mods {
        Some(mods) if !missing => cmd.arg("--mod-dir").arg(mods),
        // no mod folder: the mod of the child's own environment must not matter
        _ => cmd.env_remove("ORB_MOD_DIR"),
    };
    let o = cmd.arg("--standard-kit").output().unwrap();
    let stderr = String::from_utf8_lossy(&o.stderr);
    let warnings: Vec<&str> = stderr
        .lines()
        .filter(|l| l.contains("warning"))
        .map(|l| l.split(": warning: ").last().unwrap_or(l))
        .collect();
    let wav = std::fs::read(out).ok();
    let _ = std::fs::remove_file(out);
    if stderr.contains("panicked") || o.status.code().is_none_or(|c| c > 2) {
        return ("FAIL-panic".into(), stderr.into_owned());
    }
    if !o.status.success() {
        return ("FAIL-render".into(), stderr.trim().to_string());
    }
    detail.push_str("rendered");
    if let Some(m) = entry {
        let ours = hex(&wav.unwrap_or_default());
        let theirs = m["render_sha256"].as_str().unwrap();
        if m["mod"].is_string() && e.mods.is_none() {
            // a green run means every listed song was compared
            return (
                "FAIL-mods".into(),
                format!(
                    "{detail}; its mod {} needs ORB_THIRDPARTY_MODS_DIR (the folder of the \
                     recrypted third-party mods): not compared with rbhost's render",
                    m["mod"]
                ),
            );
        } else if ours == theirs {
            detail.push_str(&format!("; {EQUAL}"));
        } else {
            return (
                "FAIL-hash".into(),
                format!(
                    "{detail}; orb's render {ours} differs from rbhost's {theirs} (settings: {}, \
                     idle {}, mod {})",
                    m["form"], m["idle_blocks"], m["mod"]
                ),
            );
        }
    } else {
        detail.push_str(&format!("; {NOT_COMPARED}: not in the manifest"));
    }
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
        .and_then(|s| orb_engine::load::load_1x(&s).map_err(|e| e.to_string()))
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
            format!("orb {:?}, rb1x.py {:?}", a.err(), b.err()),
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
