//! Embeds every SVG part of `assets/ui` (except the composed demo `panel.svg`) in the
//! binary: writes `$OUT_DIR/parts_gen.rs` with `PARTS: &[(name, uri, bytes)]`, sorted by name.

use std::env;
use std::fmt::Write as _;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::{SystemTime, UNIX_EPOCH};

#[path = "src/version.rs"]
mod version;

/// The output of `git ARGS` in `dir`, trimmed; None when git or the repository is absent.
fn git(dir: &Path, args: &[&str]) -> Option<String> {
    let out = Command::new("git")
        // `git status` would refresh the index file, which the script watches: a rerun loop
        .env("GIT_OPTIONAL_LOCKS", "0")
        .args(args)
        .current_dir(dir)
        .output()
        .ok()?;
    out.status
        .success()
        .then(|| String::from_utf8_lossy(&out.stdout).trim().to_string())
}

/// Writes `RB_UI_VERSION` (task-102): `<YYYY.MMDD.HHMM> (<git>)`, the build time to the
/// minute (task-133), see `src/version.rs`. It never fails the build: without git the string
/// says `(unknown)`.
///
/// Freshness: the script reruns, and so takes a new time, when HEAD, the branch ref, the
/// packed refs or the index change (a commit, a branch switch, a stage; only paths that exist
/// are watched), when a file under `crates/rb-ui` or `assets/ui` changes (a plain edit of a
/// tracked file of the UI), or when one of the variables below changes. An edit elsewhere (an
/// engine crate, docs) is seen as dirty once staged (the index changes); until then the flag
/// and the time stay as they were.
fn set_version(manifest: &Path) {
    let root = manifest.join("../..");
    for v in ["SOURCE_DATE_EPOCH", "RB_BUILD_BRANCH", "GITHUB_REF_NAME"] {
        println!("cargo:rerun-if-env-changed={v}");
    }
    for p in ["crates/rb-ui", "assets/ui"] {
        println!("cargo:rerun-if-changed={}", root.join(p).display());
    }
    // cargo treats a missing path as always stale, so only an existing one is watched
    let watch = |p: PathBuf| {
        if p.exists() {
            println!("cargo:rerun-if-changed={}", p.display());
        }
    };
    let abs = |p: &str| {
        let p = PathBuf::from(p);
        if p.is_absolute() { p } else { manifest.join(p) }
    };
    // a worktree has its own HEAD and index; the branch refs are in the common dir
    if let (Some(dir), Some(common)) = (
        git(manifest, &["rev-parse", "--git-dir"]),
        git(manifest, &["rev-parse", "--git-common-dir"]),
    ) {
        let (dir, common) = (abs(&dir), abs(&common));
        watch(dir.join("HEAD"));
        watch(dir.join("index"));
        watch(common.join("packed-refs"));
        if let Some(r) = git(manifest, &["symbolic-ref", "-q", "HEAD"]) {
            // a packed-only branch has no loose file
            watch(common.join(r));
        }
    }
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_or(0, |d| d.as_secs() as i64);
    let secs = env::var("SOURCE_DATE_EPOCH")
        .ok()
        .and_then(|s| s.trim().parse::<i64>().ok())
        .unwrap_or(now);
    let hash = git(manifest, &["rev-parse", "--short", "HEAD"]).filter(|h| !h.is_empty());
    let text = hash.as_deref().map(|hash| {
        // untracked files do not make a build dirty (as `git describe --dirty`)
        let dirty = git(manifest, &["status", "--porcelain", "--untracked-files=no"])
            .is_some_and(|s| !s.is_empty());
        let branch = git(manifest, &["rev-parse", "--abbrev-ref", "HEAD"]);
        let main = version::is_main(
            env::var("RB_BUILD_BRANCH").ok().as_deref(),
            branch.as_deref(),
            env::var("GITHUB_REF_NAME").ok().as_deref(),
        );
        version::Git { hash, dirty, main }
    });
    println!(
        "cargo:rustc-env=RB_UI_VERSION={}",
        version::version_string(secs, text)
    );
}

fn main() {
    let manifest = PathBuf::from(env::var("CARGO_MANIFEST_DIR").unwrap());
    set_version(&manifest);
    let dir = manifest.join("../../assets/ui").canonicalize().unwrap();
    println!("cargo:rerun-if-changed={}", dir.display());
    let mut names: Vec<String> = fs::read_dir(&dir)
        .unwrap()
        .filter_map(|e| {
            let name = e.unwrap().file_name().into_string().unwrap();
            let stem = name.strip_suffix(".svg")?;
            (stem != "panel").then(|| stem.to_string())
        })
        .collect();
    names.sort();
    let mut out = String::from("pub static PARTS: &[(&str, &str, &[u8])] = &[\n");
    for n in &names {
        let path = dir.join(format!("{n}.svg"));
        writeln!(
            out,
            "    ({n:?}, \"bytes://rb-ui/{n}.svg\", include_bytes!({:?})),",
            path.display().to_string()
        )
        .unwrap();
    }
    out.push_str("];\n");
    fs::write(
        PathBuf::from(env::var("OUT_DIR").unwrap()).join("parts_gen.rs"),
        out,
    )
    .unwrap();
}
