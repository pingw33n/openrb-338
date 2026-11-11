//! The `.rbsx` song: an RBS 4.2 song (`.rbs`) with one more chunk, `OGEN`, that keeps the
//! GENERATE menu's settings (`generate.rs`): each bass synth's scale and key note, the buttons
//! chosen in RANDOM, ALTER and MOVE, and which of those rows are green.
//!
//! The chunk sits directly under `RB40`, after the song's own chunks, where the `rbs` crate
//! keeps any chunk it does not know; so everything that reads songs (the engine, `orb`, File >
//! Info, the export) reads an `.rbsx` as the song it holds. The chunk is taken out of the song
//! when it is read, whatever the file's name, and written only into an `.rbsx`: an `.rbs` that
//! the app saves stays a plain RBS 4.2 song for ReBirth and other programs.
//!
//! The chunk's body is text, one `name=value` per line under a first line naming the format
//! and its version, so a later version can add settings and still read this one's. Names it
//! does not know are skipped; a scale or key note that no longer fits is left out.

use std::path::Path;

use eframe::egui::{
    Align, Color32, Context, Frame, Id, Key, Layout, Margin, Modal, Modifiers, RichText, Stroke,
};

use crate::app::App;
use crate::browser::{Browser, Mode};
use crate::palette::{INK, INK_DIM, SLATE_DARK, SLATE_LINE};
use crate::generate::Op;
use crate::random::{MOVES, PARAMS};
use crate::scale::{KEYS, SCALES, ScaleKeys};

/// The chunk's tag.
pub const TAG: [u8; 4] = *b"OGEN";
/// The first line of the chunk's body: the format and its version.
const HEADER: &str = "OpenRB-338x GENERATE 1";
/// The file extension of a song with the GENERATE settings.
pub const EXTENSION: &str = "rbsx";

/// What the GENERATE menu keeps of both synths (`App::gen_state`).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct GenState {
    pub scale_keys: ScaleKeys,
    /// Indexed by row (`Op`), synth, button.
    pub sel: [[[bool; 5]; 2]; 3],
    /// Indexed by row (`Op`), synth: the row's name is green.
    pub done: [[bool; 2]; 3],
}

impl GenState {
    /// Whether nothing is set: a song like that needs no chunk.
    pub fn is_empty(&self) -> bool {
        *self == Self::default()
    }
}

/// Whether `path` names an `.rbsx` song (any case).
pub fn is_rbsx(path: &Path) -> bool {
    path.extension()
        .and_then(|e| e.to_str())
        .is_some_and(|e| e.eq_ignore_ascii_case(EXTENSION))
}

fn op_name(op: Op) -> &'static str {
    match op {
        Op::Random => "random",
        Op::Alter => "alter",
        Op::Move => "move",
    }
}

/// The chunk's body for `state`.
pub fn encode(state: &GenState) -> Vec<u8> {
    let mut lines = vec![HEADER.to_string()];
    for synth in 0..2 {
        let c = state.scale_keys.get(synth);
        let n = synth + 1;
        lines.push(format!("baze{n}.scale={}", c.scale.map_or("", |s| SCALES[s].0)));
        lines.push(format!("baze{n}.key={}", c.key.map_or("", |k| KEYS[k])));
        for op in Op::ALL {
            let names = if op == Op::Move { MOVES } else { PARAMS };
            let on: Vec<&str> = (0..5)
                .filter(|&i| state.sel[op as usize][synth][i])
                .map(|i| names[i])
                .collect();
            lines.push(format!("baze{n}.{}={}", op_name(op), on.join(",")));
            let done = state.done[op as usize][synth] as u8;
            lines.push(format!("baze{n}.{}.done={done}", op_name(op)));
        }
    }
    let mut body = lines.join("\n").into_bytes();
    body.push(b'\n');
    // IFF chunks are an even number of bytes
    if body.len() % 2 == 1 {
        body.push(b'\n');
    }
    body
}

