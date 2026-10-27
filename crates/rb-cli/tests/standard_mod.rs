//! `rb standard-mod` and the standard mod that holds the built-in drum samples (task-036; the
//! ReBirth.dll of task-020 before it): the command, the `--standard-mod` flag, the
//! `RB_STANDARD_MOD` variable, the mod folder, the errors, and 303-only songs that need none.

mod common;

use std::ffi::OsStr;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};

use common::{root, standard_mod};

/// `rb render input -o output extra...` with the environment's `RB_STANDARD_MOD` and mod
/// folder removed (or `RB_STANDARD_MOD` set to `env`), so that only the arguments and the
/// default locations decide.
fn render(input: &Path, output: &Path, extra: &[&OsStr], env: Option<&Path>) -> Output {
    let mut c = Command::new(env!("CARGO_BIN_EXE_rb"));
    c.arg("render").arg(input).arg("-o").arg(output).args(extra);
    c.env_remove("RB_MOD_DIR").env_remove("RB_RBM_DIR");
    match env {
        Some(p) => c.env("RB_STANDARD_MOD", p),
        None => c.env_remove("RB_STANDARD_MOD"),
    };
    c.output().unwrap()
}

fn standard(args: &[&OsStr]) -> Output {
    Command::new(env!("CARGO_BIN_EXE_rb"))
        .arg("standard-mod")
        .args(args)
        .output()
        .unwrap()
}

fn drum_song() -> PathBuf {
    root().join("tests/songs/diag/songs/808_bass.rbs")
}

fn flag() -> &'static OsStr {
    OsStr::new("--standard-mod")
}

/// Whether a standard mod sits next to the rb binary (a developer's `rb standard-mod` without
/// `-o`): then the "none found" cases cannot be seen.
fn default_exists() -> bool {
    Path::new(env!("CARGO_BIN_EXE_rb"))
        .with_file_name("Standard.rbm")
        .exists()
}

/// A mod with no files whose INFO menu name is `name`.
fn empty_mod(name: &str) -> Vec<u8> {
    let mut b = b"CAT \0\0\0\0PRBM".to_vec();
    b.extend(b"INFO");
    b.extend(1280u32.to_be_bytes());
    let mut info = vec![0u8; 1280];
    info[1003..1003 + name.len()].copy_from_slice(name.as_bytes());
    b.extend(info);
    let size = (b.len() - 8) as u32;
    b[4..8].copy_from_slice(&size.to_be_bytes());
    b
}

#[test]
fn the_command_writes_the_standard_mod() {
    let dir = tempfile::tempdir().unwrap();
    let out = dir.path().join("s.rbm");
    let dll = root().join("original/ReBirth.dll");
    let o = standard(&[
        "--rebirth-dll".as_ref(),
        dll.as_os_str(),
        "-o".as_ref(),
        out.as_os_str(),
    ]);
    let err = String::from_utf8_lossy(&o.stderr);
    assert!(o.status.success(), "{err}");
    assert!(
        err.contains("ReBirth Standard Kit") && err.contains("59 panel bitmaps"),
        "{err}"
    );
    // what the rbm crate builds (crates/rbm/tests/standard.rs checks its contents)
    let want = rbm::standard::build(
        &std::fs::read(&dll).unwrap(),
        &std::fs::read(root().join("original/Rebirth.exe")).unwrap(),
    )
    .unwrap();
    assert_eq!(std::fs::read(&out).unwrap(), want.bytes);
    assert_eq!(std::fs::read(standard_mod()).unwrap(), want.bytes);

    // a ReBirth.dll alone: the exe is looked for next to it, or given
    let lone = dir.path().join("ReBirth.dll");
    std::fs::copy(&dll, &lone).unwrap();
    let o = standard(&[
        "--rebirth-dll".as_ref(),
        lone.as_os_str(),
        "-o".as_ref(),
        out.as_os_str(),
    ]);
    let err = String::from_utf8_lossy(&o.stderr);
    assert_eq!(o.status.code(), Some(1), "{err}");
    assert!(
        err.contains("no Rebirth.exe") && err.contains("--rebirth-exe"),
        "{err}"
    );
    let exe = root().join("original/Rebirth.exe");
    let o = standard(&[
        "--rebirth-dll".as_ref(),
        lone.as_os_str(),
        "--rebirth-exe".as_ref(),
        exe.as_os_str(),
        "-o".as_ref(),
        out.as_os_str(),
    ]);
    assert!(o.status.success(), "{}", String::from_utf8_lossy(&o.stderr));
    assert_eq!(std::fs::read(&out).unwrap(), want.bytes);

    // the files swapped, or missing
    let o = standard(&[
        "--rebirth-dll".as_ref(),
        exe.as_os_str(),
        "--rebirth-exe".as_ref(),
        dll.as_os_str(),
        "-o".as_ref(),
        out.as_os_str(),
    ]);
    let err = String::from_utf8_lossy(&o.stderr);
    assert_eq!(o.status.code(), Some(1), "{err}");
    assert!(err.contains("cannot build the standard mod"), "{err}");
    let none = dir.path().join("none.dll");
    let o = standard(&[
        "--rebirth-dll".as_ref(),
        none.as_os_str(),
        "-o".as_ref(),
        out.as_os_str(),
    ]);
    let err = String::from_utf8_lossy(&o.stderr);
    assert_eq!(o.status.code(), Some(1), "{err}");
    assert!(
        err.contains("--rebirth-dll") && err.contains("no such file"),
        "{err}"
    );
}

