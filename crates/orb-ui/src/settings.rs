//! The UI's small settings file: the songs folder, the mods folder and the recent folders of
//! the file browser (task-046), and the startup sound preference (task-074).
//!
//! It is a TOML file (task-107) with the keys `songs_dir` and `mods_dir` (paths, absent when
//! unset), `recent` (an array of paths, newest first), `last_song_dir` and `last_mod_dir`
//! (paths, absent until a file is chosen: the folder of the last song chosen in Open, Save As
//! or Export, and of the last mod; task-144), `startup_sound` (a bool) and the table
//! `preferred_mods` (an INFO name to the file name preferred among mods that share it,
//! task-151), so it can be edited by hand. Where it lives: `$ORB_UI_CONFIG` if set; else
//! `OpenRB-338/ui.toml` in the platform's configuration folder (`$XDG_CONFIG_HOME`, or
//! `~/.config` on Linux, `~/Library/Application Support` on macOS, `%APPDATA%` on Windows).
//! Reading never fails: a missing or damaged file gives the defaults (a damaged file stays as
//! it is until the next save), a key of the wrong type is skipped and the others kept, and
//! unknown keys are
//! ignored. Saving writes the whole file through a temporary file and a rename, and returns
//! its errors for the browser to show. A path that is not valid UTF-8 cannot be stored in
//! TOML (and a lossy string would name another folder), so a save leaves it out: a `recent`
//! entry is dropped, `songs_dir`, `mods_dir`, `last_song_dir` and `last_mod_dir` are omitted and
//! fall back to the default. An empty `$ORB_UI_CONFIG` counts as unset. The old `ui.conf` is
//! neither read nor deleted.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use serde::Serialize;

/// How many recent folders are kept.
pub const MAX_RECENT: usize = 8;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Settings {
    /// The original's preference "Play Mod Startup Sound" (on by default: the preferences'
    /// constructor `sub_41CAA4` sets it, 0x41CB13): a mod change plays the mod's startup
    /// sound. Stored as the bool `startup_sound`.
    pub startup_sound: bool,
    /// Where songs are opened from and saved to by default (`None`: the documents folder).
    pub songs_dir: Option<PathBuf>,
    /// Where mods are looked for by default (`None`: the documents folder).
    pub mods_dir: Option<PathBuf>,
    /// The folders files were last chosen in, newest first.
    pub recent: Vec<PathBuf>,
    /// The folder of the last song chosen in Open, Save As or Export (task-144): where those
    /// dialogs start while it exists.
    pub last_song_dir: Option<PathBuf>,
    /// The folder of the last mod chosen in the mod dialog: where that dialog starts while it
    /// exists.
    pub last_mod_dir: Option<PathBuf>,
    /// The mod file preferred for an INFO name that several mods share (task-151): INFO name
    /// to file name. Stored as the table `preferred_mods`. Names are compared ignoring ASCII
    /// case ([`Settings::preferred_mod`]).
    pub preferred_mods: BTreeMap<String, String>,
    /// Where this was read from and is written to (`None`: never persisted, as in tests).
    pub file: Option<PathBuf>,
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            startup_sound: true,
            songs_dir: None,
            mods_dir: None,
            recent: Vec::new(),
            last_song_dir: None,
            last_mod_dir: None,
            preferred_mods: BTreeMap::new(),
            file: None,
        }
    }
}

fn home() -> Option<PathBuf> {
    let var = if cfg!(windows) { "USERPROFILE" } else { "HOME" };
    std::env::var_os(var)
        .map(PathBuf::from)
        .filter(|p| p.is_dir())
}

/// The user's documents folder: `~/Documents` if it exists, else the home folder, else the
/// current folder.
pub fn documents_dir() -> PathBuf {
    let home = home();
    home.as_ref()
        .map(|h| h.join("Documents"))
        .filter(|d| d.is_dir())
        .or(home)
        .unwrap_or_else(|| PathBuf::from("."))
}

/// The user's home folder, when it is known.
pub fn home_dir() -> Option<PathBuf> {
    home()
}

/// The settings file's default place (see the module text).
pub fn default_file() -> Option<PathBuf> {
    if let Some(p) = std::env::var_os("ORB_UI_CONFIG").filter(|p| !p.is_empty()) {
        return Some(PathBuf::from(p));
    }
    let base = if cfg!(windows) {
        std::env::var_os("APPDATA").map(PathBuf::from)
    } else if let Some(x) = std::env::var_os("XDG_CONFIG_HOME").filter(|x| !x.is_empty()) {
        Some(PathBuf::from(x))
    } else if cfg!(target_os = "macos") {
        home().map(|h| h.join("Library/Application Support"))
    } else {
        home().map(|h| h.join(".config"))
    };
    base.map(|b| b.join("OpenRB-338").join("ui.toml"))
}

