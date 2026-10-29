//! The external inputs of the gated tests (task-121). Three variables name them, and every gated
//! test of the workspace skips with one clear message when its input is not set:
//!
//! - `RB_CD_ISO`: the ReBirth 2.01 CD image (the 2001 reissue). [`cd_files`] reads from it, once
//!   per test process, everything the tests need of the original program: `RB20FUL.DAT`,
//!   `ReBirth.dll`, `Rebirth.exe` and the four shipped mods (not recrypted), each checked against
//!   its known SHA-256, and the four mods recrypted from them (`rbm::recrypt`), checked against the
//!   pinned SHA-256 of the recrypted files. Nothing of it is written to the repository: the files
//!   go to a folder of the system's temp directory, named after the image, that later test
//!   processes reuse after checking the hashes again.
//! - `RB_THIRDPARTY_MODS_DIR`: a folder of the user's third-party mods, RECRYPTED (`rb mod
//!   recrypt`), read in place ([`thirdparty_mods`]).
//! - `RB_THIRDPARTY_SONGS_DIR`: a folder of the user's third-party songs, read in place
//!   ([`thirdparty_songs`]).
//!
//! A relative path is relative to the workspace root, as cargo runs a test in its crate's folder.

use std::fs;
use std::path::{Path, PathBuf};
use std::sync::OnceLock;

use rb_cli::cd::{self, CdImage};
use sha2::{Digest, Sha256};

/// The four mods of the CD, as shipped, with the SHA-256 of each (docs/test_machine.md 7).
pub const ORIGINAL_MODS: [(&str, &str); 4] = [
    (
        "Metallicon.rbm",
        "ef68879dd40d6c3c3fd0745c9f89a7fa5784edbdc7af4a6bc3e7ac3a5962057e",
    ),
    (
        "MSM2.rbm",
        "aced273bbb6440449ed0dd14d9999a45305bd74d21dcf80c6b5ec5719c59fb40",
    ),
    (
        "Orbit2.rbm",
        "852e9f9ba207337a781eda7ac609b6bf0fe2040889aabb3be6453d95f510b8fa",
    ),
    (
        "PBE2.rbm",
        "797ea0f74d6fd2ee7d8e70ff0baf375a6d89686c64dfda47bc435006990ad951",
    ),
];

/// The same mods recrypted by `rb mod recrypt` (docs/test_machine.md 7): the files the
/// tests load, and what `rbm::recrypt` must give.
pub const RECRYPTED_MODS: [(&str, &str); 4] = [
    (
        "Metallicon.rbm",
        "12fbe6982b664abd42d03a9a329e7f0597ecddc89b674581ee5c4c7914f2bbd4",
    ),
    (
        "MSM2.rbm",
        "28cb40e0ae02a49a05c866d480ba982da84d5dba5cf3a6605e05f1f4418aa604",
    ),
    (
        "Orbit2.rbm",
        "eb1843b685514f22b379720733dbea4cf58111107eb353d53ea6634829f2114c",
    ),
    (
        "PBE2.rbm",
        "62789353966b7ca7d4c0bd55b14981fdb49b94371c535224bb1f581eb15ccb97",
    ),
];

/// The files taken out of the CD image.
#[derive(Debug)]
pub struct CdFiles {
    /// `RB20FUL.DAT`.
    pub dat: PathBuf,
    /// The installer's `ReBirth.dll` (byte for byte the installed one).
    pub dll: PathBuf,
    /// The installer's `Rebirth.exe` (the CD's, not the patched installed one).
    pub exe: PathBuf,
    /// The four shipped mods, as shipped.
    pub orig_mods: PathBuf,
    /// The four shipped mods, recrypted: what the tests that need mods load.
    pub mods: PathBuf,
}

/// The workspace root.
pub fn root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../..")
}

/// A path from the environment (`None` when unset or empty); a relative one is relative to the
/// workspace root.
pub fn env_path(name: &str) -> Option<PathBuf> {
    std::env::var_os(name)
        .filter(|v| !v.is_empty())
        .map(|v| root().join(v))
}

fn gated(var: &str, what: &str) -> Option<PathBuf> {
    let p = env_path(var);
    if p.is_none() {
        eprintln!("skipped: {var} ({what}) is not set");
    }
    p
}

/// The user's third-party mods (`RB_THIRDPARTY_MODS_DIR`, recrypted), or a skip.
pub fn thirdparty_mods() -> Option<PathBuf> {
    gated(
        "RB_THIRDPARTY_MODS_DIR",
        "the folder of the recrypted third-party mods",
    )
}

/// The user's third-party songs (`RB_THIRDPARTY_SONGS_DIR`), or a skip.
pub fn thirdparty_songs() -> Option<PathBuf> {
    gated(
        "RB_THIRDPARTY_SONGS_DIR",
        "the folder of the third-party songs",
    )
}

/// The CD image (`RB_CD_ISO`), or a skip.
pub fn cd_iso() -> Option<PathBuf> {
    gated("RB_CD_ISO", "the ReBirth 2.01 CD image")
}

