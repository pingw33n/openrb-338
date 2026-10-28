//! The menu (task-047): load, save and export the song, choose its mod, the Vintage sound flag
//! and the keyboard mode, with the unsaved-changes prompt. Nothing else of the original's menu.
//!
//! What each item does (docs/ui_spec.md section 18):
//! - **File**: New (Ctrl/Cmd+N: the state of the app's start), Open (Ctrl/Cmd+O), Save
//!   (Ctrl/Cmd+S; the texts come from [`crate::shortcuts`]; a song that was never saved goes
//!   to Save As), Save As, Info ([`crate::info`]), Export loop as audio file. The browser
//!   (task-046) returns paths only; the writes happen here and their errors are shown in the
//!   notice line.
//! - **Mods**: "Standard ReBirth", the mods of the Mods folder (one entry per INFO menu name, as
//!   the original's Mods menu: `sub_40C163`, docs/rbm.md section 14), and a file picked in the
//!   browser. A choice is the original's `sub_42D80D` -> `sub_426FB3`: the song is marked
//!   modified, the new kit goes to the player as song msg 19 ([`Link::set_mod`]) and a save
//!   writes the new name. With the preference "Play Mod Startup Sound" on
//!   ([`crate::settings::Settings::startup_sound`]) the new mod's startup sound plays, as it
//!   does when a song is opened and at the start ([`App::start`]), but only when the kit
//!   changes (task-099).
//!   "Demo Song" opens the `Demo.rbs` of the current mod as an unsaved song (task-094).
//! - **Options**: the Vintage ReBirth Synth Sound flag (GLOB byte 482; Edit menu item 323 of the
//!   original) and the keyboard mode (Ctrl/Cmd+F, Ctrl/Cmd+G; docs/ui_spec.md section 10).

use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::sync::mpsc::{Receiver, TryRecvError};

use eframe::egui::{
    self, Align, Color32, Context, Frame, Id, Key, Layout, Margin, Modal, Modifiers, RichText,
    Stroke, Ui,
};
use rb_cli::encode::Format;
use rb_cli::mods::{self, Kit, ModOpts, STANDARD};
use rb_cli::render::{DEFAULT_IDLE_BLOCKS, DEFAULT_QUALITY, Renderer, load_mod};
use rb_engine::frame::LoadRefused;
use rb_engine::samples::SampleSet;

use crate::app::App;
use crate::browser::{ExportFormat, Mode};
use crate::document::{self, Document};
use crate::keys::KeyMode;
use crate::session;
use crate::shortcuts;

/// The height of the menu bar in points.
pub const MENU_H: f32 = 22.0;

const SLATE_DARK: Color32 = Color32::from_rgb(0x2c, 0x35, 0x42);
const SLATE_LINE: Color32 = Color32::from_rgb(0x56, 0x64, 0x7a);
const INK: Color32 = Color32::from_rgb(0xe9, 0xed, 0xf2);
const INK_DIM: Color32 = Color32::from_rgb(0x9a, 0xa7, 0xb8);

/// What the unsaved-changes prompt guards: the action that waits for its answer.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Then {
    /// Open this song (it replaces the one open).
    Open(PathBuf),
    /// Open the demo song of this mod file ([`App::demo_song`]).
    Demo(PathBuf),
    /// Start over with the new song of the app's start ([`App::new_song`]).
    New,
    /// Close the window.
    Quit,
}

/// The prompt's answers.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Answer {
    Save,
    DontSave,
    Cancel,
}

/// A menu command.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Cmd {
    New,
    Open,
    Save,
    SaveAs,
    Export,
    /// File > Info: the current song's information window.
    Info,
    /// Help > Keyboard Shortcuts.
    KeyHelp,
    /// Help > About.
    About,
    /// "Choose file..." of the Mods menu: the browser for a mod.
    ChooseModFile,
    /// A kit from the Mods menu (or a mod file the browser chose).
    UseKit(Kit),
    /// "Demo Song" of the Mods menu: the current mod's `Demo.rbs`.
    DemoSong,
    /// "Refresh List" of the Mods menu: reads the Mods folder again (task-101).
    RefreshMods,
    /// "Info" of the Mods menu: the current mod's splash and details (task-106).
    ModInfo,
    ToggleVintage,
    /// "Play Mod Startup Sound" of the Options menu (the original's Edit > Preferences).
    ToggleStartupSound,
    SetKeyMode(KeyMode),
}

/// One entry of the Mods menu: a mod of the Mods folder.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ModEntry {
    pub path: PathBuf,
    /// The INFO menu name, or the file name of a file that is no mod.
    pub name: String,
}

/// An export that runs on its own thread.
pub struct Job {
    rx: Receiver<Result<(), String>>,
    path: PathBuf,
}

