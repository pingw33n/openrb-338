//! The CD image (`--iso`, task-065): the ISO 9660 reader, the installer's CABs, and
//! `rb mod recrypt --iso` / `rb mod pack-standard --iso`.
//!
//! The always-on tests build a small ISO (Joliet and primary-only) and a CAB (MSZIP: the `cab`
//! crate writes no LZX) in memory. The real image is the gated tests: `RB_CD_ISO` names the
//! user's ISO of the 2001 reissue CD, and the files they compare with (the key file, the DLL and
//! exe, the original and the recrypted shipped mods) are taken from it by `rb-testkit`, against
//! pinned SHA-256s. Without `RB_CD_ISO` the gated tests skip.

mod common;

use std::io::{Cursor, Read, Seek, SeekFrom, Write};
use std::path::{Path, PathBuf};
use std::process::{Command, Output};

use common::standard_mod;
use rb_cli::cd::{self, CdImage, Installer, Iso};

// ---------------------------------------------------------------- synthetic image

const SECTOR: usize = 2048;

/// A directory record.
fn record(name: &[u8], extent: u32, size: u32, dir: bool) -> Vec<u8> {
    let mut r = vec![0u8; 33 + name.len()];
    if r.len() % 2 == 1 {
        r.push(0);
    }
    r[0] = r.len() as u8;
    r[2..6].copy_from_slice(&extent.to_le_bytes());
    r[10..14].copy_from_slice(&size.to_le_bytes());
    r[25] = if dir { 2 } else { 0 };
    r[32] = name.len() as u8;
    r[33..33 + name.len()].copy_from_slice(name);
    r
}

fn dir_sector(records: &[Vec<u8>]) -> Vec<u8> {
    let mut d = Vec::new();
    for r in records {
        d.extend(r);
    }
    assert!(d.len() <= SECTOR);
    d.resize(SECTOR, 0);
    d
}

fn volume_descriptor(kind: u8, root: u32, escape: &[u8]) -> Vec<u8> {
    let mut d = vec![0u8; SECTOR];
    d[0] = kind;
    d[1..6].copy_from_slice(b"CD001");
    d[6] = 1;
    d[88..88 + escape.len()].copy_from_slice(escape);
    d[128..130].copy_from_slice(&2048u16.to_le_bytes());
    let r = record(&[0], root, SECTOR as u32, true);
    d[156..156 + r.len()].copy_from_slice(&r);
    d
}

fn ucs2(s: &str) -> Vec<u8> {
    s.encode_utf16().flat_map(|u| u.to_be_bytes()).collect()
}

/// A file of the image: the path of directories, its names (Joliet, primary), its data.
struct F {
    sub: Option<&'static str>,
    joliet: &'static str,
    primary: &'static str,
    data: Vec<u8>,
}

