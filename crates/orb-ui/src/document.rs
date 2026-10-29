//! The open song as a document (task-047): the song file it came from, where it is saved, whether
//! it has unsaved changes, the mod it names, and the way back from the panel's state to an `.rbs`
//! file.
//!
//! The original keeps the song in the engine and, to save, asks every device for its data block
//! (`sub_43920E` and its siblings `sub_438BCE`..`sub_43A1F4`, docs/song_load.md section 4.10) and
//! passes them to the RBS writer `sub_438025`. orb's engine holds the same blocks in the panel and
//! the link: [`compose`] takes the song file as it was read, and puts over it everything the
//! panel can change (the controllers, the pattern selection and on/off switches, the patterns,
//! the mixer, the transport's tempo, mode, loop and shuffle, the sound model and the mod name).
//! What the panel cannot change (the user text, unknown chunks, the unused bytes, and the track
//! events unless recording changed them) stays as read, so a song saved without an edit is the
//! file read, byte for byte, for a canonical 4.2 file. A song whose tracks recording changed
//! (task-057) is saved with the engine's tracks ([`Link::recorded_tracks`]), as the original
//! saves the tracks it reads back from the DLL.

use std::path::PathBuf;

use orb_cli::info::SongFacts;
use orb_cli::mods::{Kit, STANDARD};
use orb_engine::seq::Event as TrackEvent;
use orb_engine::song::{Device, Song as EngineSong};

use crate::link::{self, Link, ROW_DEVICES};
use crate::model::Panel;

/// A song that has been opened or made.
pub struct Document {
    /// The song file as read (or an empty song for a new one and a 1.x import).
    pub source: rbs::Song,
    /// The facts of a ReBirth 1.x song, for File > Info: `source` is only a stand-in for it
    /// (task-123). Dropped when the song is saved, which makes it an RBS 4.2 file.
    pub facts_1x: Option<SongFacts>,
    /// Where it is saved; `None` until the first Save As.
    pub path: Option<PathBuf>,
    /// Edited since it was opened or saved.
    pub modified: bool,
    /// The mod the song names (GLOB): "Standard ReBirth" or a mod's INFO menu name. A mod that
    /// was not found keeps its name here while the standard kit plays.
    pub mod_name: String,
    /// The kit that plays.
    pub kit: Kit,
}

impl Document {
    /// A new, empty song on the built-in kit.
    pub fn new() -> Self {
        Self {
            source: rbs::Song::empty(),
            facts_1x: None,
            path: None,
            modified: false,
            mod_name: STANDARD.into(),
            kit: Kit::Standard,
        }
    }

    /// The name shown in the window title and the prompts.
    pub fn name(&self) -> String {
        self.path
            .as_ref()
            .and_then(|p| p.file_name())
            .map_or_else(|| "Untitled".into(), |n| n.to_string_lossy().into_owned())
    }
}

impl Default for Document {
    fn default() -> Self {
        Self::new()
    }
}

/// The `rbs` song of a song loaded from a ReBirth 1.x file: the 1.x format has no `rbs::Song`,
/// so the empty song stands in, with the engine song's mod name and the song information (the
/// devices come from the panel when the song is saved). Its tracks are empty here: the import's
/// lists are in the order of the MIDI tracks, not of the ticks (several MIDI tracks feed the
/// mixer), and the document takes the tracks the engine holds after the load instead
/// ([`set_engine_tracks`] with [`Link::loaded_tracks`], `App::adopt_as`), as the original's save
/// writes them.
pub fn source_of_1x(engine: &EngineSong, song: &rbs::rb1x::Song1x) -> rbs::Song {
    let mut s = rbs::Song::empty();
    s.glob.mod_name = engine.mod_name.clone();
    // the import copies the song information into the document's USRI fields (`sub_430BA0`,
    // 0x431039..0x431078), so a save writes it (docs/song_load.md 12.6)
    let u = orb_cli::info::Usri1x::of(song);
    s.usri.window_title = u.title;
    s.usri.text = u.text;
    s.usri.web_page = u.web_page;
    s.usri.show_on_open = u.show_on_open;
    s
}

