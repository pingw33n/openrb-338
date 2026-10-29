//! `orb mod unpack` and `orb mod pack` (task-097): the round trip of the standard mod (the
//! committed `assets/Standard.rbm`, task-067) and of the four recrypted shipped mods (from the
//! CD image, `ORB_CD_ISO`, by `orb-testkit`; skipped when unset), an edited file, pack's errors,
//! the refusal of a mod that is not recrypted, and the old command names gone. The shipped mods are
//! not committed: they are read at run time.

mod common;

use std::path::{Path, PathBuf};
use std::process::{Command, Output};

use common::standard_mod;
use sha2::{Digest, Sha256};

fn orb(args: &[&str], paths: &[&Path]) -> Output {
    let mut c = Command::new(env!("CARGO_BIN_EXE_orb"));
    c.args(args).args(paths);
    c.env_remove("ORB_RB20FUL");
    c.output().unwrap()
}

fn text(o: &Output) -> String {
    String::from_utf8_lossy(&o.stderr).into_owned()
}

fn unpack(m: &Path, dir: &Path) -> Output {
    orb(&["mod", "unpack"], &[m, Path::new("-o"), dir])
}

fn pack(dir: &Path, out: &Path) -> Output {
    orb(&["mod", "pack"], &[dir, Path::new("-o"), out])
}

fn round_trip(m: &Path, work: &Path) {
    let dir = work.join("unpacked");
    let o = unpack(m, &dir);
    assert!(o.status.success(), "{}: {}", m.display(), text(&o));
    assert!(dir.join("mod.toml").is_file());
    let out = work.join("again.rbm");
    let o = pack(&dir, &out);
    assert!(o.status.success(), "{}: {}", m.display(), text(&o));
    assert!(
        std::fs::read(m).unwrap() == std::fs::read(&out).unwrap(),
        "{}: pack of the unpacked mod differs",
        m.display()
    );
}

#[test]
fn the_standard_mod_is_the_recorded_one() {
    // assets/README.md and docs/rbm.md 13.2 record this SHA-256
    let bytes = std::fs::read(standard_mod()).unwrap();
    let h = format!("{:x}", Sha256::digest(&bytes));
    assert_eq!(
        h,
        "6355b5f14fc2ebfe7ed9e6e42208f97817f96c5e30ed42fdca4205d3cdab5fbe"
    );
    assert_eq!(bytes, rbm::standard::EMBEDDED);
}

#[test]
fn the_old_command_names_are_gone() {
    for old in ["standard-mod", "recrypt"] {
        let o = orb(&[old], &[]);
        assert_eq!(o.status.code(), Some(2), "{old}: {}", text(&o));
    }
    let o = orb(&["mod", "--help"], &[]);
    let h = String::from_utf8_lossy(&o.stdout).into_owned();
    for sub in ["recrypt", "unpack", "pack", "pack-standard"] {
        assert!(h.contains(sub), "{sub} in {h}");
    }
}

#[test]
fn the_standard_mod_round_trips() {
    let dir = tempfile::tempdir().unwrap();
    round_trip(standard_mod(), dir.path());
}

#[test]
fn recrypted_mods_round_trip() {
    let Some(mods) = orb_testkit::recrypted_mods() else {
        return;
    };
    let mut n = 0;
    for e in std::fs::read_dir(mods).unwrap() {
        let p = e.unwrap().path();
        if p.extension().is_some_and(|x| x.eq_ignore_ascii_case("rbm")) {
            let dir = tempfile::tempdir().unwrap();
            round_trip(&p, dir.path());
            n += 1;
        }
    }
    assert_eq!(n, 4, "the four recrypted shipped mods");
    eprintln!("round trip of {n} recrypted mods ran");
}

#[test]
fn an_edited_file_reads_back_changed() {
    let work = tempfile::tempdir().unwrap();
    let dir = work.path().join("u");
    assert!(unpack(standard_mod(), &dir).status.success());
    let mut splash = std::fs::read(dir.join("splash.jpg")).unwrap();
    splash.extend_from_slice(b"edited");
    std::fs::write(dir.join("splash.jpg"), &splash).unwrap();
    let out = work.path().join("e.rbm");
    let o = pack(&dir, &out);
    assert!(o.status.success(), "{}", text(&o));
    let r = rbm::Rbm::parse(std::fs::read(&out).unwrap()).unwrap();
    let t = rbm::KeyTable::embedded(&r).unwrap();
    assert!(r.slots_covered(&t));
    assert_eq!(r.file_by_name("splash.jpg", &t).unwrap(), splash);
    assert!(r.is_standard());
}