impl App {
    // ---- the Mods folder ----------------------------------------------------------------

    /// The options `mods::resolve` and the Mods menu look for mods with: the settings' Mods
    /// folder first, then `RB_MOD_DIR` and the default places (`ModOpts::mod_dir`).
    pub fn mod_opts(&self) -> ModOpts {
        ModOpts {
            mod_dir: self.settings.mods_dir.clone().filter(|d| d.is_dir()),
            ..ModOpts::default()
        }
    }

    /// The mods of the Mods folder in the original's folder order, one entry per INFO name.
    pub fn list_mods(&self) -> Result<Vec<ModEntry>, String> {
        let dir = match self.mod_opts().mod_dir() {
            Ok(Some(d)) => d,
            Ok(None) => return Ok(Vec::new()),
            Err(rb_cli::Fail(_, m)) => return Err(m),
        };
        let found = mods::scan(&dir).map_err(|rb_cli::Fail(_, m)| m)?;
        let mut seen = std::collections::HashSet::new();
        Ok(found
            .into_iter()
            .filter_map(|(path, name)| {
                let name = name?;
                // the first file in the folder order owns a duplicated name (docs/rbm.md 11.3)
                seen.insert(name.clone()).then_some(ModEntry { path, name })
            })
            .collect())
    }

    /// Reads the Mods folder again (task-101): the Mods menu's list is read here and nowhere
    /// else, at the app's start, by Mods > Refresh List and when the Mods folder setting
    /// changes. The caches that depend on the folder's files go with it: the demo answer of
    /// the current mod and the splash picture of the mod window.
    pub fn refresh_mods(&mut self) {
        self.mod_list = Some(self.list_mods());
        self.demo_cache = None;
        self.splash = None;
    }

    /// Mods > Refresh List: [`App::refresh_mods`], then the result on the status line, as the
    /// export reports its end: "Mod list refreshed: N mods in <folder> (+a, -b)" with what
    /// changed against the previous list (by file), or a read error of the folder on the
    /// notice line. The scan is synchronous and fast, so there is no "in progress" line.
    pub fn refresh_mods_reported(&mut self) {
        let paths = |l: &Option<Result<Vec<ModEntry>, String>>| -> Vec<PathBuf> {
            match l {
                Some(Ok(l)) => l.iter().map(|e| e.path.clone()).collect(),
                _ => Vec::new(),
            }
        };
        let before = paths(&self.mod_list);
        // the notice an earlier refresh set for a folder error goes with a good read
        let earlier = match &self.mod_list {
            Some(Err(e)) => Some(e.clone()),
            _ => None,
        };
        self.refresh_mods();
        match &self.mod_list {
            Some(Ok(list)) => {
                let after = paths(&self.mod_list);
                let added = after.iter().filter(|p| !before.contains(p)).count();
                let removed = before.iter().filter(|p| !after.contains(p)).count();
                let folder = match self.mod_opts().mod_dir() {
                    Ok(Some(d)) => format!(" in {}", d.display()),
                    _ => String::new(),
                };
                let n = list.len();
                let s = if n == 1 { "" } else { "s" };
                if earlier.is_some() && self.notice == earlier {
                    self.notice = None;
                }
                self.status = Some(format!(
                    "Mod list refreshed: {n} mod{s}{folder} (+{added}, -{removed})"
                ));
            }
            Some(Err(e)) => {
                self.status = None;
                self.notice = Some(e.clone());
            }
            None => {}
        }
    }

    /// The Mods menu's list as last read by [`App::refresh_mods`]; None before the first read.
    pub fn mods(&self) -> Option<&Result<Vec<ModEntry>, String>> {
        self.mod_list.as_ref()
    }

    /// What the Mods menu checks for the current song (task-093): whether "Standard ReBirth" is
    /// checked, and the index in `list` of the checked mod. The song's resolved kit decides: a
    /// mod is the entry of its file, so a mod found by file name is checked, and a mod chosen
    /// from outside the folder, or a song whose mod is missing (the built-in kit under another
    /// name), checks nothing. The original checks the item whose mod object is the active
    /// song's (`sub_40B977`, 0x40BF0C).
    pub fn mod_checks(&self, list: &[ModEntry]) -> (bool, Option<usize>) {
        match &self.doc.kit {
            Kit::Standard => (self.doc.mod_name == STANDARD, None),
            Kit::Mod(p) => (false, list.iter().position(|e| &e.path == p)),
        }
    }

