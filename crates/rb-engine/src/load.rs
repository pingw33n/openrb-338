//! Song load: from an `.rbs` model to the engine's [`Song`] and to the message stream
//! Rebirth.exe sends to ReBirth.dll, as traced in `docs/song_load.md`.
//!
//! [`convert`] gives the engine-side model. [`load`] also gives the msg-37 batches, whose
//! boundaries depend on the file's own event count, and [`load_messages`] the whole stream of
//! `docs/song_load.md` section 2 steps 5 and 6: msg 36, the nine msg-41 device blocks, the msg-37
//! batches of tracks 0..8, and msg 32. `tools/rbhost/rbsmsg.py` is the validated reference of the
//! same stream (rbhost renders from it match the app's exports byte for byte,
//! `docs/export_state.md` section 13).

use crate::song::*;
use std::fmt;

/// A song the exe would refuse, or a model that no file can produce.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum LoadError {
    /// HEAD major version other than 4 (the exe throws `XBadFormat`, 0x434F4E).
    Version { major: u8, minor: u8, revision: u8 },
    /// An `rbs::Song` edited into a shape a file cannot have (wrong pattern or parameter count).
    Model(String),
    /// A ReBirth 1.x song that the exe refuses, or one whose import has no defined result
    /// (`docs/song_load.md` section 12; OB-15, OB-16).
    Rb1x(String),
}

impl fmt::Display for LoadError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            LoadError::Version {
                major,
                minor,
                revision,
            } => write!(f, "unsupported file version {major}.{minor}.{revision}"),
            LoadError::Model(m) => write!(f, "malformed song model: {m}"),
            LoadError::Rb1x(m) => write!(f, "ReBirth 1.x song: {m}"),
        }
    }
}

impl std::error::Error for LoadError {}

/// The file version as the exe compares it (CVersion, `sub_47D768`): the key
/// `major << 16 | minor << 8 | revision` (`sub_47D9ED`, HEAD bytes 5..7).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct Version {
    major: u8,
    minor: u8,
    revision: u8,
}

impl Version {
    fn of(song: &rbs::Song) -> Version {
        Version {
            major: song.head.major,
            minor: song.head.minor,
            revision: song.head.revision,
        }
    }

    fn key(self) -> u32 {
        (self.major as u32) << 16 | (self.minor as u32) << 8 | self.revision as u32
    }

    /// `sub_47DA82` against 0x4DB938: exactly 4.0.0.
    fn is_4_0(self) -> bool {
        self.key() == 0x04_00_00
    }

    /// `sub_47DA82` against 0x4DB928: exactly 4.1.0.
    fn is_4_1(self) -> bool {
        self.key() == 0x04_01_00
    }

    /// `sub_47DAB5` against 0x4DB918: key >= 4.2.0 and major <= 4.
    fn at_least_4_2(self) -> bool {
        self.key() >= 0x04_02_00 && self.major <= 4
    }
}

/// How a TRAK chunk is read (`docs/song_load.md` section 2, version table).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TrackReader {
    /// `sub_436B6F`.
    Plain,
    /// `sub_436F5A`, the 808 track before 4.2: controllers 10, 13, 16 get `v/2 + 32`.
    Old808,
    /// `sub_437380`, the mixer track of a 4.0 file: the old controller numbering.
    Mixer40,
}

/// One msg-37 packet: `[track, u32 first, u32 last]`, then `[99, u32 tick, ctrl, value]` per
/// event; `first` and `last` count the track's accepted events before and after it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TrackBatch {
    pub track: u8,
    pub first: u32,
    pub last: u32,
    pub events: Vec<TrackEvent>,
}

impl TrackBatch {
    /// The msg 37 payload.
    pub fn payload(&self) -> Vec<u8> {
        let mut b = Vec::with_capacity(9 + 7 * self.events.len());
        b.push(self.track);
        b.extend(self.first.to_be_bytes());
        b.extend(self.last.to_be_bytes());
        for e in &self.events {
            b.push(99);
            b.extend(e.tick.to_be_bytes());
            b.extend([e.ctrl, e.value]);
        }
        b
    }
}

/// File events per msg-37 batch (`sub_467E70(1284, left)` in `sub_436B6F`).
pub const BATCH_EVENTS: usize = 1284;

