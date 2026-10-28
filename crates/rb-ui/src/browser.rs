//! The in-app file browser (task-046): a modal overlay drawn with egui, used instead of any
//! native dialog to open and save songs (`.rbs`), to choose the export target (path and format
//! `.wav`, `.flac` or `.ogg`, the containers `rb render` writes) and to choose an RBM mod.
//!
//! A component for the menu (task-047). The caller makes a [`Browser`] for a [`Mode`], calls
//! [`Browser::show`] every frame while it is open, and acts on the [`Poll`] it returns:
//!
//! ```ignore
//! let mut browser = Browser::new(Mode::OpenSong, &settings);
//! // every frame:
//! match browser.show(ctx, &mut settings) {
//!     Poll::Open => {}
//!     Poll::Chosen(path) => { /* load, save or export `path` */ }
//!     Poll::Cancelled => { /* drop the browser */ }
//! }
//! ```
//!
//! What it does: lists a folder (folders first, then the files of the mode's extensions, or
//! every file with "All files"), navigates by mouse (click, double click, the Up button, the
//! places and the recent folders on the left) and by keyboard (Up/Down/PageUp/PageDown/Home/End
//! select, Enter opens a folder or accepts a file, Backspace and Alt+Up go to the parent,
//! Ctrl/Cmd+L edits the path, typing a letter jumps to the next name that starts with it, Esc
//! cancels). The save and export modes have a name field and ask before replacing a file; every
//! problem (a folder that cannot be read, a bad name, a missing folder, a mod rb cannot load) is
//! shown in the dialog. A mod that is not recrypted is listed and marked "needs rb recrypt"
//! (docs/rbm.md section 12.4); it cannot be chosen. The folders (songs, mods, recent) are kept
//! in [`Settings`].

use std::collections::HashMap;
use std::path::{Path, PathBuf};

use eframe::egui::{
    self, Align, Color32, Context, Event, Frame, Id, Key, Layout, Margin, Modal, Modifiers, Rect,
    RichText, Sense, Stroke, TextEdit, Ui, Vec2,
};

use crate::settings::{self, Settings};
use crate::shortcuts;

// The art's palette (crates/rb-ui-art/src/svg.rs `pal`).
const SLATE: Color32 = Color32::from_rgb(0x3b, 0x46, 0x56);
const SLATE_DARK: Color32 = Color32::from_rgb(0x2c, 0x35, 0x42);
const SLATE_LINE: Color32 = Color32::from_rgb(0x56, 0x64, 0x7a);
const INK: Color32 = Color32::from_rgb(0xe9, 0xed, 0xf2);
const INK_DIM: Color32 = Color32::from_rgb(0x9a, 0xa7, 0xb8);
const RED: Color32 = Color32::from_rgb(0xe2, 0x41, 0x3b);
const ORANGE: Color32 = Color32::from_rgb(0xf0, 0x8a, 0x24);
const GOLD: Color32 = Color32::from_rgb(0xcd, 0xb5, 0x4a);
const AMBER: Color32 = Color32::from_rgb(0xff, 0xb0, 0x00);
const BG: Color32 = Color32::from_rgb(0x1d, 0x21, 0x27);

const ROW_H: f32 = 20.0;

/// What the browser is asked for.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Mode {
    /// An existing `.rbs` song.
    OpenSong,
    /// A `.rbs` song to write (an existing file is replaced after a confirmation).
    SaveSong,
    /// The audio file to write: `.wav`, `.flac` or `.ogg`.
    Export,
    /// An existing `.rbm` mod.
    ChooseMod,
}

impl Mode {
    fn title(self) -> &'static str {
        match self {
            Mode::OpenSong => "Open song",
            Mode::SaveSong => "Save song",
            Mode::Export => "Export audio",
            Mode::ChooseMod => "Choose mod",
        }
    }

    fn button(self) -> &'static str {
        match self {
            Mode::OpenSong | Mode::ChooseMod => "Open",
            Mode::SaveSong => "Save",
            Mode::Export => "Export",
        }
    }

    fn writes(self) -> bool {
        matches!(self, Mode::SaveSong | Mode::Export)
    }
}

/// The container of an exported file, chosen by the extension as `rb render` does.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ExportFormat {
    Wav,
    Flac,
    Ogg,
}

impl ExportFormat {
    pub const ALL: [ExportFormat; 3] = [ExportFormat::Wav, ExportFormat::Flac, ExportFormat::Ogg];

