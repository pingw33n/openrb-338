//! `rb info` (task-122): the text form, `--json`'s keys, the errors, a ReBirth 1.x song and the
//! mod resolution (gated on `RB_CD_ISO`: the recrypted shipped mods come out of the image, by
//! `rb-testkit`).

mod common;

use std::path::Path;
use std::process::{Command, Output};

use common::root;
use rbs::rb1x::{BLOCK_LEN, Block, Event, Kind, Song1x};
use serde_json::Value;

fn info(args: &[&std::ffi::OsStr]) -> Output {
    Command::new(env!("CARGO_BIN_EXE_rb"))
        .arg("info")
        .args(args)
        // the tests say where the mods are, or that they are nowhere
        .env_remove("RB_MOD_DIR")
        .env_remove("RB_RBM_DIR")
        .output()
        .unwrap()
}

fn stdout(o: &Output) -> String {
    assert!(o.status.success(), "{}", String::from_utf8_lossy(&o.stderr));
    String::from_utf8(o.stdout.clone()).unwrap()
}

fn json(path: &Path, extra: &[&str]) -> Value {
    let mut a: Vec<&std::ffi::OsStr> = vec![path.as_ref(), "--json".as_ref()];
    a.extend(extra.iter().map(|s| std::ffi::OsStr::new(*s)));
    serde_json::from_str(&stdout(&info(&a))).unwrap()
}

#[test]
fn text_of_a_shipped_song() {
    let song = root().join("tests/songs/shipped/Calm.rbs");
    let out = stdout(&info(&[song.as_ref()]));
    let want = format!(
        "File: {}\n\
         Format: RBS 4.1\n\
         Title: it down\n\
         Text:\n  Just another song for calm moments...\n\n  by Florian B\u{f6}mers\n\
         Web page: http://www.bome.com\n\
         Copyright: (c)1997 Propellerhead Software, all rights reserved\n\
         Mod: Standard ReBirth\n\
         Mod resolution: built-in kit\n\
         Mod web: www.propellerheads.se\n\
         Mod ftp: ftp.propellerheads.se\n\
         Tempo: 130.000 BPM\n\
         Shuffle: 64\n\
         Mode: song\n\
         Loop: on, start bar 0, 172 bars\n\
         Synth sound: ReBirth 2.0\n\
         Last event: bar 176\n",
        song.display()
    );
    assert_eq!(out, want);
}

#[test]
fn json_keys_and_values() {
    let song = root().join("tests/songs/diag/songs/303_accent0.rbs");
    let v = json(&song, &[]);
    let mut keys: Vec<&str> = v.as_object().unwrap().keys().map(|k| k.as_str()).collect();
    keys.sort();
    assert_eq!(
        keys,
        [
            "copyright",
            "file",
            "format",
            "last_event_bar",
            "last_event_tick",
            "loop_bars",
            "loop_on",
            "loop_start_bar",
            "mod_file",
            "mod_ftp",
            "mod_name",
            "mod_resolution",
            "mod_web",
            "mode",
            "shuffle",
            "synth_sound",
            "tempo_bpm",
            "text",
            "title",
            "version",
            "web_page",
        ]
    );
    assert_eq!(v["format"], "rbs");
    assert_eq!(v["version"], "4.2");
    assert_eq!(v["mod_name"], "Standard ReBirth");
    assert_eq!(v["mod_resolution"], "builtin");
    assert!(v["mod_file"].is_null());
    assert_eq!(v["tempo_bpm"], 120.0);
    assert_eq!(v["mode"], "song");
    assert_eq!(v["loop_on"], true);
    assert_eq!(v["loop_bars"], 2);
    assert_eq!(v["synth_sound"], "rebirth2");

    // the more text keeps its newlines
    let calm = json(&root().join("tests/songs/shipped/Calm.rbs"), &[]);
    assert_eq!(
        calm["text"],
        "Just another song for calm moments...\n\nby Florian B\u{f6}mers"
    );
    assert_eq!(calm["shuffle"], 64);
}

#[test]
fn the_corpus_has_vintage_and_pattern_mode_songs() {
    // the diag and shipped songs: every one reads, and the facts tell both sounds and both
    // modes apart (each found by its facts, so the test needs no song's name)
    let mut seen = (0, 0, 0);
    for d in ["tests/songs/diag/songs", "tests/songs/shipped"] {
        for e in std::fs::read_dir(root().join(d)).unwrap() {
            let p = e.unwrap().path();
            let v = json(&p, &[]);
            seen.0 += (v["synth_sound"] == "vintage") as u32;
            seen.1 += (v["synth_sound"] == "rebirth2") as u32;
            seen.2 += (v["mode"] == "pattern") as u32;
        }
    }
    assert!(seen.0 > 0, "no song with the vintage sound");
    assert!(seen.1 > 0, "no song with the ReBirth 2.0 sound");
    assert!(seen.2 > 0, "no song in pattern mode");
}