/// A folder with a small mod.toml and the files it lists.
fn small(dir: &Path, files: &[&str], toml_files: &str) {
    std::fs::create_dir_all(dir).unwrap();
    for f in files {
        std::fs::write(dir.join(f), b"data").unwrap();
    }
    std::fs::write(
        dir.join("mod.toml"),
        format!("name = \"T\"\nfiles = [{toml_files}]\n"),
    )
    .unwrap();
}

#[test]
fn pack_checks() {
    let work = tempfile::tempdir().unwrap();
    let w = work.path();
    let out = w.join("o.rbm");

    small(&w.join("ok"), &["a.bin"], "\"a.bin\"");
    assert!(pack(&w.join("ok"), &out).status.success());

    small(&w.join("missing"), &[], "\"a.bin\"");
    let o = pack(&w.join("missing"), &out);
    assert_eq!(o.status.code(), Some(1));
    assert!(text(&o).contains("a.bin"), "{}", text(&o));

    small(&w.join("extra"), &["a.bin", "b.bin"], "\"a.bin\"");
    let o = pack(&w.join("extra"), &out);
    assert_eq!(o.status.code(), Some(1));
    assert!(text(&o).contains("b.bin") && text(&o).contains("not listed"));

    let names: Vec<String> = (0..200).map(|i| format!("f{i}")).collect();
    let refs: Vec<&str> = names.iter().map(String::as_str).collect();
    let list = refs
        .iter()
        .map(|n| format!("\"{n}\""))
        .collect::<Vec<_>>()
        .join(", ");
    small(&w.join("many"), &refs, &list);
    let o = pack(&w.join("many"), &out);
    assert_eq!(o.status.code(), Some(1));
    assert!(text(&o).contains("200"), "{}", text(&o));
    small(
        &w.join("fits"),
        &refs[..199],
        &list[..list.rfind(',').unwrap()],
    );
    assert!(pack(&w.join("fits"), &out).status.success());

    small(&w.join("long"), &["a.bin"], "\"a.bin\"");
    let long = format!("name = \"{}\"\nfiles = [\"a.bin\"]\n", "x".repeat(64));
    std::fs::write(w.join("long/mod.toml"), long).unwrap();
    let o = pack(&w.join("long"), &out);
    assert_eq!(o.status.code(), Some(1));
    assert!(text(&o).contains("name"), "{}", text(&o));
}

#[test]
fn unpack_refuses_a_mod_that_is_not_recrypted() {
    let work = tempfile::tempdir().unwrap();
    // INFO only: its key pages are not the embedded window's
    let mut m = b"CAT \0\0\0\0PRBM".to_vec();
    m.extend(b"EMBF");
    let mut p = b"big\0".to_vec();
    p.resize(3000, 7);
    m.extend((p.len() as u32).to_be_bytes());
    m.extend(&p);
    m.extend(b"INFO");
    m.extend(1280u32.to_be_bytes());
    let mut info = vec![0u8; 1280];
    info[1003] = b'x';
    m.extend(info);
    let size = (m.len() - 8) as u32;
    m[4..8].copy_from_slice(&size.to_be_bytes());
    let f: PathBuf = work.path().join("orig.rbm");
    std::fs::write(&f, m).unwrap();
    let o = unpack(&f, &work.path().join("d"));
    assert_eq!(o.status.code(), Some(1));
    let e = text(&o);
    assert!(
        e.contains("not recrypted") && e.contains("orb mod recrypt"),
        "{e}"
    );
    assert!(!work.path().join("d/mod.toml").exists());
}

#[test]
fn unpack_needs_an_empty_folder_unless_forced() {
    let work = tempfile::tempdir().unwrap();
    let dir = work.path().join("u");
    std::fs::create_dir(&dir).unwrap();
    std::fs::write(dir.join("keep"), b"x").unwrap();
    let o = unpack(standard_mod(), &dir);
    assert_eq!(o.status.code(), Some(1));
    assert!(text(&o).contains("--force"));
    let o = orb(
        &["mod", "unpack", "--force"],
        &[standard_mod(), Path::new("-o"), &dir],
    );
    assert!(o.status.success(), "{}", text(&o));
    // the extra file is not listed: pack says so
    let o = pack(&dir, &work.path().join("o.rbm"));
    assert!(text(&o).contains("keep"));
    // --force deleted nothing and replaced the file of the mod's name
    assert!(dir.join("keep").is_file());
}

/// A mod (in the clear, then recrypted) with these file names, made with rbm::pack_parts.
fn mod_with(names: &[&str]) -> Vec<u8> {
    let info = rbm::Info {
        text: String::new(),
        ftp: String::new(),
        web: String::new(),
        name: "Hostile".into(),
        version: None,
    };
    let data: Vec<Vec<u8>> = (0..names.len()).map(|i| vec![i as u8; 10 + i]).collect();
    let parts: Vec<(&str, &[u8])> = names
        .iter()
        .zip(&data)
        .map(|(n, d)| (*n, d.as_slice()))
        .collect();
    rbm::pack::pack_parts(
        &rbm::pack::standard_head(),
        &parts,
        &rbm::pack::info_bytes(&info).unwrap(),
    )
    .unwrap()
}

