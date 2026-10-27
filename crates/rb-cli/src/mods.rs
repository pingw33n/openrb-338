//! Finding the song's mod and its key file for `rb render` and `rb compare` (task-033).
//!
//! The song's GLOB names a mod ("Standard ReBirth" is the built-in kit). The original keeps the
//! `.rbm` files of its `Mods` folder in a list and looks the name up in it with `strcmp` against
//! each mod's INFO menu name (`sub_4155D1`, 0x4155D1: the C string at the mod object's +1011,
//! which is INFO offset 1003); a name not in the list gets a dialog, Download / Standard /
//! Cancel. rb does the same: the INFO name, exact; as a convenience a file name (`PBE2.rbm`, or
//! `PBE2`, any case) also matches when no INFO name does. Descrambling a mod needs the user's
//! `RB20FUL.DAT` (task-053).

use std::path::{Path, PathBuf};

use rbm::Rbm;

use crate::{Fail, fail};

/// The kit of the built-in drum samples.
pub const STANDARD: &str = "Standard ReBirth";

/// What the command line and the environment say about mods.
#[derive(Clone, Default)]
pub struct ModOpts {
    /// `--mod FILE`: use this mod whatever the song names.
    pub mod_file: Option<PathBuf>,
    /// `--mod-dir DIR`: where the song's mod is looked up.
    pub mod_dir: Option<PathBuf>,
    /// `--rb20ful PATH`.
    pub rb20ful: Option<PathBuf>,
    /// `--standard-kit`: a mod that is not found is replaced by the built-in kit.
    pub standard_kit: bool,
}

/// The kit a song renders with.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Kit {
    Standard,
    Mod(PathBuf),
}

fn env_path(name: &str) -> Option<PathBuf> {
    std::env::var_os(name)
        .filter(|v| !v.is_empty())
        .map(PathBuf::from)
}

impl ModOpts {
    /// The mod folder: `--mod-dir`, the `RB_MOD_DIR` environment variable (`RB_RBM_DIR` is a
    /// deprecated alias), then a `Mods` folder next to the rb binary, then the original's
    /// install locations on Windows. An explicit choice must exist.
    pub fn mod_dir(&self) -> Result<Option<PathBuf>, Fail> {
        let explicit = self
            .mod_dir
            .clone()
            .map(|p| ("--mod-dir".to_string(), p))
            .or_else(|| env_path("RB_MOD_DIR").map(|p| ("RB_MOD_DIR".to_string(), p)))
            .or_else(|| env_path("RB_RBM_DIR").map(|p| ("RB_RBM_DIR".to_string(), p)));
        if let Some((what, p)) = explicit {
            return if p.is_dir() {
                Ok(Some(p))
            } else {
                Err(fail(
                    1,
                    format!("{what} {}: no such directory", p.display()),
                ))
            };
        }
        Ok(implicit_dirs("Mods").into_iter().find(|d| d.is_dir()))
    }

    /// Whether a mod folder was asked for (`--mod-dir` or the environment).
    pub fn mod_dir_given(&self) -> bool {
        self.mod_dir.is_some()
            || env_path("RB_MOD_DIR").is_some()
            || env_path("RB_RBM_DIR").is_some()
    }

    /// The `RB20FUL.DAT`: `--rb20ful`, the `RB_RB20FUL` environment variable, then an
    /// `RB20FUL.DAT` in the mod folder, next to the rb binary, or in the original's install
    /// locations on Windows.
    pub fn rb20ful(&self) -> Result<PathBuf, Fail> {
        let explicit = self
            .rb20ful
            .clone()
            .map(|p| ("--rb20ful", p))
            .or_else(|| env_path("RB_RB20FUL").map(|p| ("RB_RB20FUL", p)));
        if let Some((what, p)) = explicit {
            return if p.is_file() {
                Ok(p)
            } else {
                Err(fail(1, format!("{what} {}: no such file", p.display())))
            };
        }
        let mut dirs = Vec::new();
        if let Ok(Some(d)) = self.mod_dir() {
            dirs.push(d.clone());
            if let Some(parent) = d.parent() {
                dirs.push(parent.to_path_buf());
            }
        }
        dirs.extend(implicit_dirs(""));
        for d in dirs {
            for name in ["RB20FUL.DAT", "rb20ful.dat", "Rb20ful.dat"] {
                let p = d.join(name);
                if p.is_file() {
                    return Ok(p);
                }
            }
        }
        Err(fail(
            1,
            "no RB20FUL.DAT found. Pass --rb20ful PATH, set RB_RB20FUL, or put it in the mod \
             folder or next to the rb binary",
        ))
    }
}