/// An ISO with the files in the root (and in one folder "Sub" for those with `sub`). With
/// `joliet` there is a Joliet tree next to the primary one; the data is shared.
fn iso(files: &[F], joliet: bool) -> Vec<u8> {
    // sectors: 16 PVD, 17 SVD or terminator, 18 terminator, 19 root, 20 Sub, 21 Joliet root,
    // 22 Joliet Sub, 23.. data
    let mut next = 23usize;
    let mut extents = Vec::new();
    for f in files {
        extents.push(next);
        next += f.data.len().div_ceil(SECTOR).max(1);
    }
    let build = |root: usize, subdir: usize, jol: bool| {
        let name = |f: &F| -> Vec<u8> {
            if jol {
                ucs2(f.joliet)
            } else {
                f.primary.as_bytes().to_vec()
            }
        };
        let mut top = vec![record(&[0], root as u32, 2048, true)];
        let mut sub = vec![record(&[0], subdir as u32, 2048, true)];
        let sub_name = if jol { ucs2("Sub") } else { b"SUB".to_vec() };
        let mut has_sub = false;
        for (f, &e) in files.iter().zip(&extents) {
            let r = record(&name(f), e as u32, f.data.len() as u32, false);
            if f.sub.is_some() {
                sub.push(r);
                has_sub = true;
            } else {
                top.push(r);
            }
        }
        if has_sub {
            top.push(record(&sub_name, subdir as u32, 2048, true));
        }
        (dir_sector(&top), dir_sector(&sub))
    };
    let mut img = vec![0u8; 16 * SECTOR];
    let (root, sub) = build(19, 20, false);
    img.extend(volume_descriptor(1, 19, b""));
    if joliet {
        img.extend(volume_descriptor(2, 21, b"%/E"));
    }
    let mut term = vec![0u8; SECTOR];
    term[0] = 255;
    term[1..6].copy_from_slice(b"CD001");
    img.extend(&term);
    if !joliet {
        img.extend(vec![0u8; SECTOR]);
    }
    img.extend(root);
    img.extend(sub);
    let (jroot, jsub) = build(21, 22, true);
    img.extend(jroot);
    img.extend(jsub);
    for f in files {
        let mut d = f.data.clone();
        d.resize(d.len().div_ceil(SECTOR).max(1) * SECTOR, 0);
        img.extend(d);
    }
    img
}

fn write_tmp(dir: &Path, name: &str, b: &[u8]) -> PathBuf {
    let p = dir.join(name);
    std::fs::write(&p, b).unwrap();
    p
}

/// A CAB (MSZIP) with the files.
fn cabinet(files: &[(&str, &[u8])]) -> Vec<u8> {
    let mut b = cab::CabinetBuilder::new();
    let folder = b.add_folder(cab::CompressionType::MsZip);
    for (n, _) in files {
        folder.add_file(*n);
    }
    let mut w = b.build(Cursor::new(Vec::new())).unwrap();
    let mut i = 0;
    while let Some(mut fw) = w.next_file().unwrap() {
        fw.write_all(files[i].1).unwrap();
        i += 1;
    }
    w.finish().unwrap().into_inner()
}

/// An installer exe: junk with a decoy "MSCF", the CAB, junk after it.
fn installer_exe(cab: &[u8]) -> Vec<u8> {
    let mut v = b"MZ\x90\0 junk MSCF not a cabinet ".to_vec();
    v.resize(4096, 0x55);
    v.extend(cab);
    v.extend(b"trailer MSCF");
    v
}

fn mod_bytes_payload() -> Vec<u8> {
    (0..3000u32).map(|i| (i * 5) as u8).collect()
}

fn mod_bytes() -> Vec<u8> {
    (0..5000u32).map(|i| (i * 7) as u8).collect()
}

fn the_cd(joliet: bool) -> Vec<u8> {
    cd_with(
        joliet,
        &[
            ("ReBirth.dll", b"a dll"),
            ("Rebirth.exe", b"an exe"),
            ("Odd.rbm", &mod_bytes()),
            ("readme.txt", b"hello"),
        ],
    )
}

fn cd_with(joliet: bool, cab_files: &[(&str, &[u8])]) -> Vec<u8> {
    let cab = cabinet(cab_files);
    iso(
        &[
            F {
                sub: None,
                joliet: "RB20FUL.DAT",
                primary: "RB20FUL.DAT;1",
                data: (0..3000u32).map(|i| (i * 3) as u8).collect(),
            },
            F {
                sub: None,
                joliet: "Install ReBirth RB-338.exe",
                primary: "INSTALL_.EXE;1",
                data: installer_exe(&cab),
            },
            F {
                sub: Some("Sub"),
                joliet: "Inner file.txt",
                primary: "INNER_FI.TXT;1",
                data: b"inner".to_vec(),
            },
        ],
        joliet,
    )
}

// ---------------------------------------------------------------- the ISO reader

