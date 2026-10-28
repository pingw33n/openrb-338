//! The song's metadata (`rb info`, and File > Info of the UI): what a song file says about
//! itself, read without rendering. One extraction serves both, so they never disagree.
//!
//! The facts of an RBS 4.x song come from its chunks (HEAD, GLOB, USRI). A ReBirth 1.x song has
//! no HEAD or GLOB: its globals are those the import makes (`rb_engine::load::convert_1x`) and
//! the song information is the title, text and web page at `0x3160` of the decoded song sysex
//! (docs/song_load.md 12.4); it has no copyright.

use std::path::{Path, PathBuf};

use rb_engine::song::{Song as EngineSong, TICKS_PER_BAR};
use rbs::rb1x::{Block, Kind, Song1x};
use serde_json::{Value, json};

use crate::Fail;
use crate::mods::{self, ModOpts, STANDARD};
use crate::render::{Parsed, read_song};

/// Where the song information starts in the decoded 1.x sysex (`sub_42FFEC`): the title (40
/// characters), the text (200, at +41) and the web page (100, at +242).
const RB1X_INFO: usize = 0x3160;

/// The facts of a song file.
#[derive(Clone, Debug, PartialEq)]
pub struct SongFacts {
    /// A ReBirth 1.x song (a MIDI file) rather than an RBS 4.x file.
    pub rb1x: bool,
    /// The format version of the file: for RBS, e.g. "4.1"; the "File version" of a 1.x file,
    /// "3.1".
    pub version: String,
    pub window_title: String,
    /// The more text, 0x0d (the file's newline) turned into `\n`.
    pub text: String,
    pub web_page: String,
    /// Empty for a 1.x song, which has none.
    pub copyright: String,
    pub mod_name: String,
    pub mod_ftp: String,
    pub mod_www: String,
    pub bpm: f64,
    pub song_mode: bool,
    pub looping: bool,
    /// The loop's first bar, counted from 0 as in `rb render --range`.
    pub loop_start_bar: u32,
    /// The loop's length in bars.
    pub loop_bars: u32,
    pub shuffle: u8,
    /// The vintage sound model rather than ReBirth 2.0's.
    pub vintage: bool,
    /// The tick of the latest track event over all tracks (768 ticks a bar); `None` for a song
    /// with no events. An .rbs song has no length field: the original runs to the end of bar
    /// 999 (tick 767231, docs/initial_investigation.md, 0x1001F484) unless the loop is on.
    pub last_event_tick: Option<u32>,
}

/// The text of a file with its newlines (0x0d, or CR LF) as `\n`.
fn newlines(text: &str) -> String {
    text.replace("\r\n", "\n").replace('\r', "\n")
}

/// The song information string of a 1.x song's block the way the original copies it
/// (`sub_465D8D`, called by `sub_42FFEC`): the bytes of `raw` from `off` up to the first zero or
/// until `max` bytes are copied, each 0x0d followed by a 0x0a that counts too and is dropped when
/// `max` is already reached. Latin-1.
fn usri_copy(raw: &[u8], off: usize, max: usize) -> String {
    let mut out = String::new();
    let mut n = 0;
    let mut i = off;
    while n < max {
        let c = raw.get(i).copied().unwrap_or(0);
        if c == 0 {
            break;
        }
        out.push(c as char);
        n += 1;
        if c == 0x0d && n < max {
            out.push('\n');
            n += 1;
        }
        i += 1;
    }
    out
}

/// The song information of a ReBirth 1.x song as the original's import stores it in the
/// document's USRI fields (`sub_42FFEC`, `sub_430BA0` 0x431039..0x431078).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Usri1x {
    /// At most 40 characters.
    pub title: String,
    /// At most 200 characters; each 0x0d is followed by a 0x0a.
    pub text: String,
    /// At most 100 characters.
    pub web_page: String,
    /// 0 or 1.
    pub show_on_open: u8,
}

impl Usri1x {
    /// Reads the information of a parsed 1.x song (it has its sysex block).
    pub fn of(song: &Song1x) -> Self {
        let block = song
            .tracks
            .iter()
            .flatten()
            .find_map(|e| match &e.kind {
                Kind::Sysex(d) => Block::decode(d).ok(),
                _ => None,
            })
            .expect("a parsed 1.x song has its sysex block");
        let b = block.bytes();
        Self {
            title: usri_copy(b, RB1X_INFO, 0x28),
            text: usri_copy(b, RB1X_INFO + 0x29, 0xC8),
            web_page: usri_copy(b, RB1X_INFO + 0xF2, 0x64),
            show_on_open: (b[RB1X_INFO + 0x157] != 0) as u8,
        }
    }
}

fn bars(ticks: u32) -> u32 {
    ticks / TICKS_PER_BAR
}

