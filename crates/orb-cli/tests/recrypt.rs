//! `orb mod recrypt` (task-064): the key file's SHA-256, the checks of the decrypted files, and
//! the bulk mode, through `orb_cli::recrypt::run` and the `orb` binary.
//!
//! The always-on tests use a stand-in RB20FUL.DAT: a sparse file of the right size and magic
//! whose two key pages for each synthetic mod hold bytes that are a function of the offset. It
//! is not the CD's file, so those tests give `run` the stand-in's own SHA-256 (the option exists
//! for this); the wrong-hash tests give it the real one. The real key file is the gated test
//! (`ORB_CD_ISO`: the key file and the original shipped mods come out of the image, by
//! `orb-testkit`, and the result is checked against the pinned SHA-256s of the recrypted mods);
//! those recrypted mods are the gated test of the already-recrypted path, which needs no key
//! file. `crates/rbm/tests/recrypt_python.rs` recrypts more shapes of mod and checks them with
//! `tools/rbm.py`.

mod common;

use std::fs::File;
use std::io::{Seek, SeekFrom, Write};
use std::path::{Path, PathBuf};
use std::process::{Command, Output};

use orb_cli::recrypt::{Options, Outcome, Report, run};
use rbm::{DAT_BODY, DAT_HEADER, DAT_MAGIC, KeyTable, Rbm, embedded};

// ---------------------------------------------------------------- synthetic material

fn aiff(frames: usize, seed: i16) -> Vec<u8> {
    let mut d = Vec::new();
    d.extend(b"FORM");
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
        d.extend(((i as i16).wrapping_mul(31).wrapping_add(seed)).to_be_bytes());
    }
    d
}

/// An AIFF whose SSND chunk comes first and whose COMM chunk is last, past byte 1024.
fn aiff_comm_last(frames: usize) -> Vec<u8> {
    let a = aiff(frames, 5);
    let (comm, ssnd) = (&a[12..38], &a[38..]);
    let mut d = b"FORM".to_vec();
    d.extend(((4 + ssnd.len() + comm.len()) as u32).to_be_bytes());
    d.extend(b"AIFF");
    d.extend(ssnd);
    d.extend(comm);
    d
}

