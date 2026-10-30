//! File > Info (task-095): the information of the current song in a window. The facts come
//! from the document's song file as it was read ([`SongInfo::of`], no rendering, no player).
//! The title, text, web page and copyright are edited in place (task-117, [`Field`],
//! [`set_field`]): an edit changes the document's song and marks it modified; the other facts
//! are read-only.

use eframe::egui::{self, Context, Key, Modifiers, RichText};
use orb_cli::info::{SongFacts, last_tick};
use orb_cli::mods::{Kit, STANDARD};
use orb_engine::seq::Event as TrackEvent;

use crate::app::App;
use crate::document::Document;
use crate::palette::{INK, INK_DIM};

/// The id of the info window of an open (`App::info_serial`).
pub fn info_window_id(serial: u32) -> egui::Id {
    egui::Id::unique(("orb_song_info_window", serial))
}

/// Where the song's mod stands.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ModState {
    /// The built-in kit under the name "Standard ReBirth".
    Standard,
    /// A mod file of the Mods folder (or one chosen) plays.
    Loaded,
    /// The song names a mod that is not there: the built-in kit plays.
    Missing,
}

impl ModState {
    pub fn text(&self) -> &'static str {
        match self {
            ModState::Standard => "built-in kit",
            ModState::Loaded => "loaded",
            ModState::Missing => "missing (the built-in kit plays)",
        }
    }
}

/// What the info window shows.
#[derive(Clone, Debug, PartialEq)]
pub struct SongInfo {
    /// The file name, "Untitled" for a song with no file.
    pub name: String,
    /// The folder of the file, if it has one.
    pub folder: Option<String>,
    pub window_title: String,
    /// The more text, 0x0d (the file's newline) turned into `\n`.
    pub text: String,
    pub web_page: String,
    pub mod_name: String,
    pub mod_state: ModState,
    pub mod_ftp: String,
    pub mod_www: String,
    /// The file's format, "RBS 4.2" or "ReBirth 1.x MIDI file (file version 3.1)"
    /// ([`SongFacts::format`]).
    pub format: String,
    /// A ReBirth 1.x song (a MIDI file) rather than an RBS 4.x file.
    pub rb1x: bool,
    /// The format version of the file as read: "4.2", or a 1.x file's "File version", "3.1".
    pub version: String,
    pub copyright: String,
    pub bpm: f64,
    pub song_mode: bool,
    pub looping: bool,
    pub shuffle: u8,
    pub vintage: bool,
    /// The 1-based bar of the song's last track event, `None` for a song with none
    /// ([`SongFacts::last_event_bar`]).
    pub last_event_bar: Option<u32>,
}

impl SongInfo {
    /// The information of a document.
    pub fn of(doc: &Document, recorded: Option<&[Vec<TrackEvent>; 9]>) -> Self {
        // the facts are those of `orb info` (orb_cli::info)
        // a 1.x song has no `rbs` song: its facts were taken when it was opened (task-123)
        let mut f = match &doc.facts_1x {
            Some(f) => f.clone(),
            None => SongFacts::of_source(&doc.source),
        };
        if let Some(tracks) = recorded {
            // a recorded song: the tracks as the engine holds them (what Save writes)
            f.last_event_tick = last_tick(tracks.iter().flatten().map(|e| e.tick as u32));
        }
        let format = f.format();
        let last_event_bar = f.last_event_bar();
        let mod_state = match &doc.kit {
            Kit::Mod(_) => ModState::Loaded,
            Kit::Standard if doc.mod_name == STANDARD => ModState::Standard,
            Kit::Standard => ModState::Missing,
        };
        Self {
            name: doc.name(),
            folder: doc
                .path
                .as_ref()
                .and_then(|p| p.parent())
                .map(|d| d.display().to_string()),
            window_title: f.window_title,
            text: f.text,
            web_page: f.web_page,
            mod_name: doc.mod_name.clone(),
            mod_state,
            mod_ftp: f.mod_ftp,
            mod_www: f.mod_www,
            format,
            rb1x: f.rb1x,
            version: f.version,
            copyright: f.copyright,
            bpm: f.bpm,
            song_mode: f.song_mode,
            looping: f.looping,
            shuffle: f.shuffle,
            vintage: f.vintage,
            last_event_bar,
        }
    }