impl SongFacts {
    /// The facts of an RBS 4.x song.
    pub fn of_source(s: &rbs::Song) -> Self {
        Self {
            rb1x: false,
            version: s.head.version(),
            window_title: s.usri.window_title.clone(),
            text: newlines(&s.usri.text),
            web_page: s.usri.web_page.clone(),
            copyright: s.head.copyright.clone(),
            mod_name: s.glob.mod_name.clone(),
            mod_ftp: s.glob.mod_ftp.clone(),
            mod_www: s.glob.mod_www.clone(),
            bpm: s.glob.tempo as f64 / 1000.0,
            song_mode: s.glob.song_mode != 0,
            looping: s.glob.loop_on != 0,
            loop_start_bar: bars(s.glob.loop_start),
            loop_bars: bars(s.glob.loop_end.saturating_sub(s.glob.loop_start)),
            shuffle: s.glob.shuffle,
            vintage: s.glob.vintage != 0,
            // a file position is in 32nd notes: 24 ticks
            last_event_tick: s
                .tracks
                .iter()
                .flat_map(|t| &t.events)
                .map(|e| e.pos.saturating_mul(24))
                .max(),
        }
    }

    /// The file's format as `rb info` and File > Info print it: "RBS 4.2", or "ReBirth 1.x MIDI
    /// file (file version 3.1)".
    pub fn format(&self) -> String {
        if self.rb1x {
            format!("ReBirth 1.x MIDI file (file version {})", self.version)
        } else {
            format!("RBS {}", self.version)
        }
    }

    /// The 1-based bar of the latest track event, `None` for a song with no events.
    pub fn last_event_bar(&self) -> Option<u32> {
        self.last_event_tick.map(|t| bars(t) + 1)
    }

    /// The facts of a ReBirth 1.x song, with the engine song the import made of it.
    pub fn of_1x(song: &Song1x, engine: &EngineSong) -> Self {
        let info = Usri1x::of(song);
        let g = &engine.globals;
        Self {
            rb1x: true,
            version: format!("{}.{}", song.version.0, song.version.1),
            window_title: info.title,
            text: newlines(&info.text),
            web_page: info.web_page,
            copyright: String::new(),
            mod_name: engine.mod_name.clone(),
            mod_ftp: String::new(),
            mod_www: String::new(),
            bpm: g.tempo as f64 / 1000.0,
            song_mode: g.song_mode != 0,
            looping: g.loop_on != 0,
            loop_start_bar: bars(g.loop_start),
            loop_bars: bars(g.loop_end.saturating_sub(g.loop_start)),
            shuffle: g.shuffle,
            vintage: g.vintage != 0,
            last_event_tick: engine.tracks.iter().flatten().map(|e| e.tick).max(),
        }
    }

    /// The facts of a song file, with `rb render`'s errors for a file that cannot be read or is
    /// not a valid song.
    pub fn of_file(path: &Path) -> Result<Self, Fail> {
        let l = read_song(path)?;
        Ok(match &l.parsed {
            Parsed::Rbs(s) => Self::of_source(s),
            Parsed::Rb1x(s) => Self::of_1x(s, &l.song),
        })
    }
}

/// Where the song's mod stands.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ModResolution {
    /// "Standard ReBirth": the built-in kit.
    Builtin,
    /// The mod file of the mod folder that `rb render` would load.
    File(PathBuf),
    /// Not in the mod folder: `rb render` fails (or, with `--standard-kit`, uses the built-in kit).
    Missing,
    /// No mod folder was given (no `--mod-dir`, no `RB_MOD_DIR`).
    NotChecked,
}

/// Resolves the mod `mod_name` as `rb render` does ([`mods::find_in_dir`]) in the folder of
/// `opts` or `RB_MOD_DIR`; with neither, the folder is not looked for.
pub fn resolve_mod(opts: &ModOpts, mod_name: &str) -> Result<ModResolution, Fail> {
    if mod_name == STANDARD {
        return Ok(ModResolution::Builtin);
    }
    if !opts.mod_dir_given() {
        return Ok(ModResolution::NotChecked);
    }
    let dir = opts.mod_dir()?.expect("a given folder is returned");
    Ok(match mods::find_in_dir(&dir, mod_name)? {
        Some(p) => ModResolution::File(p),
        None => ModResolution::Missing,
    })
}

impl ModResolution {
    /// The key of `--json`'s `mod_resolution`.
    pub fn key(&self) -> &'static str {
        match self {
            ModResolution::Builtin => "builtin",
            ModResolution::File(_) => "file",
            ModResolution::Missing => "missing",
            ModResolution::NotChecked => "not_checked",
        }
    }

    fn text(&self) -> String {
        match self {
            ModResolution::Builtin => "built-in kit".into(),
            ModResolution::File(p) => format!("file {}", p.display()),
            ModResolution::Missing => "missing (not in the mod folder)".into(),
            ModResolution::NotChecked => "not checked (no --mod-dir)".into(),
        }
    }
}