#[test]
fn joliet_names_paths_and_windows() {
    let tmp = tempfile::tempdir().unwrap();
    let p = write_tmp(tmp.path(), "a.iso", &the_cd(true));
    let mut iso = Iso::open(&p).unwrap();
    let mut names: Vec<String> = iso
        .root_entries()
        .unwrap()
        .into_iter()
        .map(|e| e.name)
        .collect();
    names.sort();
    assert_eq!(
        names,
        ["Install ReBirth RB-338.exe", "RB20FUL.DAT", "Sub"].map(String::from)
    );
    // a path in a folder, found without case
    let e = iso.find("sub/INNER FILE.txt").unwrap().unwrap();
    assert_eq!((e.size, e.is_dir), (5, false));
    let mut w = iso.window(&e).unwrap();
    let mut s = String::new();
    w.read_to_string(&mut s).unwrap();
    assert_eq!(s, "inner");
    // a window ends at the file's end, seeks both ways, and does not disturb another window
    let dat = iso.find("rb20ful.dat").unwrap().unwrap();
    assert_eq!(dat.size, 3000);
    let mut a = iso.window(&dat).unwrap();
    let mut b = iso.window(&dat).unwrap();
    let mut x = [0u8; 4];
    a.seek(SeekFrom::End(-4)).unwrap();
    b.seek(SeekFrom::Start(10)).unwrap();
    a.read_exact(&mut x).unwrap();
    assert_eq!(
        x,
        [
            (2996u32 * 3) as u8,
            (2997u32 * 3) as u8,
            (2998u32 * 3) as u8,
            (2999u32 * 3) as u8
        ]
    );
    assert_eq!(a.read(&mut x).unwrap(), 0);
    b.read_exact(&mut x).unwrap();
    assert_eq!(x, [30, 33, 36, 39]);
    assert!(a.seek(SeekFrom::Current(-5000)).is_err());
    // not found
    assert!(iso.find("nothing.txt").unwrap().is_none());
    assert!(iso.find("RB20FUL.DAT/x").unwrap().is_none());
}

#[test]
fn primary_names_lose_the_version() {
    let tmp = tempfile::tempdir().unwrap();
    let p = write_tmp(tmp.path(), "a.iso", &the_cd(false));
    let mut iso = Iso::open(&p).unwrap();
    let mut names: Vec<String> = iso
        .root_entries()
        .unwrap()
        .into_iter()
        .map(|e| e.name)
        .collect();
    names.sort();
    assert_eq!(
        names,
        ["INSTALL_.EXE", "RB20FUL.DAT", "SUB"].map(String::from)
    );
    assert!(iso.find("sub/inner_fi.txt").unwrap().is_some());
}

#[test]
fn what_is_not_an_iso_is_said() {
    let tmp = tempfile::tempdir().unwrap();
    let junk = write_tmp(tmp.path(), "j.iso", &vec![7u8; 100_000]);
    let e = Iso::open(&junk).err().unwrap();
    assert!(
        e.contains("not an ISO 9660 image") && e.contains("CD001"),
        "{e}"
    );
    let short = write_tmp(tmp.path(), "s.iso", b"tiny");
    assert!(Iso::open(&short).err().unwrap().contains("too short"));
    let none = tmp.path().join("none.iso");
    assert!(Iso::open(&none).err().unwrap().contains("cannot read"));
}

#[test]
fn a_cd_that_is_not_rebirths_is_reported_with_what_it_holds() {
    let tmp = tempfile::tempdir().unwrap();
    // primary names only: the installer's long name is not there
    let p = write_tmp(tmp.path(), "a.iso", &the_cd(false));
    let e = CdImage::open(&p).err().unwrap();
    assert!(
        e.contains("not the ReBirth 2.01 CD")
            && e.contains("no Install ReBirth RB-338.exe")
            && e.contains("INSTALL_.EXE"),
        "{e}"
    );
    // a Joliet image without the key file
    let other = iso(
        &[F {
            sub: None,
            joliet: "Readme.txt",
            primary: "README.TXT;1",
            data: b"x".to_vec(),
        }],
        true,
    );
    let p = write_tmp(tmp.path(), "b.iso", &other);
    let e = CdImage::open(&p).err().unwrap();
    assert!(
        e.contains("no RB20FUL.DAT and Install ReBirth RB-338.exe") && e.contains("Readme.txt"),
        "{e}"
    );
}

