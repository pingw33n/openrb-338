//! Shared by the CLI tests: the repository root and the standard mod, built once per test
//! binary by `rb standard-mod` from `original/ReBirth.dll` and `original/Rebirth.exe` into
//! cargo's temporary directory for integration tests (task-036).

#![allow(dead_code)]

use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::OnceLock;

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

/// The standard mod, made by `rb standard-mod` the first time it is asked for.
pub fn standard_mod() -> &'static Path {
    static MOD: OnceLock<PathBuf> = OnceLock::new();
    MOD.get_or_init(|| {
        // one file per test binary (they run in parallel), written whole and then renamed
        let dir = PathBuf::from(env!("CARGO_TARGET_TMPDIR"));
        let path = dir.join(format!("standard-{}.rbm", env!("CARGO_CRATE_NAME")));
        let tmp = dir.join(format!(
            "standard-{}.{}.tmp",
            env!("CARGO_CRATE_NAME"),
            std::process::id()
        ));
        let o = Command::new(env!("CARGO_BIN_EXE_rb"))
            .arg("standard-mod")
            .arg("--rebirth-dll")
            .arg(root().join("original/ReBirth.dll"))
            .arg("-o")
            .arg(&tmp)
            .output()
            .unwrap();
        assert!(
            o.status.success(),
            "rb standard-mod: {}",
            String::from_utf8_lossy(&o.stderr)
        );
        std::fs::rename(&tmp, &path).unwrap();
        path
    })
}