fn jpeg(len: usize, first: [u8; 2]) -> Vec<u8> {
    let mut d = first.to_vec();
    // noise like entropy-coded data, with the FF bytes stuffed (FF 00) as in real JPEGs
    let mut x = len as u64;
    d.extend((2..len).map(|_| {
        x = x
            .wrapping_mul(6364136223846793005)
            .wrapping_add(1442695040888963407);
        (x >> 33) as u8
    }));
    for i in 2..len - 2 {
        if d[i] == 0xFF {
            d[i + 1] = 0;
        }
    }
    d[len - 2..].copy_from_slice(&[0xFF, 0xD9]); // the end marker, as a real JPEG's
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

fn cat(chunks: &[([u8; 4], Vec<u8>)]) -> Vec<u8> {
    let mut b = b"CAT \0\0\0\0PRBM".to_vec();
    for (t, p) in chunks {
        chunk(&mut b, t, p);
    }
    let size = (b.len() - 8) as u32;
    b[4..8].copy_from_slice(&size.to_be_bytes());
    b
}

type Files = Vec<(&'static str, Vec<u8>)>;

/// The chunks of a mod with these files: HEAD, one EMBF per file, INFO.
fn chunks_of(files: &Files) -> Vec<([u8; 4], Vec<u8>)> {
    let mut c = vec![(*b"HEAD", vec![7u8; 256])];
    for (n, f) in files {
        let mut p = n.as_bytes().to_vec();
        p.push(0);
        p.extend(f);
        c.push((*b"EMBF", p));
    }
    let mut info = vec![0u8; 1280];
    info[1003..1011].copy_from_slice(b"Test Mod");
    info[1073..1077].copy_from_slice(&[0xBC, 1, 2, 3]);
    c.push((*b"INFO", info));
    c
}

/// The stand-in key file's byte at `off`.
fn page_byte(off: u64) -> u8 {
    let x = off.wrapping_mul(0x9E37_79B9_7F4A_7C15) ^ (off >> 5);
    (x >> 29) as u8
}

/// A sparse file of the key file's size with its magic; the rest zeros.
fn blank_dat(path: &Path) -> File {
    let mut f = File::create(path).unwrap();
    f.set_len(DAT_HEADER + DAT_BODY).unwrap();
    f.write_all(&DAT_MAGIC.to_be_bytes()).unwrap();
    f
}

struct Fixture {
    dir: tempfile::TempDir,
    /// The stand-in key file.
    dat: PathBuf,
    /// Its SHA-256.
    sha: String,
    /// The folder of the scrambled mods.
    mods: PathBuf,
    /// The files of each mod, by mod file name.
    files: Vec<(String, Files)>,
}

/// Mods with the given files, scrambled with a stand-in key file that holds their pages.
fn fixture(mods: Vec<(&str, Files)>) -> Fixture {
    let dir = tempfile::tempdir().unwrap();
    let dat = dir.path().join("RB20FUL.DAT");
    let mut f = blank_dat(&dat);
    for (_, files) in &mods {
        let clear = Rbm::parse(cat(&chunks_of(files))).unwrap();
        for off in clear.key_pages() {
            let page: Vec<u8> = (0..rbm::PAGE as u64).map(|i| page_byte(off + i)).collect();
            f.seek(SeekFrom::Start(off)).unwrap();
            f.write_all(&page).unwrap();
        }
    }
    drop(f);
    let sha = rbm::dat::sha256_hex(&mut File::open(&dat).unwrap()).unwrap();
    let dir_mods = dir.path().join("mods");
    std::fs::create_dir(&dir_mods).unwrap();
    let mut out = Vec::new();
    for (name, files) in mods {
        let mut chunks = chunks_of(&files);
        let clear = Rbm::parse(cat(&chunks)).unwrap();
        let table = KeyTable::read(&clear, &mut File::open(&dat).unwrap()).unwrap();
        for (i, c) in chunks.iter_mut().enumerate() {
            if &c.0 == b"EMBF" {
                rbm::scramble(&mut c.1, i as u16, &table).unwrap();
            }
        }
        std::fs::write(dir_mods.join(name), cat(&chunks)).unwrap();
        out.push((name.to_string(), files));
    }
    Fixture {
        dir,
        dat,
        sha,
        mods: dir_mods,
        files: out,
    }
}

fn good_a() -> Files {
    vec![
        ("TR808BD.aif", aiff(900, 5)),
        ("12017.JPG", jpeg(3000, [0xFF, 0xD8])),
        ("Demo.rbs", (0..2500).map(|i| (i * 7) as u8).collect()),
    ]
}

fn good_b() -> Files {
    vec![
        ("Startup.aif", aiff(777, 11)),
        ("TR909rcy.aif", aiff(1500, 3)),
        ("TR808SD1.aif", aiff(100, 9)), // under 1024 bytes: not scrambled
    ]
}

fn good_c() -> Files {
    vec![
        ("TR909bd.aif", aiff(2000, 1)),
        ("1.jpg", jpeg(1500, [0xFF, 0xD8])),
    ]
}

fn opts<'a>(f: &'a Fixture, inputs: &'a [PathBuf], output: &'a Path, force: bool) -> Options<'a> {
    Options {
        inputs,
        output,
        rb20ful: Some(&f.dat),
        iso: None,
        force,
        expected_sha256: &f.sha,
    }
}

fn failure(r: &Report, name: &str) -> String {
    let (_, _, o) = r
        .results
        .iter()
        .find(|(i, _, _)| i.file_name().unwrap() == name)
        .unwrap_or_else(|| panic!("no result for {name}"));
    match o {
        Outcome::Failed(why) => why.clone(),
        o => panic!("{name} did not fail: {o:?}"),
    }
}

/// `bytes` is a recrypted mod with exactly `files`, readable with the embedded bytes alone.
fn check_recrypted(bytes: &[u8], files: &Files) {
    let back = Rbm::parse(bytes.to_vec()).unwrap();
    assert_eq!(back.key_pages(), [embedded::WINDOW; 2]);
    let t = KeyTable::embedded(&back).unwrap();
    assert!(back.all_covered(&t));
    for (n, f) in files {
        assert!(
            back.file_by_name(n, &t).unwrap() == *f,
            "{n} differs after the recrypt"
        );
    }
}

fn orb() -> Command {
    let mut c = Command::new(env!("CARGO_BIN_EXE_orb"));
    c.env_remove("ORB_RB20FUL").env_remove("ORB_MOD_DIR");
    c
}

fn stderr(o: &Output) -> String {
    String::from_utf8_lossy(&o.stderr).into_owned()
}

// ---------------------------------------------------------------- tests