    /// The text of the "Last event" row: `bar N` or `none`.
    pub fn last_event_text(&self) -> String {
        match self.last_event_bar {
            Some(b) => format!("bar {b}"),
            None => "none".into(),
        }
    }
}

/// The web address a text stands for: an `http://` or `https://` address as it is, or a host-like
/// text (a dot, no space or control character, no scheme) with `http://` in front. Anything else
/// (`file:///...`, another scheme, plain words) is no link.
pub fn web_url(text: &str) -> Option<String> {
    let t = text;
    if t.is_empty() || t.chars().any(|c| c.is_whitespace() || c.is_control()) {
        return None;
    }
    let lower = t.to_ascii_lowercase();
    if lower.starts_with("http://") || lower.starts_with("https://") {
        return (t.len() > lower.find("//").unwrap() + 2).then(|| t.to_string());
    }
    let host_like = !t.contains(':') && t.contains('.') && !t.starts_with('.') && !t.ends_with('.');
    host_like.then(|| format!("http://{t}"))
}

/// A field of the song information that the window edits (task-117).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Field {
    /// USRI "Window title", at USRI +0.
    Title,
    /// USRI "More text", at USRI +41, 0x0d a line break.
    Text,
    /// USRI "Web page", at USRI +442.
    WebPage,
    /// HEAD "Copyright", at HEAD +9.
    Copyright,
}

impl Field {
    pub const ALL: [Field; 4] = [Field::Title, Field::Text, Field::WebPage, Field::Copyright];

    pub fn label(self) -> &'static str {
        match self {
            Field::Title => "Title",
            Field::Text => "Text",
            Field::WebPage => "Web page",
            Field::Copyright => "Copyright",
        }
    }

    /// The bytes the field holds, the zero after them not counted: the limits the original's
    /// readers and writers use (`sub_465D8D` / `sub_465D0E` with 0x28, 0x190 and 0x64 for USRI
    /// at 0x434960..0x434993 and 0x4348E4..0x434911, 0x80 for HEAD at 0x465BE2 / 0x465C2D;
    /// docs/song_load.md 13). The text's 400 count a line break twice ([`cost`]).
    pub fn limit(self) -> usize {
        match self {
            Field::Title => 40,
            Field::Text => 400,
            Field::WebPage => 100,
            Field::Copyright => 128,
        }
    }

    pub fn multiline(self) -> bool {
        self == Field::Text
    }

    /// The id of the field's text edit.
    pub fn edit_id(self) -> egui::Id {
        egui::Id::unique(("orb_song_info_edit", self))
    }
}

/// What a value takes of its field's [`Field::limit`]: a byte per character (Latin-1), and for
/// the text one more per line break. The original holds the text with CR LF in at most 400
/// bytes (`sub_465D8D` adds the 0x0a and counts it, 0x465DDA..0x465DED), so a text within this
/// limit is read back whole by the original.
pub fn cost(field: Field, value: &str) -> usize {
    let breaks = if field.multiline() {
        value.matches('\n').count()
    } else {
        0
    };
    value.chars().count() + breaks
}

/// The value a field can hold made from typed or pasted text: CR LF and CR are a line break,
/// which the text keeps and a one-line field turns into a space, as is a tab; any other
/// control character and any character outside Latin-1 (the file's encoding, `rbs` `get_z`)
/// becomes `?`.
pub fn sanitize(field: Field, text: &str) -> String {
    text.replace("\r\n", "\n")
        .replace('\r', "\n")
        .chars()
        .map(|c| match c {
            '\n' if field.multiline() => '\n',
            '\n' | '\t' => ' ',
            c if c as u32 > 0xFF || c.is_control() => '?',
            c => c,
        })
        .collect()
}