/// The settings of a chunk's body; None when it is not this format.
pub fn decode(body: &[u8]) -> Option<GenState> {
    let text = std::str::from_utf8(body).ok()?;
    let mut lines = text.lines();
    let header = lines.next()?.trim();
    let version: u32 = header.strip_prefix("OpenRB-338x GENERATE ")?.parse().ok()?;
    if version < 1 {
        return None;
    }
    let values: Vec<(&str, &str)> = lines
        .filter_map(|l| l.split_once('='))
        .map(|(k, v)| (k.trim(), v.trim()))
        .collect();
    let get = |name: &str| values.iter().find(|(k, _)| *k == name).map(|(_, v)| *v);
    let mut state = GenState::default();
    // BAZE 1 first: BAZE 2 is checked against it
    for synth in 0..2 {
        let n = synth + 1;
        let scale = get(&format!("baze{n}.scale"))
            .and_then(|v| SCALES.iter().position(|(name, _)| *name == v));
        let key = get(&format!("baze{n}.key")).and_then(|v| KEYS.iter().position(|k| *k == v));
        if let Some(s) = scale {
            state.scale_keys.set_scale(synth, s);
            if let Some(k) = key {
                state.scale_keys.set_key(synth, k);
            }
        }
        for op in Op::ALL {
            let names = if op == Op::Move { MOVES } else { PARAMS };
            if let Some(v) = get(&format!("baze{n}.{}", op_name(op))) {
                for item in v.split(',').map(str::trim) {
                    if let Some(i) = names.iter().position(|x| *x == item) {
                        state.sel[op as usize][synth][i] = true;
                        // MOVE has one button on at most: the first one named
                        if op == Op::Move {
                            break;
                        }
                    }
                }
            }
            state.done[op as usize][synth] =
                get(&format!("baze{n}.{}.done", op_name(op))) == Some("1");
        }
    }
    Some(state)
}

/// Takes the `OGEN` chunk out of `song` and reads it: the settings it kept, if it had one.
pub fn split(song: &mut rbs::Song) -> Option<GenState> {
    let at = song.unknown.iter().position(|u| u.tag == TAG)?;
    let chunk = song.unknown.remove(at);
    song.unknown.retain(|u| u.tag != TAG);
    decode(&chunk.body)
}

/// `song` with `state` in its `OGEN` chunk, after its own chunks; an empty state adds none.
pub fn attach(song: &rbs::Song, state: &GenState) -> rbs::Song {
    let mut out = song.clone();
    out.unknown.retain(|u| u.tag != TAG);
    if !state.is_empty() {
        out.unknown.push(rbs::Unknown {
            // after everything (the writer clamps it to the end)
            index: usize::MAX,
            tag: TAG,
            body: encode(state),
        });
    }
    out
}

/// The answers of Save's question for an `.rbs` song with GENERATE settings.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum FormatAnswer {
    /// Save As an `.rbsx` (the name and folder of the song, `.rbsx`), keeping the settings.
    Rbsx,
    /// Save the `.rbs` as it is, without the settings.
    Rbs,
    Cancel,
}

impl App {
    /// Whether Save's question about the `.rbsx` is open.
    pub fn format_prompting(&self) -> bool {
        self.format_prompt
    }

    /// Answers Save's question: an `.rbsx` opens Save As on the song's name with `.rbsx` (its
    /// overwrite check included), an `.rbs` saves the file as it is.
    pub fn format_answer(&mut self, answer: FormatAnswer) {
        if !std::mem::take(&mut self.format_prompt) {
            return;
        }
        let Some(path) = self.doc.path.clone() else {
            return;
        };
        match answer {
            FormatAnswer::Rbsx => {
                let name = path.with_extension(EXTENSION);
                let name = name.file_name().map(|n| n.to_string_lossy().into_owned());
                let mut b = Browser::new(Mode::SaveSong, &self.settings);
                if let Some(dir) = path.parent() {
                    b = b.with_dir(dir);
                }
                self.browser = Some(b.with_name(&name.unwrap_or_default()));
            }
            FormatAnswer::Rbs => self.save_to(&path),
            // a Save the unsaved-changes prompt started ends here
            FormatAnswer::Cancel => self.after_save = None,
        }
    }