#[test]
fn bulk_equals_single_and_hashes_the_dat_once() {
    let f = fixture(vec![
        ("a.rbm", good_a()),
        ("b.rbm", good_b()),
        ("c.rbm", good_c()),
    ]);
    let out = f.dir.path().join("bulk");
    let r = run(&opts(&f, std::slice::from_ref(&f.mods), &out, false)).unwrap();
    assert_eq!(r.failed(), 0, "{}", r.summary());
    assert_eq!(r.results.len(), 3);
    assert_eq!(r.dat_hashes, 1, "the key file is hashed once per run");
    assert!(
        r.summary()
            .starts_with("3 recrypted, 0 already recrypted (copied), 0 failed")
    );

    for (name, files) in &f.files {
        let bulk = std::fs::read(out.join(name)).unwrap();
        check_recrypted(&bulk, files);
        // the same bytes as recrypting the mod alone
        let one = f.dir.path().join(format!("single-{name}"));
        let r = run(&opts(&f, &[f.mods.join(name)], &one, false)).unwrap();
        assert_eq!(r.failed(), 0, "{}", r.summary());
        assert_eq!(r.dat_hashes, 1);
        assert!(
            std::fs::read(&one).unwrap() == bulk,
            "{name}: bulk != single"
        );
    }

    // a list of files is the same as the folder
    let out2 = f.dir.path().join("bulk2");
    let list: Vec<PathBuf> = f.files.iter().map(|(n, _)| f.mods.join(n)).collect();
    let r = run(&opts(&f, &list, &out2, false)).unwrap();
    assert_eq!((r.failed(), r.dat_hashes), (0, 1));
    for (name, _) in &f.files {
        assert!(std::fs::read(out2.join(name)).unwrap() == std::fs::read(out.join(name)).unwrap());
    }
}

#[test]
fn a_failing_mod_does_not_stop_the_others() {
    let mut bad_sample = good_b();
    bad_sample[1] = ("TR909rcy.aif", vec![0x55; 3000]); // scrambled, not an AIFF
    let mut bad_jpeg = good_a();
    bad_jpeg[1] = ("12017.JPG", jpeg(3000, [0x00, 0xD8]));
    let f = fixture(vec![
        ("a.rbm", good_a()),
        ("bad_aiff.rbm", bad_sample),
        ("bad_jpeg.rbm", bad_jpeg),
        ("c.rbm", good_c()),
    ]);
    let out = f.dir.path().join("out");
    let r = run(&opts(&f, std::slice::from_ref(&f.mods), &out, false)).unwrap();
    assert_eq!(r.failed(), 2, "{}", r.summary());
    assert_eq!(r.dat_hashes, 1);

    // the failure names the embedded file; the summary names the mod
    let why = failure(&r, "bad_aiff.rbm");
    assert!(why.contains("TR909rcy.aif"), "{why}");
    let why = failure(&r, "bad_jpeg.rbm");
    assert!(why.contains("12017.JPG") && why.contains("FF D8"), "{why}");
    let sum = r.summary();
    assert!(
        sum.contains("2 recrypted") && sum.contains("2 failed"),
        "{sum}"
    );
    assert!(
        sum.contains("bad_aiff.rbm") && sum.contains("bad_jpeg.rbm"),
        "{sum}"
    );

    // nothing is written for them, and the others are done
    assert!(!out.join("bad_aiff.rbm").exists() && !out.join("bad_jpeg.rbm").exists());
    for (name, files) in f.files.iter().filter(|(n, _)| !n.starts_with("bad")) {
        check_recrypted(&std::fs::read(out.join(name)).unwrap(), files);
    }
}

fn five_samples_and_a_jpeg(with_jpeg: bool) -> Files {
    let mut v: Files = [
        "TR808BD.aif",
        "TR808SD1.aif",
        "TR808SD2.aif",
        "TR808SD3.aif",
        "TR909bd.aif",
    ]
    .iter()
    .enumerate()
    .map(|(i, n)| (*n, aiff(2000 + 50 * i, i as i16)))
    .collect();
    if with_jpeg {
        v.push(("12017.jpg", jpeg(3000, [0xFF, 0xD8])));
    }
    v
}

/// Runs the recrypt of the one mod `a.rbm` of `f` with `dat` as the key file; the failure text.
fn failure_with(f: &Fixture, dat: &Path) -> String {
    let sha = rbm::dat::sha256_hex(&mut File::open(dat).unwrap()).unwrap();
    let ins = [f.mods.join("a.rbm")];
    let out = f.dir.path().join("o.rbm");
    let o = Options {
        rb20ful: Some(dat),
        expected_sha256: &sha,
        ..opts(f, &ins, &out, false)
    };
    let why = failure(&run(&o).unwrap(), "a.rbm");
    assert!(!out.exists(), "a refused mod must not be written");
    why
}

