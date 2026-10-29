//! The command line of `OpenRB-338` (task-137): the help shows only `[SONG]`, `--help` exits 0, a
//! bad option exits 2; the positional song opens RBS 4.x and ReBirth 1.x songs and paths with
//! spaces, and a bad path is an error (the one File > Open reports). The hidden options' parsing
//! is tested in `src/main.rs`.

use std::path::{Path, PathBuf};
use std::process::Command;

use orb_ui::session;
use rbs::rb1x::{BLOCK_LEN, Block, Event, Kind, Song1x};

fn root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../..")
}

fn run(args: &[&str]) -> std::process::Output {
    Command::new(env!("CARGO_BIN_EXE_OpenRB-338"))
        .args(args)
        .output()
        .unwrap()
}

#[test]
fn help_shows_only_the_song_and_exits_0() {
    for flag in ["--help", "-h"] {
        let o = run(&[flag]);
        assert_eq!(o.status.code(), Some(0), "{flag}");
        let out = String::from_utf8(o.stdout).unwrap();
        assert!(out.contains("usage: OpenRB-338 [SONG]"), "{out}");
        for hidden in [
            "--demo",
            "--size",
            "--ppp",
            "--browse",
            "--screenshot",
            "--about",
        ] {
            assert!(!out.contains(hidden), "{hidden} in {out}");
        }
    }
}

#[test]
fn a_bad_option_exits_2_with_the_same_usage() {
    for args in [&["--nope"][..], &["--size", "10"], &["a.rbs", "b.rbs"]] {
        let o = run(args);
        assert_eq!(o.status.code(), Some(2), "{args:?}");
        let err = String::from_utf8(o.stderr).unwrap();
        assert!(err.contains("usage: OpenRB-338 [SONG]"), "{err}");
        assert!(!err.contains("--demo"), "{err}");
    }
}

/// A minimal ReBirth 1.x song.
fn song_1x() -> Vec<u8> {
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
    let meta = Event {
        time: 0,
        kind: Kind::Meta {
            kind: 2,
            data: b"File version 3.1.0".to_vec(),
        },
    };
    let sysex = Event {
        time: 0,
        kind: Kind::Sysex(Block(b).encode()),
    };
    Song1x {
        format: 1,
        division: 32,
        tracks: vec![vec![meta], vec![sysex], vec![]],
        version: (3, 1),
    }
    .write()
}

#[test]
fn the_song_argument_opens_4x_and_1x_songs_in_a_folder_with_spaces() {
    let tmp = tempfile::tempdir().unwrap();
    let dir = tmp.path().join("my songs");
    std::fs::create_dir_all(&dir).unwrap();
    let four = dir.join("a song 4x.rbs");
    std::fs::copy(root().join("tests/songs/diag/songs/808_snare.rbs"), &four).unwrap();
    let one = dir.join("an old song.rbs");
    std::fs::write(&one, song_1x()).unwrap();

    let o = session::open_song(&four).expect("RBS 4.x");
    assert!(o.facts_1x.is_none());
    let o = session::open_song(&one).expect("ReBirth 1.x");
    assert!(o.facts_1x.is_some());

    // a missing file and a file that is no song: errors naming the path, as File > Open reports
    let missing = dir.join("missing.rbs");
    let e = session::open_song(&missing).err().expect("missing");
    assert!(
        e.contains("missing.rbs") && e.contains("cannot read"),
        "{e}"
    );
    let junk = dir.join("junk.rbs");
    std::fs::write(&junk, b"this is not a song").unwrap();
    let e = session::open_song(&junk).err().expect("junk");
    assert!(e.contains("junk.rbs") && e.contains("not a valid"), "{e}");
}