/// A field's value in the document, as the window shows it (the text's 0x0d, or CR LF, as a
/// line break). The document's `rbs` song holds the values for a 1.x song too
/// (`document::source_of_1x`).
pub fn field_value(doc: &Document, field: Field) -> String {
    let s = &doc.source;
    match field {
        Field::Title => s.usri.window_title.clone(),
        Field::Text => s.usri.text.replace("\r\n", "\n").replace('\r', "\n"),
        Field::WebPage => s.usri.web_page.clone(),
        Field::Copyright => s.head.copyright.clone(),
    }
}

/// Sets a field of the document from typed or pasted text ([`sanitize`]) and marks the
/// document modified. An edit that passes the field's limit is refused (false, nothing
/// changes), unless it shortens a value that was already longer (a file can hold a text the
/// limit does not take). The text is stored with 0x0d line breaks. A 1.x song's facts follow.
pub fn set_field(doc: &mut Document, field: Field, text: &str) -> bool {
    let value = sanitize(field, text);
    let old = field_value(doc, field);
    if value == old {
        return false;
    }
    let c = cost(field, &value);
    if c > field.limit() && c > cost(field, &old) {
        return false;
    }
    let s = &mut doc.source;
    match field {
        Field::Title => s.usri.window_title = value.clone(),
        Field::Text => s.usri.text = value.replace('\n', "\r"),
        Field::WebPage => s.usri.web_page = value.clone(),
        Field::Copyright => s.head.copyright = value.clone(),
    }
    if let Some(f) = &mut doc.facts_1x {
        match field {
            Field::Title => f.window_title = value,
            Field::Text => f.text = value,
            Field::WebPage => f.web_page = value,
            Field::Copyright => f.copyright = value,
        }
    }
    doc.modified = true;
    true
}

/// The count next to a field: the bytes left, in the units of [`cost`] (a line break takes two
/// in the text). With one byte left the text's count explains an Enter that does nothing
/// (egui's char limit counts a break once; `set_field` refuses it).
pub fn left_text(field: Field, value: &str) -> String {
    let left = field.limit() as i64 - cost(field, value) as i64;
    match (field.multiline(), left) {
        (false, _) => format!("{left} left"),
        (true, 1) => "1 left, too few for a line break (2)".into(),
        (true, _) => format!("{left} left"),
    }
}

/// The text edit of a field, with the bytes left; an edit applies to the document on every
/// change.
fn field_ui(ui: &mut egui::Ui, doc: &mut Document, field: Field, width: f32) {
    // the edit's width: the whole value column, less what stands beside a one-line edit (its
    // count, and the web page's "open" link); computed from the window's width, not from the
    // content, so the edit follows the window narrower too and a long line never widens it
    let gap = ui.spacing().item_spacing.x;
    let beside = match field {
        _ if field.multiline() => 0.0,
        Field::WebPage => COUNT_W + OPEN_W + 2.0 * gap,
        _ => COUNT_W + gap,
    };
    let edit_w = (width - beside).max(MIN_EDIT_W);
    // the edit applies its change to the document at once
    let edit = |ui: &mut egui::Ui, doc: &mut Document| {
        let mut buf = field_value(doc, field);
        // typing and pasting stop at the limit (a line break takes two of the text's bytes);
        // what still passes it is refused by `set_field`
        let breaks = cost(field, &buf) - buf.chars().count();
        let edit = if field.multiline() {
            egui::TextEdit::multiline(&mut buf).desired_rows(4)
        } else {
            egui::TextEdit::singleline(&mut buf)
        };
        let edit = edit
            .id(field.edit_id())
            .char_limit(field.limit().saturating_sub(breaks))
            .desired_width(edit_w);
        if ui.add(edit).changed() {
            set_field(doc, field, &buf);
        }
    };
    let count = |ui: &mut egui::Ui, doc: &Document| {
        let text = left_text(field, &field_value(doc, field));
        ui.label(RichText::new(text).small().color(INK_DIM));
    };
    if field.multiline() {
        ui.vertical(|ui| {
            edit(ui, doc);
            count(ui, doc);
        });
        return;
    }
    ui.horizontal(|ui| {
        edit(ui, doc);
        count(ui, doc);
        if field == Field::WebPage
            && let Some(url) = web_url(&field_value(doc, field))
        {
            ui.hyperlink_to("open", url);
        }
    });
}