#[test]
fn a_mod_scrambled_with_another_key_file_is_refused_past_byte_1024() {
    // The mod is scrambled with the stand-in key file, then read with one whose pages differ:
    // every header is intact (the first 1024 bytes are never scrambled), the rest is noise.
    for with_jpeg in [true, false] {
        let f = fixture(vec![("a.rbm", five_samples_and_a_jpeg(with_jpeg))]);
        let ins = [f.mods.join("a.rbm")];
        let out = f.dir.path().join("right.rbm");
        let r = run(&opts(&f, &ins, &out, false)).unwrap();
        assert_eq!(r.failed(), 0, "{}", r.summary());

        let other = f.dir.path().join("other.DAT");
        std::fs::copy(&f.dat, &other).unwrap();
        let mut dat = std::fs::OpenOptions::new()
            .write(true)
            .open(&other)
            .unwrap();
        let rbm = Rbm::parse(std::fs::read(&ins[0]).unwrap()).unwrap();
        for off in rbm.key_pages() {
            // page_byte is a ramp (the difference of two is nearly constant): use an LCG
            let mut x = off ^ 0x5555;
            let page: Vec<u8> = (0..rbm::PAGE)
                .map(|_| {
                    x = x
                        .wrapping_mul(6364136223846793005)
                        .wrapping_add(1442695040888963407);
                    (x >> 33) as u8
                })
                .collect();
            dat.seek(SeekFrom::Start(off)).unwrap();
            dat.write_all(&page).unwrap();
        }
        drop(dat);
        let why = failure_with(&f, &other);
        assert!(why.contains("another edition"), "{why}");
        // the JPEG is named when there is one; the audio says it otherwise
        let word = if with_jpeg {
            "12017.jpg"
        } else {
            "TR808BD.aif"
        };
        assert!(why.contains(word), "{why}");
    }
}

/// Rewrites `a.rbm` of `f` with the files at the chunk ordinals where `plain` holds stored
/// unscrambled (a key half of zeros) and the others scrambled with the stand-in key file.
fn store_plain(f: &Fixture, files: &Files, plain: impl Fn(usize) -> bool) {
    let mut chunks = chunks_of(files);
    let clear = Rbm::parse(cat(&chunks)).unwrap();
    let table = KeyTable::read(&clear, &mut File::open(&f.dat).unwrap()).unwrap();
    for (i, c) in chunks.iter_mut().enumerate() {
        if &c.0 == b"EMBF" && !plain(i) {
            rbm::scramble(&mut c.1, i as u16, &table).unwrap();
        }
    }
    std::fs::write(f.mods.join("a.rbm"), cat(&chunks)).unwrap();
}

#[test]
fn files_stored_unscrambled_are_recovered_per_file() {
    // A key half of zeros stores the files in the clear; the original then descrambles garbage.
    // Every file in the clear, or those of the even keys only: the mod is written, each file
    // exactly as it was, with a note naming the recovered files.
    let mut files = five_samples_and_a_jpeg(true);
    files.push(("12018.jpg", jpeg(2800, [0xFF, 0xD8])));
    files.push(("Demo.rbs", (0..1200).map(|i| (i * 7) as u8).collect()));
    for all in [true, false] {
        let f = fixture(vec![("a.rbm", files.clone())]);
        store_plain(&f, &files, |i| all || i % 2 == 0);
        let ins = [f.mods.join("a.rbm")];
        let out = f.dir.path().join("o.rbm");
        let r = run(&opts(&f, &ins, &out, false)).unwrap();
        assert_eq!(r.failed(), 0, "{}", r.summary());
        let (_, _, Outcome::Recrypted { rescued, .. }) = &r.results[0] else {
            panic!("{:?}", r.results[0].2)
        };
        let want: Vec<String> = files
            .iter()
            .enumerate()
            .filter(|(i, _)| all || (i + 1) % 2 == 0)
            .map(|(_, f)| f.0.to_string())
            .collect();
        assert_eq!(rescued, &want);
        let s = r.summary();
        assert!(s.contains("stored unscrambled"), "{s}");
        assert!(s.contains(&format!("{} files", want.len())), "{s}");
        check_recrypted(&std::fs::read(&out).unwrap(), &files);
    }
    // the audio alone, no JPEG, is recovered as well
    let audio = five_samples_and_a_jpeg(false);
    let f = fixture(vec![("a.rbm", audio.clone())]);
    store_plain(&f, &audio, |_| true);
    let ins = [f.mods.join("a.rbm")];
    let out = f.dir.path().join("o2.rbm");
    let r = run(&opts(&f, &ins, &out, false)).unwrap();
    assert_eq!(r.failed(), 0, "{}", r.summary());
    check_recrypted(&std::fs::read(&out).unwrap(), &audio);
}

#[test]
fn a_mod_with_a_file_of_a_foreign_key_is_refused_with_the_file_named() {
    // The even keys are stored in the clear, the odd ones are right, except one JPEG that is
    // neither (scrambled with a foreign key): the mod is refused and nothing is written.
    let mut files = five_samples_and_a_jpeg(true);
    files.push(("12018.jpg", jpeg(2800, [0xFF, 0xD8])));
    let f = fixture(vec![("a.rbm", files.clone())]);
    let foreign = files.len(); // the ordinal of 12018.jpg, the last file: odd (7)
    assert_eq!(foreign % 2, 1);
    store_plain(&f, &files, |i| i % 2 == 0);
    let mut bytes = std::fs::read(f.mods.join("a.rbm")).unwrap();
    let rbm = Rbm::parse(bytes.clone()).unwrap();
    let c = rbm.chunks().iter().find(|c| c.ordinal == foreign).unwrap();
    let mut x = 99u64;
    for b in &mut bytes[c.offset + 1100..c.offset + c.size] {
        x = x
            .wrapping_mul(6364136223846793005)
            .wrapping_add(1442695040888963407);
        *b = (x >> 33) as u8;
    }
    std::fs::write(f.mods.join("a.rbm"), bytes).unwrap();
    let why = failure_with(&f, &f.dat);
    assert!(why.contains("12018.jpg"), "{why}");
    assert!(why.contains("another edition"), "{why}");
}