// ---------------------------------------------------------------- the installer's CABs

#[test]
fn files_come_out_of_the_cab_in_the_installer() {
    let tmp = tempfile::tempdir().unwrap();
    let p = write_tmp(tmp.path(), "a.iso", &the_cd(true));
    let mut cd_image = CdImage::open(&p).unwrap();
    let inst = cd_image.installer().unwrap();
    let mut names = inst.names();
    names.sort();
    assert_eq!(
        names,
        ["Odd.rbm", "ReBirth.dll", "Rebirth.exe", "readme.txt"]
    );
    assert_eq!(cd::shipped_mods(&inst), ["Odd.rbm"]);
    assert_eq!(inst.read("rebirth.DLL").unwrap(), b"a dll");
    assert_eq!(inst.read("Odd.rbm").unwrap(), mod_bytes());
    let e = inst.read("Missing.dll").unwrap_err();
    assert!(
        e.contains("no Missing.dll") && e.contains("another edition"),
        "{e}"
    );
    // the checked read names both hashes
    let e = cd::read_checked(&inst, "ReBirth.dll", cd::DLL_SHA256).unwrap_err();
    assert!(
        e.contains(&cd::sha256_hex(b"a dll")) && e.contains(cd::DLL_SHA256),
        "{e}"
    );
    assert!(cd::read_checked(&inst, "ReBirth.dll", &cd::sha256_hex(b"a dll")).is_ok());
    // the key file window
    let mut dat = cd_image.rb20ful().unwrap();
    assert_eq!(dat.len(), 3000);
    assert_eq!(dat.read_all().unwrap()[..3], [0, 3, 6]);
}

/// A mod name in the CAB that is not a plain file name never reaches the file system: the run
/// fails and nothing is written inside or outside `-o`.
#[test]
fn unsafe_mod_names_in_the_cab_write_nothing() {
    for bad in [
        "../x.rbm",
        "/abs/x.rbm",
        "a/b.rbm",
        "..\\x.rbm",
        "C:x.rbm",
        "CON.rbm",
    ] {
        let tmp = tempfile::tempdir().unwrap();
        let p = write_tmp(
            tmp.path(),
            "a.iso",
            &cd_with(
                true,
                &[("Good.rbm", b"x".as_slice()), (bad, b"y".as_slice())],
            ),
        );
        let out = tmp.path().join("deep").join("out");
        let o = rb(&[
            "mod".as_ref(),
            "recrypt".as_ref(),
            "--iso".as_ref(),
            s(&p),
            "-o".as_ref(),
            s(&out),
        ]);
        let err = String::from_utf8_lossy(&o.stderr);
        assert_eq!(o.status.code(), Some(1), "{bad}: {err}");
        assert!(err.contains("is not a plain file name"), "{bad}: {err}");
        // nothing was created: not the output, nor a file where the name points
        assert!(!tmp.path().join("deep").exists(), "{bad}");
        assert!(!tmp.path().join("x.rbm").exists(), "{bad}");
        assert!(!tmp.path().join("abs").exists(), "{bad}");
        let names: Vec<_> = std::fs::read_dir(tmp.path()).unwrap().collect();
        assert_eq!(names.len(), 1, "{bad}");
    }
}