    /// The kit a song names: the standard kit, or the mod of that name in the Mods folder.
    /// A mod that is not there gives the standard kit and a notice.
    fn kit_for(&mut self, mod_name: &str) -> Kit {
        if mod_name == STANDARD {
            return Kit::Standard;
        }
        let found = match self.mod_opts().mod_dir() {
            Ok(Some(dir)) => mods::find_in_dir(&dir, mod_name),
            Ok(None) => Ok(None),
            Err(e) => Err(e),
        };
        match found {
            Ok(Some(p)) => Kit::Mod(p),
            Ok(None) => {
                self.notice = Some(format!(
                    "The mod \"{mod_name}\" was not found in the Mods folder; the built-in kit \
                     plays. Choose it in the Mods menu."
                ));
                Kit::Standard
            }
            Err(rb_cli::Fail(_, m)) => {
                self.notice = Some(m);
                Kit::Standard
            }
        }
    }

    /// The drum samples of a kit: the built-in ones, or a mod's on top of them.
    fn kit_samples(&self, kit: &Kit) -> Result<Option<Arc<SampleSet>>, String> {
        match kit {
            Kit::Standard => Ok(self.samples.clone()),
            Kit::Mod(path) => {
                // a mod's missing slots come from the built-in kit
                let builtin = self
                    .samples
                    .clone()
                    .unwrap_or_else(rbm::standard::builtin_samples);
                load_mod(path, &builtin)
                    .map(|s| Some(Arc::new(s)))
                    .map_err(|rb_cli::Fail(_, m)| m)
            }
        }
    }

    // ---- opening ------------------------------------------------------------------------

    /// Opens the song at `path`: with unsaved changes first asks what to do with them.
    pub fn open_song(&mut self, path: &Path) {
        if self.doc.modified && self.link.is_some() {
            self.prompt = Some(Then::Open(path.to_path_buf()));
        } else {
            self.open_now(path);
        }
    }

    /// Opens the song at `path` in the player, replacing the document; a failure goes to the
    /// notice line and leaves the song open as it was.
    pub fn open_now(&mut self, path: &Path) {
        if self.link.is_none() {
            self.notice = Some("There is no player to open the song in.".into());
            return;
        }
        match session::open_song(path) {
            Ok(opened) => self.adopt(path, opened),
            Err(e) => self.notice = Some(e),
        }
    }

    /// Makes a song that was read the open document: its mod is looked up, the song is loaded
    /// with that kit.
    pub fn adopt(&mut self, path: &Path, opened: session::Opened) {
        self.adopt_as(Some(path.to_path_buf()), opened, None);
    }

    /// [`App::adopt`] for a song that may have no file (`path` is `None`: Untitled) and may be
    /// given its mod (`forced`: the mod's INFO name and kit) instead of the one it names.
    fn adopt_as(
        &mut self,
        path: Option<PathBuf>,
        opened: session::Opened,
        forced: Option<(String, Kit)>,
    ) {
        self.notice = None;
        let (mod_name, kit) = match forced {
            Some(f) => f,
            None => {
                let name = opened.engine.mod_name.clone();
                let kit = self.kit_for(&name);
                (name, kit)
            }
        };
        // a mod that does not load (not recrypted, unreadable) plays the built-in kit
        let (kit, samples) = match self.kit_samples(&kit) {
            Ok(s) => (kit, s),
            Err(e) => {
                self.notice = Some(e);
                (Kit::Standard, self.samples.clone())
            }
        };
        let Some(link) = &mut self.link else {
            return;
        };
        // a song the node pool refuses opens no window (sub_40B575's STOP alert): the current
        // song, the panel and the document stay, and no startup sound plays
        if link.load(&opened.engine, samples, &mut self.panel).is_err() {
            self.notice = Some(LoadRefused::ALERT.into());
            // the start song refused: the player keeps the built-in kit, which counts as the
            // kit that played, so a later song on it plays nothing (task-099)
            self.settled = true;
            return;
        }
        // the song document's constructor `sub_425C4D` opens the mod, then `sub_426C67`
        // (0x425FAB for a 1.x song, 0x426400 for an RBS song): the old song was closed first
        // (the unsaved prompt is the close's), so this song is its mod's only user
        self.settle(path, opened.source, mod_name, kit);
    }

    /// Whether the current song's mod has a `Demo.rbs` (the Mods menu's "Demo Song"): never for
    /// the built-in kit. The answer for a mod file is kept until another mod is asked.
    pub fn has_demo(&mut self) -> bool {
        let Kit::Mod(p) = &self.doc.kit else {
            return false;
        };
        if let Some((cp, has)) = &self.demo_cache
            && cp == p
        {
            return *has;
        }
        let has = session::mod_has_demo(p);
        self.demo_cache = Some((p.clone(), has));
        has
    }

    /// Mods > Demo Song: opens the demo of the current song's mod, like [`App::open_song`]
    /// opens a file (the unsaved prompt first).
    pub fn demo_song(&mut self) {
        let Kit::Mod(p) = self.doc.kit.clone() else {
            return;
        };
        if self.doc.modified && self.link.is_some() {
            self.prompt = Some(Then::Demo(p));
        } else {
            self.open_demo_now(&p);
        }
    }