#[test]
fn a_cut_short_jpeg_and_a_noise_drum_are_not_refused() {
    // YellowBirth's JPEGs end without FF D9 but are JPEG data; one noise sample is a drum.
    let mut files = five_samples_and_a_jpeg(false);
    let mut cut = jpeg(3000, [0xFF, 0xD8]);
    cut.truncate(2500);
    cut.iter_mut().for_each(|b| {
        if *b == 0xFF {
            *b = 0xFE
        }
    });
    cut[0] = 0xFF;
    for i in (1050..2400).step_by(100) {
        cut[i] = 0xFF; // byte stuffing, as in real entropy-coded data
        cut[i + 1] = 0x00;
    }
    files.push(("12017.jpg", cut));
    files[1].1 = {
        let mut a = aiff(3000, 0);
        let mut x = 12345u32;
        for b in &mut a[54..] {
            x = x.wrapping_mul(1664525).wrapping_add(1013904223);
            *b = (x >> 24) as u8;
        }
        a
    };
    let f = fixture(vec![("a.rbm", files.clone())]);
    let ins = [f.mods.join("a.rbm")];
    let out = f.dir.path().join("o.rbm");
    let r = run(&opts(&f, &ins, &out, false)).unwrap();
    assert_eq!(r.failed(), 0, "{}", r.summary());
    check_recrypted(&std::fs::read(&out).unwrap(), &files);
}

#[test]
fn a_corrupted_sample_is_caught_before_anything_is_written() {
    // One byte of a scrambled sample's AIFF header, damaged after the scrambling: the "FORM" id
    // no longer matches, so the decrypted sample is not an AIFF file.
    let f = fixture(vec![("a.rbm", good_a())]);
    let path = f.mods.join("a.rbm");
    let mut bytes = std::fs::read(&path).unwrap();
    let rbm = Rbm::parse(bytes.clone()).unwrap();
    let c = rbm.find("TR808BD.aif").unwrap();
    // Damage the "FORM" id, bytes 0..4 of the file (the DLL ignores the form type at 8..12,
    // task-155): the first 1024 bytes of the payload (the name, then the AIFF header) are stored
    // in the clear.
    let at = c.offset + c.name_len;
    assert!(at < c.offset + 1024);
    bytes[at] ^= 0xFF;
    std::fs::write(&path, &bytes).unwrap();
    let out = f.dir.path().join("o.rbm");
    let r = run(&opts(&f, std::slice::from_ref(&path), &out, false)).unwrap();
    let why = failure(&r, "a.rbm");
    assert!(why.contains("TR808BD.aif") && why.contains("AIFF"), "{why}");
    assert!(!out.exists());

    // A wrong key file is caught here only when a file's header is past its first 1024 bytes
    // (those are never scrambled; the SHA-256 is the check of the key file). This sample has
    // its COMM chunk after the audio: it parses with the right pages and not with wrong ones.
    let mut files = good_a();
    files[0] = ("TR808BD.aif", aiff_comm_last(900));
    let f2 = fixture(vec![("a.rbm", files)]);
    let right = f2.dir.path().join("right.rbm");
    let ins = [f2.mods.join("a.rbm")];
    let r = run(&opts(&f2, &ins, &right, false)).unwrap();
    assert_eq!(r.failed(), 0, "{}", r.summary());
    let mut dat = std::fs::OpenOptions::new()
        .write(true)
        .open(&f2.dat)
        .unwrap();
    let rbm = Rbm::parse(std::fs::read(f2.mods.join("a.rbm")).unwrap()).unwrap();
    for off in rbm.key_pages() {
        dat.seek(SeekFrom::Start(off)).unwrap();
        dat.write_all(&vec![0x11; rbm::PAGE]).unwrap();
    }
    drop(dat);
    let sha = rbm::dat::sha256_hex(&mut File::open(&f2.dat).unwrap()).unwrap();
    let out = f2.dir.path().join("o.rbm");
    let o = Options {
        expected_sha256: &sha,
        ..opts(&f2, &ins, &out, false)
    };
    let r = run(&o).unwrap();
    let why = failure(&r, "a.rbm");
    assert!(why.contains("TR808BD.aif"), "{why}");
    assert!(!out.exists());
}

