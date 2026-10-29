//! Tests of the `rbm` crate. The synthetic ones always run; the real-file ones take the CD's
//! RB20FUL.DAT and the four shipped mods from the CD image (`ORB_CD_ISO`, by `orb-testkit`) and
//! skip with a message when it is not set. None of that material is in the repository.
//! `recrypt_python.rs` checks `recrypt` against `tools/rbm.py` on synthetic mods, with no input.

#[path = "../../../tests/common/python.rs"]
mod python;

use std::io::{self, Read, Seek, SeekFrom};
use std::path::{Path, PathBuf};

use orb_engine::samples::{self, Sample, SampleSet};
use rbm::*;

// ---------------------------------------------------------------- synthetic material

/// A stand-in RB20FUL.DAT of the right size whose bytes are a function of the offset.
struct VirtualDat {
    pos: u64,
    magic: bool,
    len: u64,
}

impl VirtualDat {
    fn new() -> Self {
        VirtualDat {
            pos: 0,
            magic: true,
            len: DAT_HEADER + DAT_BODY,
        }
    }

    fn byte(&self, off: u64) -> u8 {
        if off < 4 && self.magic {
            return DAT_MAGIC.to_be_bytes()[off as usize];
        }
        let x = off.wrapping_mul(0x9E37_79B9_7F4A_7C15) ^ (off >> 5);
        (x >> 29) as u8
    }
}

impl Read for VirtualDat {
    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        let n = (buf.len() as u64).min(self.len.saturating_sub(self.pos)) as usize;
        for (i, b) in buf[..n].iter_mut().enumerate() {
            *b = self.byte(self.pos + i as u64);
        }
        self.pos += n as u64;
        Ok(n)
    }
}

impl Seek for VirtualDat {
    fn seek(&mut self, from: SeekFrom) -> io::Result<u64> {
        self.pos = match from {
            SeekFrom::Start(p) => p,
            SeekFrom::End(d) => self.len.wrapping_add_signed(d),
            SeekFrom::Current(d) => self.pos.wrapping_add_signed(d),
        };
        Ok(self.pos)
    }
}