/// What the file holds, in the file's order.
#[derive(Serialize)]
struct Stored<'a> {
    #[serde(skip_serializing_if = "Option::is_none")]
    songs_dir: Option<&'a Path>,
    #[serde(skip_serializing_if = "Option::is_none")]
    mods_dir: Option<&'a Path>,
    startup_sound: bool,
    recent: Vec<&'a Path>,
    #[serde(skip_serializing_if = "Option::is_none")]
    last_song_dir: Option<&'a Path>,
    #[serde(skip_serializing_if = "Option::is_none")]
    last_mod_dir: Option<&'a Path>,
    // a table, so last
    #[serde(skip_serializing_if = "BTreeMap::is_empty")]
    preferred_mods: BTreeMap<&'a str, &'a str>,
}

/// The path when TOML can hold it (valid UTF-8). A lossy string would name another folder, so
/// any other path is left out of the file.
fn storable(p: &Path) -> Option<&Path> {
    p.to_str().map(|_| p)
}

/// A path value of the table: a non-empty string.
fn path_of(v: &toml::Value) -> Option<PathBuf> {
    v.as_str().filter(|s| !s.is_empty()).map(PathBuf::from)
}

impl Settings {
    /// Reads the settings from the default file (the defaults when there is none).
    pub fn load() -> Self {
        match default_file() {
            Some(f) => Self::load_from(&f),
            None => Self::default(),
        }
    }

    /// Reads `file`; a missing or unreadable file gives the defaults. Later writes go to
    /// `file`.
    pub fn load_from(file: &Path) -> Self {
        let mut s = Self {
            file: Some(file.to_path_buf()),
            ..Self::default()
        };
        let Ok(text) = std::fs::read_to_string(file) else {
            return s;
        };
        let Ok(table) = text.parse::<toml::Table>() else {
            return s;
        };
        if let Some(b) = table.get("startup_sound").and_then(|v| v.as_bool()) {
            s.startup_sound = b;
        }
        s.songs_dir = table.get("songs_dir").and_then(path_of);
        s.mods_dir = table.get("mods_dir").and_then(path_of);
        s.last_song_dir = table.get("last_song_dir").and_then(path_of);
        s.last_mod_dir = table.get("last_mod_dir").and_then(path_of);
        if let Some(list) = table.get("recent").and_then(|v| v.as_array()) {
            for p in list.iter().filter_map(path_of) {
                if s.recent.len() < MAX_RECENT && !s.recent.contains(&p) {
                    s.recent.push(p);
                }
            }
        }
        if let Some(t) = table.get("preferred_mods").and_then(|v| v.as_table()) {
            for (name, file) in t {
                if let Some(f) = file.as_str().filter(|f| !f.is_empty()) {
                    s.preferred_mods.insert(name.clone(), f.to_string());
                }
            }
        }
        s
    }

    /// The file preferred for the INFO name `name` (ignoring ASCII case), if one is set.
    pub fn preferred_mod(&self, name: &str) -> Option<&str> {
        self.preferred_mods
            .iter()
            .find(|(n, _)| n.eq_ignore_ascii_case(name))
            .map(|(_, f)| f.as_str())
    }

    /// Prefers `file` for `name` (`None` clears the preference), replacing an entry of the same
    /// name in another ASCII case.
    pub fn set_preferred_mod(&mut self, name: &str, file: Option<&str>) {
        self.preferred_mods
            .retain(|n, _| !n.eq_ignore_ascii_case(name));
        if let Some(f) = file {
            self.preferred_mods.insert(name.to_string(), f.to_string());
        }
    }

    /// Writes the settings to their file (a no-op without one).
    pub fn save(&self) -> Result<(), String> {
        let Some(file) = &self.file else {
            return Ok(());
        };
        let text = toml::to_string(&Stored {
            songs_dir: self.songs_dir.as_deref().and_then(storable),
            mods_dir: self.mods_dir.as_deref().and_then(storable),
            startup_sound: self.startup_sound,
            recent: self.recent.iter().filter_map(|p| storable(p)).collect(),
            last_song_dir: self.last_song_dir.as_deref().and_then(storable),
            last_mod_dir: self.last_mod_dir.as_deref().and_then(storable),
            preferred_mods: self
                .preferred_mods
                .iter()
                .map(|(n, f)| (n.as_str(), f.as_str()))
                .collect(),
        })
        .map_err(|e| format!("{}: cannot encode: {e}", file.display()))?;
        if let Some(dir) = file.parent() {
            std::fs::create_dir_all(dir)
                .map_err(|e| format!("{}: cannot create: {e}", dir.display()))?;
        }
        // Written next to the file and renamed over it, so a crash never leaves half a file.
        let mut tmp = file.as_os_str().to_owned();
        tmp.push(".tmp");
        let tmp = PathBuf::from(tmp);
        std::fs::write(&tmp, text)
            .and_then(|()| std::fs::rename(&tmp, file))
            .map_err(|e| {
                let _ = std::fs::remove_file(&tmp);
                format!("{}: cannot write: {e}", file.display())
            })
    }

    /// Puts `dir` first in the recent folders.
    pub fn note_recent(&mut self, dir: &Path) {
        self.recent.retain(|d| d != dir);
        self.recent.insert(0, dir.to_path_buf());
        self.recent.truncate(MAX_RECENT);
    }
}