/// The value range of controller `ctrl` on track `track`, or `None` when the controller is not
/// enabled for the track (`sub_42FCE5`, `sub_42FD00`; `docs/song_load.md` section 5).
pub fn control_range(track: u8, ctrl: u8) -> Option<(u8, u8)> {
    let full = Some((0, 127));
    match (track, ctrl) {
        (0, 1) => Some((0, 5)),
        (0, 2) => Some((0, 4)),
        (0, 6..=27) => match (ctrl - 6) % 6 {
            0..=2 => full,
            3 => Some((0, 1)),
            _ => None,
        },
        (1 | 2, 0 | 8) => Some((0, 1)),
        (1 | 2, 1) => Some((0, 31)),
        (1 | 2, 2..=7) => full,
        (3 | 4, 0) => Some((0, 1)),
        (3 | 4, 1) => Some((0, 31)),
        (3, 2..=28) | (4, 2..=29) => full,
        (5, 0 | 2) => Some((0, 1)),
        (5, 1) => Some((1, 32)),
        (5, 3 | 4) => full,
        (6 | 8, 0) => Some((0, 1)),
        (6 | 8, 1 | 2) => full,
        (7, 0 | 6) => Some((0, 1)),
        (7, 1..=5) => full,
        _ => None,
    }
}

/// The 808 tom tuning of files before 4.2 (`sub_432E1B`): `v/2 + 32`.
pub fn old_tom_tune(v: u8) -> u8 {
    v / 2 + 32
}

/// The tick check of `sub_42FD60`.
fn tick_valid(tick: u32) -> bool {
    tick < MAX_TICK && tick.is_multiple_of(TICKS_PER_POS)
}

/// The state of `sub_437380`'s flag conversion: the tick of the last compressor and PCF event
/// (`var_18`, `var_14`), compared signed.
#[derive(Default)]
struct Mixer40 {
    comp_last: i32,
    pcf_last: i32,
}

impl Mixer40 {
    /// Maps an old (4.0) mixer controller to the current numbering; `None` drops the event
    /// (`sub_437380`, switch at 0x43764E).
    fn convert(&mut self, tick: u32, ctrl: u8, value: u8) -> (u8, u8, bool) {
        let tick = tick as i32;
        let flag = |last: &mut i32, new_ctrl: u8, id: u8| {
            if value != 0 {
                *last = tick;
                (new_ctrl, id, true)
            } else if tick > *last {
                *last = tick;
                (new_ctrl, 0, true)
            } else {
                (new_ctrl, value, false)
            }
        };
        match ctrl {
            1 => flag(&mut self.comp_last, Mixer::CTRL_COMP_ID, 1),
            10 | 18 | 26 | 34 => flag(
                &mut self.comp_last,
                Mixer::CTRL_COMP_ID,
                (ctrl - 10) / 8 + 2,
            ),
            9 | 17 | 25 | 33 => flag(&mut self.pcf_last, Mixer::CTRL_PCF_ID, (ctrl - 9) / 8 + 1),
            4..=8 | 12..=16 | 20..=24 | 28..=32 => {
                let (c, k) = ((ctrl - 4) / 8, (ctrl - 4) % 8);
                (5 + 6 * c + k, value, true)
            }
            _ => (ctrl, value, true),
        }
    }
}

/// The msg-37 batches of one TRAK chunk, with the exe's validation (`docs/song_load.md`
/// section 5): the position is `abs += delta`, `tick = (abs * 192) >> 3` in u32 (= `abs * 24`);
/// an event with an invalid tick is dropped and does not advance `abs`; an event whose
/// controller is not enabled or whose value is out of range is dropped.
pub fn track_batches(track: u8, events: &[rbs::Event], reader: TrackReader) -> Vec<TrackBatch> {
    let mut out = Vec::new();
    let mut abs: u32 = 0;
    let mut prev_pos: u32 = 0;
    let mut accepted: u32 = 0;
    let mut mixer40 = Mixer40::default();
    for chunk in events.chunks(BATCH_EVENTS) {
        let first = accepted;
        let mut batch = Vec::new();
        for e in chunk {
            let delta = e.pos.wrapping_sub(prev_pos);
            prev_pos = e.pos;
            let cand = abs.wrapping_add(delta);
            let tick = cand.wrapping_mul(0xC0) >> 3;
            let (ctrl, mut value, mut keep) = match reader {
                TrackReader::Mixer40 => mixer40.convert(tick, e.ctrl, e.value),
                _ => (e.ctrl, e.value, true),
            };
            if reader == TrackReader::Old808 && matches!(ctrl, 10 | 13 | 16) {
                value = old_tom_tune(value);
            }
            if !tick_valid(tick) {
                continue;
            }
            abs = cand;
            match control_range(track, ctrl) {
                Some((lo, hi)) if (lo..=hi).contains(&value) => {}
                _ => keep = false,
            }
            if keep {
                batch.push(TrackEvent { tick, ctrl, value });
                accepted += 1;
            }
        }
        if accepted > first {
            out.push(TrackBatch {
                track,
                first,
                last: accepted,
                events: batch,
            });
        }
    }
    out
}