/// The text with every control character but `\n` (ESC, the C0 and C1 ranges a Latin-1 song can
/// hold, DEL) written as `\xNN`, so that a song cannot send escape sequences to the terminal.
fn escape_controls(s: &str) -> String {
    let mut o = String::with_capacity(s.len());
    for c in s.chars() {
        if c != '\n' && c.is_control() {
            o.push_str(&format!("\\x{:02x}", c as u32));
        } else {
            o.push(c);
        }
    }
    o
}

/// `Label: value`, one line; a text of several lines goes below the label, indented by two
/// spaces; an empty value leaves the label alone.
fn field(out: &mut String, label: &str, value: &str) {
    let value = &escape_controls(value);
    if value.contains('\n') {
        out.push_str(label);
        out.push(':');
        for line in value.lines() {
            out.push('\n');
            if !line.is_empty() {
                out.push_str("  ");
                out.push_str(line);
            }
        }
    } else if value.is_empty() {
        out.push_str(label);
        out.push(':');
    } else {
        out.push_str(&format!("{label}: {value}"));
    }
    out.push('\n');
}

/// The text form of `rb info`: one `Label: value` per line.
pub fn to_text(file: &Path, f: &SongFacts, mod_res: &ModResolution) -> String {
    let mut o = String::new();
    field(&mut o, "File", &file.display().to_string());
    field(&mut o, "Format", &f.format());
    field(&mut o, "Title", &f.window_title);
    field(&mut o, "Text", &f.text);
    field(&mut o, "Web page", &f.web_page);
    field(&mut o, "Copyright", &f.copyright);
    field(&mut o, "Mod", &f.mod_name);
    field(&mut o, "Mod resolution", &mod_res.text());
    field(&mut o, "Mod web", &f.mod_www);
    field(&mut o, "Mod ftp", &f.mod_ftp);
    field(&mut o, "Tempo", &format!("{:.3} BPM", f.bpm));
    field(&mut o, "Shuffle", &f.shuffle.to_string());
    field(&mut o, "Mode", if f.song_mode { "song" } else { "pattern" });
    let lp = if f.looping { "on" } else { "off" };
    field(
        &mut o,
        "Loop",
        &format!("{lp}, start bar {}, {} bars", f.loop_start_bar, f.loop_bars),
    );
    field(
        &mut o,
        "Synth sound",
        if f.vintage { "vintage" } else { "ReBirth 2.0" },
    );
    field(
        &mut o,
        "Last event",
        &f.last_event_bar()
            .map_or("none".to_string(), |b| format!("bar {b}")),
    );
    o
}

/// The `--json` form of `rb info`. Keys (all always present): `file`, `format` ("rbs" or
/// "rb1x"), `version` (a string), `title`, `text` (newlines as `\n`), `web_page`, `copyright`,
/// `mod_name`, `mod_resolution` ("builtin", "file", "missing" or "not_checked"), `mod_file` (the
/// path, else null), `mod_web`, `mod_ftp`, `tempo_bpm` (number), `shuffle` (number), `mode`
/// ("song" or "pattern"), `loop_on` (bool), `loop_start_bar`, `loop_bars` (numbers), and
/// `synth_sound` ("vintage" or "rebirth2"), `last_event_bar` (1-based bar of the latest track
/// event, null when there are none) and `last_event_tick` (its tick, null likewise).
pub fn to_json(file: &Path, f: &SongFacts, mod_res: &ModResolution) -> Value {
    json!({
        "file": file.display().to_string(),
        "format": if f.rb1x { "rb1x" } else { "rbs" },
        "version": f.version,
        "title": f.window_title,
        "text": f.text,
        "web_page": f.web_page,
        "copyright": f.copyright,
        "mod_name": f.mod_name,
        "mod_resolution": mod_res.key(),
        "mod_file": match mod_res {
            ModResolution::File(p) => Value::String(p.display().to_string()),
            _ => Value::Null,
        },
        "mod_web": f.mod_www,
        "mod_ftp": f.mod_ftp,
        "tempo_bpm": f.bpm,
        "shuffle": f.shuffle,
        "mode": if f.song_mode { "song" } else { "pattern" },
        "loop_on": f.looping,
        "loop_start_bar": f.loop_start_bar,
        "loop_bars": f.loop_bars,
        "synth_sound": if f.vintage { "vintage" } else { "rebirth2" },
        "last_event_bar": f.last_event_bar(),
        "last_event_tick": f.last_event_tick,
    })
}

/// `rb info`: the facts of `file` as text or JSON.
pub fn run(file: &Path, opts: &ModOpts, json: bool) -> Result<String, Fail> {
    let facts = SongFacts::of_file(file)?;
    let res = resolve_mod(opts, &facts.mod_name)?;
    Ok(if json {
        let mut s = serde_json::to_string_pretty(&to_json(file, &facts, &res)).unwrap();
        s.push('\n');
        s
    } else {
        to_text(file, &facts, &res)
    })
}