/// The least width of a text edit, when the window is narrower than its content.
const MIN_EDIT_W: f32 = 80.0;
/// The room kept beside a one-line edit for its count ("400 left" in the small font), and for
/// the web page's "open" link.
const COUNT_W: f32 = 50.0;
const OPEN_W: f32 = 30.0;
/// The window's default width: 570 points, 1.5 times the earlier 380 (task-154). The most is
/// the screen less 16 (`max_size`), so a small screen clamps it.
pub const INFO_W: f32 = 570.0;
/// The horizontal space between the grid's columns.
const GRID_GAP: f32 = 12.0;
/// The width of the label column, the same in the window's three grids.
const LABEL_W: f32 = 72.0;

/// A web address as a link when it is one, else as plain text.
/// A cut text shows all of it on hover (a link's hover is its address).
pub(crate) fn link(ui: &mut egui::Ui, text: &str) {
    match web_url(text) {
        Some(url) => {
            ui.hyperlink_to(text, url);
        }
        None => {
            ui.label(RichText::new(text).color(INK)).on_hover_text(text);
        }
    }
}

impl App {
    /// File > Info.
    pub fn show_info(&mut self) {
        if !self.info_open {
            // a new id: no stored place, so the window opens at its default, the center; an
            // open window stays where it is
            self.info_serial += 1;
        }
        self.info_open = true;
    }

    /// The information of the song now: the document, and the tracks recording has made.
    pub fn song_info(&self) -> SongInfo {
        SongInfo::of(
            &self.doc,
            self.link.as_ref().and_then(|l| l.recorded_tracks()),
        )
    }

    /// The id of the info window now (for tests, to look its place up).
    pub fn info_id(&self) -> egui::Id {
        info_window_id(self.info_serial)
    }

    /// Draws the info window when it is open; it shows the document of the moment, so it
    /// follows an open of another song. Escape and the close button close it.
    pub fn info_ui(&mut self, ctx: &Context) {
        if !self.info_open {
            return;
        }
        if ctx.input_mut(|i| i.consume_key(Modifiers::NONE, Key::Escape)) {
            self.info_open = false;
            self.info_shown = None;
            return;
        }
        let info = self.song_info();
        self.info_shown = Some(info.clone());
        let mut open = true;
        // The pivot is the window's center and the default place applies when the id has no
        // stored place (each open has its own id), so the window is centered from its first
        // frame whatever its size, and dragging moves the stored place.
        let window = egui::Window::new("Song Information")
            .id(self.info_id())
            .open(&mut open)
            .collapsible(false)
            .resizable(true)
            .pivot(egui::Align2::CENTER_CENTER)
            .default_pos(ctx.content_rect().center())
            .vscroll(true)
            // a small window: the info scrolls in a window that fits
            .max_size(ctx.content_rect().size() - egui::vec2(16.0, 16.0))
            .default_width(INFO_W);
        let doc = &mut self.doc;
        window.show(ctx, |ui| {
            // the value column of the edits grid is what the window leaves after the label
            // column and the grid's spacing
            let value_w = ui.available_width() - LABEL_W - GRID_GAP;
            let row = |ui: &mut egui::Ui, k: &str, v: &str| {
                ui.label(RichText::new(k).color(INK_DIM));
                // the value may be cut at the window's edge: the hover has all of it
                ui.label(RichText::new(v).color(INK)).on_hover_text(v);
                ui.end_row();
            };
            // three grids with the same label column: the file, the editable song information
            // and the read-only facts, with a line between them
            let grid = |id: &str| {
                egui::Grid::new(id)
                    .num_columns(2)
                    .spacing([GRID_GAP, 4.0])
                    .min_col_width(LABEL_W)
            };
            // in the read-only grids a long value (a deep folder, a web address) is cut at
            // the window's edge and never widens the window
            let cut = |ui: &mut egui::Ui| {
                ui.style_mut().wrap_mode = Some(egui::TextWrapMode::Truncate);
            };
            grid("orb_song_info_file").show(ui, |ui| {
                cut(ui);
                row(ui, "File", &info.name);
                if let Some(f) = &info.folder {
                    row(ui, "Folder", f);
                }
            });
            ui.separator();
            grid("orb_song_info_edits").show(ui, |ui| {
                for field in Field::ALL {
                    ui.label(RichText::new(field.label()).color(INK_DIM));
                    field_ui(ui, doc, field, value_w);
                    ui.end_row();
                }
            });
            ui.separator();
            grid("orb_song_info_grid").show(ui, |ui| {
                cut(ui);
                row(
                    ui,
                    "Mod",
                    &format!("{} ({})", info.mod_name, info.mod_state.text()),
                );
                ui.label(RichText::new("Mod web").color(INK_DIM));
                link(ui, &info.mod_www);
                ui.end_row();
                ui.label(RichText::new("Mod ftp").color(INK_DIM));
                ui.label(RichText::new(&info.mod_ftp).color(INK))
                    .on_hover_text(&info.mod_ftp);
                ui.end_row();
                row(ui, "Format", &info.format);
                row(ui, "Tempo", &format!("{:.3} BPM", info.bpm));
                row(ui, "Mode", if info.song_mode { "song" } else { "pattern" });
                row(ui, "Loop", if info.looping { "on" } else { "off" });
                row(ui, "Shuffle", &info.shuffle.to_string());
                row(
                    ui,
                    "Synth sound",
                    if info.vintage {
                        "vintage"
                    } else {
                        "ReBirth 2.0"
                    },
                );
                row(ui, "Last event", &info.last_event_text());
            });
        });
        if !open {
            self.info_open = false;
        }
    }