    /// Opens the `Demo.rbs` of the mod file `mod_path` as a song that was never saved
    /// (Untitled, so Save goes to Save As and never writes into the mod), on that mod. The
    /// load is [`App::adopt_as`]'s: the player stops the playing song, and a demo the node pool
    /// refuses opens nothing. A failure goes to the notice line.
    pub fn open_demo_now(&mut self, mod_path: &Path) {
        if self.link.is_none() {
            self.notice = Some("There is no player to open the song in.".into());
            return;
        }
        let label = format!("{}: {}", mod_path.display(), session::DEMO);
        let opened = session::mod_demo(mod_path).and_then(|b| session::open_bytes(&b, &label));
        let info = session::mod_info(mod_path);
        match (opened, info) {
            (Ok(mut opened), Ok(info)) => {
                // the song's mod is the mod it came from, whatever its GLOB names
                opened.source.glob.mod_name = info.name.clone();
                opened.source.glob.mod_ftp = info.ftp;
                opened.source.glob.mod_www = info.web;
                opened.engine.mod_name = info.name.clone();
                self.adopt_as(
                    None,
                    opened,
                    Some((info.name, Kit::Mod(mod_path.to_path_buf()))),
                );
            }
            (Err(e), _) | (_, Err(e)) => self.notice = Some(e),
        }
    }

    /// The song is loaded in the player: its mod's startup sound plays when the preference is
    /// on and the kit changed (task-099), and the document is the song, not modified. The tail
    /// of [`App::adopt`] and [`App::fresh`].
    fn settle(&mut self, path: Option<PathBuf>, source: rbs::Song, mod_name: String, kit: Kit) {
        // only a changed kit plays its sound (task-099); the first song has no previous kit
        if !self.settled || self.doc.kit != kit {
            self.mod_opened();
        }
        self.settled = true;
        self.doc = Document {
            source,
            path,
            modified: false,
            mod_name,
            kit,
        };
        self.events_seen = 0;
    }

    /// The app's start (`sub_480AA0`, docs/ui_spec.md section 18): the song named on the
    /// command line is opened ([`App::adopt`]); without one the new empty song stays, and its
    /// mod's startup sound plays as the original's Untitled song's on "Standard ReBirth"
    /// (`sub_40A50D` -> `sub_40A581` -> `sub_426CA8`, `sub_426C67` at 0x426D48) when the
    /// preference is on.
    pub fn start(&mut self, song: Option<(PathBuf, session::Opened)>) {
        self.refresh_mods();
        match song {
            Some((path, opened)) => self.adopt(&path, opened),
            None => self.fresh(),
        }
    }

    /// The state of an app started without a song: the empty song, Untitled, on the standard
    /// mod, not modified, stopped, the panel at its values; its mod's startup sound plays when
    /// the preference is on and the kit changed ([`App::mod_opened`], task-099). The settings
    /// stay. [`App::start`] and File > New
    /// ([`App::new_now`]) both come here. Without a player there is nothing to load: the
    /// panel and the document are made new.
    fn fresh(&mut self) {
        if self.link.is_none() {
            self.panel = crate::model::Panel::new();
            self.notice = None;
            self.settle(None, rbs::Song::empty(), STANDARD.into(), Kit::Standard);
            return;
        }
        self.notice = None;
        let samples = self.samples.clone();
        if let Some(link) = &mut self.link {
            link.load_new(samples, &mut self.panel);
        }
        self.settle(None, rbs::Song::empty(), STANDARD.into(), Kit::Standard);
    }

    /// File > New: with unsaved changes first asks what to do with them.
    pub fn new_song(&mut self) {
        if self.doc.modified && self.link.is_some() {
            self.prompt = Some(Then::New);
        } else {
            self.new_now();
        }
    }

    /// Replaces the open song with the start state (`Link::load_new` stops the playing song).
    pub fn new_now(&mut self) {
        self.fresh();
    }

    /// `sub_426C67` after a song's mod was opened by its only user (word +1088 is 1,
    /// 0x426C6E): top-level msg 58, the mod's startup sound, when "Play Mod Startup Sound" is
    /// on (`sub_4098C3` at 0x426C85).
    fn mod_opened(&mut self) {
        if !self.settings.startup_sound {
            return;
        }
        if let Some(link) = &mut self.link {
            link.startup_sound();
        }
    }

    // ---- saving -------------------------------------------------------------------------

