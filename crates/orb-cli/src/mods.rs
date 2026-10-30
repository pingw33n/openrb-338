//! Finding the song's mod and its key file for `orb render` and `orb compare` (task-033).
//!
//! The song's GLOB names a mod ("Standard ReBirth" is the built-in kit). The original keeps the
//! `.rbm` files of its `Mods` folder in a list and looks the name up in it with `strcmp` against
//! each mod's INFO menu name (`sub_4155D1`, 0x4155D1: the C string at the mod object's +1011,
//! which is INFO offset 1003); a name not in the list gets a dialog, Download / Standard /
//! Cancel. orb does the same with one deliberate difference (task-145, task-150): the INFO name
//! only, compared ignoring ASCII case; never a file name. orb loads only recrypted mods
//! (`orb mod recrypt`, task-059; task-063): a mod that is not recrypted is found like any other
//! (its INFO is in the clear and it keeps its place in the folder order) and fails when its
//! samples are loaded, with an error that names `orb mod recrypt`. Only `orb mod recrypt` reads
//! `RB20FUL.DAT`.
//!
//! The built-in kit itself is the standard mod orb embeds (`assets/Standard.rbm`,
//! [`rbm::standard::builtin_samples`]; task-067, docs/rbm.md section 13): no file is looked for.

use std::path::{Path, PathBuf};

use rbm::Rbm;

use crate::{Fail, fail};

/// The kit of the built-in drum samples, by the name songs give it: also the INFO name of the
/// embedded standard mod.
pub const STANDARD: &str = rbm::standard::NAME;

/// What the command line and the environment say about mods.
#[derive(Clone, Default)]
pub struct ModOpts {
    /// `--mod FILE`: use this mod whatever the song names.
    pub mod_file: Option<PathBuf>,
    /// `--mod-dir DIR`: where the song's mod is looked up.
    pub mod_dir: Option<PathBuf>,
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
    /// The mod folder: `--mod-dir`, the `ORB_MOD_DIR` environment variable, then a `Mods` folder
    /// next to the orb binary, then the original's install locations on Windows. An explicit
    /// choice must exist.
    pub fn mod_dir(&self) -> Result<Option<PathBuf>, Fail> {
        let explicit = self
            .mod_dir
            .clone()
            .map(|p| ("--mod-dir".to_string(), p))
            .or_else(|| env_path("ORB_MOD_DIR").map(|p| ("ORB_MOD_DIR".to_string(), p)));
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
        self.mod_dir.is_some() || env_path("ORB_MOD_DIR").is_some()
    }
}

/// The `RB20FUL.DAT` for `orb mod recrypt` of the mod `input` (the only command that reads it):
/// `explicit` (`--rb20ful`), the `ORB_RB20FUL` environment variable, then an `RB20FUL.DAT` in the
/// input's folder or its parent, next to the orb binary, or in the original's install locations
/// on Windows.
pub fn find_rb20ful(explicit: Option<&Path>, input: &Path) -> Result<PathBuf, Fail> {
    let explicit = explicit
        .map(|p| ("--rb20ful", p.to_path_buf()))
        .or_else(|| env_path("ORB_RB20FUL").map(|p| ("ORB_RB20FUL", p)));
    if let Some((what, p)) = explicit {
        return if p.is_file() {
            Ok(p)
        } else {
            Err(fail(1, format!("{what} {}: no such file", p.display())))
        };
    }
    let mut dirs = Vec::new();
    if let Some(d) = input.parent() {
        dirs.push(d.to_path_buf());
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
        "no RB20FUL.DAT found. Pass --rb20ful PATH, set ORB_RB20FUL, or put it next to the mod \
         or the orb binary",
    ))
}

/// `sub` (or the folder itself when empty) next to the orb binary and in the Windows install
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

/// The order of the mod files of a folder, in the CLI and in the UI's Mods menu (task-151, the
/// user's decision on task-146, 2026-10-30): the bytes of the file names, a plain `strcmp`; no
/// Unicode folding, no locale, no NTFS model. The file whose name sorts first owns an INFO name
/// that several mods share (`AXIOM.rbm` before `axiom(b3).rbm`: `A` is 0x41, `a` 0x61). This is a
/// **deliberate difference from the original**, where the volume's listing order decides
/// (`sub_4158A9`, `FindFirstFile`; on NTFS `axiom(b3).rbm` wins; docs/rbm.md 11.3). `--mod FILE`
/// picks any file.
pub fn byte_order(a: &Path, b: &Path) -> std::cmp::Ordering {
    let key = |p: &Path| p.file_name().map(|f| f.as_encoded_bytes().to_vec());
    key(a).cmp(&key(b)).then_with(|| a.cmp(b))
}