    pub fn extension(self) -> &'static str {
        match self {
            ExportFormat::Wav => "wav",
            ExportFormat::Flac => "flac",
            ExportFormat::Ogg => "ogg",
        }
    }

    pub fn from_extension(ext: Option<&str>) -> Option<Self> {
        let ext = ext?.to_ascii_lowercase();
        Self::ALL.into_iter().find(|f| f.extension() == ext)
    }

    /// The format a chosen export path asks for.
    pub fn of_path(path: &Path) -> Option<Self> {
        Self::from_extension(path.extension().and_then(|e| e.to_str()))
    }
}

/// What a frame of the browser decided.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Poll {
    /// Still open.
    Open,
    /// The user chose this path (for the writing modes: after the overwrite confirmation).
    Chosen(PathBuf),
    /// The user closed it without choosing.
    Cancelled,
}

/// Whether a mod file is usable by rb.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ModStatus {
    Ready,
    /// Parses, but is not recrypted (docs/rbm.md section 12.4).
    NeedsRecrypt,
    /// Not an RBM file.
    Invalid(String),
}

impl ModStatus {
    /// Reads the mod at `path` the way `rb` does to decide whether it can load its samples.
    pub fn of(path: &Path) -> Self {
        let data = match std::fs::read(path) {
            Ok(d) => d,
            Err(e) => return ModStatus::Invalid(format!("cannot read: {e}")),
        };
        let rbm = match rbm::Rbm::parse(data) {
            Ok(r) => r,
            Err(e) => return ModStatus::Invalid(e.to_string()),
        };
        match rbm::KeyTable::embedded(&rbm) {
            Some(t) if rbm.slots_covered(&t) => ModStatus::Ready,
            _ => ModStatus::NeedsRecrypt,
        }
    }
}

#[derive(Clone, Debug)]
struct Entry {
    name: String,
    path: PathBuf,
    dir: bool,
    size: u64,
}

pub struct Browser {
    mode: Mode,
    dir: PathBuf,
    path_text: String,
    name: String,
    format: ExportFormat,
    show_all: bool,
    entries: Vec<Entry>,
    selected: Option<usize>,
    error: Option<String>,
    /// A path that exists and waits for the overwrite confirmation.
    confirm: Option<PathBuf>,
    mods: HashMap<PathBuf, ModStatus>,
    scroll: f32,
    view_h: f32,
    scroll_to_selected: bool,
    first_frame: bool,
    /// Where the widgets were drawn in the last frame, by name (`row:NAME`, `place:LABEL`,
    /// `button:LABEL`): lets tests (and a UI test driver) click them.
    hits: Vec<(String, Rect)>,
}

impl Browser {
    /// A browser for `mode`, starting in the mode's default folder: the songs folder (songs,
    /// export) or the mods folder (mods) of `settings`, else the documents folder.
    pub fn new(mode: Mode, settings: &Settings) -> Self {
        let configured = match mode {
            Mode::ChooseMod => settings.mods_dir.as_ref(),
            _ => settings.songs_dir.as_ref(),
        };
        let dir = configured
            .filter(|d| d.is_dir())
            .cloned()
            .unwrap_or_else(settings::documents_dir);
        let mut b = Self {
            mode,
            dir: dir.clone(),
            path_text: String::new(),
            name: String::new(),
            format: ExportFormat::Wav,
            show_all: false,
            entries: Vec::new(),
            selected: None,
            error: None,
            confirm: None,
            mods: HashMap::new(),
            scroll: 0.0,
            view_h: 0.0,
            scroll_to_selected: false,
            first_frame: true,
            hits: Vec::new(),
        };
        b.go(dir);
        b
    }

    /// Starts in `dir` (when it is a folder), for example the folder of the song that is open.
    pub fn with_dir(mut self, dir: &Path) -> Self {
        if dir.is_dir() {
            self.go(dir.to_path_buf());
        }
        self
    }

    /// The suggested file name of a writing mode ("song.rbs"); an export takes the format from
    /// its extension.
    pub fn with_name(mut self, name: &str) -> Self {
        self.name = name.to_string();
        if let Some(f) =
            ExportFormat::from_extension(Path::new(name).extension().and_then(|e| e.to_str()))
        {
            self.format = f;
        }
        self
    }