/// An installer that the image cuts short is an error, not a short read.
#[test]
fn a_truncated_installer_is_an_error() {
    let tmp = tempfile::tempdir().unwrap();
    let mut img = the_cd(true);
    img.truncate(img.len() - 4100);
    let p = write_tmp(tmp.path(), "a.iso", &img);
    let mut cd_image = CdImage::open(&p).unwrap();
    let e = cd_image.installer().err().unwrap();
    assert!(
        e.contains("cannot read") && e.contains("the image ends after"),
        "{e}"
    );
}

#[test]
fn an_installer_without_a_cab_has_no_files() {
    let inst = Installer::new(b"MSCF\0\0\0\0 and then nothing".to_vec());
    assert!(inst.names().is_empty());
    assert!(
        inst.read("ReBirth.dll")
            .unwrap_err()
            .contains("0 CAB archives")
    );
}

// ---------------------------------------------------------------- the commands

fn rb(args: &[&std::ffi::OsStr]) -> Output {
    Command::new(env!("CARGO_BIN_EXE_rb"))
        .args(args)
        .env_remove("RB_RB20FUL")
        .output()
        .unwrap()
}

fn s(p: &Path) -> &std::ffi::OsStr {
    p.as_os_str()
}

#[test]
fn the_commands_refuse_a_wrong_image_and_conflicting_options() {
    let tmp = tempfile::tempdir().unwrap();
    let good = write_tmp(tmp.path(), "good.iso", &the_cd(true));
    let junk = write_tmp(tmp.path(), "junk.iso", &vec![1u8; 70_000]);
    let out = tmp.path().join("out");
    let one = tmp.path().join("one.rbm");

    // not an image
    for args in [
        vec![
            "mod".as_ref(),
            "recrypt".as_ref(),
            "--iso".as_ref(),
            s(&junk),
            "-o".as_ref(),
            s(&out),
        ],
        vec![
            "mod".as_ref(),
            "pack-standard".as_ref(),
            "--iso".as_ref(),
            s(&junk),
            "-o".as_ref(),
            s(&one),
        ],
    ] {
        let o = rb(&args);
        let err = String::from_utf8_lossy(&o.stderr);
        assert_eq!(o.status.code(), Some(1), "{err}");
        assert!(err.contains("not an ISO 9660 image"), "{err}");
    }
    assert!(!out.exists() && !one.exists());

    // the CD's own mods: the synthetic one is no mod, and nothing is written
    let o = rb(&[
        "mod".as_ref(),
        "recrypt".as_ref(),
        "--iso".as_ref(),
        s(&good),
        "-o".as_ref(),
        s(&out),
    ]);
    let err = String::from_utf8_lossy(&o.stderr);
    assert_eq!(o.status.code(), Some(1), "{err}");
    assert!(err.contains(":Odd.rbm: failed: not a ReBirth mod"), "{err}");
    assert!(!out.exists());

    // a mod that needs the key file, with the synthetic CD's: it is not the CD's file, and is
    // refused before anything is written
    let mut mod_bytes = b"CAT \0\0\0\0PRBM".to_vec();
    let mut embf = b"TR808BD.aif\0".to_vec();
    embf.extend(mod_bytes_payload());
    mod_bytes.extend(b"EMBF");
    mod_bytes.extend((embf.len() as u32).to_be_bytes());
    mod_bytes.extend(&embf);
    let size = (mod_bytes.len() - 8) as u32;
    mod_bytes[4..8].copy_from_slice(&size.to_be_bytes());
    let input = write_tmp(tmp.path(), "m.rbm", &mod_bytes);
    let o = rb(&[
        "mod".as_ref(),
        "recrypt".as_ref(),
        s(&input),
        "--iso".as_ref(),
        s(&good),
        "-o".as_ref(),
        s(&out),
    ]);
    let err = String::from_utf8_lossy(&o.stderr);
    assert_eq!(o.status.code(), Some(1), "{err}");
    assert!(
        err.contains("RB20FUL.DAT in") && err.contains("bad magic, not the ReBirth CD's"),
        "{err}"
    );
    assert!(!out.exists());

    // the installer's files are not the 2001 reissue's
    let o = rb(&[
        "mod".as_ref(),
        "pack-standard".as_ref(),
        "--iso".as_ref(),
        s(&good),
        "-o".as_ref(),
        s(&one),
    ]);
    let err = String::from_utf8_lossy(&o.stderr);
    assert_eq!(o.status.code(), Some(1), "{err}");
    assert!(
        err.contains("ReBirth.dll has SHA-256") && err.contains("another edition"),
        "{err}"
    );
    assert!(!one.exists());

    // --iso excludes the other sources; recrypt needs inputs or --iso; -o must be a folder for
    // the CD's own mods
    let dll = tmp.path().join("x.dll");
    for args in [
        vec![
            "mod".as_ref(),
            "recrypt".as_ref(),
            "--iso".as_ref(),
            s(&good),
            "--rb20ful".as_ref(),
            s(&dll),
            "-o".as_ref(),
            s(&out),
        ],
        vec![
            "mod".as_ref(),
            "pack-standard".as_ref(),
            "--iso".as_ref(),
            s(&good),
            "--rebirth-dll".as_ref(),
            s(&dll),
            "-o".as_ref(),
            s(&one),
        ],
        vec!["mod".as_ref(), "recrypt".as_ref(), "-o".as_ref(), s(&out)],
        vec![
            "mod".as_ref(),
            "recrypt".as_ref(),
            "--iso".as_ref(),
            s(&good),
            "-o".as_ref(),
            s(&one),
        ],
    ] {
        let o = rb(&args);
        assert_eq!(
            o.status.code(),
            Some(2),
            "{}",
            String::from_utf8_lossy(&o.stderr)
        );
    }
}

