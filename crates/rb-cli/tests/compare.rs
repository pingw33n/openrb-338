//! `rb compare` end to end (task-029): one song against a FLAC or a WAV, manifest entries, the
//! first difference, and the exit codes.

mod common;

use std::ffi::OsStr;
use std::path::PathBuf;
use std::process::{Command, Output};

use common::{root, standard_mod};

fn rb(args: &[&OsStr]) -> Output {
    Command::new(env!("CARGO_BIN_EXE_rb"))
        .arg("compare")
        .args(args)
        .arg("--standard-mod")
        .arg(standard_mod())
        .output()
        .unwrap()
}

fn out(o: &Output) -> String {
    format!(
        "{}{}",
        String::from_utf8_lossy(&o.stdout),
        String::from_utf8_lossy(&o.stderr)
    )
}

fn manifest() -> PathBuf {
    root().join("tests/refs/manifest.json")
}

#[test]
fn a_song_equals_its_flac_and_its_wav() {
    let song = root().join("tests/songs/diag/songs/808_snare.rbs");
    let flac = root().join("tests/refs/diag/808_snare.flac");
    let o = rb(&[song.as_ref(), "--reference".as_ref(), flac.as_ref()]);
    assert!(o.status.success(), "{}", out(&o));
    assert!(out(&o).contains("808_snare.rbs: EQUAL"), "{}", out(&o));

    // the WAV rb render writes is the reference for itself, byte for byte
    let dir = tempfile::tempdir().unwrap();
    let wav = dir.path().join("s.wav");
    let r = Command::new(env!("CARGO_BIN_EXE_rb"))
        .arg("render")
        .arg(&song)
        .arg("-o")
        .arg(&wav)
        .arg("--standard-mod")
        .arg(standard_mod())
        .output()
        .unwrap();
    assert!(r.status.success(), "{}", out(&r));
    let o = rb(&[song.as_ref(), "--reference".as_ref(), wav.as_ref()]);
    assert!(o.status.success(), "{}", out(&o));

    // another idle count differs: the first difference with time, bar, channel and values
    let o = rb(&[
        song.as_ref(),
        "--reference".as_ref(),
        wav.as_ref(),
        "--idle-blocks".as_ref(),
        "0".as_ref(),
    ]);
    assert_eq!(o.status.code(), Some(1), "{}", out(&o));
    let s = out(&o);
    assert!(
        s.contains("DIFFERENT (N 0): first difference at frame "),
        "{s}"
    );
    assert!(
        s.contains(" s, bar ") && s.contains("channel ") && s.contains("ref "),
        "{s}"
    );
}

#[test]
fn manifest_entries_by_name_and_set() {
    let m = manifest();
    let o = rb(&[
        "--manifest".as_ref(),
        m.as_ref(),
        "--set".as_ref(),
        "diag,app".as_ref(),
        "808_snare".as_ref(),
        "diag/full_mix".as_ref(),
    ]);
    assert!(o.status.success(), "{}", out(&o));
    let s = out(&o);
    for l in [
        "diag/808_snare: EQUAL",
        "app/808_snare: EQUAL",
        "diag/full_mix: EQUAL",
        "3 of 3 equal",
    ] {
        assert!(s.contains(l), "{l}: {s}");
    }
}

/// The app export of `808_snare` matches at its fitted N (9040), not at the corpus' 9088: the
/// report names the first difference, and the exit code is 1.
#[test]
fn an_app_export_at_another_n_differs() {
    let m = manifest();
    let o = rb(&[
        "--manifest".as_ref(),
        m.as_ref(),
        "--set".as_ref(),
        "app".as_ref(),
        "808_snare".as_ref(),
        "--idle-blocks".as_ref(),
        "9088".as_ref(),
    ]);
    assert_eq!(o.status.code(), Some(1), "{}", out(&o));
    let s = out(&o);
    assert!(
        s.contains("app/808_snare: DIFFERENT (N 9088): first difference at frame "),
        "{s}"
    );
    assert!(s.contains("0 of 1 equal"), "{s}");
}

#[test]
fn usage_errors_exit_2() {
    let song = root().join("tests/songs/diag/songs/silence.rbs");
    let m = manifest();
    assert_eq!(rb(&[]).status.code(), Some(2));
    assert_eq!(rb(&[song.as_ref()]).status.code(), Some(2));
    let o = rb(&["--manifest".as_ref(), m.as_ref(), "no_such_song".as_ref()]);
    assert_eq!(o.status.code(), Some(2), "{}", out(&o));
    let o = rb(&[
        "--manifest".as_ref(),
        m.as_ref(),
        "--set".as_ref(),
        "x".as_ref(),
    ]);
    assert_eq!(o.status.code(), Some(2), "{}", out(&o));
    let o = rb(&[song.as_ref(), "--reference".as_ref(), song.as_ref()]);
    assert_eq!(o.status.code(), Some(2), "{}", out(&o));
}