/// The `.rbm` files of `dir` in [`byte_order`], each with its INFO menu name when it has one.
pub fn scan(dir: &Path) -> Result<Vec<(PathBuf, Option<String>)>, Fail> {
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
    files.sort_by(|a, b| byte_order(a, b));
    Ok(files
        .into_iter()
        .map(|p| {
            // a file that is no mod has no name and matches nothing
            let name = std::fs::read(&p)
                .ok()
                .and_then(|b| Rbm::parse(b).ok())
                .and_then(|r| r.info().map(|i| i.name.clone()));
            (p, name)
        })
        .collect())
}

/// Whether a mod's INFO name `info` is the name `want` a song gives: equal ignoring ASCII case
/// only (a non-ASCII letter compares exactly). The original compares exactly (`strcmp`); the
/// task-145 decision is this deliberate difference.
pub fn names_match(info: &str, want: &str) -> bool {
    info.eq_ignore_ascii_case(want)
}

/// Resolves the mod called `name` in `dir`: the first mod in [`byte_order`] whose INFO name
/// matches ([`names_match`]). A file name never matches. The command line uses this and nothing
/// else: it knows no preference, so a render is deterministic.
pub fn find_in_dir(dir: &Path, name: &str) -> Result<Option<PathBuf>, Fail> {
    find_preferring(dir, name, None)
}