#[test]
fn missing_and_invalid_files_exit_1_with_renders_messages() {
    let dir = tempfile::tempdir().unwrap();
    let missing = dir.path().join("none.rbs");
    let bad = dir.path().join("bad.rbs");
    std::fs::write(&bad, b"this is not a song").unwrap();
    let out = dir.path().join("o.wav");
    for song in [&missing, &bad] {
        let i = info(&[song.as_ref()]);
        assert_eq!(i.status.code(), Some(1));
        assert!(i.stdout.is_empty());
        let r = Command::new(env!("CARGO_BIN_EXE_rb"))
            .args(["render".as_ref(), song.as_os_str(), "-o".as_ref()])
            .arg(&out)
            .output()
            .unwrap();
        assert_eq!(r.status.code(), Some(1));
        assert_eq!(
            String::from_utf8_lossy(&i.stderr),
            String::from_utf8_lossy(&r.stderr)
        );
    }
    let e = String::from_utf8(info(&[missing.as_ref()]).stderr).unwrap();
    assert!(e.contains("cannot read the song"), "{e}");
    let e = String::from_utf8(info(&[bad.as_ref()]).stderr).unwrap();
    assert!(e.contains("not a valid .rbs song"), "{e}");
}

/// A 1.x song: format 1, version text, the song sysex (loop from bar 2, 4 bars, song mode,
/// 128 BPM, title and text), as `rb_engine::load::rb1x`'s tests build one.
fn song_1x() -> Vec<u8> {
    song_1x_with(b"Old song", None)
}

fn song_1x_titled(title: &[u8]) -> Vec<u8> {
    song_1x_with(title, None)
}

/// With `cc_time`, one controller change (the 303 #1's controller 14) at that MIDI time, which
/// is the tick 24 * `cc_time`.
fn song_1x_with(title: &[u8], cc_time: Option<u32>) -> Vec<u8> {
    let mut b = Box::new([0u8; BLOCK_LEN]);
    b[..5].copy_from_slice(&[0xBC, 3, 1, 0, 0]);
    b[5..9].copy_from_slice(b"TinY");
    b[9] = 90;
    b[10..14].copy_from_slice(&2u32.to_be_bytes());
    b[14..18].copy_from_slice(&4u32.to_be_bytes());
    b[0x12] = 1;
    b[0x2A..0x2E].copy_from_slice(&128_000u32.to_be_bytes());
    b[0x2E] = 1;
    for j in 0..3 {
        let r = 0x100 + 32 * j;
        b[r + 2..r + 5].copy_from_slice(&[100, 64, 20]);
        b[r + 7] = 0x3F;
        b[r + 0x17] = 1;
    }
    let put = |b: &mut [u8], off: usize, s: &[u8]| b[off..off + s.len()].copy_from_slice(s);
    put(&mut b[..], 0x3160, title);
    put(&mut b[..], 0x3160 + 41, b"first\rsecond \xe9");
    put(&mut b[..], 0x3160 + 242, b"www.example.org");
    Song1x {
        format: 1,
        division: 32,
        tracks: vec![
            vec![Event {
                time: 0,
                kind: Kind::Meta {
                    kind: 2,
                    data: b"File version 3.1.0".to_vec(),
                },
            }],
            vec![Event {
                time: 0,
                kind: Kind::Sysex(Block(b).encode()),
            }],
            cc_time
                .map(|time| Event {
                    time,
                    kind: Kind::Cc { ctrl: 14, value: 1 },
                })
                .into_iter()
                .collect(),
        ],
        version: (3, 1),
    }
    .write()
}

#[test]
fn a_rebirth_1x_song() {
    let dir = tempfile::tempdir().unwrap();
    let p = dir.path().join("old.rbs");
    std::fs::write(&p, song_1x()).unwrap();
    let v = json(&p, &[]);
    assert_eq!(v["format"], "rb1x");
    assert_eq!(v["version"], "3.1");
    assert_eq!(v["title"], "Old song");
    assert_eq!(v["text"], "first\nsecond \u{e9}");
    assert_eq!(v["web_page"], "www.example.org");
    assert_eq!(v["copyright"], "");
    assert_eq!(v["mod_name"], "Standard ReBirth");
    assert_eq!(v["mod_resolution"], "builtin");
    assert_eq!(v["tempo_bpm"], 128.0);
    assert_eq!(v["mode"], "song");
    assert_eq!(v["loop_on"], true);
    assert_eq!(v["loop_start_bar"], 2);
    assert_eq!(v["loop_bars"], 4);
    // the 1.x import sets the shuffle to 64 and the vintage sound (docs/song_load.md 12.4)
    assert_eq!(v["shuffle"], 64);
    assert_eq!(v["synth_sound"], "vintage");
    let t = stdout(&info(&[p.as_ref()]));
    assert!(
        t.contains("Format: ReBirth 1.x MIDI file (file version 3.1)\n"),
        "{t}"
    );
    assert!(t.contains("Text:\n  first\n  second \u{e9}\n"), "{t}");
    assert!(t.contains("Loop: on, start bar 2, 4 bars\n"), "{t}");
}