    /// The song as the panel shows it, as an `.rbs` file's bytes.
    pub fn song_bytes(&self) -> Result<Vec<u8>, String> {
        let Some(link) = &self.link else {
            return Err("There is no song to save.".into());
        };
        let song = document::compose(&self.doc.source, &self.panel, link, &self.doc.mod_name);
        song.write()
            .map_err(|e| format!("cannot write the song: {e}"))
    }

    /// Save: to the song's file, or Save As for a song that has none.
    pub fn save(&mut self) {
        match self.doc.path.clone() {
            Some(p) => self.save_to(&p),
            None => self.open_browser(Mode::SaveSong),
        }
    }

    /// Writes the song to `path`; it becomes the song's file and the song is no longer modified.
    /// An error is shown in the notice line and nothing else changes.
    pub fn save_to(&mut self, path: &Path) {
        let result = self
            .song_bytes()
            .and_then(|bytes| session::write_file(path, &bytes));
        match result {
            Ok(()) => {
                self.doc.path = Some(path.to_path_buf());
                self.doc.modified = false;
                self.notice = None;
                self.status = Some(format!("Saved {}", path.display()));
                if let Some(t) = self.after_save.take() {
                    self.proceed(t);
                }
            }
            Err(e) => {
                self.after_save = None;
                self.notice = Some(e);
            }
        }
    }

    // ---- exporting ----------------------------------------------------------------------

    /// Starts the export of the loop to `path` (the format is its extension) on its own thread:
    /// the song as saved is rendered by `rb render`'s code (`rb_cli::render`), so the file is
    /// the one `rb render` writes for the saved song.
    pub fn export_to(&mut self, path: &Path) {
        let Some(format) = ExportFormat::of_path(path).map(|f| match f {
            ExportFormat::Wav => Format::Wav,
            ExportFormat::Flac => Format::Flac,
            ExportFormat::Ogg => Format::Ogg,
        }) else {
            self.notice = Some(format!(
                "{}: the export's extension must be .wav, .flac or .ogg",
                path.display()
            ));
            return;
        };
        if self.export.is_some() {
            self.notice = Some("An export is still running.".into());
            return;
        }
        let song = self.song_bytes().and_then(|bytes| {
            let parsed = rbs::Song::parse(&bytes).map_err(|e| e.to_string())?;
            let mut song = rb_engine::load::convert(&parsed).map_err(|e| format!("{e:?}"))?;
            // the kit that plays, whatever the name says (a mod that was not found keeps its
            // name in the song)
            if matches!(self.doc.kit, Kit::Standard) {
                song.mod_name = STANDARD.into();
            }
            Ok(song)
        });
        let song = match song {
            Ok(s) => s,
            Err(e) => {
                self.notice = Some(e);
                return;
            }
        };
        let mods = ModOpts {
            mod_file: match &self.doc.kit {
                Kit::Mod(p) => Some(p.clone()),
                Kit::Standard => None,
            },
            ..ModOpts::default()
        };
        let out = path.to_path_buf();
        let label = self.doc.name();
        let (tx, rx) = std::sync::mpsc::channel();
        std::thread::spawn(move || {
            let mut r = Renderer::new(mods);
            let result = r
                .render_to_file(
                    &label,
                    &song,
                    DEFAULT_IDLE_BLOCKS,
                    format,
                    DEFAULT_QUALITY,
                    &out,
                )
                .map_err(|rb_cli::Fail(_, m)| m);
            let _ = tx.send(result);
        });
        self.notice = None;
        self.status = Some(format!("Exporting {} ...", path.display()));
        self.export = Some(Job {
            rx,
            path: path.to_path_buf(),
        });
    }

    /// Takes the result of a running export.
    pub fn poll_export(&mut self, ctx: &Context) {
        let Some(job) = &self.export else {
            return;
        };
        match job.rx.try_recv() {
            Ok(Ok(())) => {
                self.status = Some(format!("Exported {}", job.path.display()));
                self.export = None;
            }
            Ok(Err(e)) => {
                self.status = None;
                self.notice = Some(e);
                self.export = None;
            }
            Err(TryRecvError::Empty) => {
                ctx.request_repaint_after(std::time::Duration::from_millis(50))
            }
            Err(TryRecvError::Disconnected) => {
                self.status = None;
                self.notice = Some("The export stopped unexpectedly.".into());
                self.export = None;
            }
        }
    }

    /// Whether an export is running.
    pub fn exporting(&self) -> bool {
        self.export.is_some()
    }

    // ---- the mod ------------------------------------------------------------------------