/// The files of the CD image, extracted and checked once per test process; `None` (a skip, with
/// the message) when `RB_CD_ISO` is not set. A set variable that does not give the files is a
/// panic naming what is wrong.
pub fn cd_files() -> Option<&'static CdFiles> {
    static FILES: OnceLock<Option<CdFiles>> = OnceLock::new();
    let iso = cd_iso()?;
    FILES
        .get_or_init(|| Some(extract(&iso).unwrap_or_else(|e| panic!("RB_CD_ISO: {e}"))))
        .as_ref()
}

/// The recrypted shipped mods (`Metallicon.rbm`, `MSM2.rbm`, `Orbit2.rbm`, `PBE2.rbm`), or a
/// skip when `RB_CD_ISO` is not set.
pub fn recrypted_mods() -> Option<&'static Path> {
    cd_files().map(|f| f.mods.as_path())
}

/// The SHA-256 of `b` in lowercase hex.
pub fn sha256_hex(b: &[u8]) -> String {
    Sha256::digest(b)
        .iter()
        .map(|x| format!("{x:02x}"))
        .collect()
}

/// A file of the cache folder: present with the expected hash, else written by `make`.
fn cached(
    dir: &Path,
    name: &str,
    sha256: &str,
    make: impl FnOnce() -> Result<Vec<u8>, String>,
) -> Result<PathBuf, String> {
    let path = dir.join(name);
    if let Ok(b) = fs::read(&path)
        && sha256_hex(&b) == sha256
    {
        return Ok(path);
    }
    let b = make()?;
    let got = sha256_hex(&b);
    if got != sha256 {
        return Err(format!("{name} has SHA-256 {got}, expected {sha256}"));
    }
    // a temporary name, then a rename: another test process never sees half a file
    let tmp = dir.join(format!("{name}.{}.tmp", std::process::id()));
    fs::write(&tmp, &b).map_err(|e| format!("{}: {e}", tmp.display()))?;
    // Windows can refuse a rename over a file another test process has open: then the target
    // is fine when it has the right hash (that process wrote it), else retry a few times
    let mut last = String::new();
    for attempt in 0..5 {
        match fs::rename(&tmp, &path) {
            Ok(()) => return Ok(path),
            Err(e) => last = format!("{}: {e}", path.display()),
        }
        if fs::read(&path).is_ok_and(|b| sha256_hex(&b) == sha256) {
            let _ = fs::remove_file(&tmp);
            return Ok(path);
        }
        std::thread::sleep(std::time::Duration::from_millis(50 * (attempt + 1)));
    }
    let _ = fs::remove_file(&tmp);
    Err(last)
}

fn extract(iso: &Path) -> Result<CdFiles, String> {
    let meta = fs::metadata(iso).map_err(|e| format!("{}: {e}", iso.display()))?;
    let stamp = meta
        .modified()
        .ok()
        .and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok())
        .map_or(0, |d| d.as_secs());
    let key = sha256_hex(format!("{}|{}|{stamp}", iso.display(), meta.len()).as_bytes());
    let dir = std::env::temp_dir().join(format!("rb-testkit-{}", &key[..16]));
    let orig_dir = dir.join("orig");
    let mods_dir = dir.join("recrypted");
    for d in [&orig_dir, &mods_dir] {
        fs::create_dir_all(d).map_err(|e| format!("{}: {e}", d.display()))?;
    }
    let mut image = CdImage::open(iso)?;

    let dat = cached(&dir, "RB20FUL.DAT", rbm::embedded::DAT_SHA256, || {
        image
            .rb20ful()?
            .read_all()
            .map_err(|e| format!("cannot read RB20FUL.DAT: {e}"))
    })?;
    // the installer is read once, when a file of it is missing
    let mut installer = None;
    let mut from_installer = |name: &str, sha: &str| -> Result<Vec<u8>, String> {
        if installer.is_none() {
            installer = Some(image.installer()?);
        }
        cd::read_checked(installer.as_ref().unwrap(), name, sha)
    };
    let dll = cached(&dir, "ReBirth.dll", cd::DLL_SHA256, || {
        from_installer("ReBirth.dll", cd::DLL_SHA256)
    })?;
    let exe = cached(&dir, "Rebirth.exe", cd::EXE_SHA256, || {
        from_installer("Rebirth.exe", cd::EXE_SHA256)
    })?;
    let mut originals = Vec::new();
    for (name, sha) in ORIGINAL_MODS {
        let p = cached(&orig_dir, name, sha, || from_installer(name, sha))?;
        originals.push((name, p));
    }
    for (name, sha) in RECRYPTED_MODS {
        let orig = originals
            .iter()
            .find(|(n, _)| *n == name)
            .map(|(_, p)| p.clone())
            .unwrap();
        cached(&mods_dir, name, sha, || {
            let bytes = fs::read(&orig).map_err(|e| format!("{}: {e}", orig.display()))?;
            let rbm = rbm::Rbm::parse(bytes).map_err(|e| format!("{name}: {e}"))?;
            let mut d = fs::File::open(&dat).map_err(|e| format!("{}: {e}", dat.display()))?;
            let table = rbm::KeyTable::read(&rbm, &mut d).map_err(|e| format!("{name}: {e}"))?;
            rbm::recrypt(&rbm, &table).map_err(|e| format!("{name}: {e}"))
        })?;
    }
    Ok(CdFiles {
        dat,
        dll,
        exe,
        orig_mods: orig_dir,
        mods: mods_dir,
    })
}