/// [`find_in_dir`] with a preferred file name (the UI's remembered choice among mods that share
/// an INFO name, task-151): that file when it is in `dir` and its INFO name matches, else the
/// first match in [`byte_order`].
pub fn find_preferring(
    dir: &Path,
    name: &str,
    preferred: Option<&str>,
) -> Result<Option<PathBuf>, Fail> {
    let found: Vec<PathBuf> = scan(dir)?
        .into_iter()
        .filter(|(_, n)| n.as_deref().is_some_and(|n| names_match(n, name)))
        .map(|(p, _)| p)
        .collect();
    let pref = preferred.and_then(|f| found.iter().find(|p| p.file_name().is_some_and(|n| n == f)));
    Ok(pref.or(found.first()).cloned())
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
        if !info.as_deref().is_some_and(|n| names_match(n, mod_name)) {
            eprintln!(
                "orb: warning: {label}: --mod {} (mod name {:?}) is not the mod the song names \
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
                "orb: warning: {label}: the mod {mod_name:?} was not found; rendering with the \
                 built-in kit (--standard-kit)"
            );
            Ok(Kit::Standard)
        }
        None => Err(fail(
            1,
            format!(
                "{label}: the song uses the mod {mod_name:?}, which was not found. Pass \
                 --mod-dir DIR (or set ORB_MOD_DIR) with its .rbm file, --mod FILE, or \
                 --standard-kit to render with the built-in kit (the original's \"Standard\" \
                 choice)"
            ),
        )),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A mod with no files whose INFO menu name is `name`.
    fn empty_mod(name: &str) -> Vec<u8> {
        let mut b = b"CAT \0\0\0\0PRBM".to_vec();
        b.extend(b"INFO");
        b.extend(1280u32.to_be_bytes());
        let mut info = vec![0u8; 1280];
        // INFO text is Latin-1
        let name: Vec<u8> = name.chars().map(|c| c as u8).collect();
        info[1003..1003 + name.len()].copy_from_slice(&name);
        b.extend(info);
        let size = (b.len() - 8) as u32;
        b[4..8].copy_from_slice(&size.to_be_bytes());
        b
    }

    #[test]
    fn byte_order_compares_file_name_bytes() {
        let mut v: Vec<PathBuf> = [
            "axiom(b3).rbm",
            "b.rbm",
            "AXIOM.rbm",
            "alpha3.rbm",
            "Alpha2.rbm",
        ]
        .iter()
        .map(PathBuf::from)
        .collect();
        v.sort_by(|a, b| byte_order(a, b));
        let names: Vec<_> = v.iter().map(|p| p.to_string_lossy().into_owned()).collect();
        assert_eq!(
            names,
            [
                "AXIOM.rbm",
                "Alpha2.rbm",
                "alpha3.rbm",
                "axiom(b3).rbm",
                "b.rbm"
            ]
        );
        // no Unicode folding: 'É' (C3 89) sorts before 'é' (C3 A9)
        assert_eq!(
            byte_order(Path::new("é.rbm"), Path::new("É.rbm")),
            std::cmp::Ordering::Greater
        );
    }

    #[test]
    fn the_first_file_by_bytes_owns_a_duplicated_name() {
        let d = tempfile::tempdir().unwrap();
        std::fs::write(d.path().join("AXIOM.rbm"), empty_mod("AXIOM")).unwrap();
        std::fs::write(d.path().join("axiom(b3).rbm"), empty_mod("AXIOM")).unwrap();
        let p = find_in_dir(d.path(), "AXIOM").ok().flatten().unwrap();
        assert_eq!(p.file_name().unwrap(), "AXIOM.rbm");
        // also when the song's name differs in case
        let p = find_in_dir(d.path(), "Axiom").ok().flatten().unwrap();
        assert_eq!(p.file_name().unwrap(), "AXIOM.rbm");
        // INFO names that differ only in ASCII case are the same name
        std::fs::write(d.path().join("Aa.rbm"), empty_mod("Dup")).unwrap();
        std::fs::write(d.path().join("Ab.rbm"), empty_mod("DUP")).unwrap();
        let p = find_in_dir(d.path(), "dup").ok().flatten().unwrap();
        assert_eq!(p.file_name().unwrap(), "Aa.rbm");
    }

    #[test]
    fn a_preferred_file_wins_when_it_is_there() {
        let d = tempfile::tempdir().unwrap();
        std::fs::write(d.path().join("AXIOM.rbm"), empty_mod("AXIOM")).unwrap();
        std::fs::write(d.path().join("axiom(b3).rbm"), empty_mod("AXIOM")).unwrap();
        std::fs::write(d.path().join("other.rbm"), empty_mod("Other")).unwrap();
        let get = |pref| {
            let p = find_preferring(d.path(), "AXIOM", pref).unwrap().unwrap();
            p.file_name().unwrap().to_string_lossy().into_owned()
        };
        assert_eq!(get(Some("axiom(b3).rbm")), "axiom(b3).rbm");
        // a missing file, or one of another name, falls back to the default
        assert_eq!(get(Some("gone.rbm")), "AXIOM.rbm");
        assert_eq!(get(Some("other.rbm")), "AXIOM.rbm");
        assert_eq!(get(None), "AXIOM.rbm");
    }

    #[test]
    fn a_name_matches_ignoring_ascii_case() {
        let d = tempfile::tempdir().unwrap();
        std::fs::write(d.path().join("Omega.rbm"), empty_mod("Omega")).unwrap();
        for want in ["Omega", "omega", "OMEGA"] {
            let p = find_in_dir(d.path(), want).ok().flatten().unwrap();
            assert_eq!(p.file_name().unwrap(), "Omega.rbm", "{want}");
        }
        assert!(find_in_dir(d.path(), "Omega2").unwrap().is_none());
    }

    #[test]
    fn a_file_name_does_not_match() {
        let d = tempfile::tempdir().unwrap();
        std::fs::write(d.path().join("PBE2.rbm"), empty_mod("Bass Machine")).unwrap();
        for want in ["PBE2", "pbe2", "PBE2.rbm", "pbe2.RBM"] {
            assert!(find_in_dir(d.path(), want).unwrap().is_none(), "{want}");
        }
        assert!(find_in_dir(d.path(), "bass machine").unwrap().is_some());
    }

    #[test]
    fn a_non_ascii_letter_compares_exactly() {
        assert!(!names_match("çoNaNbox 2.0", "ÇoNaNbox 2.0"));
        assert!(names_match("çoNaNbox 2.0", "çONanBOX 2.0"));
        let d = tempfile::tempdir().unwrap();
        std::fs::write(d.path().join("c.rbm"), empty_mod("çoNaNbox 2.0")).unwrap();
        assert!(find_in_dir(d.path(), "ÇoNaNbox 2.0").unwrap().is_none());
        assert!(find_in_dir(d.path(), "çonanbox 2.0").unwrap().is_some());
    }

    #[test]
    fn the_mod_option_overrides_the_songs_name() {
        let d = tempfile::tempdir().unwrap();
        let f = d.path().join("m.rbm");
        std::fs::write(&f, empty_mod("Omega")).unwrap();
        let opts = ModOpts {
            mod_file: Some(f.clone()),
            ..ModOpts::default()
        };
        // resolves either way: --mod overrides; the warning is on stderr
        assert_eq!(resolve(&opts, "s", "omega").ok(), Some(Kit::Mod(f.clone())));
        assert_eq!(resolve(&opts, "s", "other").ok(), Some(Kit::Mod(f)));
        // also over a folder whose default for the name is another file
        let dup = tempfile::tempdir().unwrap();
        std::fs::write(dup.path().join("AXIOM.rbm"), empty_mod("AXIOM")).unwrap();
        let second = dup.path().join("axiom(b3).rbm");
        std::fs::write(&second, empty_mod("AXIOM")).unwrap();
        let opts = ModOpts {
            mod_file: Some(second.clone()),
            mod_dir: Some(dup.path().to_path_buf()),
            ..ModOpts::default()
        };
        assert_eq!(resolve(&opts, "s", "AXIOM").ok(), Some(Kit::Mod(second)));
        let opts = ModOpts {
            mod_dir: Some(dup.path().to_path_buf()),
            ..ModOpts::default()
        };
        assert_eq!(
            resolve(&opts, "s", "AXIOM").ok(),
            Some(Kit::Mod(dup.path().join("AXIOM.rbm")))
        );
    }
}