fn aiff(frames: usize, seed: i16) -> Vec<u8> {
    let mut d = Vec::new();
    d.extend(b"FORM");
    d.extend(((4 + 8 + 18 + 8 + 8 + 2 * frames) as u32).to_be_bytes());
    d.extend(b"AIFF");
    d.extend(b"COMM");
    d.extend(18u32.to_be_bytes());
    d.extend(1u16.to_be_bytes());
    d.extend((frames as u32).to_be_bytes());
    d.extend(16u16.to_be_bytes());
    d.extend([0x40, 0x0E, 0xAC, 0x44, 0, 0, 0, 0, 0, 0]); // 44100 Hz, 80-bit extended
    d.extend(b"SSND");
    d.extend((8 + 2 * frames as u32).to_be_bytes());
    d.extend([0u8; 8]);
    for i in 0..frames {
        d.extend(((i as i16).wrapping_mul(31).wrapping_add(seed)).to_be_bytes());
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

fn embf(name: &str, file: &[u8]) -> Vec<u8> {
    let mut p = name.as_bytes().to_vec();
    p.push(0);
    p.extend(file);
    p
}

fn cat(chunks: &[(&[u8; 4], Vec<u8>)]) -> Vec<u8> {
    let mut b = b"CAT PRBM".to_vec();
    b.splice(4..4, [0u8; 4]);
    for (t, p) in chunks {
        chunk(&mut b, t, p);
    }
    let size = (b.len() - 8) as u32;
    b[4..8].copy_from_slice(&size.to_be_bytes());
    b
}

fn info_chunk() -> Vec<u8> {
    let mut i = vec![0u8; 1280];
    i[..5].copy_from_slice(b"hello");
    i[601..605].copy_from_slice(b"ftp:");
    i[802..806].copy_from_slice(b"web:");
    i[1003..1011].copy_from_slice(b"Test Mod");
    i[1073..1077].copy_from_slice(&[0xBC, 1, 2, 3]);
    i
}

/// The scrambled mod that holds `files` (EMBF name, content), HEAD first and INFO last, and the
/// key table it was scrambled with (from the virtual DAT).
fn synthetic_mod(files: &[(&str, Vec<u8>)]) -> (Vec<u8>, KeyTable) {
    let mut chunks: Vec<(&[u8; 4], Vec<u8>)> = vec![(b"HEAD", vec![7u8; 256])];
    for (n, f) in files {
        chunks.push((b"EMBF", embf(n, f)));
    }
    chunks.push((b"INFO", info_chunk()));
    let clear = cat(&chunks);
    let table = KeyTable::read(&Rbm::parse(clear).unwrap(), &mut VirtualDat::new()).unwrap();
    for (i, c) in chunks.iter_mut().enumerate() {
        if c.0 == b"EMBF" {
            scramble(&mut c.1, i as u16, &table).unwrap();
        }
    }
    (cat(&chunks), table)
}

fn builtin() -> SampleSet {
    SampleSet::from_samples(
        (0..samples::COUNT)
            .map(|i| Sample::from_audio(&[i as i16 + 1; 7]))
            .collect(),
    )
}

fn test_files() -> Vec<(&'static str, Vec<u8>)> {
    vec![
        ("12017.JPG", vec![0xFF; 3001]),
        ("tr808bd.AIF", aiff(900, 5)),   // other case, scrambled
        ("TR808SD1.aif", aiff(100, 9)),  // under 1024 bytes: not scrambled
        ("TR909rcy.aif", aiff(1500, 3)), // scrambled
        ("Startup.aif", aiff(777, 11)),
        ("TR808BD.aif", aiff(600, 99)), // a second TR808BD: the first wins
        ("Demo.rbs", (0..2500).map(|i| (i * 7) as u8).collect()),
    ]
}

#[test]
fn synthetic_mod_round_trips() {
    let files = test_files();
    let (data, table) = synthetic_mod(&files);
    let rbm = Rbm::parse(data).unwrap();
    assert!(rbm.warnings().is_empty());
    assert_eq!(rbm.chunks().len(), files.len() + 2);
    assert_eq!(rbm.embedded().count(), files.len());

    let info = rbm.info().unwrap();
    assert_eq!(info.name, "Test Mod");
    assert_eq!(info.text, "hello");
    assert_eq!(info.ftp, "ftp:");
    assert_eq!(info.web, "web:");
    assert_eq!(info.version, Some((1, 2, 3)));

    // The scrambled bytes differ from the clear ones where they should; the API undoes it.
    for (i, (n, f)) in files.iter().enumerate() {
        // The second TR808BD.aif (index 5) is shadowed by the first (index 1).
        let want = if i == 5 { &files[1].1 } else { f };
        assert!(&rbm.file_by_name(n, &table).unwrap() == want, "{n}");
    }
    assert_eq!(rbm.file_by_name("demo.RBS", &table).unwrap(), files[6].1);
    assert!(matches!(
        rbm.file_by_name("nope", &table),
        Err(RbmError::NoSuchFile(_))
    ));
    // The table read back from the same DAT is the same: key pages are a function of the mod.
    let again = KeyTable::read(&rbm, &mut VirtualDat::new()).unwrap();
    assert_eq!(again, table);
}

#[test]
fn samples_map_by_name_with_builtin_fallback() {
    let files = test_files();
    let (data, table) = synthetic_mod(&files);
    let rbm = Rbm::parse(data).unwrap();
    let bi = builtin();
    let l = rbm.load_samples(&table, &bi).unwrap();

    let expect = |name: &str, f: &Vec<u8>| (name.to_string(), Sample::from_aiff(f).unwrap());
    // (resource id, expected sample)
    let from_mod = [
        (1000u16, expect("TR808BD", &files[1].1).1), // first match, case-insensitive
        (1001, expect("TR808SD1", &files[2].1).1),
        (1045, expect("TR909rcy", &files[3].1).1),
        (1024, expect("Startup", &files[4].1).1),
    ];
    for (id, s) in &from_mod {
        assert!(l.is_from_mod(*id), "{id}");
        assert_eq!(l.set.get(*id), s, "{id}");
    }
    // Every other slot is the built-in one.
    let mut n_mod = 0;
    for id in samples::FIRST_ID..=samples::LAST_ID {
        if l.is_from_mod(id) {
            n_mod += 1;
        } else {
            assert_eq!(l.set.get(id), bi.get(id), "{id}");
        }
    }
    assert_eq!(n_mod, from_mod.len());
    assert!(!l.is_from_mod(0) && !l.is_from_mod(2000));

    // A mod with no samples at all is the built-in set.
    let (data, table) = synthetic_mod(&[("12017.JPG", vec![1; 10])]);
    let l = Rbm::parse(data).unwrap().load_samples(&table, &bi).unwrap();
    assert_eq!(l.set, bi);
}

#[test]
fn bad_sample_is_an_error() {
    let (data, table) = synthetic_mod(&[("TR808BD.aif", vec![0x55; 3000])]);
    let r = Rbm::parse(data).unwrap().load_samples(&table, &builtin());
    assert!(matches!(r, Err(RbmError::Sample(..))), "{r:?}");
}

#[test]
fn slot_names_cover_the_ids() {
    assert_eq!(slot_name(1000), Some("TR808BD.aif"));
    assert_eq!(slot_name(1023), Some("TR808RS.aif"));
    assert_eq!(slot_name(1024), Some("Startup.aif"));
    assert_eq!(slot_name(1025), Some("TR909bd1.aif"));
    assert_eq!(slot_name(1045), Some("TR909rcy.aif"));
    assert_eq!(slot_name(999), None);
    assert_eq!(slot_name(1046), None);
}

#[test]
fn non_prbm_type_warns() {
    let (mut data, _) = synthetic_mod(&test_files());
    data[8..12].copy_from_slice(b"XXXX");
    let rbm = Rbm::parse(data).unwrap();
    assert_eq!(rbm.warnings().len(), 1);
}

#[test]
fn bad_key_file() {
    let (data, _) = synthetic_mod(&test_files());
    let rbm = Rbm::parse(data).unwrap();
    let mut d = VirtualDat::new();
    d.magic = false;
    assert!(matches!(
        KeyTable::read(&rbm, &mut d),
        Err(RbmError::KeyFile(_))
    ));
    let mut d = VirtualDat::new();
    d.len = 100;
    assert!(matches!(
        KeyTable::read(&rbm, &mut d),
        Err(RbmError::KeyFile(_))
    ));
    let mut d = VirtualDat::new();
    d.len = 1000;
    assert!(KeyTable::read(&rbm, &mut d).is_err());
    assert!(matches!(
        KeyTable::from_bytes(&[0; 10]),
        Err(RbmError::KeyTable)
    ));
}

/// Parses, reads the key table and loads everything; the result does not matter, only that
/// nothing panics.
fn exercise(data: Vec<u8>, bi: &SampleSet) {
    if let Ok(rbm) = Rbm::parse(data) {
        if let Ok(t) = KeyTable::read(&rbm, &mut VirtualDat::new()) {
            for c in rbm.chunks().to_vec() {
                let _ = rbm.file(&c, &t);
            }
            let _ = rbm.load_samples(&t, bi);
        }
        let _ = rbm.info();
        let _ = rbm.find("TR808BD.aif");
    }
}

#[test]
fn malformed_input_does_not_panic() {
    let (data, _) = synthetic_mod(&test_files());
    let bi = builtin();
    // Every truncation point of the first 600 bytes, then a stride over the rest.
    for n in (0..600).chain((600..data.len()).step_by(97)) {
        exercise(data[..n].to_vec(), &bi);
    }
    // Byte flips, with a fixed generator.
    let mut x = 0x1234_5678u32;
    for _ in 0..400 {
        let mut d = data.clone();
        for _ in 0..1 + x % 3 {
            x = x.wrapping_mul(1_664_525).wrapping_add(1_013_904_223);
            let at = (x >> 8) as usize % d.len();
            x = x.wrapping_mul(1_664_525).wrapping_add(1_013_904_223);
            d[at] ^= 1 << ((x >> 20) % 8);
        }
        exercise(d, &bi);
    }
    // A header-only and a hostile-size file.
    exercise(b"CAT ".to_vec(), &bi);
    let mut h = b"CAT \xFF\xFF\xFF\xFFPRBMEMBF\xFF\xFF\xFF\xFF".to_vec();
    exercise(h.clone(), &bi);
    h.extend([0; 8]);
    exercise(h, &bi);
}

// ---------------------------------------------------------------- real files

/// The CD's key file and the four shipped mods (not recrypted), from the CD image
/// (`ORB_CD_ISO`, by `orb-testkit`), or a skip.
fn real_env() -> Option<(PathBuf, Vec<PathBuf>)> {
    let cd = orb_testkit::cd_files()?;
    let mut mods: Vec<PathBuf> = std::fs::read_dir(&cd.orig_mods)
        .expect("the extracted mods are not readable")
        .map(|e| e.unwrap().path())
        .filter(|p| p.extension().is_some_and(|e| e.eq_ignore_ascii_case("rbm")))
        .collect();
    mods.sort();
    assert_eq!(mods.len(), 4, "the four shipped mods");
    Some((cd.dat.clone(), mods))
}

/// `tools/rbm.py extract` of the mod into a fresh directory, or None when python is missing.
fn python_extract(modp: &Path, dat: &Path) -> Option<PathBuf> {
    let script = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../tools/rbm.py");
    let out = std::env::temp_dir().join(format!(
        "rbm-test-{}-{}",
        std::process::id(),
        modp.file_stem()?.to_string_lossy()
    ));
    let _ = std::fs::remove_dir_all(&out);
    let r = python::python()
        .arg(&script)
        .arg("extract")
        .arg(modp)
        .arg(dat)
        .arg("-o")
        .arg(&out)
        .output();
    match r {
        Ok(o) if o.status.success() => Some(out),
        Ok(o) => panic!("rbm.py failed: {}", String::from_utf8_lossy(&o.stderr)),
        Err(e) => panic!("cannot run python for the rbm.py comparison: {e}"),
    }
}

#[test]
fn real_mods_equal_the_reference_extraction() {
    let Some((dat, mods)) = real_env() else {
        return;
    };
    let bi = builtin();
    for m in &mods {
        let rbm = Rbm::parse(std::fs::read(m).unwrap()).unwrap();
        let mut f = std::fs::File::open(&dat).unwrap();
        let table = KeyTable::read(&rbm, &mut f).unwrap();
        let info = rbm.info().expect("INFO");
        assert!(!info.name.is_empty());
        eprintln!(
            "{}: {:?} {:?}, {} chunks",
            m.display(),
            info.name,
            info.version,
            rbm.chunks().len()
        );

        // All 46 slots come from the mod (docs/rbm.md section 3) and parse.
        let l = rbm.load_samples(&table, &bi).unwrap();
        for id in samples::FIRST_ID..=samples::LAST_ID {
            assert!(l.is_from_mod(id), "{} slot {id}", m.display());
        }

        let dir = python_extract(m, &dat).expect("rbm.py extract");
        let embedded: Vec<&Chunk> = rbm.embedded().collect();
        let base = |c: &Chunk| {
            Path::new(c.name.as_deref().unwrap())
                .file_name()
                .unwrap()
                .to_owned()
        };
        let mut checked = 0;
        for (i, c) in embedded.iter().enumerate() {
            // rbm.py writes by base name, so a repeated name could not be compared: the shipped
            // mods have none.
            assert!(
                !embedded[i + 1..].iter().any(|d| base(d) == base(c)),
                "{} has a repeated file name {:?}",
                m.display(),
                base(c)
            );
            let want = std::fs::read(dir.join(base(c))).unwrap();
            assert_eq!(
                rbm.file(c, &table).unwrap(),
                want,
                "{} {}",
                m.display(),
                c.name.as_deref().unwrap()
            );
            checked += 1;
        }
        // The samples equal the reference's AIFF files, parsed.
        for id in samples::FIRST_ID..=samples::LAST_ID {
            let name = slot_name(id).unwrap();
            let want =
                std::fs::read(dir.join(rbm.find(name).unwrap().name.as_deref().unwrap())).unwrap();
            assert_eq!(
                l.set.get(id),
                &Sample::from_aiff(&want).unwrap(),
                "{} {name}",
                m.display()
            );
        }
        eprintln!("  {checked} files and 46 samples equal tools/rbm.py");
        let _ = std::fs::remove_dir_all(dir);
    }
}

// ---------------------------------------------------------------- recrypt (task-059)

/// The files of `r` by lower-case name, in file order per name; the recrypt pad left out.
fn files_by_name(r: &Rbm, t: &KeyTable) -> std::collections::BTreeMap<String, Vec<Vec<u8>>> {
    let mut m = std::collections::BTreeMap::<String, Vec<Vec<u8>>>::new();
    for c in r.embedded() {
        let n = c.name.clone().unwrap();
        if n != PAD_NAME {
            m.entry(n.to_ascii_lowercase())
                .or_default()
                .push(r.file(c, t).unwrap());
        }
    }
    m
}

/// Checks a recrypted mod against its source: the pages are the embedded window, the embedded
/// bytes cover every file, every file and sample is the same, every scrambled file sits at an
/// ordinal whose period under its reader's key fits the window, the non-EMBF chunks keep their
/// bytes, and recrypting again changes nothing. Returns the embedded table.
fn check_recrypted(src: &Rbm, table: &KeyTable, out: &[u8], label: &str) -> KeyTable {
    let back = Rbm::parse(out.to_vec()).unwrap();
    assert_eq!(back.key_pages(), [embedded::WINDOW; 2], "{label}");
    let t = KeyTable::embedded(&back).expect("the embedded window");
    assert_eq!(t.known(), [embedded::WINDOW_LEN; 2]);
    assert!(back.all_covered(&t) && back.slots_covered(&t), "{label}");
    assert_eq!(
        files_by_name(&back, &t),
        files_by_name(src, table),
        "{label}"
    );
    for c in back.embedded() {
        if c.size > CLEAR {
            assert!(period(back.chunk_key(c)) <= embedded::WINDOW_LEN, "{label}");
        }
    }
    assert_eq!(
        back.embedded().count(),
        src.embedded().count() + 1,
        "{label}"
    );
    assert!(back.embedded().count() <= MAX_EMBF);
    let others = |r: &Rbm| -> Vec<(Vec<u8>, Vec<u8>)> {
        r.chunks()
            .iter()
            .filter(|c| &c.tag != b"EMBF" && &c.tag != b"    ")
            .map(|c| (c.tag.to_vec(), r.raw_payload(c).to_vec()))
            .collect()
    };
    assert_eq!(others(&back), others(src), "{label}");
    assert_eq!(back.info(), src.info(), "{label}");
    let bi = builtin();
    assert_eq!(
        back.load_samples(&t, &bi).unwrap().set,
        src.load_samples(table, &bi).unwrap().set,
        "{label}"
    );
    assert_eq!(recrypt(&back, &t).unwrap(), out, "{label}: recrypt again");
    t
}

#[test]
fn recrypt_needs_only_the_embedded_window() {
    let files = test_files();
    let (data, table) = synthetic_mod(&files);
    let rbm = Rbm::parse(data).unwrap();
    // A mod as packed selects other pages: the embedded bytes are not its table.
    assert!(KeyTable::embedded(&rbm).is_none());
    let out = recrypt(&rbm, &table).unwrap();
    check_recrypted(&rbm, &table, &out, "synthetic");
    // The second TR808BD.aif still comes after the first (the lookup takes the first).
    let back = Rbm::parse(out).unwrap();
    let bds: Vec<usize> = back
        .embedded()
        .filter(|c| {
            c.name
                .as_deref()
                .unwrap()
                .eq_ignore_ascii_case("tr808bd.aif")
        })
        .map(|c| c.ordinal)
        .collect();
    assert_eq!(bds.len(), 2);
    assert_eq!(back.reader(back.find("TR808BD.aif").unwrap()), Reader::Dll);
    assert_eq!(
        back.file_by_name("TR808BD.aif", &KeyTable::embedded(&back).unwrap())
            .unwrap(),
        files[1].1
    );
    // HEAD stays first and INFO last.
    assert_eq!(&back.chunks()[0].tag, b"HEAD");
    assert_eq!(&back.chunks().last().unwrap().tag, b"INFO");
}

#[test]
fn the_exe_keys_its_files_by_the_whole_ordinal() {
    // A JPEG (read by the exe) and a slot sample (read by the DLL) past ordinal 255.
    let jpg: Vec<u8> = (0..4000).map(|i| (i * 13) as u8).collect();
    let bd = aiff(1500, 7);
    let mut chunks: Vec<(&[u8; 4], Vec<u8>)> = vec![(b"HEAD", vec![7u8; 256])];
    chunks.extend((0..299).map(|_| (b"    ", Vec::new())));
    chunks.push((b"EMBF", embf("12017.jpg", &jpg))); // ordinal 300
    chunks.push((b"EMBF", embf("TR808BD.aif", &bd))); // ordinal 301
    chunks.push((b"INFO", info_chunk()));
    let table = KeyTable::read(&Rbm::parse(cat(&chunks)).unwrap(), &mut VirtualDat::new()).unwrap();
    scramble(&mut chunks[300].1, 300, &table).unwrap(); // the exe's key
    scramble(&mut chunks[301].1, 301 % 256, &table).unwrap(); // the DLL's key
    let rbm = Rbm::parse(cat(&chunks)).unwrap();
    let c = rbm.find("12017.jpg").unwrap();
    assert_eq!((rbm.reader(c), rbm.chunk_key(c)), (Reader::Exe, 300));
    assert_eq!(rbm.file(c, &table).unwrap(), jpg);
    let c = rbm.find("tr808bd.aif").unwrap();
    assert_eq!(
        (rbm.reader(c), rbm.chunk_key(c), c.key()),
        (Reader::Dll, 45, 45)
    );
    assert_eq!(rbm.file(c, &table).unwrap(), bd);
    // and the recrypt of such a mod (fillers of an unknown origin are dropped and redone)
    let out = recrypt(&rbm, &table).unwrap();
    check_recrypted(&rbm, &table, &out, "ordinals past 255");
}

#[test]
fn recrypt_limits_and_coverage() {
    // Over the original's 200 EMBF entries with the pad: refused.
    let many: Vec<(String, Vec<u8>)> = (0..200).map(|i| (format!("f{i}"), vec![1u8; 4])).collect();
    let refs: Vec<(&str, Vec<u8>)> = many.iter().map(|(n, b)| (n.as_str(), b.clone())).collect();
    let (data, table) = synthetic_mod(&refs);
    let r = recrypt(&Rbm::parse(data).unwrap(), &table);
    assert!(matches!(r, Err(RbmError::Recrypt(_))), "{r:?}");
    // 199 is fine.
    let (data, table) = synthetic_mod(&refs[..199]);
    let rbm = Rbm::parse(data).unwrap();
    let out = recrypt(&rbm, &table).unwrap();
    check_recrypted(&rbm, &table, &out, "199 files");

    // The embedded window covers every key: the longest period is 2047.
    let (data, table) = synthetic_mod(&test_files());
    let out = recrypt(&Rbm::parse(data).unwrap(), &table).unwrap();
    let back = Rbm::parse(out.clone()).unwrap();
    let t = KeyTable::embedded(&back).unwrap();
    assert_eq!((0..=u16::MAX).map(period).max(), Some(2047));
    assert_eq!((period(0), period(113)), (123, 2044));
    assert!((0..=u16::MAX).all(|k| t.covers(CLEAR + 5000, k)));
    // The original order is kept and no filler is needed: HEAD, the pad, the files, INFO.
    assert_eq!(back.chunks().len(), test_files().len() + 3);
    let names: Vec<&str> = back
        .embedded()
        .map(|c| c.name.as_deref().unwrap())
        .collect();
    let want: Vec<&str> = std::iter::once(PAD_NAME)
        .chain(test_files().iter().map(|(n, _)| *n))
        .collect();
    assert_eq!(names, want);

    // A mod with only page 0 at the window: page 1 is unknown, and its files need the DAT.
    let mut bad = out;
    let pad = back.find(PAD_NAME).unwrap();
    bad[pad.offset + 1020] ^= 1;
    let bad = Rbm::parse(bad).unwrap();
    let t = KeyTable::embedded(&bad).unwrap();
    assert_eq!(t.known(), [embedded::WINDOW_LEN, 0]);
    assert!(!t.covers(CLEAR + 1, 1) && t.covers(CLEAR, 1) && t.covers(CLEAR + 5000, 2));
    let odd = bad
        .embedded()
        .find(|c| c.size > CLEAR && bad.chunk_key(c) % 2 == 1)
        .unwrap();
    assert!(matches!(bad.file(odd, &t), Err(RbmError::KeyNotCovered(_))));
    assert!(!bad.all_covered(&t));
    let mut p = vec![0u8; CLEAR + 200];
    assert!(matches!(
        scramble(&mut p, 3, &t),
        Err(RbmError::KeyNotCovered(_))
    ));
}

#[test]
fn embedded_window_is_the_dats() {
    use sha2::{Digest, Sha256};
    let Some(cd) = orb_testkit::cd_files() else {
        return;
    };
    let bytes = std::fs::read(&cd.dat).unwrap();
    assert_eq!(bytes.len() as u64, DAT_HEADER + DAT_BODY);
    let w = embedded::WINDOW as usize;
    assert_eq!(
        &bytes[w..w + embedded::WINDOW_LEN],
        &embedded::WINDOW_BYTES[..]
    );
    let hex: String = Sha256::digest(&bytes)
        .iter()
        .map(|b| format!("{b:02x}"))
        .collect();
    assert_eq!(hex, embedded::DAT_SHA256);
    let hex: String = Sha256::digest(embedded::WINDOW_BYTES)
        .iter()
        .map(|b| format!("{b:02x}"))
        .collect();
    assert_eq!(hex, embedded::WINDOW_SHA256);
    // The window is PBE2.rbm's page 0 (docs/rbm.md 5.3).
    assert_eq!(embedded::WINDOW, 118_280_160);
}

/// Recrypts every mod of `mods` with the real table and checks it (`check_recrypted`).
fn recrypt_all(dat: &Path, mods: &[PathBuf]) {
    for m in mods {
        let rbm = Rbm::parse(std::fs::read(m).unwrap()).unwrap();
        let table = KeyTable::read(&rbm, &mut std::fs::File::open(dat).unwrap()).unwrap();
        let out = recrypt(&rbm, &table).unwrap();
        check_recrypted(&rbm, &table, &out, &m.display().to_string());
        eprintln!(
            "{}: {} -> {} chunks, {} -> {} bytes",
            m.display(),
            rbm.chunks().len(),
            Rbm::parse(out.clone()).unwrap().chunks().len(),
            std::fs::metadata(m).unwrap().len(),
            out.len()
        );
    }
}

#[test]
fn real_mods_recrypt() {
    let Some((dat, mods)) = real_env() else {
        return;
    };
    recrypt_all(&dat, &mods);
}

/// The real-file tests of `tools/rbm_test.py` (the key file's stride pages, the shipped mods'
/// key pages, every descrambled file valid), run here with the files of the CD image: the
/// script takes their paths from `RBM_TEST_DAT` and `RBM_TEST_MODS`, which this test sets for
/// the child process only.
#[test]
fn the_python_tool_tests_pass_on_the_real_files() {
    let Some(cd) = orb_testkit::cd_files() else {
        return;
    };
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let o = python::python()
        .current_dir(&root)
        .args(["-W", "error::ResourceWarning", "-m", "unittest"])
        .args(["discover", "-s", "tools", "-p", "rbm_test.py"])
        .env("RBM_TEST_DAT", &cd.dat)
        .env("RBM_TEST_MODS", &cd.orig_mods)
        .output()
        .expect("cannot run python for tools/rbm_test.py");
    let err = String::from_utf8_lossy(&o.stderr);
    assert!(o.status.success(), "{err}");
    // the real-file classes ran (they are not skipped)
    assert!(!err.contains("skipped"), "{err}");
}
