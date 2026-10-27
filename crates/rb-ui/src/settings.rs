//! The UI's small settings file: the songs folder, the mods folder and the recent folders of
//! the file browser (task-046), and the startup sound preference (task-074).
//!
//! It is a plain text file of `key=value` lines (`songs=`, `mods=`, `recent=` once per folder,
//! newest first, `startup_sound=0|1`), so it needs no extra crate and can be edited by hand. Where it lives:
//! `$RB_UI_CONFIG` if set; else `rb/ui.conf` in the platform's configuration folder
//! (`$XDG_CONFIG_HOME`, or `~/.config` on Linux, `~/Library/Application Support` on macOS,
//! `%APPDATA%` on Windows). Reading never fails (a missing or damaged file gives the defaults);
//! writing errors are returned for the browser to show.

use std::path::{Path, PathBuf};

/// How many recent folders are kept.
pub const MAX_RECENT: usize = 8;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Settings {
    /// The original's preference "Play Mod Startup Sound" (on by default: the preferences'
    /// constructor `sub_41CAA4` sets it, 0x41CB13): a mod change plays the mod's startup
    /// sound. Stored as `startup_sound=0` or `1`.
    pub startup_sound: bool,
    /// Where songs are opened from and saved to by default (`None`: the documents folder).
    pub songs_dir: Option<PathBuf>,
    /// Where mods are looked for by default (`None`: the documents folder).
    pub mods_dir: Option<PathBuf>,
    /// The folders files were last chosen in, newest first.
    pub recent: Vec<PathBuf>,
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
    if let Some(p) = std::env::var_os("RB_UI_CONFIG") {
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
    base.map(|b| b.join("rb").join("ui.conf"))
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
        for line in text.lines() {
            let Some((k, v)) = line.split_once('=') else {
                continue;
            };
            if v.is_empty() {
                continue;
            }
            let k = k.trim();
            if k == "startup_sound" {
                match v.trim() {
                    "0" => s.startup_sound = false,
                    "1" => s.startup_sound = true,
                    _ => {}
                }
                continue;
            }
            let v = PathBuf::from(v);
            match k {
                "songs" => s.songs_dir = Some(v),
                "mods" => s.mods_dir = Some(v),
                "recent" if s.recent.len() < MAX_RECENT && !s.recent.contains(&v) => {
                    s.recent.push(v)
                }
                _ => {}
            }
        }
        s
    }

    /// Writes the settings to their file (a no-op without one).
    pub fn save(&self) -> Result<(), String> {
        let Some(file) = &self.file else {
            return Ok(());
        };
        let mut text = String::new();
        let mut line = |k: &str, v: &Path| {
            // A path with a line break cannot be stored in this format.
            let v = v.to_string_lossy();
            if !v.contains('\n') && !v.contains('\r') {
                text.push_str(&format!("{k}={v}\n"));
            }
        };
        if let Some(p) = &self.songs_dir {
            line("songs", p);
        }
        if let Some(p) = &self.mods_dir {
            line("mods", p);
        }
        for p in &self.recent {
            line("recent", p);
        }
        text.push_str(&format!("startup_sound={}\n", self.startup_sound as u8));
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