    /// The rectangle a widget had in the last frame (`row:NAME`, `place:LABEL`,
    /// `recent:LABEL`, `button:LABEL`).
    pub fn hit(&self, name: &str) -> Option<Rect> {
        self.hits.iter().find(|(n, _)| n == name).map(|(_, r)| *r)
    }

    pub fn mode(&self) -> Mode {
        self.mode
    }

    pub fn dir(&self) -> &Path {
        &self.dir
    }

    /// The text of the dialog's error line, if any.
    pub fn error(&self) -> Option<&str> {
        self.error.as_deref()
    }

    /// The path waiting for the overwrite confirmation, if any.
    pub fn pending_overwrite(&self) -> Option<&Path> {
        self.confirm.as_deref()
    }

    pub fn name(&self) -> &str {
        &self.name
    }

    pub fn set_name(&mut self, name: &str) {
        self.name = name.to_string();
    }

    pub fn format(&self) -> ExportFormat {
        self.format
    }

    /// The listed entries' names, in order (folders first).
    pub fn names(&self) -> Vec<&str> {
        self.entries.iter().map(|e| e.name.as_str()).collect()
    }

    pub fn selected_name(&self) -> Option<&str> {
        self.selected.map(|i| self.entries[i].name.as_str())
    }

    pub fn set_show_all(&mut self, all: bool) {
        self.show_all = all;
        self.reload();
    }

