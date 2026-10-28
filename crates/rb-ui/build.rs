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
fn set_version(manifest: &Path) -> String {
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
    version::version_number(secs)
}

/// The Windows resources of the exe (task-111), for a Windows target only: the app icon
/// (`assets/app_icon.ico`, made by `ui-art ico`) as icon 1, which Explorer, the taskbar and the
/// shortcuts show, and the version info (Details tab of the file's properties).
/// `version` is `YYYY.MMDD.HHMM`
/// (task-133), whose three numbers fit the four 16-bit fields of a Windows file version
/// (year, MMDD, HHMM, 0). Cross builds (cargo-xwin on a Mac or Linux) compile the `.rc` with
/// `llvm-rc`, which must be on PATH (docs/building.md); a failure fails the build rather than
/// leaving the exe without its icon.
fn embed_windows_resources(manifest: &Path, version: &str) {
    println!("cargo:rerun-if-env-changed=CARGO_CFG_TARGET_OS");
    if env::var("CARGO_CFG_TARGET_OS").as_deref() != Ok("windows") {
        return;
    }
    // Not canonicalized: on Windows that gives a `\\?\D:\...` path, which rc.exe may not open.
    // The `..` parts and (below) forward slashes are fine for rc.exe and llvm-rc.
    let ico = manifest.join("../../assets/app_icon.ico");
    assert!(
        ico.is_file(),
        "{} is missing (cargo run -p rb-ui-art --bin ui-art -- ico)",
        ico.display()
    );
    println!("cargo:rerun-if-changed={}", ico.display());
    let n: Vec<&str> = version.split('.').collect();
    assert_eq!(n.len(), 3, "version {version}");
    let part = |s: &str| s.parse::<u32>().unwrap();
    let nums = format!("{},{},{},0", part(n[0]), part(n[1]), part(n[2]));
    let rc = format!(
        "1 ICON \"{ico}\"\n\
         1 VERSIONINFO\n\
         FILEVERSION {nums}\n\
         PRODUCTVERSION {nums}\n\
         FILEOS 0x40004\n\
         FILETYPE 1\n\
         BEGIN\n\
         BLOCK \"StringFileInfo\"\n\
         BEGIN\n\
         BLOCK \"040904B0\"\n\
         BEGIN\n\
         VALUE \"CompanyName\", \"OpenRB-338\\0\"\n\
         VALUE \"FileDescription\", \"OpenRB-338\\0\"\n\
         VALUE \"FileVersion\", \"{version}\\0\"\n\
         VALUE \"InternalName\", \"OpenRB-338\\0\"\n\
         VALUE \"LegalCopyright\", \"GPL-3.0-only\\0\"\n\
         VALUE \"OriginalFilename\", \"OpenRB-338.exe\\0\"\n\
         VALUE \"ProductName\", \"OpenRB-338\\0\"\n\
         VALUE \"ProductVersion\", \"{version}\\0\"\n\
         END\n\
         END\n\
         BLOCK \"VarFileInfo\"\n\
         BEGIN\n\
         VALUE \"Translation\", 0x0409, 1200\n\
         END\n\
         END\n",
        ico = ico.display().to_string().replace('\\', "/"),
    );
    let path = PathBuf::from(env::var("OUT_DIR").unwrap()).join("openrb.rc");
    fs::write(&path, rc).unwrap();
    embed_resource::compile(&path, embed_resource::NONE)
        .manifest_required()
        .unwrap();
}

fn main() {
    let manifest = PathBuf::from(env::var("CARGO_MANIFEST_DIR").unwrap());
    let version = set_version(&manifest);
    embed_windows_resources(&manifest, &version);
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