#[test]
fn a_dat_with_the_wrong_hash_is_rejected_before_anything_is_written() {
    let dir = tempfile::tempdir().unwrap();
    let f = fixture(vec![("a.rbm", good_a()), ("b.rbm", good_b())]);
    // the right magic and length, zeros after: the real hash does not match
    let blank = dir.path().join("RB20FUL.DAT");
    drop(blank_dat(&blank));
    let out = dir.path().join("out");
    let ins = [f.mods.clone()];
    let o = Options {
        rb20ful: Some(&blank),
        iso: None,
        expected_sha256: embedded::DAT_SHA256,
        ..opts(&f, &ins, &out, false)
    };
    let r = run(&o).unwrap();
    assert_eq!(r.failed(), 2, "{}", r.summary());
    assert_eq!(r.dat_hashes, 1);
    let actual = rbm::dat::sha256_hex(&mut File::open(&blank).unwrap()).unwrap();
    for name in ["a.rbm", "b.rbm"] {
        let why = failure(&r, name);
        assert!(why.contains(embedded::DAT_SHA256), "{why}");
        assert!(why.contains(&actual), "{why}");
        assert!(why.contains("another edition is not"), "{why}");
        assert!(why.contains("differ too"), "{why}");
        assert!(!out.join(name).exists());
    }

    // with the embedded window bytes in place, the message says the rest differs
    let mut w = std::fs::OpenOptions::new()
        .write(true)
        .open(&blank)
        .unwrap();
    w.seek(SeekFrom::Start(embedded::WINDOW)).unwrap();
    w.write_all(embedded::WINDOW_BYTES).unwrap();
    drop(w);
    let r = run(&o).unwrap();
    assert!(failure(&r, "a.rbm").contains("differs elsewhere"));

    // the binary: exit 1, the message, no output
    let o = orb()
        .args(["mod", "recrypt"])
        .arg(f.mods.join("a.rbm"))
        .arg("-o")
        .arg(dir.path().join("single.rbm"))
        .arg("--rb20ful")
        .arg(&blank)
        .output()
        .unwrap();
    let err = stderr(&o);
    assert_eq!(o.status.code(), Some(1), "{err}");
    assert!(
        err.contains(embedded::DAT_SHA256) && err.contains("SHA-256"),
        "{err}"
    );
    assert!(!dir.path().join("single.rbm").exists());

    // a key file of the wrong size or magic
    let short = dir.path().join("short.dat");
    std::fs::write(&short, DAT_MAGIC.to_be_bytes()).unwrap();
    let ins = [f.mods.join("a.rbm")];
    let o = Options {
        rb20ful: Some(&short),
        iso: None,
        ..opts(&f, &ins, &out, false)
    };
    let why = failure(&run(&o).unwrap(), "a.rbm");
    assert!(why.contains("expected 134217984"), "{why}");
    std::fs::write(&short, [0u8; 64]).unwrap();
    assert!(failure(&run(&o).unwrap(), "a.rbm").contains("bad magic"));
}

#[test]
fn existing_outputs_need_force_and_the_output_folder_is_not_an_input_folder() {
    let f = fixture(vec![("a.rbm", good_a()), ("b.rbm", good_b())]);
    let out = f.dir.path().join("out");
    let ins = [f.mods.clone()];
    assert_eq!(run(&opts(&f, &ins, &out, false)).unwrap().failed(), 0);
    let first = std::fs::read(out.join("a.rbm")).unwrap();

    // again: nothing is overwritten
    std::fs::write(out.join("a.rbm"), b"mine").unwrap();
    let r = run(&opts(&f, &ins, &out, false)).unwrap();
    assert_eq!(r.failed(), 2, "{}", r.summary());
    assert!(failure(&r, "a.rbm").contains("--force"));
    assert_eq!(std::fs::read(out.join("a.rbm")).unwrap(), b"mine");
    assert_eq!(r.dat_hashes, 0, "nothing needed the key file");

    let r = run(&opts(&f, &ins, &out, true)).unwrap();
    assert_eq!(r.failed(), 0, "{}", r.summary());
    assert!(std::fs::read(out.join("a.rbm")).unwrap() == first);

    // the single-file form
    let single = f.dir.path().join("one.rbm");
    let one = [f.mods.join("a.rbm")];
    assert_eq!(run(&opts(&f, &one, &single, false)).unwrap().failed(), 0);
    assert!(std::fs::read(&single).unwrap() == first);
    assert_eq!(run(&opts(&f, &one, &single, false)).unwrap().failed(), 1);

    // the output folder is the folder of an input; the output is the input
    let err = run(&opts(&f, &ins, &f.mods, true)).unwrap_err();
    assert!(err.1.contains("folder of the input"), "{}", err.1);
    let files: Vec<PathBuf> = vec![f.mods.join("a.rbm"), f.mods.join("b.rbm")];
    assert!(run(&opts(&f, &files, &f.mods, true)).is_err());
    assert!(run(&opts(&f, &one, &one[0], true)).is_err());
    assert_eq!(std::fs::read_dir(&f.mods).unwrap().count(), 2);

    // two inputs with the same file name
    let other = f.dir.path().join("other");
    std::fs::create_dir(&other).unwrap();
    std::fs::copy(f.mods.join("a.rbm"), other.join("a.rbm")).unwrap();
    let twice = [f.mods.join("a.rbm"), other.join("a.rbm")];
    let out2 = f.dir.path().join("out2");
    let r = run(&opts(&f, &twice, &out2, false)).unwrap();
    assert_eq!((r.failed(), r.dat_hashes), (1, 1), "{}", r.summary());
}