    /// The extensions the mode lists (lowercase).
    fn extensions(&self) -> Vec<&'static str> {
        match self.mode {
            Mode::OpenSong | Mode::SaveSong => vec!["rbs"],
            Mode::Export => vec![self.format.extension()],
            Mode::ChooseMod => vec!["rbm"],
        }
    }

    /// Lists `dir` and makes it the current folder; on an error the folder stays and the error
    /// is shown.
    pub fn go(&mut self, dir: PathBuf) {
        let dir = clean(dir);
        match self.read_entries(&dir) {
            Ok(entries) => {
                self.dir = dir;
                self.path_text = self.dir.display().to_string();
                self.entries = entries;
                self.selected = None;
                self.scroll = 0.0;
                self.error = None;
                self.confirm = None;
            }
            Err(e) => self.error = Some(e),
        }
    }

    fn reload(&mut self) {
        let dir = self.dir.clone();
        let keep = self.selected_name().map(str::to_string);
        self.go(dir);
        self.selected = keep.and_then(|n| self.entries.iter().position(|e| e.name == n));
    }

    fn read_entries(&self, dir: &Path) -> Result<Vec<Entry>, String> {
        let rd =
            std::fs::read_dir(dir).map_err(|e| format!("{}: cannot open: {e}", dir.display()))?;
        let exts = self.extensions();
        let mut out = Vec::new();
        for e in rd.flatten() {
            let name = e.file_name().to_string_lossy().into_owned();
            if name.starts_with('.') {
                continue;
            }
            let path = e.path();
            // `metadata` follows links, so a link to a folder is a folder.
            let Ok(md) = std::fs::metadata(&path) else {
                continue;
            };
            let dir = md.is_dir();
            if !dir && !self.show_all {
                let ext = path
                    .extension()
                    .and_then(|x| x.to_str())
                    .map(str::to_ascii_lowercase);
                if !ext.is_some_and(|x| exts.contains(&x.as_str())) {
                    continue;
                }
            }
            out.push(Entry {
                name,
                path,
                dir,
                size: md.len(),
            });
        }
        out.sort_by_key(|e| (!e.dir, e.name.to_lowercase()));
        Ok(out)
    }

    /// Goes to the parent folder, selecting the folder that was left.
    pub fn up(&mut self) {
        let Some(parent) = self.dir.parent().map(Path::to_path_buf) else {
            return;
        };
        let left = self
            .dir
            .file_name()
            .map(|n| n.to_string_lossy().into_owned());
        self.go(parent);
        self.selected = left.and_then(|n| self.entries.iter().position(|e| e.name == n));
        self.scroll_to_selected = true;
    }

    /// Moves the selection by `delta` rows (clamped); `None` selects the first or last.
    pub fn move_selection(&mut self, delta: isize) {
        if self.entries.is_empty() {
            return;
        }
        let last = self.entries.len() as isize - 1;
        let from = self
            .selected
            .map_or(if delta > 0 { -1 } else { last + 1 }, |i| i as isize);
        self.select((from + delta).clamp(0, last) as usize);
    }

    pub fn select(&mut self, i: usize) {
        self.selected = Some(i);
        self.scroll_to_selected = true;
        self.confirm = None;
        self.error = None;
        let e = &self.entries[i];
        if !e.dir && self.mode.writes() {
            self.name = e.name.clone();
            if let Some(f) = ExportFormat::of_path(&e.path) {
                self.format = f;
            }
        }
    }

    /// Selects the first entry after the selection whose name starts with `prefix`
    /// (case-insensitive).
    pub fn type_ahead(&mut self, prefix: &str) {
        let p = prefix.to_lowercase();
        let n = self.entries.len();
        let start = self.selected.map_or(0, |i| i + 1);
        if let Some(i) = (0..n)
            .map(|k| (start + k) % n)
            .find(|&i| self.entries[i].name.to_lowercase().starts_with(&p))
        {
            self.select(i);
        }
    }

    /// Replaces the extension of the name with the one of `format` (the export format buttons).
    pub fn set_format(&mut self, format: ExportFormat) {
        self.format = format;
        if !self.name.is_empty() {
            let stem = Path::new(&self.name)
                .file_stem()
                .map(|s| s.to_string_lossy().into_owned())
                .unwrap_or_default();
            self.name = format!("{stem}.{}", format.extension());
        }
        self.reload();
    }

    /// Enter on the path bar: a folder is entered, a file is chosen.
    pub fn go_to_text(&mut self, settings: &mut Settings) -> Poll {
        let text = self.path_text.trim();
        let p = expand_home(text);
        let p = if p.is_relative() { self.dir.join(p) } else { p };
        if p.is_dir() {
            self.go(p);
            Poll::Open
        } else if p.is_file() && !self.mode.writes() {
            self.finish(p, settings)
        } else if p.is_file() {
            self.error = Some(format!(
                "{text} is a file; type its name in the Name field, or enter a folder here."
            ));
            Poll::Open
        } else {
            self.error = Some(format!("{}: no such folder", text));
            Poll::Open
        }
    }

    /// Enter (or a double click) on the list: enters the selected folder, accepts the selected
    /// file or the typed name.
    pub fn activate(&mut self, settings: &mut Settings) -> Poll {
        if let Some(i) = self.selected
            && self.entries[i].dir
        {
            let p = self.entries[i].path.clone();
            self.go(p);
            return Poll::Open;
        }
        self.accept(settings)
    }

    /// The path the dialog would choose now, or why it cannot.
    pub fn target(&self) -> Result<PathBuf, String> {
        if !self.mode.writes() {
            return match self.selected.map(|i| &self.entries[i]) {
                Some(e) if !e.dir => Ok(e.path.clone()),
                _ => Err("Select a file first.".into()),
            };
        }
        let name = self.name.trim();
        if name.is_empty() {
            return Err("Type a file name.".into());
        }
        if let Some(c) = name
            .chars()
            .find(|c| c.is_control() || "/\\:*?\"<>|".contains(*c))
        {
            return Err(format!("The name cannot contain {c:?}."));
        }
        let ext = Path::new(name).extension().and_then(|e| e.to_str());
        let name = match self.mode {
            Mode::SaveSong => {
                if ext.is_some_and(|e| e.eq_ignore_ascii_case("rbs")) {
                    name.to_string()
                } else {
                    format!("{name}.rbs")
                }
            }
            _ => match (ext, ExportFormat::from_extension(ext)) {
                (_, Some(_)) => name.to_string(),
                (None, _) => format!("{name}.{}", self.format.extension()),
                (Some(_), None) => {
                    return Err("The extension must be .wav, .flac or .ogg.".into());
                }
            },
        };
        Ok(self.dir.join(name))
    }

    /// Validates the target and chooses it; an existing file of a writing mode first asks
    /// (see [`Browser::pending_overwrite`]).
    pub fn accept(&mut self, settings: &mut Settings) -> Poll {
        let path = match self.target() {
            Ok(p) => p,
            Err(e) => {
                self.error = Some(e);
                return Poll::Open;
            }
        };
        if self.mode.writes() {
            if path.is_dir() {
                self.error = Some(format!("{} is a folder.", path.display()));
                return Poll::Open;
            }
            if !self.dir.is_dir() {
                self.error = Some(format!(
                    "{}: the folder does not exist.",
                    self.dir.display()
                ));
                return Poll::Open;
            }
            if std::fs::metadata(&self.dir).is_ok_and(|m| m.permissions().readonly()) {
                self.error = Some(format!("{}: the folder is read-only.", self.dir.display()));
                return Poll::Open;
            }
            if path.exists() {
                self.error = None;
                self.confirm = Some(path);
                return Poll::Open;
            }
        }
        self.finish(path, settings)
    }

    /// Yes to the overwrite question.
    pub fn confirm_overwrite(&mut self, settings: &mut Settings) -> Poll {
        match self.confirm.take() {
            Some(p) => self.finish(p, settings),
            None => Poll::Open,
        }
    }

    /// No to the overwrite question.
    pub fn decline_overwrite(&mut self) {
        self.confirm = None;
    }

    fn finish(&mut self, path: PathBuf, settings: &mut Settings) -> Poll {
        if self.mode == Mode::ChooseMod {
            match ModStatus::of(&path) {
                ModStatus::Ready => {}
                ModStatus::NeedsRecrypt => {
                    self.error = Some(format!(
                        "{}: not recrypted. Run `rb recrypt --rb20ful RB20FUL.DAT FILE -o OUT.rbm` \
                         once and choose OUT.rbm.",
                        path.display()
                    ));
                    return Poll::Open;
                }
                ModStatus::Invalid(e) => {
                    self.error = Some(format!("{}: {e}", path.display()));
                    return Poll::Open;
                }
            }
        } else if !self.mode.writes() && !path.is_file() {
            self.error = Some(format!("{}: no such file.", path.display()));
            return Poll::Open;
        }
        if let Some(dir) = path.parent() {
            settings.note_recent(dir);
            if let Err(e) = settings.save() {
                // The choice stands; the failed note is not worth blocking it.
                eprintln!("rb-ui: settings: {e}");
            }
        }
        Poll::Chosen(path)
    }

    fn mod_status(&mut self, path: &Path) -> ModStatus {
        self.mods
            .entry(path.to_path_buf())
            .or_insert_with(|| ModStatus::of(path))
            .clone()
    }

    /// Draws the dialog over the app and handles its input for one frame.
    pub fn show(&mut self, ctx: &Context, settings: &mut Settings) -> Poll {
        let name_id = Id::unique("rb_browser_name");
        let path_id = Id::unique("rb_browser_path");
        let mut poll = Poll::Open;
        let screen = ctx.content_rect();
        let width = (screen.width() - 24.0).clamp(300.0, 620.0);
        let list_h = (screen.height() - 230.0).clamp(80.0, 270.0);

        self.hits.clear();
        // Keys first, so the widgets do not take them.
        let path_focus = ctx.memory(|m| m.has_focus(path_id));
        let name_focus = ctx.memory(|m| m.has_focus(name_id));
        let typing = path_focus || name_focus;
        let key = |k: Key, m: Modifiers| ctx.input_mut(|i| i.consume_key(m, k));
        if self.confirm.is_some() {
            // The safe answer is the default: Enter, Esc and N go back, only Y replaces. The
            // typed letters must not reach the Name field behind the question.
            let yes = key(Key::Y, Modifiers::NONE);
            let no = key(Key::Enter, Modifiers::NONE)
                || key(Key::Escape, Modifiers::NONE)
                || key(Key::N, Modifiers::NONE);
            ctx.input_mut(|i| i.events.retain(|e| !matches!(e, Event::Text(_))));
            if yes {
                poll = self.confirm_overwrite(settings);
            } else if no {
                self.decline_overwrite();
            }
        } else {
            if key(Key::Escape, Modifiers::NONE) {
                return Poll::Cancelled;
            }
            if key(Key::ArrowDown, Modifiers::NONE) {
                self.move_selection(1);
            }
            if key(Key::ArrowUp, Modifiers::NONE) {
                self.move_selection(-1);
            }
            let page = (self.view_h / ROW_H).max(2.0) as isize - 1;
            if key(Key::PageDown, Modifiers::NONE) {
                self.move_selection(page);
            }
            if key(Key::PageUp, Modifiers::NONE) {
                self.move_selection(-page);
            }
            if !typing && key(Key::Home, Modifiers::NONE) && !self.entries.is_empty() {
                self.select(0);
            }
            if !typing && key(Key::End, Modifiers::NONE) && !self.entries.is_empty() {
                self.select(self.entries.len() - 1);
            }
            if key(Key::ArrowUp, Modifiers::ALT)
                || (!typing && key(Key::Backspace, Modifiers::NONE))
            {
                self.up();
            }
            if ctx.input_mut(|i| i.consume_shortcut(&shortcuts::EDIT_PATH)) {
                ctx.memory_mut(|m| m.request_focus(path_id));
            }
            if key(Key::Enter, Modifiers::NONE) {
                poll = if path_focus {
                    self.go_to_text(settings)
                } else if name_focus {
                    self.accept(settings)
                } else {
                    self.activate(settings)
                };
            }
            if !typing {
                let typed: String = ctx.input(|i| {
                    i.events
                        .iter()
                        .filter_map(|e| match e {
                            Event::Text(t) => Some(t.as_str()),
                            _ => None,
                        })
                        .collect()
                });
                if !typed.trim().is_empty() {
                    self.type_ahead(typed.trim());
                }
            }
        }
        if poll != Poll::Open {
            return poll;
        }

        let frame = Frame::new()
            .fill(SLATE_DARK)
            .stroke(Stroke::new(1.0, SLATE_LINE))
            .inner_margin(Margin::same(10))
            .corner_radius(3.0);
        let modal = Modal::new(Id::unique("rb_browser"))
            .frame(frame)
            .backdrop_color(Color32::from_black_alpha(170));
        let first = std::mem::take(&mut self.first_frame);
        let resp = modal.show(ctx, |ui| {
            ui.set_width(width);
            style(ui);
            ui.label(
                RichText::new(self.mode.title())
                    .color(INK)
                    .strong()
                    .size(15.0),
            );
            ui.add_space(4.0);
            self.path_row(ui, path_id);
            ui.add_space(4.0);
            ui.horizontal_top(|ui| {
                self.sidebar(ui, settings, list_h);
                ui.vertical(|ui| {
                    if self.draw_list(ui, list_h) {
                        poll = self.activate(settings);
                    }
                });
            });
            ui.add_space(4.0);
            if self.mode.writes() {
                ui.horizontal(|ui| {
                    ui.label(RichText::new("Name").color(INK_DIM));
                    let w = ui.available_width()
                        - if self.mode == Mode::Export {
                            150.0
                        } else {
                            0.0
                        };
                    ui.add(
                        TextEdit::singleline(&mut self.name)
                            .id(name_id)
                            .desired_width(w.max(60.0)),
                    );
                    if self.mode == Mode::Export {
                        for f in ExportFormat::ALL {
                            let on = self.format == f;
                            if ui.selectable_label(on, f.extension()).clicked() && !on {
                                self.set_format(f);
                            }
                        }
                    }
                });
                if first {
                    ui.memory_mut(|m| m.request_focus(name_id));
                }
            }
            self.status_and_buttons(ui, settings, &mut poll);
        });
        if resp.should_close() && self.confirm.is_none() && poll == Poll::Open {
            return Poll::Cancelled;
        }
        poll
    }

    fn path_row(&mut self, ui: &mut Ui, path_id: Id) {
        ui.horizontal(|ui| {
            if ui
                .add_enabled(self.dir.parent().is_some(), egui::Button::new("Up"))
                .on_hover_text("Parent folder (Backspace, Alt+Up)")
                .clicked()
            {
                self.up();
            }
            let r = ui.add(
                TextEdit::singleline(&mut self.path_text)
                    .id(path_id)
                    .desired_width(f32::INFINITY),
            );
            let tip = format!(
                "{} edits the path; Enter goes there",
                shortcuts::text(ui.ctx(), &shortcuts::EDIT_PATH)
            );
            r.on_hover_text(tip);
        });
    }

    fn sidebar(&mut self, ui: &mut Ui, settings: &mut Settings, height: f32) {
        let size = Vec2::new(130.0, height + 60.0);
        ui.allocate_ui_with_layout(size, Layout::top_down(Align::Min), |ui| {
            ui.set_width(130.0);
            let mut places: Vec<(String, PathBuf)> = Vec::new();
            let songs = settings
                .songs_dir
                .clone()
                .unwrap_or_else(settings::documents_dir);
            let mods = settings
                .mods_dir
                .clone()
                .unwrap_or_else(settings::documents_dir);
            places.push(("Songs".into(), songs));
            places.push(("Mods".into(), mods));
            places.push(("Documents".into(), settings::documents_dir()));
            if let Some(h) = settings::home_dir() {
                places.push(("Home".into(), h));
            }
            ui.label(RichText::new("Places").color(INK_DIM).small());
            for (label, p) in places {
                let r = ui
                    .selectable_label(self.dir == p, label.as_str())
                    .on_hover_text(p.display().to_string());
                self.hits.push((format!("place:{label}"), r.rect));
                if r.clicked() {
                    self.go(p);
                }
            }
            ui.add_space(4.0);
            ui.label(RichText::new("Recent folders").color(INK_DIM).small());
            let recent = settings.recent.clone();
            if recent.is_empty() {
                ui.label(RichText::new("none yet").color(INK_DIM).small());
            }
            for p in recent {
                let label = p.file_name().map_or_else(
                    || p.display().to_string(),
                    |n| n.to_string_lossy().into_owned(),
                );
                let r = ui
                    .selectable_label(self.dir == p, label.as_str())
                    .on_hover_text(p.display().to_string());
                self.hits.push((format!("recent:{label}"), r.rect));
                if r.clicked() {
                    self.go(p);
                }
            }
            ui.add_space(6.0);
            let small = |t: &str| egui::Button::new(RichText::new(t).small());
            if ui.add(small("Use as songs folder")).clicked() {
                settings.songs_dir = Some(self.dir.clone());
                self.save_settings(settings);
            }
            if ui.add(small("Use as mods folder")).clicked() {
                settings.mods_dir = Some(self.dir.clone());
                self.save_settings(settings);
            }
        });
    }

    fn save_settings(&mut self, settings: &Settings) {
        if let Err(e) = settings.save() {
            self.error = Some(e);
        }
    }

    /// Draws the list; true when a row was double clicked.
    fn draw_list(&mut self, ui: &mut Ui, height: f32) -> bool {
        let mut activated = false;
        let mut clicked = None;
        let n = self.entries.len();
        ui.allocate_ui(Vec2::new(ui.available_width(), height), |ui| {
            Frame::new().fill(BG).show(ui, |ui| {
                ui.set_min_size(Vec2::new(ui.available_width(), height));
                let mut area = egui::ScrollArea::vertical()
                    .id_salt("rb_browser_list")
                    .auto_shrink([false, false])
                    .max_height(height);
                if self.scroll_to_selected {
                    if let Some(i) = self.selected {
                        let top = i as f32 * ROW_H;
                        let mut off = self.scroll;
                        if top < off {
                            off = top;
                        } else if top + ROW_H > off + self.view_h {
                            off = top + ROW_H - self.view_h;
                        }
                        area = area.vertical_scroll_offset(off.max(0.0));
                    }
                    self.scroll_to_selected = false;
                }
                let out = area.show_rows(ui, ROW_H, n, |ui, range| {
                    for i in range {
                        let e = self.entries[i].clone();
                        let is_mod = self.mode == Mode::ChooseMod && !e.dir;
                        let status = is_mod.then(|| self.mod_status(&e.path));
                        let (rect, r) = ui.allocate_exact_size(
                            Vec2::new(ui.available_width(), ROW_H),
                            Sense::click(),
                        );
                        self.hits.push((format!("row:{}", e.name), rect));
                        if self.selected == Some(i) {
                            ui.painter().rect_filled(rect, 2.0, SLATE);
                            ui.painter().rect_stroke(
                                rect,
                                2.0,
                                Stroke::new(1.0, ORANGE),
                                egui::StrokeKind::Inside,
                            );
                        } else if r.hovered() {
                            ui.painter().rect_filled(rect, 2.0, SLATE_DARK);
                        }
                        let font = egui::FontId::proportional(13.0);
                        let (label, color) = if e.dir {
                            (format!("{}/", e.name), GOLD)
                        } else if matches!(status, Some(ModStatus::NeedsRecrypt)) {
                            (e.name.clone(), AMBER)
                        } else if matches!(status, Some(ModStatus::Invalid(_))) {
                            (e.name.clone(), RED)
                        } else {
                            (e.name.clone(), INK)
                        };
                        let p = ui.painter();
                        p.text(
                            rect.left_center() + Vec2::new(6.0, 0.0),
                            egui::Align2::LEFT_CENTER,
                            label,
                            font.clone(),
                            color,
                        );
                        let right = match &status {
                            Some(ModStatus::NeedsRecrypt) => {
                                Some(("needs rb recrypt".to_string(), AMBER))
                            }
                            Some(ModStatus::Invalid(_)) => Some(("not a mod".to_string(), RED)),
                            _ if !e.dir => Some((human_size(e.size), INK_DIM)),
                            _ => None,
                        };
                        if let Some((t, c)) = right {
                            p.text(
                                rect.right_center() - Vec2::new(6.0, 0.0),
                                egui::Align2::RIGHT_CENTER,
                                t,
                                egui::FontId::proportional(11.0),
                                c,
                            );
                        }
                        if r.clicked() {
                            clicked = Some(i);
                        }
                        if r.double_clicked() {
                            activated = true;
                        }
                    }
                });
                self.scroll = out.state.clamped_offset().y;
                self.view_h = out.inner_rect.height();
                if n == 0 {
                    ui.painter().text(
                        out.inner_rect.center(),
                        egui::Align2::CENTER_CENTER,
                        "no matching files",
                        egui::FontId::proportional(12.0),
                        INK_DIM,
                    );
                }
            });
        });
        if let Some(i) = clicked {
            // A click does not move the view: the row is under the pointer already.
            self.select(i);
            self.scroll_to_selected = false;
        }
        activated
    }

    fn status_and_buttons(&mut self, ui: &mut Ui, settings: &mut Settings, poll: &mut Poll) {
        ui.horizontal(|ui| {
            let mut all = self.show_all;
            if ui.checkbox(&mut all, "All files").changed() {
                self.set_show_all(all);
            }
            let exts = self
                .extensions()
                .iter()
                .map(|e| format!("*.{e}"))
                .collect::<Vec<_>>();
            ui.label(RichText::new(exts.join(" ")).color(INK_DIM).small());
        });
        if let Some(path) = self.confirm.clone() {
            ui.add_space(2.0);
            ui.colored_label(
                AMBER,
                format!(
                    "{} already exists. Replace it?",
                    path.file_name()
                        .map_or_else(String::new, |n| n.to_string_lossy().into_owned())
                ),
            );
            ui.horizontal(|ui| {
                let replace = ui.button("Replace  (Y)");
                self.hits.push(("button:Replace".into(), replace.rect));
                if replace.clicked() {
                    *poll = self.confirm_overwrite(settings);
                }
                let back = ui.button("Back  (Enter, Esc, N)");
                self.hits.push(("button:Back".into(), back.rect));
                if back.clicked() {
                    self.decline_overwrite();
                }
            });
            return;
        }
        if let Some(e) = &self.error {
            ui.add_space(2.0);
            ui.colored_label(RED, e.as_str());
        }
        ui.add_space(4.0);
        ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
            if ui.button(self.mode.button()).clicked() {
                *poll = self.activate(settings);
            }
            if ui.button("Cancel").clicked() {
                *poll = Poll::Cancelled;
            }
        });
    }
}