    /// Changes the song's kit (`sub_426FB3`): the same kit again does nothing; otherwise the
    /// song is marked modified, its mod name becomes the kit's, and the player takes the kit's
    /// samples. A failure (a mod that is not recrypted) is shown and changes nothing.
    pub fn use_kit(&mut self, kit: Kit) {
        let name = match &kit {
            Kit::Standard => STANDARD.to_string(),
            Kit::Mod(p) => match session::mod_info(p) {
                Ok(info) => info.name,
                Err(e) => {
                    self.notice = Some(e);
                    return;
                }
            },
        };
        if kit == self.doc.kit && name == self.doc.mod_name {
            return;
        }
        let samples = match self.kit_samples(&kit) {
            Ok(s) => s,
            Err(e) => {
                self.notice = Some(e);
                return;
            }
        };
        let Some(link) = &mut self.link else {
            self.notice = Some("There is no player to change the mod of.".into());
            return;
        };
        if let Some(samples) = samples {
            link.set_mod(samples, &mut self.panel);
            // `sub_426C67` after the change (0x427030): rb has one song, the only user of
            // its mod; a kit that did not change plays nothing (task-099)
            if kit != self.doc.kit {
                self.mod_opened();
            }
        }
        // the save writes the mod's INFO URLs with its name (`sub_4270F7`, docs/rbm.md 14.1)
        let (ftp, web) = match &kit {
            Kit::Mod(p) => session::mod_info(p)
                .map(|i| (i.ftp, i.web))
                .unwrap_or_default(),
            Kit::Standard => Default::default(),
        };
        self.doc.source.glob.mod_ftp = ftp;
        self.doc.source.glob.mod_www = web;
        self.doc.kit = kit;
        self.doc.mod_name = name;
        self.doc.modified = true;
        self.notice = None;
    }

    // ---- commands -----------------------------------------------------------------------

    /// Runs a menu command.
    pub fn command(&mut self, cmd: Cmd) {
        match cmd {
            Cmd::New => self.new_song(),
            Cmd::Open => self.open_browser(Mode::OpenSong),
            Cmd::Save => self.save(),
            Cmd::SaveAs => self.open_browser(Mode::SaveSong),
            Cmd::Export => {
                if self.panel.transport.song_mode {
                    self.open_browser(Mode::Export);
                } else {
                    self.notice = Some(
                        "Export needs song mode: the original's export is disabled in pattern \
                         mode."
                            .into(),
                    );
                }
            }
            Cmd::Info => self.show_info(),
            Cmd::KeyHelp => self.show_key_help(),
            Cmd::About => self.show_about(),
            Cmd::ChooseModFile => self.open_browser(Mode::ChooseMod),
            Cmd::UseKit(kit) => self.use_kit(kit),
            Cmd::DemoSong => self.demo_song(),
            Cmd::RefreshMods => self.refresh_mods_reported(),
            Cmd::ModInfo => self.show_mod_info(),
            Cmd::ToggleVintage => {
                let on = !self.panel.vintage;
                self.panel.set_vintage(on);
            }
            Cmd::ToggleStartupSound => {
                self.settings.startup_sound = !self.settings.startup_sound;
                if let Err(e) = self.settings.save() {
                    self.notice = Some(e);
                }
            }
            Cmd::SetKeyMode(m) => self.key_mode = m,
        }
    }

    /// What a browser chose, by its mode (not an open: `open_song`).
    pub fn chosen(&mut self, mode: Mode, path: PathBuf) {
        match mode {
            Mode::OpenSong => self.open_song(&path),
            Mode::SaveSong => self.save_to(&path),
            Mode::Export => self.export_to(&path),
            Mode::ChooseMod => self.use_kit(Kit::Mod(path)),
        }
    }

    /// Notes the edits the panel made since the last frame: an edit marks the song modified. The
    /// transport's play, stop, locate and record are not edits.
    pub fn note_edits(&mut self) {
        let events = &self.panel.events[self.events_seen.min(self.panel.events.len())..];
        let edited = events.iter().any(|e| {
            use crate::model::Event;
            match e {
                Event::Press { .. } => false,
                Event::Set { id, .. } => !matches!(
                    id.as_str(),
                    "transport.position" | "transport.bar" | "transport.record"
                ),
                _ => true,
            }
        });
        self.doc.modified |= edited;
        // with a player the events are drained at the sync that follows
        self.events_seen = if self.link.is_some() {
            0
        } else {
            self.panel.events.len()
        };
    }

    // ---- the unsaved-changes prompt -----------------------------------------------------

    /// Does what the prompt guarded.
    fn proceed(&mut self, then: Then) {
        match then {
            Then::Open(p) => self.open_now(&p),
            Then::Demo(p) => self.open_demo_now(&p),
            Then::New => self.new_now(),
            Then::Quit => self.allow_close = true,
        }
    }

