//! `rbm::recrypt` against the independent decoder (task-121). Synthetic mods, built here with a
//! synthetic key table (a stand-in `RB20FUL.DAT` whose pages are a function of the offset), are
//! recrypted; the result is decoded by `crates/rbm` and by `tools/rbm.py`, and every file must
//! equal the one the mod was built from. The source is decoded by both as well, so the scrambling
//! the test applies is checked too. No environment variable and no real file is needed, so these
//! run in CI on every platform.
//!
//! The mods cover (docs/rbm.md 12.1, 12.2):
//! - ordinals past 255, where a slot sample's DLL key (the low byte of the ordinal) and a file's
//!   exe key (the whole u16) differ;
//! - payloads of 1024 bytes and less (never scrambled), 1025 and large ones;
//! - 199 files, the most that fit with the pad;
//! - duplicate names (the first one is the one the lookup finds, and the DLL keys it);
//! - foreign chunks (kept), zero-size filler chunks and an earlier recrypt's pad (dropped, redone);
//! - the 46 slot samples next to other files.

#[path = "../../../tests/common/python.rs"]
mod python;

use std::fs::File;
use std::io::{Seek, SeekFrom, Write};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicUsize, Ordering};

use rb_engine::samples;
use rbm::*;
use sha2::{Digest, Sha256};

fn sha(b: &[u8]) -> String {
    Sha256::digest(b)
        .iter()
        .map(|x| format!("{x:02x}"))
        .collect()
}

/// Deterministic bytes: `len` of them, a function of `seed`.
fn bytes(len: usize, seed: u32) -> Vec<u8> {
    let mut x = seed.wrapping_mul(2_654_435_761).wrapping_add(12345);
    (0..len)
        .map(|_| {
            x ^= x << 13;
            x ^= x >> 17;
            x ^= x << 5;
            (x >> 11) as u8
        })
        .collect()
}

/// The stand-in key file's byte at `off`.
fn page_byte(off: u64) -> u8 {
    let x = off.wrapping_mul(0x9E37_79B9_7F4A_7C15) ^ (off >> 5);
    (x >> 29) as u8
}

/// A stand-in key file: the magic, the pages at `offsets` filled with `page_byte`, and the
/// embedded window at its place; everything else zero (a sparse file).
fn write_dat(path: &Path, offsets: &[u64], window: bool) {
    let mut f = File::create(path).unwrap();
    f.set_len(DAT_HEADER + DAT_BODY).unwrap();
    f.write_all(&DAT_MAGIC.to_be_bytes()).unwrap();
    for &off in offsets {
        let page: Vec<u8> = (0..PAGE as u64).map(|i| page_byte(off + i)).collect();
        f.seek(SeekFrom::Start(off)).unwrap();
        f.write_all(&page).unwrap();
    }
    if window {
        f.seek(SeekFrom::Start(embedded::WINDOW)).unwrap();
        f.write_all(embedded::WINDOW_BYTES).unwrap();
    }
}

