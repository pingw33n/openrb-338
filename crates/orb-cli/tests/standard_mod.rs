//! The standard mod (task-036) and orb's built-in kit (task-067): `orb mod pack-standard`, the
//! committed `assets/Standard.rbm` it reproduces, and the kit orb embeds, which render, compare
//! and play use with no option, variable or file of the original.
//!
//! The tests that build the mod need the CD's `ReBirth.dll` and `Rebirth.exe`, which
//! `orb-testkit` takes from the CD image (`ORB_CD_ISO`); they skip without it.

mod common;

use std::ffi::OsStr;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};

use common::{original, root, standard_mod};
use sha2::{Digest, Sha256};

/// The SHA-256 of the CD's Rebirth.exe (docs/rbm.md 13.1), the splash of assets/Standard.rbm.
const CD_EXE_SHA256: &str = "e69fef4153856f4ad985797a3f312a0027633c00f835c3348415b031bbd87004";

/// `orb render input -o output extra...` in an empty working folder, with the mod folder variable
/// removed.
fn render(input: &Path, output: &Path, extra: &[&OsStr]) -> Output {
    let cwd = tempfile::tempdir().unwrap();
    let mut c = Command::new(env!("CARGO_BIN_EXE_orb"));
    c.current_dir(cwd.path());
    c.arg("render").arg(input).arg("-o").arg(output).args(extra);
    c.env_remove("ORB_MOD_DIR");
    c.output().unwrap()
}

fn standard(args: &[&OsStr]) -> Output {
    Command::new(env!("CARGO_BIN_EXE_orb"))
        .args(["mod", "pack-standard"])
        .args(args)
        .output()
        .unwrap()
}

fn drum_song() -> PathBuf {
    root().join("tests/songs/diag/songs/808_bass.rbs")
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

fn sha256(b: &[u8]) -> String {
    format!("{:x}", Sha256::digest(b))
}

/// `orb mod pack-standard` with the CD's Rebirth.exe gives assets/Standard.rbm byte for byte.
/// Gated: `ORB_CD_ISO` names the CD image; its `ReBirth.dll` and `Rebirth.exe` (SHA-256
/// e69fef41...) are taken from it by `orb-testkit`.
#[test]
fn the_command_reproduces_the_committed_mod() {
    let dir = tempfile::tempdir().unwrap();
    let (Some(dll), Some(exe)) = (original("ReBirth.dll"), original("Rebirth.exe")) else {
        return;
    };
    assert_eq!(sha256(&std::fs::read(&exe).unwrap()), CD_EXE_SHA256);
    let out = dir.path().join("Standard.rbm");
    let o = standard(&[
        "--rebirth-dll".as_ref(),
        dll.as_os_str(),
        "--rebirth-exe".as_ref(),
        exe.as_os_str(),
        "-o".as_ref(),
        out.as_os_str(),
    ]);
    assert!(o.status.success(), "{}", String::from_utf8_lossy(&o.stderr));
    let got = std::fs::read(&out).unwrap();
    assert!(got == std::fs::read(standard_mod()).unwrap(), "differs");
    assert!(
        got == rbm::standard::EMBEDDED,
        "differs from the embedded kit"
    );
}

#[test]
fn the_command_writes_the_standard_mod() {
    let (Some(dll), Some(exe)) = (original("ReBirth.dll"), original("Rebirth.exe")) else {
        return;
    };
    let dir = tempfile::tempdir().unwrap();
    let out = dir.path().join("s.rbm");
    let o = standard(&[
        "--rebirth-dll".as_ref(),
        dll.as_os_str(),
        "-o".as_ref(),
        out.as_os_str(),
    ]);
    let err = String::from_utf8_lossy(&o.stderr);
    assert!(o.status.success(), "{err}");
    assert!(
        err.contains("\"Standard ReBirth\"") && err.contains("59 panel bitmaps"),
        "{err}"
    );
    // what the rbm crate builds (crates/rbm/tests/standard.rs checks its contents)
    let want =
        rbm::standard::build(&std::fs::read(&dll).unwrap(), &std::fs::read(&exe).unwrap()).unwrap();
    assert_eq!(std::fs::read(&out).unwrap(), want.bytes);

    // -o is required: nothing is looked for next to the binary any more
    let o = standard(&["--rebirth-dll".as_ref(), dll.as_os_str()]);
    assert_eq!(o.status.code(), Some(2));

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

/// The drum machines play the embedded kit: no option, no variable, no file. The old lookup's
/// option and the DLL option are gone (usage errors).
#[test]
fn drums_play_the_embedded_kit() {
    let dir = tempfile::tempdir().unwrap();
    let a = dir.path().join("a.wav");
    let o = render(&drum_song(), &a, &[]);
    assert!(o.status.success(), "{}", String::from_utf8_lossy(&o.stderr));

    let junk = dir.path().join("junk.rbm");
    std::fs::write(&junk, b"CAT but not really").unwrap();

    for flag in ["--standard-mod", "--rebirth-dll"] {
        let o = render(
            &drum_song(),
            &dir.path().join("c.wav"),
            &[flag.as_ref(), junk.as_os_str()],
        );
        assert_eq!(o.status.code(), Some(2), "{flag}");
    }
}

/// "Standard ReBirth" in a song is the embedded kit, even when the mod folder holds a mod of
/// that name: no folder is searched for it.
#[test]
fn standard_rebirth_is_never_looked_up() {
    let dir = tempfile::tempdir().unwrap();
    let a = dir.path().join("a.wav");
    let o = render(&drum_song(), &a, &[]);
    assert!(o.status.success(), "{}", String::from_utf8_lossy(&o.stderr));

    let mods = dir.path().join("Mods");
    std::fs::create_dir(&mods).unwrap();
    // a mod of the kit's name without a sample: it would make the drums silent
    std::fs::write(mods.join("kit.rbm"), empty_mod("Standard ReBirth")).unwrap();
    let b = dir.path().join("b.wav");
    let o = render(&drum_song(), &b, &["--mod-dir".as_ref(), mods.as_os_str()]);
    assert!(o.status.success(), "{}", String::from_utf8_lossy(&o.stderr));
    assert_eq!(std::fs::read(&a).unwrap(), std::fs::read(&b).unwrap());
}

/// Trakka Rekka has both drum machines off in their chunks; enable events of their tracks turn
/// them on. They count as playing: the samples are given and both drum machines play (no "not
/// implemented" warning names them, and the render is not silent).
#[test]
fn drums_enabled_only_by_track_events_play() {
    let dir = tempfile::tempdir().unwrap();
    let out = dir.path().join("t.wav");
    let song = root().join("tests/songs/shipped/Trakka Rekka.rbs");
    let o = render(&song, &out, &[]);
    let err = String::from_utf8_lossy(&o.stderr);
    assert!(o.status.success(), "{err}");
    assert!(
        !err.lines()
            .any(|l| l.contains("not implemented yet") && l.contains("TR-")),
        "{err}"
    );
    assert!(std::fs::read(&out).unwrap()[44..].iter().any(|&b| b != 0));
}

#[test]
fn a_303_only_song_renders() {
    let dir = tempfile::tempdir().unwrap();
    let out = dir.path().join("t.wav");
    let song = root().join("tests/songs/diag/songs/303_saw_lowcut.rbs");
    let o = render(&song, &out, &[]);
    assert!(o.status.success(), "{}", String::from_utf8_lossy(&o.stderr));
}