    /// The prompt's answer: Save saves (through Save As for a song with no file) and then does
    /// the guarded action; Don't Save does it at once; Cancel drops it.
    pub fn answer(&mut self, answer: Answer) {
        let Some(then) = self.prompt.take() else {
            return;
        };
        match answer {
            Answer::Cancel => {}
            Answer::DontSave => self.proceed(then),
            Answer::Save => {
                self.after_save = Some(then);
                self.save();
            }
        }
    }

    /// Whether the unsaved-changes prompt is open.
    pub fn prompting(&self) -> bool {
        self.prompt.is_some()
    }

    /// Draws the prompt and takes its keys: S or Enter saves, D or N does not, Esc cancels.
    pub fn prompt_ui(&mut self, ctx: &Context) {
        let Some(then) = self.prompt.clone() else {
            return;
        };
        let key = |k: Key| ctx.input_mut(|i| i.consume_key(Modifiers::NONE, k));
        let mut answer = None;
        if key(Key::S) || key(Key::Y) || key(Key::Enter) {
            answer = Some(Answer::Save);
        } else if key(Key::D) || key(Key::N) {
            answer = Some(Answer::DontSave);
        } else if key(Key::Escape) {
            answer = Some(Answer::Cancel);
        }
        let what = match then {
            Then::Open(_) | Then::Demo(_) => "before opening another song",
            Then::New => "before starting a new song",
            Then::Quit => "before closing",
        };
        let frame = Frame::new()
            .fill(SLATE_DARK)
            .stroke(Stroke::new(1.0, SLATE_LINE))
            .inner_margin(Margin::same(12))
            .corner_radius(3.0);
        Modal::new(Id::unique("rb_unsaved"))
            .frame(frame)
            .backdrop_color(Color32::from_black_alpha(170))
            .show(ctx, |ui| {
                ui.set_width(320.0);
                ui.label(
                    RichText::new(format!("Save changes to {}?", self.doc.name()))
                        .color(INK)
                        .strong()
                        .size(15.0),
                );
                ui.label(
                    RichText::new(format!("The song has unsaved changes {what}.")).color(INK_DIM),
                );
                ui.add_space(8.0);
                ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                    if ui.button("Cancel").clicked() {
                        answer = Some(Answer::Cancel);
                    }
                    if ui.button("Don't Save").clicked() {
                        answer = Some(Answer::DontSave);
                    }
                    if ui.button("Save").clicked() {
                        answer = Some(Answer::Save);
                    }
                });
            });
        if let Some(a) = answer {
            self.answer(a);
        }
    }

    /// The window's close request: with unsaved changes it asks first.
    pub fn close_check(&mut self, ctx: &Context) {
        if ctx.input(|i| i.viewport().close_requested()) && !self.request_close() {
            ctx.send_viewport_cmd(egui::ViewportCommand::CancelClose);
        }
    }

    /// A request to close the window: true when it may close now; with unsaved changes it
    /// opens the prompt instead and is false until the answer allows it.
    pub fn request_close(&mut self) -> bool {
        if self.allow_close || !self.doc.modified {
            return true;
        }
        if self.prompt.is_none() {
            self.prompt = Some(Then::Quit);
        }
        false
    }

    /// Whether an answer of the prompt has allowed the window to close.
    pub fn close_allowed(&self) -> bool {
        self.allow_close
    }

    /// Closes the window after an answer that allowed it.
    pub fn close_if_allowed(&mut self, ctx: &Context) {
        if self.allow_close {
            ctx.send_viewport_cmd(egui::ViewportCommand::Close);
        }
    }

    /// The window's title: the song's name, marked when modified.
    pub fn update_title(&mut self, ctx: &Context) {
        let title = format!(
            "OpenRB-338 - {}{}",
            self.doc.name(),
            if self.doc.modified { " *" } else { "" }
        );
        if title != self.title {
            ctx.send_viewport_cmd(egui::ViewportCommand::Title(title.clone()));
            self.title = title;
        }
    }

    // ---- drawing ------------------------------------------------------------------------

    /// Draws the menu bar and runs what it chooses.
    pub fn menu_bar(&mut self, ui: &mut Ui) {
        let mut cmd: Option<Cmd> = None;
        let enabled = !self.browsing() && !self.prompting();
        egui::Panel::top("rb_menu")
            .exact_size(MENU_H)
            .show(ui, |ui| {
                ui.add_enabled_ui(enabled, |ui| {
                    egui::MenuBar::new().ui(ui, |ui| {
                        ui.menu_button("File", |ui| {
                            if item(ui, "New", Some(&shortcuts::NEW)) {
                                cmd = Some(Cmd::New);
                            }
                            if item(ui, "Open...", Some(&shortcuts::OPEN)) {
                                cmd = Some(Cmd::Open);
                            }
                            if item(ui, "Save", Some(&shortcuts::SAVE)) {
                                cmd = Some(Cmd::Save);
                            }
                            if item(ui, "Save As...", None) {
                                cmd = Some(Cmd::SaveAs);
                            }
                            if item(ui, "Info...", None) {
                                cmd = Some(Cmd::Info);
                            }
                            ui.separator();
                            let ok = self.panel.transport.song_mode && !self.exporting();
                            let r = ui
                                .add_enabled(ok, egui::Button::new("Export Loop as Audio File..."));
                            let r = if self.panel.transport.song_mode {
                                r
                            } else {
                                r.on_disabled_hover_text("Export needs song mode.")
                            };
                            if r.clicked() {
                                cmd = Some(Cmd::Export);
                                ui.close();
                            }
                        });
                        ui.menu_button("Mods", |ui| {
                            let (std_on, checked) = match &self.mod_list {
                                Some(Ok(l)) => self.mod_checks(l),
                                _ => self.mod_checks(&[]),
                            };
                            if ui.selectable_label(std_on, STANDARD).clicked() {
                                cmd = Some(Cmd::UseKit(Kit::Standard));
                                ui.close();
                            }
                            match &self.mod_list {
                                None => {}
                                Some(Ok(list)) => {
                                    egui::ScrollArea::vertical()
                                        .max_height(300.0)
                                        .show(ui, |ui| {
                                            for (i, e) in list.iter().enumerate() {
                                                let on = checked == Some(i);
                                                if ui.selectable_label(on, &e.name).clicked() {
                                                    cmd =
                                                        Some(Cmd::UseKit(Kit::Mod(e.path.clone())));
                                                    ui.close();
                                                }
                                            }
                                        });
                                }
                                Some(Err(e)) => {
                                    ui.label(RichText::new(e).color(INK_DIM));
                                }
                            }
                            ui.separator();
                            if item(ui, "Choose file...", None) {
                                cmd = Some(Cmd::ChooseModFile);
                            }
                            if item(ui, "Refresh List", None) {
                                cmd = Some(Cmd::RefreshMods);
                            }
                            ui.separator();
                            let has_demo = self.has_demo();
                            let r = ui.add_enabled(has_demo, egui::Button::new("Demo Song"));
                            if r.clicked() {
                                cmd = Some(Cmd::DemoSong);
                                ui.close();
                            }
                            if item(ui, "Info...", None) {
                                cmd = Some(Cmd::ModInfo);
                            }
                        });
                        ui.menu_button("Options", |ui| {
                            let mut v = self.panel.vintage;
                            if ui.checkbox(&mut v, "Vintage ReBirth Synth Sound").clicked() {
                                cmd = Some(Cmd::ToggleVintage);
                                ui.close();
                            }
                            let mut st = self.settings.startup_sound;
                            if ui.checkbox(&mut st, "Play Mod Startup Sound").clicked() {
                                cmd = Some(Cmd::ToggleStartupSound);
                                ui.close();
                            }
                            ui.separator();
                            let ctx = ui.ctx().clone();
                            let synth = format!(
                                "Program Synth ({})",
                                shortcuts::text(&ctx, &shortcuts::KEYS_SYNTH)
                            );
                            let patterns = format!(
                                "Select Patterns ({})",
                                shortcuts::text(&ctx, &shortcuts::KEYS_PATTERNS)
                            );
                            if ui.radio(self.key_mode == KeyMode::Synth, synth).clicked() {
                                cmd = Some(Cmd::SetKeyMode(KeyMode::Synth));
                                ui.close();
                            }
                            if ui
                                .radio(self.key_mode == KeyMode::Patterns, patterns)
                                .clicked()
                            {
                                cmd = Some(Cmd::SetKeyMode(KeyMode::Patterns));
                                ui.close();
                            }
                        });
                        // Last in the bar. An item is one line of `HELP_ITEMS`.
                        ui.menu_button("Help", |ui| {
                            for (text, c) in HELP_ITEMS {
                                if item(ui, text, None) {
                                    cmd = Some(c.clone());
                                }
                            }
                        });
                    });
                });
            });
        if let Some(c) = cmd {
            self.command(c);
        }
    }
}

/// The items of the Help menu, in order.
const HELP_ITEMS: &[(&str, Cmd)] = &[
    ("Keyboard Shortcuts...", Cmd::KeyHelp),
    ("About OpenRB-338...", Cmd::About),
];

/// A menu item with its shortcut; true when clicked (the menu closes).
fn item(ui: &mut Ui, text: &str, shortcut: Option<&egui::KeyboardShortcut>) -> bool {
    let mut b = egui::Button::new(text);
    if let Some(sc) = shortcut {
        b = b.shortcut_text(shortcuts::text(ui.ctx(), sc));
    }
    let clicked = ui.add(b).clicked();
    if clicked {
        ui.close();
    }
    clicked
}