// ---------------------------------------------------------------- the real image (gated)

fn real_iso() -> Option<PathBuf> {
    rb_testkit::cd_iso()
}

/// The files read from the image are the known ones.
#[test]
fn the_real_image_holds_the_known_files() {
    let Some(iso) = real_iso() else { return };
    let mut cd_image = CdImage::open(&iso).unwrap();
    let mut dat = cd_image.rb20ful().unwrap();
    assert_eq!(dat.len(), 134_217_984);
    assert_eq!(
        rbm::dat::sha256_hex(&mut dat).unwrap(),
        rbm::embedded::DAT_SHA256
    );
    let inst = cd_image.installer().unwrap();
    assert_eq!(inst.names().len(), 32 + 13 + 13);
    let dll = cd::read_checked(&inst, "ReBirth.dll", cd::DLL_SHA256).unwrap();
    let exe = cd::read_checked(&inst, "Rebirth.exe", cd::EXE_SHA256).unwrap();
    assert_eq!(exe.len(), 1_606_656);
    // (the DLL's hash is the one of the installed ReBirth.dll, checked by read_checked)
    drop(dll);
    assert_eq!(
        cd::shipped_mods(&inst),
        ["MSM2.rbm", "Metallicon.rbm", "Orbit2.rbm", "PBE2.rbm"]
    );
    // the shipped mods are the original files (docs/test_machine.md 7)
    for n in cd::shipped_mods(&inst) {
        let want = rb_testkit::ORIGINAL_MODS
            .iter()
            .find(|(m, _)| *m == n)
            .unwrap_or_else(|| panic!("{n} is no known shipped mod"))
            .1;
        assert_eq!(cd::sha256_hex(&inst.read(&n).unwrap()), want, "{n}");
    }
}

