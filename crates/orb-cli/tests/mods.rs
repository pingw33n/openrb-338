//! Mods in `orb render` (task-033), with a synthetic mod and key file (the real ones are not in
//! the repository): the song's mod found by name in `--mod-dir`, `--mod` overriding it, a
//! missing mod as an error, `--standard-kit` as the fallback, and (task-063) that orb render
//! loads only mods recrypted by `orb mod recrypt`, which is the only command that takes the key
//! file.
//!
//! The synthetic key file is the right size with the right magic and zeros after it, so its
//! key table is zeros and the scrambling is the identity: the mod is built in the clear. The
//! setup recrypts it in process into `mods/`; `raw/` keeps the original. (`orb mod recrypt`
//! itself checks the key file's SHA-256, task-064, so it rejects this stand-in; the CLI's
//! recrypt is tested in tests/recrypt.rs.)

mod common;

use std::ffi::OsStr;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};

use common::root;

fn aiff(frames: usize) -> Vec<u8> {
    let mut d = b"FORM".to_vec();
    d.extend(((4 + 8 + 18 + 8 + 8 + 2 * frames) as u32).to_be_bytes());
    d.extend(b"AIFFCOMM");
    d.extend(18u32.to_be_bytes());
    d.extend(1u16.to_be_bytes());
    d.extend((frames as u32).to_be_bytes());
    d.extend(16u16.to_be_bytes());
    d.extend([0x40, 0x0E, 0xAC, 0x44, 0, 0, 0, 0, 0, 0]);
    d.extend(b"SSND");
    d.extend((8 + 2 * frames as u32).to_be_bytes());
    d.extend([0u8; 8]);
    for i in 0..frames {
        // a loud square wave: unlike any built-in drum
        let v: i16 = if (i / 40) % 2 == 0 { 20000 } else { -20000 };
        d.extend(v.to_be_bytes());
    }
    d
}

fn chunk(out: &mut Vec<u8>, tag: &[u8; 4], payload: &[u8]) {
    out.extend(tag);
    out.extend((payload.len() as u32).to_be_bytes());
    out.extend(payload);
    if payload.len() & 1 == 1 {
        out.push(0);
    }
}

/// A mod whose menu name is `name`, replacing every TR-808 sample.
fn synthetic_mod(name: &str) -> Vec<u8> {
    let mut b = b"CAT \0\0\0\0PRBM".to_vec();
    chunk(&mut b, b"HEAD", &[7u8; 256]);
    for slot in rbm::SLOTS_808 {
        let mut p = slot.as_bytes().to_vec();
        p.push(0);
        p.extend(aiff(3000));
        chunk(&mut b, b"EMBF", &p);
    }
    let mut info = vec![0u8; 1280];
    info[1003..1003 + name.len()].copy_from_slice(name.as_bytes());
    info[1073..1077].copy_from_slice(&[0xBC, 1, 0, 0]);
    chunk(&mut b, b"INFO", &info);
    let size = (b.len() - 8) as u32;
    b[4..8].copy_from_slice(&size.to_be_bytes());
    b
}

struct Setup {
    dir: tempfile::TempDir,
    dat: PathBuf,
    /// The recrypted mods (and a junk file).
    mods: PathBuf,
    /// The same mod, not recrypted.
    raw: PathBuf,
}

fn orb() -> Command {
    let mut c = Command::new(env!("CARGO_BIN_EXE_orb"));
    c.env_remove("ORB_MOD_DIR").env_remove("ORB_RB20FUL");
    c
}

fn setup() -> Setup {
    let dir = tempfile::tempdir().unwrap();
    let mods = dir.path().join("mods");
    let raw = dir.path().join("raw");
    std::fs::create_dir(&mods).unwrap();
    std::fs::create_dir(&raw).unwrap();
    std::fs::write(raw.join("tm.rbm"), synthetic_mod("Test Mod")).unwrap();
    std::fs::write(mods.join("junk.rbm"), b"not a mod").unwrap();
    // not in the mod folder's parent either, where the search would find it
    std::fs::create_dir(dir.path().join("keys")).unwrap();
    let dat = dir.path().join("keys/RB20FUL.DAT");
    let f = std::fs::File::create(&dat).unwrap();
    f.set_len(rbm::DAT_HEADER + rbm::DAT_BODY).unwrap();
    drop(f);
    use std::io::{Seek, Write};
    let mut f = std::fs::OpenOptions::new().write(true).open(&dat).unwrap();
    f.seek(std::io::SeekFrom::Start(0)).unwrap();
    f.write_all(&rbm::DAT_MAGIC.to_be_bytes()).unwrap();
    drop(f);
    // `orb mod recrypt` checks the key file's SHA-256 (task-064), which this stand-in cannot
    // pass: recrypt in process, with the stand-in's table.
    let rbm = rbm::Rbm::parse(std::fs::read(raw.join("tm.rbm")).unwrap()).unwrap();
    let table = rbm::KeyTable::read(&rbm, &mut std::fs::File::open(&dat).unwrap()).unwrap();
    std::fs::write(mods.join("tm.rbm"), rbm::recrypt(&rbm, &table).unwrap()).unwrap();
    Setup {
        dir,
        dat,
        mods,
        raw,
    }
}

