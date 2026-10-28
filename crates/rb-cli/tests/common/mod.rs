//! Shared by the CLI tests: the repository root and the standard mod, the committed
//! `assets/Standard.rbm` that rb embeds as its built-in kit (task-067; none of the tests needs
//! `original/`, except those that build the mod, which skip without it).

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

/// The standard mod: `assets/Standard.rbm`, the file rb embeds.
pub fn standard_mod() -> &'static Path {
    static MOD: std::sync::OnceLock<PathBuf> = std::sync::OnceLock::new();
    MOD.get_or_init(|| root().join("assets/Standard.rbm"))
}

/// A file of `original/`, or `None` (with a "skipped" line) when it is not there.
pub fn original(name: &str) -> Option<PathBuf> {
    let p = root().join("original").join(name);
    if p.is_file() {
        Some(p)
    } else {
        eprintln!("skipped: {} is not there", p.display());
        None
    }
}