fn globals(song: &rbs::Song, v: Version) -> Globals {
    let g = &song.glob;
    Globals {
        song_mode: g.song_mode,
        play: 0,
        byte2: 0,
        loop_on: g.loop_on,
        tempo: g.tempo,
        locate: 0,
        loop_start: g.loop_start,
        loop_end: g.loop_end,
        shuffle: g.shuffle,
        vintage: if v.at_least_4_2() { g.vintage } else { 0 },
        byte22: 1,
    }
}

fn mixer(m: &rbs::Mixer, v: Version) -> Mixer {
    if v.is_4_0() {
        // 0x4352C0..0x43561B: 7-byte channels [?, level, pan, send, dist, PCF flag, comp flag].
        let raw = m.bytes();
        let mut out = Mixer {
            master: raw[0],
            comp_id: (raw[1] != 0) as u8,
            pcf_id: 0,
            channels: [MixerChannel::default(); 4],
        };
        for c in 0..4 {
            let ch = &raw[16 + 12 * c..16 + 12 * c + 7];
            out.channels[c] = MixerChannel {
                enabled: 1,
                level: ch[1],
                pan: ch[2],
                delay_send: ch[3],
                dist: ch[4],
            };
            if ch[5] != 0 {
                out.pcf_id = c as u8 + 1;
            }
            if ch[6] != 0 {
                out.comp_id = c as u8 + 2;
            }
        }
        return out;
    }
    let ch = |c: &rbs::MixChannel| MixerChannel {
        // 0x435708: a 4.1.0 file gets every channel enabled.
        enabled: if v.is_4_1() { 1 } else { c.enabled },
        level: c.level,
        pan: c.pan,
        delay_send: c.delay_send,
        dist: c.dist,
    };
    Mixer {
        master: m.master,
        comp_id: m.comp_id,
        pcf_id: m.pcf_id,
        channels: [
            ch(&m.channels[0]),
            ch(&m.channels[1]),
            ch(&m.channels[2]),
            ch(&m.channels[3]),
        ],
    }
}

fn params<const N: usize>(name: &str, values: &[u8], tail: &[u8]) -> Result<[u8; N], LoadError> {
    let mut v = values.to_vec();
    v.extend_from_slice(tail);
    v.try_into()
        .map_err(|v: Vec<u8>| LoadError::Model(format!("{name}: {} parameter bytes", v.len())))
}

fn thirty_two<T: Copy + Default, S>(
    name: &str,
    src: &[S],
    f: impl Fn(&S) -> T,
) -> Result<[T; 32], LoadError> {
    if src.len() != 32 {
        return Err(LoadError::Model(format!("{name}: {} patterns", src.len())));
    }
    Ok(std::array::from_fn(|i| f(&src[i])))
}

fn tb303(t: &rbs::Tb303) -> Result<Tb303, LoadError> {
    // sub_434309: each flag bit normalized to 0/1; the bits above 4 are dropped.
    let patterns = thirty_two("303", &t.patterns, |p| Pattern303 {
        shuffle: p.shuffle,
        length: p.length,
        steps: p.steps.map(|s| Step303 {
            note: s.note,
            slide: s.slide,
            accent: s.accent,
            up: s.up,
            down: s.down,
            rest: s.rest,
        }),
    })?;
    Ok(Tb303 {
        params: params("303", &t.params, &[0; 3])?,
        patterns,
    })
}

fn drum_patterns(
    name: &str,
    d: &rbs::Drums,
    step: impl Fn(u8) -> u8,
) -> Result<[DrumPattern; 32], LoadError> {
    thirty_two(name, &d.patterns, |p| DrumPattern {
        shuffle: p.shuffle,
        length: p.length,
        steps: p.steps.map(|s| s.map(&step)),
    })
}

fn tr808(d: &rbs::Drums, v: Version) -> Result<Tr808, LoadError> {
    let mut params: [u8; 32] = params("808", &d.params, &[0; 3])?;
    if !v.at_least_4_2() {
        // sub_432C58 (0x432DA2): the tom tunings of older files.
        for i in [10, 13, 16] {
            params[i] = old_tom_tune(params[i]);
        }
    }
    Ok(Tr808 {
        params,
        // The 808 steps are `raw != 0`.
        patterns: drum_patterns("808", d, |x| (x != 0) as u8)?,
    })
}

fn tr909(d: &rbs::Drums) -> Result<Tr909, LoadError> {
    let mut params: [u8; 33] = params("909", &d.params, &[0; 3])?;
    // 0x433558: the parse copies LowTomDecay (13) into MidTomDecay (16).
    params[16] = params[13];
    Ok(Tr909 {
        params,
        // The 909 steps pass unchanged (0..3).
        patterns: drum_patterns("909", d, |x| x)?,
    })
}

/// A converted song and its msg-37 batches per track.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Loaded {
    pub song: Song,
    pub batches: [Vec<TrackBatch>; 9],
    /// The order the exe sends the batches in: `(track, index into batches[track])`. Tracks
    /// 0..8 one after the other for an RBS file; for a 1.x song, as its track buffers fill.
    pub order: Vec<(usize, usize)>,
}