    /// Draws Save's question and takes its keys: Enter or X saves an `.rbsx`, R the `.rbs`,
    /// Escape cancels.
    pub fn format_prompt_ui(&mut self, ctx: &Context) {
        if !self.format_prompt {
            return;
        }
        let key = |k: Key| ctx.input_mut(|i| i.consume_key(Modifiers::NONE, k));
        let mut answer = None;
        if key(Key::Enter) || key(Key::X) {
            answer = Some(FormatAnswer::Rbsx);
        } else if key(Key::R) {
            answer = Some(FormatAnswer::Rbs);
        } else if key(Key::Escape) {
            answer = Some(FormatAnswer::Cancel);
        }
        let frame = Frame::new()
            .fill(SLATE_DARK)
            .stroke(Stroke::new(1.0, SLATE_LINE))
            .inner_margin(Margin::same(12))
            .corner_radius(3.0);
        Modal::new(Id::unique("orb_rbsx_format"))
            .frame(frame)
            .backdrop_color(Color32::from_black_alpha(170))
            .show(ctx, |ui| {
                ui.set_width(360.0);
                ui.label(
                    RichText::new(format!("Save {} as .rbsx?", self.doc.name()))
                        .color(INK)
                        .strong()
                        .size(15.0),
                );
                ui.label(
                    RichText::new(
                        "The song has GENERATE settings. An .rbsx keeps them with the song; \
                         an .rbs stays a plain ReBirth song without them.",
                    )
                    .color(INK_DIM),
                );
                ui.add_space(8.0);
                ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                    if ui.button("Cancel").clicked() {
                        answer = Some(FormatAnswer::Cancel);
                    }
                    if ui.button("Save as .rbs").clicked() {
                        answer = Some(FormatAnswer::Rbs);
                    }
                    if ui.button("Save as .rbsx").clicked() {
                        answer = Some(FormatAnswer::Rbsx);
                    }
                });
            });
        if let Some(a) = answer {
            self.format_answer(a);
        }
    }

    /// The GENERATE menu's settings of both synths, as an `.rbsx` keeps them.
    pub fn gen_state(&self) -> GenState {
        GenState {
            scale_keys: self.scale_keys,
            sel: self.op_sel,
            done: self.op_done,
        }
    }

    /// Sets the GENERATE menu as a song kept it (an empty state for a song without one); the
    /// menu's open list and a row being chosen are let go.
    pub fn set_gen_state(&mut self, state: GenState) {
        self.scale_keys = state.scale_keys;
        self.op_sel = state.sel;
        self.op_done = state.done;
        self.generate_list = None;
        self.op_picking = None;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn s(name: &str) -> usize {
        SCALES.iter().position(|(n, _)| *n == name).unwrap()
    }

    fn k(name: &str) -> usize {
        KEYS.iter().position(|n| *n == name).unwrap()
    }

    fn sample() -> GenState {
        let mut st = GenState::default();
        st.scale_keys.set_scale(0, s("IONIAN"));
        st.scale_keys.set_key(0, k("C"));
        st.scale_keys.set_scale(1, s("DORIAN"));
        st.scale_keys.set_key(1, k("D"));
        st.sel[Op::Random as usize][0] = [false, false, true, true, true];
        st.sel[Op::Alter as usize][1] = [true, false, false, false, true];
        st.sel[Op::Move as usize][0] = [false, false, false, true, false];
        st.done[Op::Random as usize][0] = true;
        st.done[Op::Move as usize][0] = true;
        st
    }

    #[test]
    fn the_settings_come_back_as_they_were_written() {
        let st = sample();
        let body = encode(&st);
        assert_eq!(body.len() % 2, 0, "an even chunk");
        assert_eq!(decode(&body), Some(st));
        let text = String::from_utf8(body).unwrap();
        assert!(text.starts_with("OpenRB-338x GENERATE 1\n"));
        assert!(text.contains("baze1.scale=IONIAN\n"));
        assert!(text.contains("baze1.random=ACCENT,SLIDE,NOTE\n"));
        assert!(text.contains("baze1.move=+1\n"));
        assert_eq!(decode(&encode(&GenState::default())), Some(GenState::default()));
    }

    #[test]
    fn decoding_is_lenient_but_keeps_the_rules() {
        // unknown names are skipped, MOVE keeps one button, a BAZE 2 that does not fit BAZE 1
        // is left out, and something else than this format is no state
        let body = b"OpenRB-338x GENERATE 2\nfuture.thing=7\nbaze1.scale=IONIAN\nbaze1.key=C\n\
                     baze1.move=-2,+2\nbaze2.scale=BLUES?\nbaze2.key=D\n\
                     baze2.scale=HARM MINOR\n";
        let st = decode(body).unwrap();
        assert_eq!(st.scale_keys.get(0).mask(), Some(crate::scale::mask(s("IONIAN"), 0)));
        assert_eq!(st.sel[Op::Move as usize][0], [true, false, false, false, false]);
        assert_eq!(st.scale_keys.get(1), Default::default());
        assert_eq!(decode(b"something else"), None);
        assert_eq!(decode(&[0xff, 0xfe]), None);
    }

    #[test]
    fn an_rbs_song_carries_the_chunk_at_its_end_and_gives_it_back() {
        let song = rbs::Song::empty();
        let plain = song.write().unwrap();
        // nothing set: no chunk, the same bytes
        assert_eq!(attach(&song, &GenState::default()).write().unwrap(), plain);
        let with = attach(&song, &sample()).write().unwrap();
        assert!(with.len() > plain.len());
        // the song's own chunks come first, unchanged; the RB40 size grows
        assert_eq!(with[12..plain.len()], plain[12..]);
        let mut back = rbs::Song::parse(&with).unwrap();
        assert_eq!(split(&mut back), Some(sample()));
        // taken out again, the song writes as it was
        assert_eq!(back.write().unwrap(), plain);
        let mut none = rbs::Song::parse(&plain).unwrap();
        assert_eq!(split(&mut none), None);
    }

    #[test]
    fn the_rbsx_extension_is_known_in_any_case() {
        assert!(is_rbsx(Path::new("a/Song.rbsx")));
        assert!(is_rbsx(Path::new("Song.RBSX")));
        assert!(!is_rbsx(Path::new("Song.rbs")));
        assert!(!is_rbsx(Path::new("Song")));
    }
}