/// The 808 song of the diagnostic corpus with its GLOB mod name set to `mod_name`.
fn song(s: &Setup, file: &str, mod_name: &str) -> PathBuf {
    let src = root().join("tests/songs/diag/songs/808_bass.rbs");
    let mut song = rbs::Song::parse(&std::fs::read(src).unwrap()).unwrap();
    song.glob.mod_name = mod_name.to_string();
    let p = s.dir.path().join(file);
    std::fs::write(&p, song.write().unwrap()).unwrap();
    p
}

/// `orb render song -o out extra...` with the mod environment cleared (the built-in kit is
/// embedded).
fn render(s: &Setup, song: &Path, name: &str, extra: &[&OsStr]) -> (Output, PathBuf) {
    let out = s.dir.path().join(name);
    let o = orb()
        .arg("render")
        .arg(song)
        .arg("-o")
        .arg(&out)
        .args(extra)
        .output()
        .unwrap();
    (o, out)
}

fn ok(o: &Output) {
    assert!(o.status.success(), "{}", String::from_utf8_lossy(&o.stderr));
}

fn os(s: &str) -> &OsStr {
    OsStr::new(s)
}

#[test]
fn the_songs_mod_is_found_by_name_in_the_mod_dir() {
    let s = setup();
    let standard = song(&s, "std.rbs", "Standard ReBirth");
    let named = song(&s, "named.rbs", "Test Mod");
    let by_file = song(&s, "byfile.rbs", "TM"); // the file name, any case, without .rbm
    let dirs = [os("--mod-dir"), s.mods.as_os_str()];

    let (o, std_out) = render(&s, &standard, "std.wav", &[]);
    ok(&o);
    let (o, mod_out) = render(&s, &named, "mod.wav", &dirs);
    ok(&o);
    let (o, file_out) = render(&s, &by_file, "file.wav", &dirs);
    ok(&o);
    let std_wav = std::fs::read(std_out).unwrap();
    let mod_wav = std::fs::read(mod_out).unwrap();
    assert_ne!(std_wav, mod_wav, "the mod's drums are audible");
    assert_eq!(mod_wav, std::fs::read(file_out).unwrap());
}

/// The synthetic mod has no TR-909 sample: a 909 song with it plays the standard mod's 909
/// samples slot by slot, as the original plays its built-in ones (task-036).
#[test]
fn a_mods_missing_slots_come_from_the_standard_mod() {
    let s = setup();
    let src = root().join("tests/songs/diag/songs/909_bass.rbs");
    let mut song = rbs::Song::parse(&std::fs::read(src).unwrap()).unwrap();
    let mut write = |file: &str, name: &str| {
        song.glob.mod_name = name.to_string();
        let p = s.dir.path().join(file);
        std::fs::write(&p, song.write().unwrap()).unwrap();
        p
    };
    let (standard, named) = (
        write("s909.rbs", "Standard ReBirth"),
        write("m909.rbs", "Test Mod"),
    );
    let (o, a) = render(&s, &standard, "s909.wav", &[]);
    ok(&o);
    let (o, b) = render(
        &s,
        &named,
        "m909.wav",
        &[os("--mod-dir"), s.mods.as_os_str()],
    );
    ok(&o);
    assert_eq!(std::fs::read(a).unwrap(), std::fs::read(b).unwrap());
}

