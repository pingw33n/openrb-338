//! File > Info (task-095): the information of the current song in a read-only window. The
//! facts come from the document's song file as it was read ([`SongInfo::of`], no rendering, no
//! player); editing the song information is not done.

use eframe::egui::{self, Color32, Context, Key, Modifiers, RichText};
use rb_cli::info::SongFacts;
use rb_cli::mods::{Kit, STANDARD};

use crate::app::App;
use crate::document::Document;

const INK: Color32 = Color32::from_rgb(0xe9, 0xed, 0xf2);
const INK_DIM: Color32 = Color32::from_rgb(0x9a, 0xa7, 0xb8);

/// The id of the info window of an open (`App::info_serial`).
pub fn info_window_id(serial: u32) -> egui::Id {
    egui::Id::unique(("rb_song_info_window", serial))
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
    /// The format version of the file as read, "4.2".
    pub version: String,
    pub copyright: String,
    pub bpm: f64,
    pub song_mode: bool,
    pub looping: bool,
    pub shuffle: u8,
    pub vintage: bool,
}

impl SongInfo {
    /// The information of a document.
    pub fn of(doc: &Document) -> Self {
        // the facts are those of `rb info` (rb_cli::info)
        let f = SongFacts::of_source(&doc.source);
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
            version: f.version,
            copyright: f.copyright,
            bpm: f.bpm,
            song_mode: f.song_mode,
            looping: f.looping,
            shuffle: f.shuffle,
            vintage: f.vintage,
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

/// A web address as a link when it is one, else as plain text.
pub(crate) fn link(ui: &mut egui::Ui, text: &str) {
    match web_url(text) {
        Some(url) => {
            ui.hyperlink_to(text, url);
        }
        None => {
            ui.label(RichText::new(text).color(INK));
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
        let info = SongInfo::of(&self.doc);
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
            .default_width(380.0);
        window.show(ctx, |ui| {
            egui::Grid::new("rb_song_info_grid")
                .num_columns(2)
                .spacing([12.0, 4.0])
                .show(ui, |ui| {
                    let row = |ui: &mut egui::Ui, k: &str, v: &str| {
                        ui.label(RichText::new(k).color(INK_DIM));
                        ui.label(RichText::new(v).color(INK));
                        ui.end_row();
                    };
                    row(ui, "File", &info.name);
                    if let Some(f) = &info.folder {
                        row(ui, "Folder", f);
                    }
                    row(ui, "Title", &info.window_title);
                    ui.label(RichText::new("Text").color(INK_DIM));
                    ui.label(RichText::new(&info.text).color(INK));
                    ui.end_row();
                    ui.label(RichText::new("Web page").color(INK_DIM));
                    link(ui, &info.web_page);
                    ui.end_row();
                    row(
                        ui,
                        "Mod",
                        &format!("{} ({})", info.mod_name, info.mod_state.text()),
                    );
                    ui.label(RichText::new("Mod web").color(INK_DIM));
                    link(ui, &info.mod_www);
                    ui.end_row();
                    ui.label(RichText::new("Mod ftp").color(INK_DIM));
                    ui.label(RichText::new(&info.mod_ftp).color(INK));
                    ui.end_row();
                    row(ui, "Format version", &info.version);
                    row(ui, "Copyright", &info.copyright);
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
        let i = SongInfo::of(&doc());
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
    fn crlf_latin1_and_mod_addresses() {
        let mut d = doc();
        d.source.usri.text = "a\r\nb\u{e9}".into();
        d.source.glob.mod_www = "www.mod.org".into();
        d.source.glob.mod_ftp = "ftp://mod.org/x".into();
        let i = SongInfo::of(&d);
        assert_eq!(i.text, "a\nb\u{e9}");
        assert_eq!(i.mod_www, "www.mod.org");
        assert_eq!(i.mod_ftp, "ftp://mod.org/x");
        // a Latin-1 byte of the file reads as that character
        let mut s = rbs::Song::empty();
        s.usri.window_title = "caf\u{e9}".into();
        let back = rbs::Song::parse(&s.write().unwrap()).unwrap();
        assert_eq!(back.usri.window_title, "caf\u{e9}");
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
        let i = SongInfo::of(&d);
        assert_eq!(i.mod_state, ModState::Missing);
        assert_eq!(i.name, "a.rbs");
        assert_eq!(i.folder.as_deref(), Some("/songs"));
        d.kit = Kit::Mod(PathBuf::from("/mods/some.rbm"));
        let i = SongInfo::of(&d);
        assert_eq!(i.mod_name, "Some Mod");
        assert_eq!(i.mod_state, ModState::Loaded);
    }
}