    /// Whether the info window is open.
    pub fn info_is_open(&self) -> bool {
        self.info_open
    }

    /// The information the window drew in its last frame (for tests).
    pub fn info_shown(&self) -> Option<&SongInfo> {
        self.info_shown.as_ref()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    fn doc() -> Document {
        let mut d = Document::new();
        d.source.usri.window_title = "My song".into();
        d.source.usri.text = "line one\rline two".into();
        d.source.usri.web_page = "example.com/x".into();
        d.source.glob.tempo = 133_500;
        d
    }

    #[test]
    fn untitled_song_strings_and_standard_mod() {
        let i = SongInfo::of(&doc(), None);
        assert_eq!(i.name, "Untitled");
        assert_eq!(i.folder, None);
        assert_eq!(i.window_title, "My song");
        assert_eq!(i.text, "line one\nline two");
        assert_eq!(i.web_page, "example.com/x");
        assert_eq!(i.mod_name, STANDARD);
        assert_eq!(i.mod_state, ModState::Standard);
        assert_eq!(i.version, "4.2");
        assert_eq!(i.bpm, 133.5);
    }

    #[test]
    fn last_event_row_from_the_facts() {
        // no events: none
        let i = SongInfo::of(&doc(), None);
        assert_eq!(i.last_event_bar, None);
        assert_eq!(i.last_event_text(), "none");
        // 4.x: a position is in 32nd notes (32 a bar): bar 3 starts at 64
        let mut d = doc();
        d.source.tracks[0].events.push(rbs::Event {
            pos: 70,
            ctrl: 1,
            value: 2,
        });
        let i = SongInfo::of(&d, None);
        assert_eq!(i.last_event_bar, Some(3));
        assert_eq!(i.last_event_text(), "bar 3");
        // 1.x: the row is the facts' own, whatever the (empty) `rbs` source says
        let mut f = SongFacts::of_source(&d.source);
        f.rb1x = true;
        f.last_event_tick = Some(768 * 9 + 5);
        d.facts_1x = Some(f);
        let i = SongInfo::of(&d, None);
        assert_eq!(i.last_event_bar, Some(10));
        assert_eq!(i.last_event_text(), "bar 10");
    }

    #[test]
    fn crlf_latin1_and_mod_addresses() {
        let mut d = doc();
        d.source.usri.text = "a\r\nb\u{e9}".into();
        d.source.glob.mod_www = "www.mod.org".into();
        d.source.glob.mod_ftp = "ftp://mod.org/x".into();
        let i = SongInfo::of(&d, None);
        assert_eq!(i.text, "a\nb\u{e9}");
        assert_eq!(i.mod_www, "www.mod.org");
        assert_eq!(i.mod_ftp, "ftp://mod.org/x");
        // a Latin-1 byte of the file reads as that character
        let mut s = rbs::Song::empty();
        s.usri.window_title = "caf\u{e9}".into();
        let back = rbs::Song::parse(&s.write().unwrap()).unwrap();
        assert_eq!(back.usri.window_title, "caf\u{e9}");
    }

    /// The body of the chunk `tag` in a written song.
    fn chunk<'a>(file: &'a [u8], tag: &[u8; 4]) -> &'a [u8] {
        let at = file.windows(4).position(|w| w == tag).unwrap() + 8;
        let size = u32::from_be_bytes(file[at - 4..at].try_into().unwrap()) as usize;
        &file[at..at + size]
    }

    #[test]
    fn an_edit_of_each_field_is_saved_in_its_chunk() {
        let mut d = doc();
        assert!(!d.modified);
        assert!(set_field(&mut d, Field::Title, "New title"));
        assert!(d.modified);
        assert!(set_field(&mut d, Field::Text, "one\ntwo\r\nthree"));
        assert!(set_field(&mut d, Field::WebPage, "www.example.org"));
        assert!(set_field(&mut d, Field::Copyright, "(c) me"));
        let bytes = d.source.write().unwrap();
        let usri = chunk(&bytes, b"USRI");
        assert_eq!(&usri[..10], b"New title\0");
        assert_eq!(&usri[41..56], b"one\rtwo\rthree\0\0");
        assert_eq!(&usri[442..458], b"www.example.org\0");
        let head = chunk(&bytes, b"HEAD");
        assert_eq!(&head[9..16], b"(c) me\0");
        let back = rbs::Song::parse(&bytes).unwrap();
        assert_eq!(back.usri.window_title, "New title");
        assert_eq!(back.usri.text, "one\rtwo\rthree");
        assert_eq!(back.usri.web_page, "www.example.org");
        assert_eq!(back.head.copyright, "(c) me");
        // the window shows the 0x0d as line breaks again
        d.source = back;
        assert_eq!(field_value(&d, Field::Text), "one\ntwo\nthree");
        // the same value again is no edit
        d.modified = false;
        assert!(!set_field(&mut d, Field::Title, "New title"));
        assert!(!d.modified);
    }

    #[test]
    fn the_byte_limits_refuse_longer_values() {
        let mut d = Document::new();
        for f in Field::ALL {
            let full = "x".repeat(f.limit());
            assert!(set_field(&mut d, f, &full), "{f:?}");
            assert!(!set_field(&mut d, f, &format!("{full}y")), "{f:?}");
            assert_eq!(field_value(&d, f), full);
        }
        // every field at its limit is written
        let back = rbs::Song::parse(&d.source.write().unwrap()).unwrap();
        assert_eq!(back.usri.window_title.len(), 40);
        assert_eq!(back.usri.text.len(), 400);
        assert_eq!(back.usri.web_page.len(), 100);
        assert_eq!(back.head.copyright.len(), 128);
        // a line break takes two of the text's 400 bytes
        assert_eq!(cost(Field::Text, "a\nb"), 4);
        assert_eq!(cost(Field::Title, "a b"), 3);
        let breaks = "ab\n".repeat(100);
        assert_eq!(cost(Field::Text, &breaks), 400);
        assert!(set_field(&mut d, Field::Text, &breaks));
        assert!(!set_field(&mut d, Field::Text, &format!("{breaks}c")));
        // the count is in the same units: with one byte left a line break does not fit
        let almost = "ab\n".repeat(99) + "abc";
        assert_eq!(cost(Field::Text, &almost), 399);
        assert_eq!(
            left_text(Field::Text, &almost),
            "1 left, too few for a line break (2)"
        );
        assert!(set_field(&mut d, Field::Text, &almost));
        assert!(!set_field(&mut d, Field::Text, &format!("{almost}\n")));
        assert!(set_field(&mut d, Field::Text, &format!("{almost}c")));
        assert_eq!(left_text(Field::Text, "a\n"), "397 left");
        assert_eq!(left_text(Field::Title, "abc"), "37 left");
        // a text already longer than the limit (a file's) may still be shortened
        d.source.usri.text = "y".repeat(400) + "\r";
        assert!(!set_field(&mut d, Field::Text, &("y".repeat(400) + "\nz")));
        assert!(set_field(&mut d, Field::Text, &"y".repeat(400)));
    }

    #[test]
    fn characters_outside_latin1_become_question_marks() {
        assert_eq!(
            sanitize(Field::Title, "caf\u{e9} \u{20ac}5 \u{1f600}"),
            "caf\u{e9} ?5 ?"
        );
        // control characters: a tab and a line break are a space in a one-line field
        assert_eq!(
            sanitize(Field::Title, "a\tb\nc\r\nd\u{1b}e\u{85}"),
            "a b c d?e?"
        );
        assert_eq!(sanitize(Field::Text, "a\r\nb\rc\nd\te"), "a\nb\nc\nd e");
        let mut d = Document::new();
        assert!(set_field(&mut d, Field::Copyright, "\u{a9} \u{2122}"));
        assert_eq!(d.source.head.copyright, "\u{a9} ?");
        let back = rbs::Song::parse(&d.source.write().unwrap()).unwrap();
        assert_eq!(back.head.copyright, "\u{a9} ?");
        assert_eq!(chunk(&d.source.write().unwrap(), b"HEAD")[9], 0xa9);
    }

    #[test]
    fn a_1x_songs_facts_follow_an_edit() {
        let mut d = doc();
        d.facts_1x = Some(SongFacts::of_source(&d.source));
        assert!(set_field(&mut d, Field::Text, "x\ny"));
        assert!(set_field(&mut d, Field::Copyright, "c"));
        let i = SongInfo::of(&d, None);
        assert_eq!(i.text, "x\ny");
        assert_eq!(i.copyright, "c");
        assert_eq!(d.source.usri.text, "x\ry");
    }

    #[test]
    fn web_url_accepts_web_addresses_only() {
        assert_eq!(web_url("http://a.org/x").as_deref(), Some("http://a.org/x"));
        assert_eq!(web_url("HTTPS://a.org").as_deref(), Some("HTTPS://a.org"));
        assert_eq!(
            web_url("www.a.org/x").as_deref(),
            Some("http://www.a.org/x")
        );
        for bad in [
            "",
            "file:///etc/passwd",
            "ftp://a.org",
            "javascript:alert(1)",
            "http://",
            "words",
            "a b.org",
            "a.org\n",
            ".org",
            "a.",
            "mailto:a@b.org",
            "x\u{1}.org",
        ] {
            assert_eq!(web_url(bad), None, "{bad:?}");
        }
    }

    #[test]
    fn mod_loaded_and_missing() {
        let mut d = doc();
        d.path = Some(PathBuf::from("/songs/a.rbs"));
        d.mod_name = "Some Mod".into();
        let i = SongInfo::of(&d, None);
        assert_eq!(i.mod_state, ModState::Missing);
        assert_eq!(i.name, "a.rbs");
        assert_eq!(i.folder.as_deref(), Some("/songs"));
        d.kit = Kit::Mod(PathBuf::from("/mods/some.rbm"));
        let i = SongInfo::of(&d, None);
        assert_eq!(i.mod_name, "Some Mod");
        assert_eq!(i.mod_state, ModState::Loaded);
    }
}