#[test]
fn the_info_name_match_is_case_sensitive_and_mod_warns_on_a_mismatch() {
    let s = setup();
    let standard = song(&s, "std.rbs", "Standard ReBirth");
    // Not the INFO name "Test Mod", no file "test mod".
    let lower = song(&s, "lower.rbs", "test mod");
    let dirs = [os("--mod-dir"), s.mods.as_os_str()];
    let (o, _) = render(&s, &lower, "l.wav", &dirs);
    let err = String::from_utf8_lossy(&o.stderr);
    assert_eq!(o.status.code(), Some(1), "{err}");
    assert!(err.contains("not found"), "{err}");

    // --mod with an INFO name other than the song's: renders, with a warning
    let modfile = s.mods.join("tm.rbm");
    let (o, _) = render(&s, &standard, "w.wav", &[os("--mod"), modfile.as_os_str()]);
    ok(&o);
    let err = String::from_utf8_lossy(&o.stderr);
    assert!(err.contains("not the mod the song names"), "{err}");
    // and none when they agree
    let named = song(&s, "named.rbs", "Test Mod");
    let (o, _) = render(&s, &named, "w2.wav", &[os("--mod"), modfile.as_os_str()]);
    ok(&o);
    assert!(!String::from_utf8_lossy(&o.stderr).contains("warning"));
}

#[test]
fn mod_overrides_the_song_and_the_environment_gives_the_folders() {
    let s = setup();
    let standard = song(&s, "std.rbs", "Standard ReBirth");
    let named = song(&s, "named.rbs", "Test Mod");
    let modfile = s.mods.join("tm.rbm");
    let (o, a) = render(&s, &standard, "a.wav", &[os("--mod"), modfile.as_os_str()]);
    ok(&o);
    let (o, b) = render(&s, &named, "b.wav", &[os("--mod-dir"), s.mods.as_os_str()]);
    ok(&o);
    assert_eq!(std::fs::read(&a).unwrap(), std::fs::read(&b).unwrap());

    // --mod wins over what the song names, even a mod that does not exist
    let ghost = song(&s, "ghost.rbs", "No Such Mod");
    let (o, c) = render(&s, &ghost, "c.wav", &[os("--mod"), modfile.as_os_str()]);
    ok(&o);
    assert_eq!(std::fs::read(&a).unwrap(), std::fs::read(&c).unwrap());

    // ORB_MOD_DIR stands in for the option
    let out = s.dir.path().join("d.wav");
    let o = orb()
        .arg("render")
        .arg(&named)
        .arg("-o")
        .arg(&out)
        .env("ORB_MOD_DIR", &s.mods)
        .output()
        .unwrap();
    ok(&o);
    assert_eq!(std::fs::read(&a).unwrap(), std::fs::read(out).unwrap());
}

#[test]
fn a_missing_mod_is_an_error_unless_standard_kit() {
    let s = setup();
    let ghost = song(&s, "ghost.rbs", "No Such Mod");
    let dirs = [os("--mod-dir"), s.mods.as_os_str()];

    let (o, out) = render(&s, &ghost, "g.wav", &dirs);
    let err = String::from_utf8_lossy(&o.stderr);
    assert_eq!(o.status.code(), Some(1), "{err}");
    assert!(
        err.contains("No Such Mod") && err.contains("--mod-dir") && err.contains("--standard-kit"),
        "{err}"
    );
    assert!(!out.exists());

    // --mod-dir naming no folder
    let none = s.dir.path().join("none");
    let (o, _) = render(&s, &ghost, "g.wav", &[os("--mod-dir"), none.as_os_str()]);
    let err = String::from_utf8_lossy(&o.stderr);
    assert_eq!(o.status.code(), Some(1), "{err}");
    assert!(err.contains("no such directory"), "{err}");

    // --standard-kit: the built-in kit, with a warning
    let standard = song(&s, "std.rbs", "Standard ReBirth");
    let (o, a) = render(&s, &standard, "a.wav", &[]);
    ok(&o);
    let mut kit = dirs.to_vec();
    kit.push(os("--standard-kit"));
    let (o, b) = render(&s, &ghost, "b.wav", &kit);
    ok(&o);
    let err = String::from_utf8_lossy(&o.stderr);
    assert!(
        err.contains("warning") && err.contains("built-in kit"),
        "{err}"
    );
    assert_eq!(std::fs::read(a).unwrap(), std::fs::read(b).unwrap());
}

