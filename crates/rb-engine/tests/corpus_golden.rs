//! The golden test (task-029): every entry of the reference corpus (`tests/refs/manifest.json`)
//! rendered by the engine and compared with its reference by the pass criterion of task-013, a
//! byte-exact WAV (the sha256 of the whole file, header included, against `wav_sha256`).
//!
//! - `diag`: the 145 diagnostic songs, rbhost at 9088 idle blocks, FLAC references.
//! - `app`: the 11 exports of the original app itself, each at the N at which rbhost reproduces
//!   it (`docs/export_state.md` section 13), FLAC references. The strongest check: no host of
//!   ours between the original and the reference.
//! - `shipped`: the 8 shipped songs that use the built-in kit ("Standard ReBirth"), rbhost at
//!   9088, sha256 and frame count only (no audio committed). They render 41 minutes of audio
//!   (108170728 frames), measured at 1.5 s in a release build but 24 s in a debug one (10 cores;
//!   the longest song, TGV's KiloMix '98 at 15 minutes, runs on one thread), so the test is
//!   ignored in debug builds: `cargo test --release` runs it, as does `cargo test -- --ignored`.
//!
//! Out of this file: the 8 shipped songs that use another mod than "Standard ReBirth" (the sets
//! `mod`, rbhost with the mod, and `modapp`, 3 app exports; task-031) are checked by
//! `crates/rb-cli/tests/corpus_mods.rs` (task-033), which needs the `rbm` crate and the user's
//! recrypted `.rbm` files (`RB_MOD_DIR`; task-063). The two pattern-mode Default Songs
//! cannot be exported (`docs/test_machine.md` section 8).
//! `shipped_songs_out_of_the_corpus_are_mod_or_pattern_mode` checks that these are the only songs
//! left out of `shipped`, and that each modded one is in `mod`.
//!
//! Every song renders in a fresh engine with every device the engine has
//! ([`Devices::implemented`]) and the built-in samples of `original/ReBirth.dll`, as rbhost
//! renders each reference in a fresh process. On failure each failing song is printed with its
//! first difference: the frame, its time (s) and bar (from the song's tempo, a bar is four
//! beats), the channel and both sample values, from the decoded FLAC reference. `rb compare`
//! prints the same for a song or a whole manifest.
//!
//! The device tests (`corpus_tb303.rs` ...) keep their controls, which prove each device is
//! audible in its songs; their own reference comparisons were the same renders against the same
//! references as this test, and are gone.

mod common;

use std::sync::Arc;
use std::sync::Mutex;
use std::sync::atomic::{AtomicUsize, Ordering};

use common::{manifest, root, samples, sha256, song};
use rb_engine::compare::{describe, first_difference};
use rb_engine::frame::export::{self, ExportRange};
use rb_engine::frame::{Devices, Engine};
use rb_engine::samples::SampleSet;
use rb_engine::tables::Tables;
use serde_json::Value;

/// The int16 interleaved samples of a FLAC reference under `tests/refs/`.
fn flac_samples(rel: &str) -> Vec<i16> {
    let p = root().join("tests/refs").join(rel);
    let mut r = claxon::FlacReader::open(&p).unwrap_or_else(|e| panic!("{}: {e}", p.display()));
    let info = r.streaminfo();
    assert_eq!(
        (info.channels, info.sample_rate, info.bits_per_sample),
        (2, 44100, 16),
        "{}",
        p.display()
    );
    r.samples().map(|s| s.unwrap() as i16).collect()
}

/// `None` when the entry's render equals its reference, else what differs.
fn check(name: &str, e: &Value, tables: &Arc<Tables>, set: &Arc<SampleSet>) -> Option<String> {
    let s = song(e);
    let idle = e["idle_blocks"].as_u64().unwrap() as u32;
    let mut eng = Engine::new(tables.clone(), Devices::implemented(Some(set.clone())));
    let got = export::export_song(&mut eng, &s, idle).unwrap();
    let wav = export::wav_bytes(&got);
    if sha256(&wav) == e["wav_sha256"].as_str().unwrap() {
        return None;
    }
    let range = ExportRange::from(&s.globals);
    let what = match e["flac"].as_str() {
        Some(flac) => match first_difference(&got, &flac_samples(flac)) {
            Some(d) => describe(d, range),
            None => "the samples equal the FLAC's but the sha256 differs from wav_sha256 (the \
                     manifest and the FLAC disagree: tools/rbcorpus.py check-flac)"
                .to_string(),
        },
        None => format!(
            "sha256 differs ({} frames, reference {}); no audio committed, so no first \
             difference: re-capture the WAV (tools/rbhost/run.py) and run rb compare SONG \
             --reference WAV",
            got.len() / 2,
            e["frames"]
        ),
    };
    Some(format!("{name} (N {idle}): {what}"))
}