#[test]
fn an_already_recrypted_mod_is_checked_and_copied_unchanged() {
    let f = fixture(vec![("a.rbm", good_a()), ("b.rbm", good_b())]);
    let first = f.dir.path().join("first");
    assert_eq!(
        run(&opts(&f, std::slice::from_ref(&f.mods), &first, false))
            .unwrap()
            .failed(),
        0
    );

    // no key file is given, found or needed
    let second = f.dir.path().join("second");
    let ins = [first.clone()];
    let o = Options {
        rb20ful: None,
        iso: None,
        ..opts(&f, &ins, &second, false)
    };
    let r = run(&o).unwrap();
    assert_eq!(r.failed(), 0, "{}", r.summary());
    assert_eq!(r.dat_hashes, 0);
    assert!(
        r.summary()
            .starts_with("0 recrypted, 2 already recrypted (copied), 0 failed")
    );
    for (name, files) in &f.files {
        let b = std::fs::read(second.join(name)).unwrap();
        assert!(b == std::fs::read(first.join(name)).unwrap());
        check_recrypted(&b, files);
    }

    // a mix: the key file is hashed once, for the mod that needs it
    let mix = f.dir.path().join("mix");
    let ins = [first.join("a.rbm"), f.mods.join("b.rbm")];
    let r = run(&opts(&f, &ins, &mix, false)).unwrap();
    assert_eq!((r.failed(), r.dat_hashes), (0, 1), "{}", r.summary());
    assert!(
        r.summary()
            .starts_with("1 recrypted, 1 already recrypted (copied), 0 failed")
    );
}

#[test]
fn the_binary_summarizes_failures_and_exits_1() {
    let f = fixture(vec![("a.rbm", good_a())]);
    let first = f.dir.path().join("first");
    assert_eq!(
        run(&opts(&f, std::slice::from_ref(&f.mods), &first, false))
            .unwrap()
            .failed(),
        0
    );
    let junk = f.dir.path().join("junk.rbm");
    std::fs::write(&junk, b"not a mod").unwrap();
    let out = f.dir.path().join("out");
    let run_orb = |extra: &[&Path]| {
        orb()
            .args(["mod", "recrypt"])
            .arg(first.join("a.rbm"))
            .arg(&junk)
            .arg("-o")
            .arg(&out)
            .args(extra)
            .output()
            .unwrap()
    };
    let o = run_orb(&[]);
    let err = stderr(&o);
    assert_eq!(o.status.code(), Some(1), "{err}");
    assert!(
        err.contains("0 recrypted, 1 already recrypted (copied), 1 failed"),
        "{err}"
    );
    assert!(err.contains("junk.rbm"), "{err}");
    assert!(
        std::fs::read(out.join("a.rbm")).unwrap() == std::fs::read(first.join("a.rbm")).unwrap()
    );

    // --force overwrites the existing output but the junk file still fails: exit 1
    let o = run_orb(&[Path::new("--force")]);
    assert_eq!(o.status.code(), Some(1), "{}", stderr(&o));
    // only the good mod, with --force over its existing output: all good, exit 0
    let o = orb()
        .args(["mod", "recrypt", "--force"])
        .arg(first.join("a.rbm"))
        .arg("-o")
        .arg(&out)
        .output()
        .unwrap();
    assert_eq!(o.status.code(), Some(0), "{}", stderr(&o));
}