/// orb render takes no key file and reads none (task-063): a mod that is not recrypted is an
/// error that names `orb mod recrypt`, however a key file is offered.
#[test]
fn a_mod_that_is_not_recrypted_is_an_error_naming_orb_recrypt() {
    let s = setup();
    let named = song(&s, "named.rbs", "Test Mod");
    let dirs = [os("--mod-dir"), s.raw.as_os_str()];
    let (o, out) = render(&s, &named, "n.wav", &dirs);
    let err = String::from_utf8_lossy(&o.stderr);
    assert_eq!(o.status.code(), Some(1), "{err}");
    assert!(
        err.contains("tm.rbm") && err.contains("not recrypted") && err.contains("orb mod recrypt"),
        "{err}"
    );
    assert!(!out.exists());

    // --mod names the file the same way
    let raw_file = s.raw.join("tm.rbm");
    let (o, out) = render(&s, &named, "n.wav", &[os("--mod"), raw_file.as_os_str()]);
    let err = String::from_utf8_lossy(&o.stderr);
    assert_eq!(o.status.code(), Some(1), "{err}");
    assert!(err.contains("orb mod recrypt"), "{err}");
    assert!(!out.exists());

    // a key file in the mod folder or in ORB_RB20FUL is not read
    std::fs::copy(&s.dat, s.raw.join("RB20FUL.DAT")).unwrap();
    let o = orb()
        .arg("render")
        .arg(&named)
        .arg("-o")
        .arg(s.dir.path().join("m.wav"))
        .args(dirs)
        .env("ORB_RB20FUL", &s.dat)
        .output()
        .unwrap();
    let err = String::from_utf8_lossy(&o.stderr);
    assert_eq!(o.status.code(), Some(1), "{err}");
    assert!(err.contains("orb mod recrypt"), "{err}");

    // and the option is gone from render, compare and play
    for cmd in ["render", "compare", "play"] {
        let o = orb()
            .arg(cmd)
            .arg(&named)
            .arg("--rb20ful")
            .arg(&s.dat)
            .output()
            .unwrap();
        assert_eq!(o.status.code(), Some(2), "{cmd}");
        assert!(String::from_utf8_lossy(&o.stderr).contains("--rb20ful"));
    }
}

#[test]
fn orb_recrypt_is_the_one_command_that_needs_the_key_file() {
    let s = setup();
    let named = song(&s, "named.rbs", "Test Mod");
    let src = s.raw.join("tm.rbm");
    let re_dir = s.dir.path().join("re");
    std::fs::create_dir(&re_dir).unwrap();
    let re = re_dir.join("tm.rbm");
    let recrypt = |extra: &[&OsStr]| {
        orb()
            .args(["mod", "recrypt"])
            .arg(&src)
            .arg("-o")
            .arg(&re)
            .args(extra)
            .output()
            .unwrap()
    };
    // without the key file it fails, naming it
    let o = recrypt(&[]);
    let err = String::from_utf8_lossy(&o.stderr);
    assert_eq!(o.status.code(), Some(1), "{err}");
    assert!(err.contains("RB20FUL.DAT"), "{err}");
    assert!(!re.exists());
    // the stand-in key file is found and then rejected by its SHA-256 (task-064): nothing is
    // written. (The accepting path is tests/recrypt.rs.)
    let rejected = |o: &Output, dat: &Path| {
        let err = String::from_utf8_lossy(&o.stderr).into_owned();
        assert_eq!(o.status.code(), Some(1), "{err}");
        assert!(err.contains(&dat.display().to_string()), "{err}");
        assert!(err.contains("SHA-256") && err.contains("expected"), "{err}");
        assert!(!re.exists());
    };
    rejected(&recrypt(&[os("--rb20ful"), s.dat.as_os_str()]), &s.dat);

    // a key file that is not one
    let junk = s.dir.path().join("junk.dat");
    std::fs::write(&junk, vec![0u8; 5000]).unwrap();
    let o = recrypt(&[os("--rb20ful"), junk.as_os_str()]);
    let err = String::from_utf8_lossy(&o.stderr);
    assert_eq!(o.status.code(), Some(1), "{err}");
    assert!(err.contains("bad magic"), "{err}");

    // the ORB_RB20FUL variable and the key file next to the mod are found
    let o = orb()
        .args(["mod", "recrypt"])
        .arg(&src)
        .arg("-o")
        .arg(&re)
        .env("ORB_RB20FUL", &s.dat)
        .output()
        .unwrap();
    rejected(&o, &s.dat);
    let beside = s.raw.join("RB20FUL.DAT");
    std::fs::copy(&s.dat, &beside).unwrap();
    rejected(&recrypt(&[]), &beside);

    // the recrypted mod renders with no key file anywhere, found by its INFO name
    std::fs::copy(s.mods.join("tm.rbm"), &re).unwrap();
    let (o, c) = render(&s, &named, "c.wav", &[os("--mod-dir"), re_dir.as_os_str()]);
    ok(&o);
    let (o, a) = render(&s, &named, "a.wav", &[os("--mod"), re.as_os_str()]);
    ok(&o);
    assert_eq!(std::fs::read(a).unwrap(), std::fs::read(c).unwrap());
}