#[test]
fn drums_need_the_standard_mod() {
    let dir = tempfile::tempdir().unwrap();
    let out = dir.path().join("d.wav");

    // --standard-mod naming no file.
    let none = dir.path().join("none.rbm");
    let o = render(&drum_song(), &out, &[flag(), none.as_os_str()], None);
    let err = String::from_utf8_lossy(&o.stderr);
    assert_eq!(o.status.code(), Some(1), "{err}");
    assert!(
        err.contains("TR-808") && err.contains("no such file"),
        "{err}"
    );
    assert!(!out.exists());

    // A file that is not a mod.
    let junk = dir.path().join("junk.rbm");
    std::fs::write(&junk, b"CAT but not really").unwrap();
    let o = render(&drum_song(), &out, &[flag(), junk.as_os_str()], None);
    let err = String::from_utf8_lossy(&o.stderr);
    assert_eq!(o.status.code(), Some(1), "{err}");
    assert!(err.contains("not a ReBirth mod"), "{err}");

    // A mod that is not the standard mod.
    let other = dir.path().join("other.rbm");
    std::fs::write(&other, empty_mod("Some Mod")).unwrap();
    let o = render(&drum_song(), &out, &[flag(), other.as_os_str()], None);
    let err = String::from_utf8_lossy(&o.stderr);
    assert_eq!(o.status.code(), Some(1), "{err}");
    assert!(
        err.contains("not a standard mod") && err.contains("rb standard-mod"),
        "{err}"
    );

    // The ReBirth.dll itself is no longer taken.
    let dll = root().join("original/ReBirth.dll");
    let o = render(
        &drum_song(),
        &out,
        &["--rebirth-dll".as_ref(), dll.as_os_str()],
        None,
    );
    assert_eq!(o.status.code(), Some(2));

    // No flag, no variable, none next to the binary or in a mod folder: the error says how to
    // make one. (On Windows the machine's own ReBirth installation's Mods folder is searched.)
    if !cfg!(windows) && !default_exists() {
        let o = render(&drum_song(), &out, &[], None);
        let err = String::from_utf8_lossy(&o.stderr);
        assert_eq!(o.status.code(), Some(1), "{err}");
        assert!(
            err.contains("no standard mod found")
                && err.contains("rb standard-mod --rebirth-dll")
                && err.contains("--standard-mod"),
            "{err}"
        );
    }
}

#[test]
fn drums_render_with_the_mod_from_the_flag_the_variable_or_the_mod_folder() {
    let dir = tempfile::tempdir().unwrap();
    let a = dir.path().join("a.wav");
    let o = render(
        &drum_song(),
        &a,
        &[flag(), standard_mod().as_os_str()],
        None,
    );
    assert!(o.status.success(), "{}", String::from_utf8_lossy(&o.stderr));

    let b = dir.path().join("b.wav");
    let o = render(&drum_song(), &b, &[], Some(standard_mod()));
    assert!(o.status.success(), "{}", String::from_utf8_lossy(&o.stderr));
    assert_eq!(std::fs::read(&a).unwrap(), std::fs::read(&b).unwrap());

    // in the mod folder under any file name: found by its INFO name
    let mods = dir.path().join("Mods");
    std::fs::create_dir(&mods).unwrap();
    std::fs::write(mods.join("a.rbm"), empty_mod("Some Mod")).unwrap();
    std::fs::copy(standard_mod(), mods.join("kit.rbm")).unwrap();
    if !default_exists() {
        let c = dir.path().join("c.wav");
        let o = render(
            &drum_song(),
            &c,
            &["--mod-dir".as_ref(), mods.as_os_str()],
            None,
        );
        assert!(o.status.success(), "{}", String::from_utf8_lossy(&o.stderr));
        assert_eq!(std::fs::read(&a).unwrap(), std::fs::read(&c).unwrap());
    }

    // A variable naming no file is an error, not a silent fallback.
    let none = dir.path().join("none.rbm");
    let o = render(&drum_song(), &b, &[], Some(&none));
    assert_eq!(o.status.code(), Some(1));
    assert!(String::from_utf8_lossy(&o.stderr).contains("RB_STANDARD_MOD"));
}

/// Trakka Rekka has both drum machines off in their chunks; enable events of their tracks turn
/// them on. They count as playing: the samples are needed and loaded, and both drum machines
/// play (no "not implemented" warning names them).
#[test]
fn drums_enabled_only_by_track_events_load_the_samples() {
    let dir = tempfile::tempdir().unwrap();
    let out = dir.path().join("t.wav");
    let song = root().join("tests/songs/shipped/Trakka Rekka.rbs");
    let none = dir.path().join("none.rbm");
    let o = render(&song, &out, &[flag(), none.as_os_str()], None);
    let err = String::from_utf8_lossy(&o.stderr);
    assert_eq!(o.status.code(), Some(1), "{err}");
    assert!(err.contains("TR-808 and TR-909"), "{err}");

    let o = render(&song, &out, &[flag(), standard_mod().as_os_str()], None);
    let err = String::from_utf8_lossy(&o.stderr);
    assert!(o.status.success(), "{err}");
    assert!(
        !err.lines()
            .any(|l| l.contains("not implemented yet") && l.contains("TR-")),
        "{err}"
    );
}

#[test]
fn a_303_only_song_renders_without_the_standard_mod() {
    let dir = tempfile::tempdir().unwrap();
    let out = dir.path().join("t.wav");
    let song = root().join("tests/songs/diag/songs/303_saw_lowcut.rbs");
    let o = render(&song, &out, &[], None);
    assert!(o.status.success(), "{}", String::from_utf8_lossy(&o.stderr));
    // Even a wrong --standard-mod is not looked at when no drum machine plays.
    let none = dir.path().join("none.rbm");
    let o = render(&song, &out, &[flag(), none.as_os_str()], None);
    assert!(o.status.success(), "{}", String::from_utf8_lossy(&o.stderr));
}
