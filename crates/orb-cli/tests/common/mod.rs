//! Shared by the CLI tests: the repository root and the standard mod, the committed
//! `assets/Standard.rbm` that orb embeds as its built-in kit (task-067; none of the tests needs
//! the original's files or the CD; those that build the mod take the CD's files from the CD image,
//! by `orb-testkit`, and skip without `ORB_CD_ISO`).

#![allow(dead_code)]

use std::path::{Path, PathBuf};

pub fn root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../..")
}

/// A path from the environment (`None` when unset or empty); a relative one is relative to the
/// workspace root, as cargo runs the tests in the crate's directory.
pub fn env_path(name: &str) -> Option<PathBuf> {
    std::env::var_os(name)
        .filter(|v| !v.is_empty())
        .map(|v| root().join(v))
}

/// The standard mod: `assets/Standard.rbm`, the file orb embeds.
pub fn standard_mod() -> &'static Path {
    static MOD: std::sync::OnceLock<PathBuf> = std::sync::OnceLock::new();
    MOD.get_or_init(|| root().join("assets/Standard.rbm"))
}

/// `ReBirth.dll` or `Rebirth.exe` of the CD image (`ORB_CD_ISO`, extracted once per process by
/// `orb-testkit`), or `None` (with the "skipped" line) when it is not set. The DLL is byte for
/// byte the installed one; the exe is the CD's, not the patched installed one.
pub fn original(name: &str) -> Option<PathBuf> {
    let cd = orb_testkit::cd_files()?;
    match name {
        "ReBirth.dll" => Some(cd.dll.clone()),
        "Rebirth.exe" => Some(cd.exe.clone()),
        _ => panic!("{name}: not a file of the CD helper"),
    }
}