/// `sub` (or the folder itself when empty) next to the rb binary and in the Windows install
/// locations of the original.
fn implicit_dirs(sub: &str) -> Vec<PathBuf> {
    let mut roots: Vec<PathBuf> = Vec::new();
    if let Some(d) = std::env::current_exe()
        .ok()
        .and_then(|e| e.parent().map(Path::to_path_buf))
    {
        roots.push(d);
    }
    if cfg!(windows) {
        for var in ["ProgramFiles(x86)", "ProgramFiles"] {
            if let Some(pf) = std::env::var_os(var) {
                let pf = PathBuf::from(pf);
                roots.push(pf.join("Propellerhead").join("ReBirth"));
                roots.push(pf.join("ReBirth RB-338"));
            }
        }
        roots.push(PathBuf::from(
            r"C:\Program Files (x86)\Propellerhead\ReBirth",
        ));
        roots.push(PathBuf::from(r"C:\ReBirth RB-338"));
    }
    roots
        .into_iter()
        .map(|r| if sub.is_empty() { r } else { r.join(sub) })
        .collect()
}

/// The `.rbm` files of `dir` (sorted), each with its INFO menu name when it has one.
fn scan(dir: &Path) -> Result<Vec<(PathBuf, Option<String>)>, Fail> {
    let rd = std::fs::read_dir(dir).map_err(|e| {
        fail(
            1,
            format!("{}: cannot read the mod folder: {e}", dir.display()),
        )
    })?;
    let mut files: Vec<PathBuf> = rd
        .filter_map(Result::ok)
        .map(|e| e.path())
        .filter(|p| p.is_file() && p.extension().is_some_and(|x| x.eq_ignore_ascii_case("rbm")))
        .collect();
    files.sort();
    Ok(files
        .into_iter()
        .map(|p| {
            // a file that is no mod has no name and matches by its file name only
            let name = std::fs::read(&p)
                .ok()
                .and_then(|b| Rbm::parse(b).ok())
                .and_then(|r| r.info().map(|i| i.name.clone()));
            (p, name)
        })
        .collect())
}

/// Resolves the mod called `name` in `dir`: the INFO name first (exact), then a file name.
pub fn find_in_dir(dir: &Path, name: &str) -> Result<Option<PathBuf>, Fail> {
    let mods = scan(dir)?;
    if let Some((p, _)) = mods.iter().find(|(_, n)| n.as_deref() == Some(name)) {
        return Ok(Some(p.clone()));
    }
    let want = name.to_ascii_lowercase();
    Ok(mods.into_iter().map(|(p, _)| p).find(|p| {
        let f = p
            .file_name()
            .map(|f| f.to_string_lossy().to_ascii_lowercase())
            .unwrap_or_default();
        f == want || f.strip_suffix(".rbm") == Some(want.as_str())
    }))
}

/// The kit of a song whose GLOB names `mod_name`. `label` names the song in messages.
pub fn resolve(opts: &ModOpts, label: &str, mod_name: &str) -> Result<Kit, Fail> {
    if let Some(f) = &opts.mod_file {
        if !f.is_file() {
            return Err(fail(1, format!("--mod {}: no such file", f.display())));
        }
        // the original loads the mod the song names, by its INFO name (sub_4155D1)
        let info = std::fs::read(f)
            .ok()
            .and_then(|b| Rbm::parse(b).ok())
            .and_then(|r| r.info().map(|i| i.name.clone()));
        if info.as_deref() != Some(mod_name) {
            eprintln!(
                "rb: warning: {label}: --mod {} (mod name {:?}) is not the mod the song names \
                 ({mod_name:?}); the original would not load it for this song",
                f.display(),
                info.unwrap_or_default()
            );
        }
        return Ok(Kit::Mod(f.clone()));
    }
    if mod_name == STANDARD {
        return Ok(Kit::Standard);
    }
    let found = match opts.mod_dir()? {
        Some(d) => find_in_dir(&d, mod_name)?,
        None => None,
    };
    match found {
        Some(p) => Ok(Kit::Mod(p)),
        None if opts.standard_kit => {
            eprintln!(
                "rb: warning: {label}: the mod {mod_name:?} was not found; rendering with the \
                 built-in kit (--standard-kit)"
            );
            Ok(Kit::Standard)
        }
        None => Err(fail(
            1,
            format!(
                "{label}: the song uses the mod {mod_name:?}, which was not found. Pass \
                 --mod-dir DIR (or set RB_MOD_DIR) with its .rbm file, --mod FILE, or \
                 --standard-kit to render with the built-in kit (the original's \"Standard\" \
                 choice)"
            ),
        )),
    }
}