#[test]
fn mod_resolution_without_and_with_a_folder() {
    let song = root().join("tests/songs/shipped/Alloy Cowboy Resurrection.rbs");
    // no folder: not checked
    let v = json(&song, &[]);
    assert_eq!(v["mod_name"], "Pitch Black Edition 2.0");
    assert_eq!(v["mod_resolution"], "not_checked");
    let t = stdout(&info(&[song.as_ref()]));
    assert!(
        t.contains("Mod resolution: not checked (no --mod-dir)\n"),
        "{t}"
    );

    // an empty folder: missing
    let empty = tempfile::tempdir().unwrap();
    let v = json(&song, &["--mod-dir", empty.path().to_str().unwrap()]);
    assert_eq!(v["mod_resolution"], "missing");
    assert!(v["mod_file"].is_null());

    // a folder that does not exist is an error, as in render
    let o = info(&[
        song.as_ref(),
        "--mod-dir".as_ref(),
        empty.path().join("nope").as_os_str(),
    ]);
    assert_eq!(o.status.code(), Some(1));
}

#[test]
fn mod_resolved_in_rb_mod_dir() {
    let Some(dir) = rb_testkit::recrypted_mods().map(Path::to_path_buf) else {
        return;
    };
    let song = root().join("tests/songs/shipped/Alloy Cowboy Resurrection.rbs");
    let want = dir.join("PBE2.rbm");
    if !want.is_file() {
        eprintln!("skipped: {} is not there", want.display());
        return;
    }
    // the environment alone, then --mod-dir
    let o = Command::new(env!("CARGO_BIN_EXE_rb"))
        .args(["info".as_ref(), song.as_os_str(), "--json".as_ref()])
        .env("RB_MOD_DIR", &dir)
        .output()
        .unwrap();
    let v: Value = serde_json::from_str(&stdout(&o)).unwrap();
    assert_eq!(v["mod_resolution"], "file");
    assert_eq!(v["mod_file"], want.display().to_string());
    let v = json(&song, &["--mod-dir", dir.to_str().unwrap()]);
    assert_eq!(v["mod_resolution"], "file");
    assert_eq!(v["mod_file"], want.display().to_string());
    let t = stdout(&info(&[
        song.as_ref(),
        "--mod-dir".as_ref(),
        dir.as_os_str(),
    ]));
    assert!(
        t.contains(&format!("Mod resolution: file {}\n", want.display())),
        "{t}"
    );
}

#[test]
fn control_characters_of_the_song_are_escaped_in_the_text() {
    let dir = tempfile::tempdir().unwrap();
    let p = dir.path().join("esc.rbs");
    std::fs::write(&p, song_1x_titled(b"\x1b[2Jhi\x9b")).unwrap();
    let t = stdout(&info(&[p.as_ref()]));
    assert!(t.contains("Title: \\x1b[2Jhi\\x9b\n"), "{t:?}");
    assert!(!t.contains('\x1b') && !t.contains('\u{9b}'), "{t:?}");
    // the line breaks of the text stay
    assert!(t.contains("Text:\n  first\n  second"), "{t:?}");
    // --json keeps the characters (JSON escapes them itself)
    assert_eq!(json(&p, &[])["title"], "\u{1b}[2Jhi\u{9b}");
}

#[test]
fn last_event_of_a_corpus_song_a_song_without_events_and_a_1x_song() {
    // Calm's latest event is at tick 134712 (5613 32nd notes; checked with tools/rbs.py):
    // 134712 / 768 = 175.4, so bar 176 counted from 1
    let calm = json(&root().join("tests/songs/shipped/Calm.rbs"), &[]);
    assert_eq!(calm["last_event_tick"], 134712);
    assert_eq!(calm["last_event_bar"], 176);

    // a song with no events: none, and null
    let dir = tempfile::tempdir().unwrap();
    let empty = dir.path().join("empty.rbs");
    std::fs::write(&empty, rbs::Song::empty().write().unwrap()).unwrap();
    let v = json(&empty, &[]);
    assert!(v["last_event_bar"].is_null());
    assert!(v["last_event_tick"].is_null());
    let t = stdout(&info(&[empty.as_ref()]));
    assert!(t.ends_with("Last event: none\n"), "{t}");

    // a 1.x song: the import always puts an event at tick 0 (the 909's, docs/song_load.md
    // 12.2), so bar 1; a controller change at MIDI time 70 is the tick 1680, in bar 3
    let old = dir.path().join("old.rbs");
    std::fs::write(&old, song_1x()).unwrap();
    assert_eq!(json(&old, &[])["last_event_bar"], 1);
    std::fs::write(&old, song_1x_with(b"x", Some(70))).unwrap();
    let v = json(&old, &[]);
    assert_eq!(v["last_event_tick"], 1680);
    assert_eq!(v["last_event_bar"], 3);
    let t = stdout(&info(&[old.as_ref()]));
    assert!(t.ends_with("Last event: bar 3\n"), "{t}");
}