/// An output that ends in .rbm is a file name: never made a folder; and a run in which nothing
/// passes the checks writes nothing, not even the output folder.
#[test]
fn a_failed_run_creates_no_output_folder() {
    let f = fixture(vec![("a.rbm", good_a()), ("b.rbm", good_b())]);
    let dir = f.dir.path();

    // a missing input with a .rbm output: a usage error, no folder (the probe of the review)
    let ghost = [dir.join("nosuch.rbm")];
    let named = dir.join("out.rbm");
    let err = run(&opts(&f, &ghost, &named, false)).unwrap_err();
    assert_eq!(err.0, 2, "{}", err.1);
    assert!(!named.exists());
    let o = orb()
        .args(["mod", "recrypt"])
        .arg(&ghost[0])
        .arg("-o")
        .arg(&named)
        .output()
        .unwrap();
    assert_eq!(o.status.code(), Some(2), "{}", stderr(&o));
    assert!(!named.exists());

    // several inputs with a .rbm output
    let two = [f.mods.join("a.rbm"), f.mods.join("b.rbm")];
    assert_eq!(run(&opts(&f, &two, &named, false)).unwrap_err().0, 2);
    assert!(!named.exists());

    // a missing input with a folder output: it fails, nothing is made
    let out = dir.join("outdir");
    let r = run(&opts(&f, &ghost, &out, false)).unwrap();
    assert_eq!(r.failed(), 1);
    assert!(!out.exists());

    // a bad key file in bulk mode: every mod fails, nothing is made
    let blank = dir.join("blank.dat");
    drop(blank_dat(&blank));
    let o = Options {
        rb20ful: Some(&blank),
        iso: None,
        expected_sha256: embedded::DAT_SHA256,
        ..opts(&f, std::slice::from_ref(&f.mods), &out, false)
    };
    let r = run(&o).unwrap();
    assert_eq!(r.failed(), 2, "{}", r.summary());
    assert!(!out.exists());

    // and then a good mod to out.rbm is written as the file out.rbm
    let one = [f.mods.join("a.rbm")];
    assert_eq!(run(&opts(&f, &one, &named, false)).unwrap().failed(), 0);
    assert!(named.is_file());
}

/// The recrypted shipped mods (from the CD image, `ORB_CD_ISO`) are recrypted: copied unchanged,
/// no key file.
#[test]
fn the_recrypted_shipped_mods_pass_the_checks() {
    let Some(dir) = orb_testkit::recrypted_mods().map(Path::to_path_buf) else {
        return;
    };
    let tmp = tempfile::tempdir().unwrap();
    let out = tmp.path().join("out");
    let o = Options {
        inputs: std::slice::from_ref(&dir),
        output: &out,
        rb20ful: None,
        iso: None,
        force: false,
        expected_sha256: embedded::DAT_SHA256,
    };
    let r = run(&o).unwrap();
    assert_eq!(r.failed(), 0, "{}", r.summary());
    assert_eq!(r.dat_hashes, 0);
    assert!(!r.results.is_empty());
    for (i, o, outcome) in &r.results {
        assert_eq!(*outcome, Outcome::AlreadyRecrypted);
        assert!(std::fs::read(o).unwrap() == std::fs::read(i).unwrap());
    }
    eprintln!("{} recrypted mods checked and copied", r.results.len());
}

/// Gated (`ORB_CD_ISO`: the CD's key file and the original shipped mods are taken from it): bulk
/// over the originals equals recrypting each alone, hashes the key file once, and gives the
/// pinned SHA-256s of the recrypted mods (docs/test_machine.md 7).
#[test]
fn shipped_mods_bulk_equals_single_with_the_real_key_file() {
    let Some(cd) = orb_testkit::cd_files() else {
        return;
    };
    let (orig, dat) = (cd.orig_mods.clone(), cd.dat.clone());
    let tmp = tempfile::tempdir().unwrap();
    let out = tmp.path().join("bulk");
    let o = Options {
        inputs: &[orig],
        output: &out,
        rb20ful: Some(&dat),
        iso: None,
        force: false,
        expected_sha256: embedded::DAT_SHA256,
    };
    let r = run(&o).unwrap();
    assert_eq!(r.failed(), 0, "{}", r.summary());
    assert_eq!(r.dat_hashes, 1);
    assert!(!r.results.is_empty());
    for (input, output, outcome) in &r.results {
        assert!(
            matches!(outcome, Outcome::Recrypted { .. }),
            "{input:?}: {outcome:?}"
        );
        let name = input.file_name().unwrap();
        let one = tmp.path().join(name);
        let single = Options {
            inputs: std::slice::from_ref(input),
            output: &one,
            ..o
        };
        assert_eq!(run(&single).unwrap().failed(), 0);
        let bulk = std::fs::read(output).unwrap();
        assert!(
            std::fs::read(&one).unwrap() == bulk,
            "{name:?}: bulk != single"
        );
        let pinned = orb_testkit::RECRYPTED_MODS
            .iter()
            .find(|(m, _)| std::ffi::OsStr::new(m) == name);
        assert_eq!(
            Some(orb_testkit::sha256_hex(&bulk).as_str()),
            pinned.map(|p| p.1),
            "{name:?}: != the pinned SHA-256"
        );
    }
    eprintln!(
        "{} shipped mods: bulk equals single, one hash",
        r.results.len()
    );
}
