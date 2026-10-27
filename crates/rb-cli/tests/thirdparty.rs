//! The check of a third-party collection of mods and songs (task-058): the user's own files,
//! read in place and never in the repository. Set `RB_THIRDPARTY_DIR` to a folder with `Mods/`
//! (`.rbm` files) and `songs/` (`.rbs` files, any name), and `RB_RB20FUL` to the CD's
//! RB20FUL.DAT; each test skips with a message when they are unset. Run it in release, with the
//! summary shown:
//!
//! ```text
//! RB_THIRDPARTY_DIR=~/rb_stuff RB_RB20FUL=.../RB20FUL.DAT \
//!     cargo test --release -p rb-cli --test thirdparty -- --nocapture --test-threads 1
//! ```
//!
//! - `mods`: every `.rbm` through `crates/rbm` (parse, key pages, every embedded file, the 46
//!   slots with the built-in fallback), compared with `tools/rbm.py check-dir --json`: the same
//!   SHA-256 for every embedded file and the same channel-0 PCM for every slot.
//! - `songs`: every distinct song (by SHA-256): an RBS 4.2 song is read and written back
//!   (byte-identical) and rendered with `rb render --mod-dir DIR/Mods --standard-kit`, its mod
//!   resolved as the original does; a ReBirth 1.x song (a MIDI file, `MThd`) must be refused
//!   with an error. No song may panic.

use std::collections::{BTreeMap, HashMap};
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::Mutex;

use rb_engine::samples::{self, Sample, SampleSet};
use serde_json::Value;
use sha2::{Digest, Sha256};

fn root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../..")
}

struct Env {
    mods: PathBuf,
    songs: PathBuf,
    dat: PathBuf,
}

fn env() -> Option<Env> {
    let (Some(dir), Some(dat)) = (
        std::env::var_os("RB_THIRDPARTY_DIR").filter(|v| !v.is_empty()),
        std::env::var_os("RB_RB20FUL").filter(|v| !v.is_empty()),
    ) else {
        eprintln!("skipped: RB_THIRDPARTY_DIR and RB_RB20FUL are not both set");
        return None;
    };
    let dir = PathBuf::from(dir);
    Some(Env {
        mods: dir.join("Mods"),
        songs: dir.join("songs"),
        dat: PathBuf::from(dat),
    })
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
    let Some(e) = env() else { return };
    let out = Command::new("/usr/bin/python3")
        .arg("-I")
        .arg(root().join("tools/rbm.py"))
        .arg("check-dir")
        .arg(&e.mods)
        .arg(&e.dat)
        .arg("--json")
        .output()
        .expect("cannot run /usr/bin/python3 for tools/rbm.py");
    let reference: Vec<Value> =
        serde_json::from_slice(&out.stdout).expect("rbm.py check-dir --json output");
    assert!(!reference.is_empty(), "no .rbm in {}", e.mods.display());

    let bi = builtin();
    let (mut ok, mut failed, mut partial, mut files_checked, mut slots_checked) = (0, 0, 0, 0, 0);
    let mut problems = Vec::new();
    for m in &reference {
        let file = m["file"].as_str().unwrap();
        let path = e.mods.join(file);
        let parsed = rbm::Rbm::parse(std::fs::read(&path).unwrap());
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
        let table = rbm::KeyTable::read(&rbm, &mut std::fs::File::open(&e.dat).unwrap())
            .unwrap_or_else(|err| panic!("{file}: {err}"));

        // Every embedded file, in file order.
        let py_files = m["files"].as_array().unwrap();
        let ours: Vec<_> = rbm.embedded().cloned().collect();
        assert_eq!(ours.len(), py_files.len(), "{file}: EMBF count");
        for (c, p) in ours.iter().zip(py_files) {
            let b = rbm.file(c, &table).unwrap();
            if c.name.as_deref() != p["name"].as_str() || hex(&b) != p["sha256"] {
                problems.push(format!("{file}: {:?} differs from rbm.py", c.name));
            }
            files_checked += 1;
        }

        // The 46 slots: the mod's where rbm.py found one, the built-in sample elsewhere.
        let slots = m["slots"].as_object().unwrap();
        match rbm.load_samples(&table, &bi) {
            Err(err) => problems.push(format!("{file}: load_samples: {err}")),
            Ok(l) => {
                for id in samples::FIRST_ID..=samples::LAST_ID {
                    let sname = rbm::slot_name(id).unwrap();
                    match slots.get(sname) {
                        Some(s) if s.get("error").is_none() => {
                            let pcm: Vec<u8> = l
                                .set
                                .get(id)
                                .audio()
                                .iter()
                                .flat_map(|x| x.to_be_bytes())
                                .collect();
                            if !l.is_from_mod(id) || hex(&pcm) != s["sha256"] {
                                problems.push(format!("{file}: slot {sname} differs from rbm.py"));
                            }
                            slots_checked += 1;
                        }
                        Some(s) => problems.push(format!(
                            "{file}: rb loads {sname}, rbm.py says {}",
                            s["error"]
                        )),
                        None => {
                            if l.is_from_mod(id) || l.set.get(id) != bi.get(id) {
                                problems.push(format!("{file}: slot {sname} should be built in"));
                            }
                        }
                    }
                }
            }
        }
        let notes: Vec<&str> = m["notes"]
            .as_array()
            .unwrap()
            .iter()
            .filter_map(Value::as_str)
            .collect();
        if slots.len() < samples::COUNT {
            partial += 1;
        }
        println!(
            "OK   {file}: {:?}, {} files, {}/46 slots{}{}",
            name.unwrap_or_default(),
            ours.len(),
            slots.len(),
            if notes.is_empty() { "" } else { "; " },
            notes.join("; ")
        );
        ok += 1;
    }
    println!(
        "mods: {} checked, {ok} load, {failed} fail; {partial} keep built-in slots; {files_checked} \
         embedded files and {slots_checked} slots equal rbm.py's",
        reference.len()
    );
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
    let mods = info_names(&e.mods);
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

/// (class, detail) of one song: `RBS42`, `RBS1X` (refused as the CLI should), or `FAIL...`.
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
    let o = Command::new(env!("CARGO_BIN_EXE_rb"))
        .arg("render")
        .arg(path)
        .arg("-o")
        .arg(out)
        .arg("--mod-dir")
        .arg(&e.mods)
        .arg("--rb20ful")
        .arg(&e.dat)
        .arg("--rebirth-dll")
        .arg(root().join("original/ReBirth.dll"))
        .arg("--standard-kit")
        .output()
        .unwrap();
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
    if rbs42 {
        if !o.status.success() {
            return ("FAIL-render".into(), stderr.trim().to_string());
        }
        detail.push_str("rendered");
        for w in warnings {
            detail.push_str("; ");
            detail.push_str(w.chars().take(100).collect::<String>().as_str());
        }
        ("RBS42".into(), detail)
    } else if o.status.success() {
        ("FAIL-1x".into(), "a ReBirth 1.x song rendered?".into())
    } else {
        ("RBS1X".into(), "ReBirth 1.x MIDI-file song, refused".into())
    }
}