/// Puts the engine's tracks into `s`: the TRAK writer `sub_437CFF` reads each track back from
/// the DLL (song msg 22, 0x437DE6, in list order, `sub_10020EC0`) and writes each tick as
/// `8 * tick / 192` (0x437E9E) after the one before (0x437EA4). The DLL's lists are sorted by
/// tick (`sub_10014750`), so the positions never go back.
pub fn set_engine_tracks(s: &mut rbs::Song, tracks: &[Vec<TrackEvent>; 9]) {
    for (t, events) in tracks.iter().enumerate() {
        s.tracks[t].events = events
            .iter()
            .map(|e| rbs::Event {
                pos: e.tick as u32 / 24,
                ctrl: e.ctrl,
                value: e.value,
            })
            .collect();
    }
}

/// The track events the load accepted (converted, as an older file's values are), as the file
/// holds them: a position in 32nd notes is a tick over 24.
fn set_tracks(s: &mut rbs::Song, engine: &EngineSong) {
    for (t, events) in engine.tracks.iter().enumerate() {
        s.tracks[t].events = events
            .iter()
            .map(|e| rbs::Event {
                pos: e.tick / 24,
                ctrl: e.ctrl,
                value: e.value,
            })
            .collect();
    }
}

/// The song to write: `base` with the panel's state (`link` has the patterns) over it.
pub fn compose(base: &rbs::Song, panel: &Panel, link: &Link, mod_name: &str) -> rbs::Song {
    let mut s = base.clone();
    // What the load made of the file's bytes, for a file of the current version: a controller
    // the panel still shows as loaded keeps the byte of the file, so that the quirks of the
    // load (the 909's mid tom decay is copied from the low tom's, `docs/song_load.md` 4.8) do
    // not rewrite a byte the user did not touch. An older file is written with the values the
    // load made of it (a 4.1 file has every mixer channel enabled): it is a 4.2 file now.
    let current = (base.head.major, base.head.minor, base.head.revision) == (4, 2, 0);
    let engine = orb_engine::load::convert(base).ok();
    let loaded = engine.as_ref().filter(|_| current);
    if let (false, Some(e)) = (current, &engine) {
        // the events of an older file, as the load converted them (the 808's tom tunings)
        set_tracks(&mut s, e);
    }
    if let Some(tracks) = link.recorded_tracks() {
        // a recorded song: the tracks as the engine holds them, as the original's save does
        set_engine_tracks(&mut s, tracks);
    }
    s.head.major = 4;
    s.head.minor = 2;
    s.head.revision = 0;
    // sets a byte to the panel's value (when it has one) unless that is what the load made
    let put = |slot: &mut u8, new: Option<u8>, was: Option<u8>| {
        if let Some(n) = new
            && was != Some(n)
        {
            *slot = n;
        }
    };

    let t = &panel.transport;
    let g = &mut s.glob;
    g.song_mode = t.song_mode as u8;
    g.loop_on = t.looping as u8;
    g.tempo = t.tempo as u32;
    g.loop_start = t.loop_start as u32;
    g.loop_end = t.loop_end as u32;
    g.shuffle = panel.value("transport.shuffle").unwrap_or(g.shuffle);
    g.mod_name = mod_name.into();
    g.vintage = panel.vintage as u8;

    let lm = loaded.as_ref().map(|e| &e.mixer);
    put(
        &mut s.mixer.master,
        panel.value("master.fader"),
        lm.map(|m| m.master),
    );
    put(
        &mut s.mixer.comp_id,
        Some(panel.comp_id),
        lm.map(|m| m.comp_id),
    );
    put(
        &mut s.mixer.pcf_id,
        Some(panel.pcf_id),
        lm.map(|m| m.pcf_id),
    );
    for (c, ch) in s.mixer.channels.iter_mut().enumerate() {
        let n = c + 1;
        let get = |name: &str| panel.value(&format!("mixer{n}.{name}"));
        let was = lm.map(|m| m.channels[c]);
        put(&mut ch.enabled, get("mute"), was.map(|w| w.enabled));
        put(&mut ch.level, get("level"), was.map(|w| w.level));
        put(&mut ch.pan, get("pan"), was.map(|w| w.pan));
        put(&mut ch.delay_send, get("delay"), was.map(|w| w.delay_send));
        put(&mut ch.dist, get("dist"), was.map(|w| w.dist));
    }

    // The controllers: controller c of a device is parameter c of its block.
    let was = |d: Device, c: usize| -> Option<u8> {
        let e = loaded.as_ref()?;
        match d {
            Device::Tb303_1 => e.tb303[0].params.get(c),
            Device::Tb303_2 => e.tb303[1].params.get(c),
            Device::Tr808 => e.tr808.params.get(c),
            Device::Tr909 => e.tr909.params.get(c),
            Device::Delay => e.delay.params.get(c),
            Device::Pcf => e.pcf.params.get(c),
            Device::Distortion => e.dist.params.get(c),
            Device::Compressor => e.comp.params.get(c),
            Device::Mixer => None,
        }
        .copied()
    };
    for (id, (d, c)) in link::controls() {
        let (c, Some(v)) = (c as usize, panel.value(&id)) else {
            continue;
        };
        let slot = match d {
            Device::Tb303_1 => s.tb303[0].params.get_mut(c),
            Device::Tb303_2 => s.tb303[1].params.get_mut(c),
            Device::Tr808 => s.tr808.params.get_mut(c),
            Device::Tr909 => s.tr909.params.get_mut(c),
            Device::Delay => s.delay.values.get_mut(c),
            Device::Pcf => s.pcf.values.get_mut(c),
            Device::Distortion => s.dist.values.get_mut(c),
            Device::Compressor => s.comp.values.get_mut(c),
            Device::Mixer => None,
        };
        if let Some(slot) = slot {
            put(slot, Some(v), was(d, c));
        }
    }
    // The selected pattern of each pattern device (controller 1).
    for (r, d) in ROW_DEVICES.into_iter().enumerate() {
        let p = panel.rows[r].pattern;
        let slot = match d {
            Device::Tb303_1 => &mut s.tb303[0].params[1],
            Device::Tb303_2 => &mut s.tb303[1].params[1],
            Device::Tr808 => &mut s.tr808.params[1],
            _ => &mut s.tr909.params[1],
        };
        put(slot, Some(p), was(d, 1));
    }

    // The patterns: 98 bytes (a 303) or 194 (a drum machine) each, as the engine holds them.
    let pats = link.patterns();
    for (synth, bytes) in s.tb303.iter_mut().zip(pats) {
        for (p, pat) in synth.patterns.iter_mut().enumerate() {
            let b = &bytes[p * 98..(p + 1) * 98];
            pat.shuffle = b[0];
            pat.length = b[1];
            for (i, st) in pat.steps.iter_mut().enumerate() {
                let b = &b[2 + 6 * i..8 + 6 * i];
                // the flag bits 5..7 are not the engine's: they stay as read
                (st.note, st.slide, st.accent) = (b[0], b[1], b[2]);
                (st.up, st.down, st.rest) = (b[3], b[4], b[5]);
            }
        }
    }
    for (row, drums, is808) in [(2, &mut s.tr808, true), (3, &mut s.tr909, false)] {
        for (p, pat) in drums.patterns.iter_mut().enumerate() {
            let b = &pats[row][p * 194..(p + 1) * 194];
            pat.shuffle = b[0];
            pat.length = b[1];
            for (i, st) in pat.steps.iter_mut().enumerate() {
                for (j, v) in st.iter_mut().enumerate() {
                    let new = b[2 + 12 * i + j];
                    // the 808 stores a step as `raw != 0`: a nonzero byte read stays as read
                    if !is808 || (*v != 0) != (new != 0) {
                        *v = new;
                    }
                }
            }
        }
    }
    s
}