/// `rb mod pack-standard --iso` gives the committed standard mod (which the other test of
/// standard_mod.rs shows the `--rebirth-dll` / `--rebirth-exe` form gives too).
#[test]
fn pack_standard_from_the_image_equals_the_committed_mod() {
    let Some(iso) = real_iso() else { return };
    let tmp = tempfile::tempdir().unwrap();
    let out = tmp.path().join("Standard.rbm");
    let o = rb(&[
        "mod".as_ref(),
        "pack-standard".as_ref(),
        "--iso".as_ref(),
        s(&iso),
        "-o".as_ref(),
        s(&out),
    ]);
    assert!(o.status.success(), "{}", String::from_utf8_lossy(&o.stderr));
    assert!(
        std::fs::read(&out).unwrap() == std::fs::read(standard_mod()).unwrap(),
        "differs from assets/Standard.rbm"
    );
    // and the same bytes as the form with files
    let cd = rb_testkit::cd_files().unwrap();
    let out2 = tmp.path().join("files.rbm");
    let o = rb(&[
        "mod".as_ref(),
        "pack-standard".as_ref(),
        "--rebirth-dll".as_ref(),
        s(&cd.dll),
        "--rebirth-exe".as_ref(),
        s(&cd.exe),
        "-o".as_ref(),
        s(&out2),
    ]);
    assert!(o.status.success(), "{}", String::from_utf8_lossy(&o.stderr));
    assert!(std::fs::read(&out).unwrap() == std::fs::read(&out2).unwrap());
}

/// `rb mod recrypt --iso` (no input: the shipped mods of the CD) gives the bytes of the
/// `--rb20ful` form over the original shipped mods; and with files as input, the key file is read
/// from the image.
#[test]
fn recrypt_from_the_image_equals_the_rb20ful_form() {
    let Some(iso) = real_iso() else { return };
    let cd = rb_testkit::cd_files().unwrap();
    let (orig, dat) = (cd.orig_mods.clone(), cd.dat.clone());
    let tmp = tempfile::tempdir().unwrap();
    let from_cd = tmp.path().join("cd");
    let o = rb(&[
        "mod".as_ref(),
        "recrypt".as_ref(),
        "--iso".as_ref(),
        s(&iso),
        "-o".as_ref(),
        s(&from_cd),
    ]);
    let err = String::from_utf8_lossy(&o.stderr);
    assert!(o.status.success(), "{err}");
    assert!(
        err.contains("4 recrypted, 0 already recrypted (copied), 0 failed"),
        "{err}"
    );

    let from_files = tmp.path().join("files");
    let o = rb(&[
        "mod".as_ref(),
        "recrypt".as_ref(),
        s(&orig),
        "--rb20ful".as_ref(),
        s(&dat),
        "-o".as_ref(),
        s(&from_files),
    ]);
    assert!(o.status.success(), "{}", String::from_utf8_lossy(&o.stderr));
    let mut n = 0;
    for e in std::fs::read_dir(&from_files).unwrap() {
        let e = e.unwrap();
        let a = std::fs::read(e.path()).unwrap();
        let b = std::fs::read(from_cd.join(e.file_name())).unwrap();
        assert!(a == b, "{:?} differs", e.file_name());
        // and both are the pinned recrypted mods (docs/test_machine.md 7)
        let name = e.file_name().to_string_lossy().into_owned();
        let pinned = rb_testkit::RECRYPTED_MODS.iter().find(|(m, _)| *m == name);
        assert_eq!(
            Some(rb_testkit::sha256_hex(&a).as_str()),
            pinned.map(|p| p.1),
            "{name}"
        );
        n += 1;
    }
    assert_eq!(n, 4);

    // files given, key file from the image
    let again = tmp.path().join("again");
    let o = rb(&[
        "mod".as_ref(),
        "recrypt".as_ref(),
        s(&orig),
        "--iso".as_ref(),
        s(&iso),
        "-o".as_ref(),
        s(&again),
    ]);
    assert!(o.status.success(), "{}", String::from_utf8_lossy(&o.stderr));
    for e in std::fs::read_dir(&from_files).unwrap() {
        let e = e.unwrap();
        assert!(
            std::fs::read(e.path()).unwrap() == std::fs::read(again.join(e.file_name())).unwrap()
        );
    }
}