fn style(ui: &mut Ui) {
    let v = ui.visuals_mut();
    v.override_text_color = Some(INK);
    v.selection.bg_fill = SLATE_LINE;
    v.extreme_bg_color = BG;
    v.widgets.inactive.weak_bg_fill = SLATE;
    v.widgets.hovered.weak_bg_fill = SLATE_LINE;
    v.widgets.active.weak_bg_fill = ORANGE;
}

/// `/a/b/../c` without the dots, so the path bar stays short (no link resolution).
fn clean(p: PathBuf) -> PathBuf {
    let mut out = PathBuf::new();
    for c in p.components() {
        match c {
            std::path::Component::ParentDir => {
                if !out.pop() {
                    out.push(c);
                }
            }
            std::path::Component::CurDir => {}
            _ => out.push(c),
        }
    }
    if out.as_os_str().is_empty() {
        PathBuf::from(".")
    } else {
        out
    }
}

fn expand_home(s: &str) -> PathBuf {
    if let (Some(rest), Some(h)) = (s.strip_prefix('~'), settings::home_dir())
        && (rest.is_empty() || rest.starts_with(['/', '\\']))
    {
        return h.join(rest.trim_start_matches(['/', '\\']));
    }
    PathBuf::from(s)
}

fn human_size(n: u64) -> String {
    if n < 10_000 {
        format!("{n} B")
    } else if n < 10_000_000 {
        format!("{:.0} kB", n as f64 / 1000.0)
    } else {
        format!("{:.1} MB", n as f64 / 1e6)
    }
}