/// One chunk between HEAD and INFO.
enum Item {
    /// An embedded file: name and content.
    File(String, Vec<u8>),
    /// A chunk of another tag.
    Foreign(&'static [u8; 4], Vec<u8>),
    /// A zero-size chunk of the filler id.
    Filler,
    /// A pad of an earlier recrypt.
    OldPad,
}

fn file(name: &str, len: usize, seed: u32) -> Item {
    Item::File(name.to_string(), bytes(len, seed))
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

fn embf(name: &str, content: &[u8]) -> Vec<u8> {
    let mut p = name.as_bytes().to_vec();
    p.push(0);
    p.extend(content);
    p
}

/// A synthetic mod, scrambled against the stand-in key file: its bytes, its key pages, the files
/// it holds (name and content, in file order, the old pad left out) and its other chunks.
struct Built {
    data: Vec<u8>,
    pages: [u64; 2],
    files: Vec<(String, Vec<u8>)>,
    foreign: Vec<([u8; 4], Vec<u8>)>,
}

static NEXT: AtomicUsize = AtomicUsize::new(0);

fn build(items: &[Item]) -> Built {
    let mut info = vec![0u8; 1280];
    info[1003..1011].copy_from_slice(b"Test Mod");
    info[1073..1077].copy_from_slice(&[0xBC, 1, 2, 3]);
    let mut chunks: Vec<([u8; 4], Vec<u8>)> = vec![(*b"HEAD", vec![7u8; 256])];
    let (mut files, mut foreign) = (Vec::new(), Vec::new());
    for it in items {
        match it {
            Item::File(n, c) => {
                chunks.push((*b"EMBF", embf(n, c)));
                files.push((n.clone(), c.clone()));
            }
            Item::Foreign(t, p) => {
                chunks.push((**t, p.clone()));
                foreign.push((**t, p.clone()));
            }
            Item::Filler => chunks.push((*b"    ", Vec::new())),
            // 1024 bytes in all: the name, its NUL and the rest
            Item::OldPad => chunks.push((*b"EMBF", embf(PAD_NAME, &bytes(1024 - 15, 77)))),
        }
    }
    chunks.push((*b"INFO", info.clone()));
    foreign.push((*b"INFO", info));
    foreign.insert(0, (*b"HEAD", vec![7u8; 256]));

    // the pages follow from the first 1024 bytes of the EMBF payloads, which stay in the clear
    let clear = Rbm::parse(cat(&chunks)).unwrap();
    let pages = clear.key_pages();
    // KeyTable::read wants a key file: a stand-in with just these pages
    let dir = scratch(&format!("build{}", NEXT.fetch_add(1, Ordering::Relaxed)));
    let dat = dir.join("build.dat");
    write_dat(&dat, &pages, false);
    let kt = KeyTable::read(&clear, &mut File::open(&dat).unwrap()).unwrap();
    let _ = std::fs::remove_dir_all(&dir);
    for (i, c) in chunks.iter_mut().enumerate() {
        if &c.0 == b"EMBF" {
            let ch = clear.chunks()[i].clone();
            scramble(&mut c.1, clear.chunk_key(&ch), &kt).unwrap();
        }
    }
    Built {
        data: cat(&chunks),
        pages,
        files,
        foreign,
    }
}

/// (ordinal, name, sha256 of the file) of every EMBF of `mod_file`, decoded by tools/rbm.py with
/// the key file `dat`.
fn python_files(mod_file: &Path, dat: &Path) -> Vec<(usize, String, String)> {
    const DUMP: &str = r#"
import hashlib, json, os, sys
sys.path.insert(0, os.path.join(sys.argv[1], "tools"))
import rbm
r = rbm.read(sys.argv[2])
t = rbm.key_table(r, sys.argv[3])
out = []
for c in r.embf():
    out.append([c.ordinal, c.name, hashlib.sha256(rbm.embedded_file(r, c, t)).hexdigest()])
print(json.dumps(out))
"#;
    let o = python::python()
        .arg("-c")
        .arg(DUMP)
        .arg(Path::new(env!("CARGO_MANIFEST_DIR")).join("../.."))
        .arg(mod_file)
        .arg(dat)
        .output()
        .expect("cannot run python for the rbm.py comparison");
    assert!(
        o.status.success(),
        "tools/rbm.py failed: {}",
        String::from_utf8_lossy(&o.stderr)
    );
    let v: Vec<(usize, String, String)> = serde_json::from_slice(&o.stdout).unwrap();
    v
}

/// The (name, sha256) of the files a mod must hold, in order.
fn expected(b: &Built) -> Vec<(String, String)> {
    b.files.iter().map(|(n, c)| (n.clone(), sha(c))).collect()
}

fn scratch(label: &str) -> PathBuf {
    let d = std::env::temp_dir().join(format!("rbm-recrypt-python-{}-{label}", std::process::id()));
    let _ = std::fs::remove_dir_all(&d);
    std::fs::create_dir_all(&d).unwrap();
    d
}

/// Builds the mod, recrypts it, and requires `crates/rbm` and `tools/rbm.py` to read the same
/// files from the source and from the result. Returns the recrypted mod.
fn check(label: &str, items: &[Item]) -> Vec<u8> {
    let b = build(items);
    let dir = scratch(label);
    let src = dir.join("src.rbm");
    std::fs::write(&src, &b.data).unwrap();
    let src_dat = dir.join("src.dat");
    write_dat(&src_dat, &b.pages, false);
    let out_dat = dir.join("window.dat");
    write_dat(&out_dat, &[], true);
    let want = expected(&b);

    // the source, by tools/rbm.py (the pad of an earlier recrypt is not a file of the mod)
    let from_py = |mod_file: &Path, dat: &Path| -> Vec<(String, String)> {
        python_files(mod_file, dat)
            .into_iter()
            .filter(|(_, n, _)| n != PAD_NAME)
            .map(|(_, n, h)| (n, h))
            .collect()
    };
    assert_eq!(
        from_py(&src, &src_dat),
        want,
        "{label}: the source by rbm.py"
    );

    // recrypt
    let rbm = Rbm::parse(b.data.clone()).unwrap();
    let table = KeyTable::read(&rbm, &mut File::open(&src_dat).unwrap()).unwrap();
    let out = recrypt(&rbm, &table).unwrap();

    // by crates/rbm
    let back = Rbm::parse(out.clone()).unwrap();
    assert_eq!(back.key_pages(), [embedded::WINDOW; 2], "{label}");
    let t = KeyTable::embedded(&back).expect("the embedded window");
    assert!(back.all_covered(&t) && back.slots_covered(&t), "{label}");
    let ours: Vec<(String, String)> = back
        .embedded()
        .filter(|c| c.name.as_deref() != Some(PAD_NAME))
        .map(|c| (c.name.clone().unwrap(), sha(&back.file(c, &t).unwrap())))
        .collect();
    assert_eq!(ours, want, "{label}: the result by crates/rbm");
    assert_eq!(
        back.embedded()
            .filter(|c| c.name.as_deref() == Some(PAD_NAME))
            .count(),
        1,
        "{label}: exactly one pad"
    );

    // by tools/rbm.py, with a key file that holds only the window
    let out_file = dir.join("out.rbm");
    std::fs::write(&out_file, &out).unwrap();
    assert_eq!(
        from_py(&out_file, &out_dat),
        want,
        "{label}: the result by rbm.py"
    );

    // a scrambled file sits at an ordinal whose period fits the window
    for c in back.embedded() {
        if c.size > CLEAR {
            assert!(period(back.chunk_key(c)) <= embedded::WINDOW_LEN, "{label}");
        }
    }
    // the foreign chunks keep their bytes; the fillers and the old pad are gone or redone
    let others: Vec<([u8; 4], Vec<u8>)> = back
        .chunks()
        .iter()
        .filter(|c| &c.tag != b"EMBF" && &c.tag != b"    ")
        .map(|c| (c.tag, back.raw_payload(c).to_vec()))
        .collect();
    let mut a = others.clone();
    let mut e = b.foreign.clone();
    a.sort();
    e.sort();
    assert_eq!(a, e, "{label}: the other chunks");
    // recrypting a recrypted mod changes nothing
    assert_eq!(recrypt(&back, &t).unwrap(), out, "{label}: again");
    let _ = std::fs::remove_dir_all(&dir);
    out
}

/// A payload (name, NUL, file) of exactly `payload` bytes.
fn sized(name: &str, payload: usize, seed: u32) -> Item {
    file(name, payload - name.len() - 1, seed)
}

#[test]
fn ordinals_past_255_use_the_dlls_byte_key_and_the_exes_u16_key() {
    // 150 foreign chunks stay before the first file, so the files start at ordinal 151 and the
    // last ones are past 255: the slot samples (DLL key, ordinal mod 256) come last
    let mut items: Vec<Item> = (0..150)
        .map(|i| Item::Foreign(b"NOTE", bytes(3 + i % 5, i as u32)))
        .collect();
    items.extend((0..110).map(|i| file(&format!("{}.jpg", 12015 + i), 1500 + 37 * i, i as u32)));
    items.push(file("Demo.rbs", 4000, 5));
    for id in samples::FIRST_ID..=samples::LAST_ID {
        items.push(file(
            slot_name(id).unwrap(),
            1100 + 13 * id as usize,
            id as u32,
        ));
    }
    let out = check("ordinals", &items);
    let back = Rbm::parse(out).unwrap();
    // the premise: a slot sample past ordinal 255 whose two keys differ, and a jpg past it too
    let late: Vec<_> = back.embedded().filter(|c| c.ordinal > 255).collect();
    assert!(late.len() >= 40);
    let slot = late
        .iter()
        .find(|c| back.reader(c) == Reader::Dll)
        .expect("a slot sample past ordinal 255");
    assert_ne!(slot.ordinal & 0xFF, slot.ordinal & 0xFFFF);
    assert_eq!(back.chunk_key(slot), (slot.ordinal & 0xFF) as u16);
    assert!(late.iter().any(|c| back.reader(c) == Reader::Exe));
}

#[test]
fn payload_sizes_around_the_unscrambled_head() {
    let mut items = Vec::new();
    for (i, p) in [
        8usize, 9, 16, 1000, 1022, 1023, 1024, 1025, 1026, 1027, 2047, 2048, 2049, 2050, 4095,
        4099, 65_536, 100_001, 1_000_003, 3_000_000,
    ]
    .into_iter()
    .enumerate()
    {
        items.push(sized(&format!("f{i:02}.bin"), p, 100 + i as u32));
    }
    // an empty file (the payload is the name and its NUL), and sample-sized slot files
    items.push(file("empty.bin", 0, 1));
    items.push(sized("TR808BD.aif", 1024, 9));
    items.push(sized("TR808SD1.aif", 1025, 10));
    check("sizes", &items);
}

#[test]
fn a_mod_of_199_files_recrypts() {
    let items: Vec<Item> = (0..199)
        .map(|i| {
            // sizes on both sides of 1024, names of both readers
            let name = if i % 7 == 0 {
                format!("{}.jpg", 12000 + i)
            } else {
                format!("file{i}.dat")
            };
            file(&name, (i * 53) % 3000, i as u32)
        })
        .collect();
    let out = check("199", &items);
    // the 199 files and the pad: the most the original's 200 entries hold
    assert_eq!(Rbm::parse(out).unwrap().embedded().count(), 200);
}

#[test]
fn duplicate_names_keep_the_first_as_the_one_the_lookup_finds() {
    let items = vec![
        file("TR808BD.aif", 2000, 1),
        file("tr808bd.AIF", 3000, 2), // the second: not a slot lookup, so the exe's key
        file("Splash.jpg", 1500, 3),
        file("SPLASH.JPG", 2500, 4),
        file("same.dat", 1100, 5),
        file("same.dat", 1100, 6),
        file("Startup.aif", 1300, 7),
        file("startup.aif", 1300, 8),
    ];
    let out = check("duplicates", &items);
    let back = Rbm::parse(out).unwrap();
    let t = KeyTable::embedded(&back).unwrap();
    // the lookup finds the first of each name
    let bytes_of = |n: &str| back.file_by_name(n, &t).unwrap();
    assert_eq!(bytes_of("TR808BD.aif"), bytes(2000, 1));
    assert_eq!(bytes_of("splash.jpg"), bytes(1500, 3));
}

#[test]
fn foreign_chunks_fillers_and_an_old_pad() {
    let items = vec![
        Item::Foreign(b"ANNO", bytes(33, 1)), // odd size: an IFF pad byte follows
        Item::Filler,
        file("a.dat", 1500, 2),
        Item::Filler,
        Item::Filler,
        Item::OldPad,
        Item::Foreign(b"FOO ", bytes(2000, 3)), // between files: goes last
        file("TR909bd.aif", 5000, 4),
        Item::Filler,
        file("1.jpg", 1025, 5),
        Item::Foreign(b"BAR ", Vec::new()),
    ];
    check("foreign", &items);
}
