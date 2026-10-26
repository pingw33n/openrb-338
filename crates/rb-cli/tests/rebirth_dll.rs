//! `rb render` and the ReBirth.dll that holds the built-in drum samples (task-020): the
//! `--rebirth-dll` flag, the `RB_REBIRTH_DLL` variable, the errors, and 303-only songs that do
//! not need it.

use std::ffi::OsStr;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};

fn root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../..")
}

/// `rb render input -o output extra...` with the environment's `RB_REBIRTH_DLL` removed (or set
/// to `env`), so that only the arguments and the default locations decide.
fn render(input: &Path, output: &Path, extra: &[&OsStr], env: Option<&Path>) -> Output {
    let mut c = Command::new(env!("CARGO_BIN_EXE_rb"));
    c.arg("render").arg(input).arg("-o").arg(output).args(extra);
    match env {
        Some(p) => c.env("RB_REBIRTH_DLL", p),
        None => c.env_remove("RB_REBIRTH_DLL"),
    };
    c.output().unwrap()
}

fn drum_song() -> PathBuf {
    root().join("tests/songs/diag/songs/808_bass.rbs")
}

fn flag() -> &'static OsStr {
    OsStr::new("--rebirth-dll")
}

#[test]
fn drums_need_a_rebirth_dll() {
    let dir = tempfile::tempdir().unwrap();
    let out = dir.path().join("d.wav");

    // --rebirth-dll naming no file.
    let none = dir.path().join("none.dll");
    let o = render(&drum_song(), &out, &[flag(), none.as_os_str()], None);
    let err = String::from_utf8_lossy(&o.stderr);
    assert_eq!(o.status.code(), Some(1), "{err}");
    assert!(
        err.contains("TR-808") && err.contains("no such file"),
        "{err}"
    );
    assert!(!out.exists());

    // A file that is not a ReBirth DLL.
    let junk = dir.path().join("junk.dll");
    std::fs::write(&junk, b"MZ but not really").unwrap();
    let o = render(&drum_song(), &out, &[flag(), junk.as_os_str()], None);
    let err = String::from_utf8_lossy(&o.stderr);
    assert_eq!(o.status.code(), Some(1), "{err}");
    assert!(err.contains("not a ReBirth 2.01"), "{err}");

    // Rebirth.exe is a PE too, without the samples.
    let exe = root().join("original/Rebirth.exe");
    let o = render(&drum_song(), &out, &[flag(), exe.as_os_str()], None);
    let err = String::from_utf8_lossy(&o.stderr);
    assert_eq!(o.status.code(), Some(1), "{err}");
    assert!(err.contains("AIFF"), "{err}");

    // No flag, no variable, no DLL next to the binary: the error says how to give one. (On
    // Windows the machine's own ReBirth installation may be found, so skip it there.)
    if !cfg!(windows) {
        let o = render(&drum_song(), &out, &[], None);
        let err = String::from_utf8_lossy(&o.stderr);
        assert_eq!(o.status.code(), Some(1), "{err}");
        assert!(
            err.contains("no ReBirth.dll found") && err.contains("--rebirth-dll"),
            "{err}"
        );
    }
}

#[test]
fn drums_render_with_the_dll_from_the_flag_or_the_variable() {
    let dir = tempfile::tempdir().unwrap();
    let dll = root().join("original/ReBirth.dll");
    let a = dir.path().join("a.wav");
    let o = render(&drum_song(), &a, &[flag(), dll.as_os_str()], None);
    assert!(o.status.success(), "{}", String::from_utf8_lossy(&o.stderr));

    let b = dir.path().join("b.wav");
    let o = render(&drum_song(), &b, &[], Some(&dll));
    assert!(o.status.success(), "{}", String::from_utf8_lossy(&o.stderr));
    assert_eq!(std::fs::read(&a).unwrap(), std::fs::read(&b).unwrap());

    // A variable naming no file is an error, not a silent fallback.
    let none = dir.path().join("none.dll");
    let o = render(&drum_song(), &b, &[], Some(&none));
    assert_eq!(o.status.code(), Some(1));
    assert!(String::from_utf8_lossy(&o.stderr).contains("RB_REBIRTH_DLL"));
}

/// Trakka Rekka has both drum machines off in their chunks; enable events of their tracks turn
/// them on. They count as playing: the samples are needed and loaded, the TR-808 plays (no
/// "not implemented" warning for it), the TR-909 is reported as not implemented.
#[test]
fn drums_enabled_only_by_track_events_load_the_samples() {
    let dir = tempfile::tempdir().unwrap();
    let out = dir.path().join("t.wav");
    let song = root().join("tests/songs/shipped/Trakka Rekka.rbs");
    let none = dir.path().join("none.dll");
    let o = render(&song, &out, &[flag(), none.as_os_str()], None);
    let err = String::from_utf8_lossy(&o.stderr);
    assert_eq!(o.status.code(), Some(1), "{err}");
    assert!(err.contains("TR-808 and TR-909"), "{err}");

    let dll = root().join("original/ReBirth.dll");
    let o = render(&song, &out, &[flag(), dll.as_os_str()], None);
    let err = String::from_utf8_lossy(&o.stderr);
    assert!(o.status.success(), "{err}");
    let line = err
        .lines()
        .find(|l| l.contains("not implemented yet"))
        .unwrap_or_else(|| panic!("{err}"));
    assert!(
        line.contains("TR-909") && !line.contains("TR-808"),
        "{line}"
    );
}

#[test]
fn a_303_only_song_renders_without_a_dll() {
    let dir = tempfile::tempdir().unwrap();
    let out = dir.path().join("t.wav");
    let song = root().join("tests/songs/diag/songs/303_saw_lowcut.rbs");
    let o = render(&song, &out, &[], None);
    assert!(o.status.success(), "{}", String::from_utf8_lossy(&o.stderr));
    // Even a wrong --rebirth-dll is not looked at when no drum machine plays.
    let none = dir.path().join("none.dll");
    let o = render(&song, &out, &[flag(), none.as_os_str()], None);
    assert!(o.status.success(), "{}", String::from_utf8_lossy(&o.stderr));
}