/// Checks every entry of the manifest's `set` (in parallel, one engine per song) and fails with
/// the first difference of each failing song. Returns the number checked.
fn check_set(set_name: &str) -> usize {
    let m = manifest();
    let entries: Vec<(&String, &Value)> = m[set_name].as_object().unwrap().iter().collect();
    let tables = Arc::new(Tables::new());
    let set = samples();
    let next = AtomicUsize::new(0);
    let failures = Mutex::new(Vec::new());
    let threads = std::thread::available_parallelism().map_or(4, |n| n.get());
    std::thread::scope(|sc| {
        for _ in 0..threads.min(entries.len()) {
            sc.spawn(|| {
                loop {
                    let i = next.fetch_add(1, Ordering::Relaxed);
                    let Some((name, e)) = entries.get(i) else {
                        break;
                    };
                    if let Some(f) = check(name, e, &tables, &set) {
                        failures.lock().unwrap().push(f);
                    }
                }
            });
        }
    });
    let mut failures = failures.into_inner().unwrap();
    failures.sort();
    assert!(
        failures.is_empty(),
        "{} of {} {set_name} songs differ from their references:\n{}",
        failures.len(),
        entries.len(),
        failures.join("\n")
    );
    entries.len()
}

/// AC #1, #2: the 145 diagnostic songs at 9088 idle blocks.
#[test]
fn diag_songs_match_their_references() {
    assert_eq!(check_set("diag"), 145);
}

/// AC #1, #2: the 11 exports of the original app, each at its own fitted N.
#[test]
fn app_exports_match_at_their_fitted_n() {
    let m = manifest();
    let ns: Vec<u64> = m["app"]
        .as_object()
        .unwrap()
        .values()
        .map(|e| e["idle_blocks"].as_u64().unwrap())
        .collect();
    // not all at one N: the test covers the idle count as a parameter
    assert!(ns.iter().any(|&n| n != 9088), "{ns:?}");
    assert_eq!(check_set("app"), 11);
}

/// AC #1, #2: the 8 shipped songs that use the built-in kit (sha256 and frames only).
#[test]
#[cfg_attr(
    debug_assertions,
    ignore = "41 minutes of audio, 24 s in a debug build; runs in cargo test --release"
)]
fn shipped_songs_match_their_references() {
    assert_eq!(check_set("shipped"), 8);
}

/// The shipped songs that are not in the corpus are exactly the ones out of m-1's scope: those
/// that use another mod than "Standard ReBirth" (m-2), and those in pattern mode, which the
/// original cannot export.
#[test]
fn shipped_songs_out_of_the_corpus_are_mod_or_pattern_mode() {
    let m = manifest();
    let shipped = m["shipped"].as_object().unwrap();
    let dir = root().join("tests/songs/shipped");
    let (mut modded, mut pattern) = (Vec::new(), Vec::new());
    for f in std::fs::read_dir(&dir).unwrap() {
        let p = f.unwrap().path();
        if p.extension().is_none_or(|x| x != "rbs") {
            continue;
        }
        let name = p.file_stem().unwrap().to_str().unwrap().to_string();
        let s = rb_engine::load::convert(&rbs::Song::parse(&std::fs::read(&p).unwrap()).unwrap())
            .unwrap();
        let in_corpus = shipped.contains_key(&name);
        let standard = s.mod_name == "Standard ReBirth";
        let exportable = s.globals.song_mode != 0;
        assert_eq!(in_corpus, standard && exportable, "{name}");
        // task-031: the mod songs have rbhost references of their own, in the set `mod`
        assert_eq!(
            m["mod"].as_object().unwrap().contains_key(&name),
            !standard,
            "{name}"
        );
        if !standard {
            modded.push(name);
        } else if !exportable {
            pattern.push(name);
        }
    }
    modded.sort();
    pattern.sort();
    assert_eq!(
        modded,
        [
            "Alloy Cowboy Resurrection",
            "Clodhopper",
            "Fonda Workout",
            "Groove It Up a 2nd Time!",
            "Metallicon City",
            "Mutilated Species II",
            "RSI",
            "SuperNova",
        ]
    );
    assert_eq!(pattern, ["ReBirth 1.0 Default Song", "Silent Default Song"]);
}
