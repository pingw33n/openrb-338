//! The standard mod (task-036) and rb's built-in kit (task-067): `rb mod pack-standard`, the
//! committed `assets/Standard.rbm` it reproduces, and the kit rb embeds, which render, compare
//! and play use with no option, variable or file of the original.
//!
//! The tests that build the mod need `original/` (and the reproduction the CD's Rebirth.exe,
//! named by `RB_CD_REBIRTH_EXE`); they skip without them.

mod common;

use std::ffi::OsStr;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};

use common::{env_path, original, root, standard_mod};
use sha2::{Digest, Sha256};

/// The SHA-256 of the CD's Rebirth.exe (docs/rbm.md 13.1), the splash of assets/Standard.rbm.
const CD_EXE_SHA256: &str = "e69fef4153856f4ad985797a3f312a0027633c00f835c3348415b031bbd87004";

/// `rb render input -o output extra...` in an empty working folder, with the mod folder and
/// the standard mod variable of the old lookup removed (or `RB_STANDARD_MOD` set to `env`):
/// neither may matter.
fn render(input: &Path, output: &Path, extra: &[&OsStr], env: Option<&Path>) -> Output {
    let cwd = tempfile::tempdir().unwrap();
    let mut c = Command::new(env!("CARGO_BIN_EXE_rb"));
    c.current_dir(cwd.path());
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

/// `rb mod pack-standard` with the CD's Rebirth.exe gives assets/Standard.rbm byte for byte.
/// Gated: `RB_CD_REBIRTH_EXE` names the CD's Rebirth.exe (SHA-256 e69fef41...), or `RB_CD_ISO`
/// the CD image it is read from (task-065), and
/// `original/ReBirth.dll` is there.
#[test]
fn the_command_reproduces_the_committed_mod() {
    let dir = tempfile::tempdir().unwrap();
    // the exe: the file named by the variable, else the one inside the CD image (RB_CD_ISO)
    let exe = match (env_path("RB_CD_REBIRTH_EXE"), env_path("RB_CD_ISO")) {
        (Some(exe), _) => exe,
        (None, Some(iso)) => {
            let mut cd = rb_cli::cd::CdImage::open(&iso).unwrap();
            let bytes = cd.installer().unwrap().read("Rebirth.exe").unwrap();
            let p = dir.path().join("Rebirth.exe");
            std::fs::write(&p, bytes).unwrap();
            p
        }
        (None, None) => {
            eprintln!(
                "skipped: RB_CD_REBIRTH_EXE (the CD's Rebirth.exe) and RB_CD_ISO are not set"
            );
            return;
        }
    };
    let Some(dll) = original("ReBirth.dll") else {
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
/// option and the DLL option are gone (usage errors), and its variable is not read.
#[test]
fn drums_play_the_embedded_kit() {
    let dir = tempfile::tempdir().unwrap();
    let a = dir.path().join("a.wav");
    let o = render(&drum_song(), &a, &[], None);
    assert!(o.status.success(), "{}", String::from_utf8_lossy(&o.stderr));

    // RB_STANDARD_MOD naming no file, or a file that is no mod, changes nothing
    let junk = dir.path().join("junk.rbm");
    std::fs::write(&junk, b"CAT but not really").unwrap();
    for env in [dir.path().join("none.rbm"), junk.clone()] {
        let b = dir.path().join("b.wav");
        let o = render(&drum_song(), &b, &[], Some(&env));
        assert!(o.status.success(), "{}", String::from_utf8_lossy(&o.stderr));
        assert_eq!(std::fs::read(&a).unwrap(), std::fs::read(&b).unwrap());
    }

    for flag in ["--standard-mod", "--rebirth-dll"] {
        let o = render(
            &drum_song(),
            &dir.path().join("c.wav"),
            &[flag.as_ref(), junk.as_os_str()],
            None,
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
    let o = render(&drum_song(), &a, &[], None);
    assert!(o.status.success(), "{}", String::from_utf8_lossy(&o.stderr));

    let mods = dir.path().join("Mods");
    std::fs::create_dir(&mods).unwrap();
    // a mod of the kit's name without a sample: it would make the drums silent
    std::fs::write(mods.join("kit.rbm"), empty_mod("Standard ReBirth")).unwrap();
    let b = dir.path().join("b.wav");
    let o = render(
        &drum_song(),
        &b,
        &["--mod-dir".as_ref(), mods.as_os_str()],
        None,
    );
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
    let o = render(&song, &out, &[], None);
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
    let o = render(&song, &out, &[], None);
    assert!(o.status.success(), "{}", String::from_utf8_lossy(&o.stderr));
}
