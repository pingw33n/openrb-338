//! The menu (task-047): load, save and export the song, choose its mod, the Vintage sound flag
//! and the keyboard mode, with the unsaved-changes prompt. Nothing else of the original's menu.
//!
//! What each item does (docs/ui_spec.md section 18):
//! - **File**: Open (Ctrl+O), Save (Ctrl+S; a song that was never saved goes to Save As), Save
//!   As, Export loop as audio file. The browser (task-046) returns paths only; the writes happen
//!   here and their errors are shown in the notice line.
//! - **Mods**: "Standard ReBirth", the mods of the Mods folder (one entry per INFO menu name, as
//!   the original's Mods menu: `sub_40C163`, docs/rbm.md section 14), and a file picked in the
//!   browser. A choice is the original's `sub_42D80D` -> `sub_426FB3`: the song is marked
//!   modified, the new kit goes to the player as song msg 19 ([`Link::set_mod`]) and a save
//!   writes the new name.
//! - **Options**: the Vintage ReBirth Synth Sound flag (GLOB byte 482; Edit menu item 323 of the
//!   original) and the keyboard mode (Ctrl+F, Ctrl+G; docs/ui_spec.md section 10).

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
use rb_engine::samples::SampleSet;

use crate::app::App;
use crate::browser::{ExportFormat, Mode};
use crate::document::{self, Document};
use crate::keys::KeyMode;
use crate::session;

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
    Open,
    Save,
    SaveAs,
    Export,
    /// "Choose file..." of the Mods menu: the browser for a mod.
    ChooseModFile,
    /// A kit from the Mods menu (or a mod file the browser chose).
    UseKit(Kit),
    ToggleVintage,
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
                let Some(builtin) = &self.samples else {
                    return Err(
                        "No drum samples: a mod needs the standard mod (rb-ui --standard-mod \
                         FILE, or RB_STANDARD_MOD)."
                            .into(),
                    );
                };
                load_mod(path, builtin)
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
        self.notice = None;
        let mod_name = opened.engine.mod_name.clone();
        let kit = self.kit_for(&mod_name);
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
        link.load(&opened.engine, samples, &mut self.panel);
        self.doc = Document {
            source: opened.source,
            path: Some(path.to_path_buf()),
            modified: false,
            mod_name,
            kit,
        };
        self.events_seen = 0;
        self.mod_list = None;
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
        let standard = self.standard_path.clone();
        let out = path.to_path_buf();
        let label = self.doc.name();
        let (tx, rx) = std::sync::mpsc::channel();
        std::thread::spawn(move || {
            let mut r = Renderer::new(standard.as_deref(), mods);
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
            Cmd::ChooseModFile => self.open_browser(Mode::ChooseMod),
            Cmd::UseKit(kit) => self.use_kit(kit),
            Cmd::ToggleVintage => {
                let on = !self.panel.vintage;
                self.panel.set_vintage(on);
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
            Then::Open(_) => "before opening another song",
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
            "rb - {}{}",
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
        let mods_open = std::cell::Cell::new(false);
        egui::Panel::top("rb_menu")
            .exact_size(MENU_H)
            .show(ui, |ui| {
                ui.add_enabled_ui(enabled, |ui| {
                    egui::MenuBar::new().ui(ui, |ui| {
                        ui.menu_button("File", |ui| {
                            if item(ui, "Open...", "Ctrl+O") {
                                cmd = Some(Cmd::Open);
                            }
                            if item(ui, "Save", "Ctrl+S") {
                                cmd = Some(Cmd::Save);
                            }
                            if item(ui, "Save As...", "") {
                                cmd = Some(Cmd::SaveAs);
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
                        let r = ui.menu_button("Mods", |ui| {
                            mods_open.set(true);
                            if self.mod_list.is_none() {
                                self.mod_list = Some(self.list_mods());
                            }
                            let cur = (self.doc.kit.clone(), self.doc.mod_name.clone());
                            let std_on = cur.0 == Kit::Standard && cur.1 == STANDARD;
                            if ui.selectable_label(std_on, STANDARD).clicked() {
                                cmd = Some(Cmd::UseKit(Kit::Standard));
                                ui.close();
                            }
                            match self.mod_list.clone() {
                                None => {}
                                Some(Ok(list)) => {
                                    egui::ScrollArea::vertical()
                                        .max_height(300.0)
                                        .show(ui, |ui| {
                                            for e in list {
                                                let on = cur.0 == Kit::Mod(e.path.clone());
                                                if ui.selectable_label(on, &e.name).clicked() {
                                                    cmd = Some(Cmd::UseKit(Kit::Mod(e.path)));
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
                            if item(ui, "Choose file...", "") {
                                cmd = Some(Cmd::ChooseModFile);
                            }
                        });
                        // the list is read again the next time the menu opens
                        if r.inner.is_none() && !mods_open.get() {
                            self.mod_list = None;
                        }
                        ui.menu_button("Options", |ui| {
                            let mut v = self.panel.vintage;
                            if ui.checkbox(&mut v, "Vintage ReBirth Synth Sound").clicked() {
                                cmd = Some(Cmd::ToggleVintage);
                                ui.close();
                            }
                            ui.separator();
                            if ui
                                .radio(self.key_mode == KeyMode::Synth, "Program Synth (Ctrl+F)")
                                .clicked()
                            {
                                cmd = Some(Cmd::SetKeyMode(KeyMode::Synth));
                                ui.close();
                            }
                            if ui
                                .radio(
                                    self.key_mode == KeyMode::Patterns,
                                    "Select Patterns (Ctrl+G)",
                                )
                                .clicked()
                            {
                                cmd = Some(Cmd::SetKeyMode(KeyMode::Patterns));
                                ui.close();
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

/// A menu item with its shortcut; true when clicked (the menu closes).
fn item(ui: &mut Ui, text: &str, shortcut: &str) -> bool {
    let mut b = egui::Button::new(text);
    if !shortcut.is_empty() {
        b = b.shortcut_text(shortcut);
    }
    let clicked = ui.add(b).clicked();
    if clicked {
        ui.close();
    }
    clicked
}