/// Unpacks `names` into `work/u`, checks that nothing but `u` and the mod is in `work` and that
/// `u` holds exactly the files and mod.toml, and that pack gives the mod back.
fn hostile(names: &[&str]) -> Vec<String> {
    let work = tempfile::tempdir().unwrap();
    let m = mod_with(names);
    let f = work.path().join("h.rbm");
    std::fs::write(&f, &m).unwrap();
    let dir = work.path().join("u");
    let o = unpack(&f, &dir);
    assert!(o.status.success(), "{names:?}: {}", text(&o));
    let mut outer: Vec<String> = std::fs::read_dir(work.path())
        .unwrap()
        .map(|e| e.unwrap().file_name().to_string_lossy().into_owned())
        .collect();
    outer.sort();
    assert_eq!(
        outer,
        ["h.rbm", "u"],
        "{names:?}: something escaped the folder"
    );
    let mut inner: Vec<String> = std::fs::read_dir(&dir)
        .unwrap()
        .map(|e| e.unwrap().file_name().to_string_lossy().into_owned())
        .collect();
    assert_eq!(inner.len(), names.len() + 1, "{names:?}: {inner:?}");
    inner.sort();
    let out = work.path().join("again.rbm");
    let o = pack(&dir, &out);
    assert!(o.status.success(), "{names:?}: {}", text(&o));
    assert!(
        std::fs::read(&out).unwrap() == m,
        "{names:?}: round trip differs"
    );
    inner
}

#[test]
fn hostile_names_stay_in_the_folder() {
    hostile(&[
        "../x", "a/b", "/abs", "C:x", "CON", "nul.aif", "con .txt", "Com1", "lpt9.x", "a.", "a ",
        "..", ".", "mod.toml", "MOD.TOML", "a\\b", "x\u{1}y", "a*b", "ok.bin",
    ]);
}

#[test]
fn a_sanitized_path_never_clashes() {
    let inner = hostile(&["001_a_b", "a/b", "A/B", "002_a_b"]);
    assert_eq!(inner.len(), 5);
    let mut lower: Vec<String> = inner.iter().map(|s| s.to_lowercase()).collect();
    lower.sort();
    lower.dedup();
    assert_eq!(lower.len(), 5, "{inner:?}");
}

#[test]
fn latin1_case_pairs_and_duplicates_get_distinct_files() {
    let inner = hostile(&["\u{c9}.jpg", "\u{e9}.jpg", "a.bin", "A.BIN", "a.bin"]);
    assert_eq!(inner.len(), 6);
}

#[test]
fn a_nul_in_a_name_cannot_be_packed() {
    let info = rbm::Info {
        text: String::new(),
        ftp: String::new(),
        web: String::new(),
        name: "x".into(),
        version: None,
    };
    let r = rbm::pack::pack_parts(
        &rbm::pack::standard_head(),
        &[("a\0b", &[1u8][..])],
        &rbm::pack::info_bytes(&info).unwrap(),
    );
    assert!(r.is_err());
}

#[test]
fn pack_rejects_bad_paths_in_mod_toml() {
    let work = tempfile::tempdir().unwrap();
    let out = work.path().join("o.rbm");
    let long = "x".repeat(201);
    for bad in [
        "../x",
        "a/b",
        "/abs",
        "C:x",
        "CON",
        "nul.aif",
        "a.",
        "a ",
        "mod.toml",
        "MOD.toml",
        "..",
        "a\\\\b",
        long.as_str(),
    ] {
        let dir = work.path().join("d");
        let _ = std::fs::remove_dir_all(&dir);
        small(&dir, &[], &format!("\"{bad}\""));
        let o = pack(&dir, &out);
        assert_eq!(o.status.code(), Some(1), "{bad}");
        assert!(text(&o).contains("bad file path"), "{bad}: {}", text(&o));
    }
}

#[test]
fn an_unlisted_case_variant_is_reported() {
    let work = tempfile::tempdir().unwrap();
    let dir = work.path().join("d");
    small(&dir, &["A.bin"], "\"A.bin\"");
    std::fs::write(dir.join("a.bin"), b"other").unwrap();
    if !dir.join("a.bin").exists() || std::fs::read_dir(&dir).unwrap().count() < 3 {
        eprintln!("skipped: the file system folds case");
        return;
    }
    let o = pack(&dir, &work.path().join("o.rbm"));
    assert_eq!(o.status.code(), Some(1));
    assert!(text(&o).contains("not listed"), "{}", text(&o));
}