/// The batch order of an RBS load: every batch of track 0, then of track 1, and so on.
fn track_order(batches: &[Vec<TrackBatch>; 9]) -> Vec<(usize, usize)> {
    (0..9)
        .flat_map(|t| (0..batches[t].len()).map(move |i| (t, i)))
        .collect()
}

/// [`Song::batches`]: each batch of `order` as its track and event count.
fn send_plan(batches: &[Vec<TrackBatch>; 9], order: &[(usize, usize)]) -> Vec<Batch> {
    order
        .iter()
        .map(|&(t, i)| {
            let b = &batches[t][i];
            Batch {
                track: b.track,
                len: b.last - b.first,
            }
        })
        .collect()
}

/// Converts an `.rbs` song as Rebirth.exe loads it.
pub fn load(song: &rbs::Song) -> Result<Loaded, LoadError> {
    let v = Version::of(song);
    if v.major != 4 {
        return Err(LoadError::Version {
            major: v.major,
            minor: v.minor,
            revision: v.revision,
        });
    }
    let batches: [Vec<TrackBatch>; 9] = std::array::from_fn(|t| {
        let reader = match t {
            0 if v.is_4_0() => TrackReader::Mixer40,
            3 if !v.at_least_4_2() => TrackReader::Old808,
            _ => TrackReader::Plain,
        };
        track_batches(t as u8, &song.tracks[t].events, reader)
    });
    let tracks = std::array::from_fn(|t| {
        batches[t]
            .iter()
            .flat_map(|b| b.events.iter().copied())
            .collect()
    });
    let order = track_order(&batches);
    let out = Song {
        form: LoadForm::Rbs,
        mod_name: song.glob.mod_name.clone(),
        globals: globals(song, v),
        mixer: mixer(&song.mixer, v),
        delay: Delay {
            params: params("DELY", &song.delay.values, &[0])?,
        },
        pcf: Pcf {
            params: params("PCF", &song.pcf.values, &[0])?,
        },
        dist: Distortion {
            params: params("DIST", &song.dist.values, &[0])?,
        },
        comp: Compressor {
            params: params("COMP", &song.comp.values, &[127, 0])?,
        },
        tb303: [tb303(&song.tb303[0])?, tb303(&song.tb303[1])?],
        tr808: tr808(&song.tr808, v)?,
        tr909: tr909(&song.tr909)?,
        tracks,
        batches: send_plan(&batches, &order),
    };
    Ok(Loaded {
        song: out,
        batches,
        order,
    })
}

/// The engine-side model of an `.rbs` song.
pub fn convert(song: &rbs::Song) -> Result<Song, LoadError> {
    Ok(load(song)?.song)
}

/// Song message type of the top-level message 78.
pub const MSG_SONG: u8 = 78;
/// Song message: globals.
pub const MSG_GLOBALS: u8 = 36;
/// Song message: track events.
pub const MSG_TRACK: u8 = 37;
/// Song message: device data.
pub const MSG_DEVICE: u8 = 41;
/// Song message: relocate to the current position.
pub const MSG_RELOCATE: u8 = 32;

fn song_msg(slot: u8, msg: u8, payload: &[u8]) -> Vec<u8> {
    let mut b = vec![MSG_SONG, slot, msg];
    b.extend_from_slice(payload);
    b
}

impl Loaded {
    /// The messages the exe sends after the engine gave the song slot `slot` (msg 52):
    /// `[78, slot, msg, payload...]` each, in the order of `docs/song_load.md` section 2 (an RBS
    /// file) or section 12 (a 1.x song).
    pub fn messages(&self, slot: u8) -> Vec<Vec<u8>> {
        let mut out = vec![song_msg(slot, MSG_GLOBALS, &self.song.globals.payload())];
        for &dev in self.song.form.device_blocks() {
            let mut p = vec![dev.id(), dev.data_sub()];
            p.extend(self.song.device_payload(dev));
            out.push(song_msg(slot, MSG_DEVICE, &p));
        }
        if self.song.form == LoadForm::Rb1x {
            out.push(song_msg(slot, MSG_DEVICE, &rb1x::TR909_OFF));
        }
        for &(t, i) in &self.order {
            out.push(song_msg(slot, MSG_TRACK, &self.batches[t][i].payload()));
        }
        out.push(song_msg(slot, MSG_RELOCATE, &[]));
        out
    }
}

/// The whole load message stream of `song` for song slot `slot`.
pub fn load_messages(song: &rbs::Song, slot: u8) -> Result<Vec<Vec<u8>>, LoadError> {
    Ok(load(song)?.messages(slot))
}

mod rb1x;
pub use rb1x::{convert_1x, load_1x};

#[cfg(test)]
mod tests;
