//! The panel wired to the real-time player (task-045).
//!
//! [`Link`] owns the UI half of an [`rb_player::Player`] and sits between it and the
//! [`Panel`] model:
//!
//! - **Out.** The model's events (`Panel::events`) become the messages the original's panel
//!   sends ([`Msg`]: song msg 35 `{id, v}` and the device messages of msg 41, the senders listed
//!   in `docs/song_load.md` section 4.10), and each message becomes the player's [`Command`]
//!   that stands for it. The messages are kept in [`Link::log`] when asked, which is what the
//!   tests compare with the original's wire format.
//! - **In.** After each engine call the snapshot the audio callback publishes moves the panel:
//!   the transport (position, play, mode, loop, tempo), every control's live value (so the
//!   song's automation moves the knobs), the pattern each section selects, the running lights of
//!   the drum machines and the meters. The panel shows the engine's values, as the original's
//!   does (`docs/ui_spec.md` section 11).
//!
//! # Timing
//!
//! A command takes effect in the next engine call (`rb_player` crate doc), and the snapshot
//! shows it after that call. Until then the snapshot still has the old values, and taking them
//! would put the controls back: [`Link::sync`] takes the engine's values only from a snapshot
//! that is newer than the last commands it sent. A loaded song that waits for Play (the old one
//! plays on) is not followed: the panel shows the song the commands address, the newest, as the
//! original's panel does.
//!
//! # Patterns
//!
//! The panel shows the selected pattern of each section at once, and an edit goes to that
//! pattern (the original's views keep a current pattern, `docs/song_load.md` section 4.10). The
//! link keeps a copy of the four pattern arrays ([`Link::patterns`], from the song and the edits
//! it sends) to fill the 303 step editor, the drum step strips and the length and shuffle
//! displays when a pattern is selected.

use std::collections::{HashMap, VecDeque};
use std::sync::Arc;

use rb_engine::samples::SampleSet;
use rb_engine::song::{Device, Song};
use rb_engine::tables::Tables;
use rb_engine::tr909;
use rb_player::{Audio, CALL_FRAMES, Command, Player};

use crate::model::{Event, Panel, Step303};

/// The devices of the four pattern sections (rows).
pub const ROW_DEVICES: [Device; 4] = [
    Device::Tb303_1,
    Device::Tb303_2,
    Device::Tr808,
    Device::Tr909,
];

/// What the original's panel sends: a song message of the exe's `sub_423C98` family, without the
/// song and slot bytes.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Msg {
    /// Song msg 35 `{id, v}` (`docs/song_load.md` section 3): 0 song mode, 1 play, 2 record,
    /// 3 loop, 4 tempo (BPM * 1000), 8 locate (ticks), 12 loop start, 16 loop end, 20 shuffle.
    Global { id: u8, value: u32 },
    /// Song msg 41 `[device, sub, data...]`.
    Device { dev: u8, sub: u8, data: Vec<u8> },
}

impl Msg {
    /// The message's bytes after the song and slot bytes, as the exe writes them
    /// (`docs/song_load.md` sections 2 and 3): msg 35 is `[35, id, value]` with the value a
    /// big-endian u32 for the ids 4, 8, 12 and 16 and one byte for the others; msg 41 is
    /// `[41, dev, sub, data...]`.
    pub fn encode(&self) -> Vec<u8> {
        match self {
            Msg::Global { id, value } => {
                let mut b = vec![35, *id];
                if matches!(id, 4 | 8 | 12 | 16) {
                    b.extend(value.to_be_bytes());
                } else {
                    b.push(*value as u8);
                }
                b
            }
            Msg::Device { dev, sub, data } => {
                let mut b = vec![41, *dev, *sub];
                b.extend(data);
                b
            }
        }
    }
}

/// The `{ctrl, v}` sub of a device (`docs/song_load.md` section 4.10).
pub fn param_sub(d: Device) -> Option<u8> {
    Some(match d {
        Device::Tb303_1 | Device::Tb303_2 => 40,
        Device::Tr808 => 57,
        Device::Tr909 => 74,
        Device::Delay => 84,
        Device::Distortion => 92,
        Device::Compressor => 102,
        Device::Pcf => 114,
        Device::Mixer => return None,
    })
}

/// The pattern header message `{p, field, v}` of a pattern device (303 42, 808 59, 909 76); the
/// pattern (44, 61, 78), step byte (45, 62, 79) messages follow at +2 and +3.
fn header_sub(d: Device) -> u8 {
    match d {
        Device::Tb303_1 | Device::Tb303_2 => 42,
        Device::Tr808 => 59,
        _ => 76,
    }
}

/// Where a pattern array starts in the device's data block, the pattern length and the step
/// length (`docs/song_load.md` sections 4.6 to 4.8).
fn layout(row: usize) -> (usize, usize, usize) {
    match row {
        0 | 1 => (12, 98, 6),
        2 => (32, 194, 12),
        _ => (33, 194, 12),
    }
}

/// The index of a control's device and controller: the knobs, faders and switches of the 303s,
/// the 808, the 909 and the effects, and the pattern sections' on/off buttons (controller 0).
/// The controller numbers are those of `docs/song_load.md` section 5.
pub fn controls() -> HashMap<String, (Device, u8)> {
    let mut m = HashMap::new();
    let mut add = |id: String, d: Device, c: u8| {
        m.insert(id, (d, c));
    };
    for (k, d) in [(1, Device::Tb303_1), (2, Device::Tb303_2)] {
        for (n, c) in [
            ("tune", 2),
            ("cutoff", 3),
            ("reso", 4),
            ("envmod", 5),
            ("decay", 6),
            ("accent", 7),
            ("wave", 8),
        ] {
            add(format!("tb303_{k}.{n}"), d, c);
        }
    }
    for (n, c) in [
        ("ac_level", 2),
        ("bd_level", 3),
        ("bd_tone", 4),
        ("bd_decay", 5),
        ("sd_level", 6),
        ("sd_tone", 7),
        ("sd_snappy", 8),
        ("lt_level", 9),
        ("lt_tuning", 10),
        ("slot3_switch", 11),
        ("mt_level", 12),
        ("mt_tuning", 13),
        ("slot4_switch", 14),
        ("ht_level", 15),
        ("ht_tuning", 16),
        ("slot5_switch", 17),
        ("rs_level", 18),
        ("slot6_switch", 19),
        ("cp_level", 20),
        ("slot7_switch", 21),
        ("cb_level", 22),
        ("cy_level", 23),
        ("cy_tone", 24),
        ("cy_decay", 25),
        ("oh_level", 26),
        ("oh_decay", 27),
        ("ch_level", 28),
    ] {
        add(format!("tr808.{n}"), Device::Tr808, c);
    }
    for (n, c) in [
        ("ac_level", 2),
        ("bd_level", 3),
        ("bd_tune", 4),
        ("bd_attack", 5),
        ("bd_decay", 6),
        ("sd_level", 7),
        ("sd_tune", 8),
        ("sd_tone", 9),
        ("sd_snappy", 10),
        ("lt_level", 11),
        ("lt_tune", 12),
        ("lt_decay", 13),
        ("mt_level", 14),
        ("mt_tune", 15),
        ("mt_decay", 16),
        ("ht_level", 17),
        ("ht_tune", 18),
        ("ht_decay", 19),
        ("rs_level", 20),
        ("cp_level", 21),
        ("hh_level", 22),
        ("ch_decay", 23),
        ("oh_decay", 24),
        ("cc_level", 25),
        ("cc_tune", 26),
        ("rc_level", 27),
        ("rc_tune", 28),
        ("flam", 29),
    ] {
        add(format!("tr909.{n}"), Device::Tr909, c);
    }
    for (id, d, c) in [
        ("delay.on", Device::Delay, 0),
        ("delay.steps", Device::Delay, 1),
        ("delay.triplet", Device::Delay, 2),
        ("delay.feedback", Device::Delay, 3),
        ("delay.pan", Device::Delay, 4),
        ("dist.on", Device::Distortion, 0),
        ("dist.amount", Device::Distortion, 1),
        ("dist.shape", Device::Distortion, 2),
        ("comp.on", Device::Compressor, 0),
        ("comp.ratio", Device::Compressor, 1),
        ("comp.threshold", Device::Compressor, 2),
        ("pcf.on", Device::Pcf, 0),
        ("pcf.freq", Device::Pcf, 1),
        ("pcf.q", Device::Pcf, 2),
        ("pcf.amt", Device::Pcf, 3),
        ("pcf.pattern", Device::Pcf, 4),
        ("pcf.dec", Device::Pcf, 5),
        ("pcf.mode", Device::Pcf, 6),
    ] {
        add(id.to_string(), d, c);
    }
    for (r, d) in ROW_DEVICES.into_iter().enumerate() {
        add(format!("pattern{}.on", r + 1), d, 0);
    }
    m
}

/// The mixer message a control id sends: `Global(id)` is msg 145 `{id, v}`, `Channel(c, k)` is
/// msg 147 `{c, k, v}`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum MixerTarget {
    Global(u8),
    Channel(u8, u8),
}

pub fn mixer_target(id: &str) -> Option<MixerTarget> {
    match id {
        "master.fader" => return Some(MixerTarget::Global(0)),
        "mixer.comp_id" => return Some(MixerTarget::Global(1)),
        "mixer.pcf_id" => return Some(MixerTarget::Global(2)),
        _ => {}
    }
    let (sec, name) = id.split_once('.')?;
    let c = sec
        .strip_prefix("mixer")?
        .parse::<u8>()
        .ok()?
        .checked_sub(1)?;
    let k = match name {
        "mute" => 0,
        "level" => 1,
        "pan" => 2,
        "delay" => 3,
        "dist" => 4,
        _ => return None,
    };
    (c < 4).then_some(MixerTarget::Channel(c, k))
}

/// The meter state 0..=10 of a meter byte 0..=127: linear, rounded up so that any signal shows
/// [G: the scale was not read, `sub_424220`].
pub fn meter_state(byte: u8) -> u8 {
    ((byte as u32 * 10).div_ceil(127)).min(10) as u8
}

/// The panel's player link.
pub struct Link {
    player: Player,
    controls: HashMap<String, (Device, u8)>,
    /// The four pattern arrays (32 patterns each), as the engine holds them.
    patterns: [Vec<u8>; 4],
    /// The pattern each section edits: the selected one.
    row_pattern: [u8; 4],
    /// The pattern whose steps the model shows, per section.
    shown: [Option<u8>; 4],
    backlog: VecDeque<Command>,
    /// The engine's frame count when the last commands were queued.
    mark: u64,
    /// The messages sent so far, when turned on (`Some`).
    pub log: Option<Vec<Msg>>,
}

impl Link {
    /// A link to a new player with the drum samples `samples` and the audio half to run in the
    /// callback.
    pub fn new(tables: Arc<Tables>, samples: Option<Arc<SampleSet>>) -> (Link, Audio) {
        let (player, mut audio) = Player::new(tables, samples);
        // the new empty song's patterns, as its constructor made them
        let patterns = std::array::from_fn(|c| {
            audio.current_mut().frame.devices.channels[c]
                .patterns_mut()
                .map(|p| p.to_vec())
                .unwrap_or_default()
        });
        let link = Link {
            player,
            controls: controls(),
            patterns,
            row_pattern: [0; 4],
            shown: [None; 4],
            backlog: VecDeque::new(),
            mark: 0,
            log: None,
        };
        (link, audio)
    }

    /// The engine's values are taken only from a snapshot with more frames than this: the one
    /// after the first call that can have drained the commands sent last.
    pub fn adopts_after(&self) -> u64 {
        self.mark
    }

    pub fn player(&mut self) -> &mut Player {
        &mut self.player
    }

    /// The pattern arrays as the link knows them (the song's, with the panel's edits).
    pub fn patterns(&self) -> &[Vec<u8>; 4] {
        &self.patterns
    }

    // ---- loading ------------------------------------------------------------------------

    /// Opens `song`: builds it and queues it as the newest song, and resets the panel to its
    /// values (the data blocks as the pattern mode shows them: a song that is already playing
    /// keeps playing, and the new one starts when Play is pressed). The song's own drum samples
    /// are `samples`.
    pub fn load(&mut self, song: &Song, samples: Option<Arc<SampleSet>>, panel: &mut Panel) {
        // what the panel did before is sent first (the panel is replaced below)
        self.forward(panel);
        let engine = self.player.build(song, samples);
        self.backlog.push_back(Command::Load(engine));
        let g = &song.globals;
        *panel = Panel::new();
        let t = &mut panel.transport;
        t.tempo = g.tempo as i32;
        t.song_mode = g.song_mode != 0;
        t.looping = g.loop_on != 0;
        t.loop_start = g.loop_start as i32;
        t.loop_end = g.loop_end as i32;
        panel.adopt("transport.shuffle", g.shuffle as i32);
        let payloads: Vec<Vec<u8>> = Device::ALL
            .iter()
            .map(|&d| song.device_payload(d))
            .collect();
        for (r, d) in ROW_DEVICES.into_iter().enumerate() {
            let payload = &payloads[d as usize];
            let (base, plen, _) = layout(r);
            self.patterns[r] = payload[base..base + 32 * plen].to_vec();
            // a song file may hold a selection past 31; the arrays have 32 patterns
            let sel = payload[1].min(31);
            self.row_pattern[r] = sel;
            panel.rows[r].pattern = sel;
            panel.rows[r].bank = sel / 8;
        }
        self.shown = [None; 4];
        for (id, &(d, c)) in &self.controls {
            panel.adopt(id, payloads[d as usize][c as usize] as i32);
        }
        let m = &payloads[Device::Mixer as usize];
        panel.adopt("master.fader", m[0] as i32);
        panel.adopt("mixer.comp_id", m[1] as i32);
        panel.adopt("mixer.pcf_id", m[2] as i32);
        for c in 0..4 {
            for (k, name) in ["mute", "level", "pan", "delay", "dist"]
                .into_iter()
                .enumerate()
            {
                panel.adopt(&format!("mixer{}.{name}", c + 1), m[5 + 6 * c + k] as i32);
            }
        }
        self.fill_views(panel);
        self.flush_marked();
    }

    /// Fills the model's pattern views from the pattern arrays: the 303s' steps, the lengths and
    /// shuffle flags, and the drum step strips of a section whose selected pattern changed.
    fn fill_views(&mut self, panel: &mut Panel) {
        for r in 0..4 {
            let (_, plen, slen) = layout(r);
            let pat = &self.patterns[r];
            if self.shown[r].is_none() {
                for p in 0..32 {
                    let h = &pat[p * plen..];
                    panel.rows[r].shuffle[p] = h[0] != 0;
                    panel.rows[r].len[p] = h[1];
                    if r < 2 {
                        for s in 0..16 {
                            let b = &h[2 + slen * s..2 + slen * s + 6];
                            panel.synth[r].patterns[p][s] = Step303 {
                                note: b[0],
                                slide: b[1] != 0,
                                accent: b[2] != 0,
                                up: b[3] != 0,
                                down: b[4] != 0,
                                rest: b[5] != 0,
                            };
                        }
                    }
                }
            }
            let sel = panel.rows[r].pattern;
            if self.shown[r] != Some(sel) {
                self.shown[r] = Some(sel);
                self.row_pattern[r] = sel;
                let h = &pat[sel as usize * plen..];
                // only the drum rows have 12-byte steps; a 303 pattern is 98 bytes and the last
                // pattern (31) would run past the array
                if r >= 2 {
                    for s in 0..16 {
                        for i in 0..12 {
                            let b = h[2 + slen * s + i];
                            if r == 2 {
                                panel.steps808[i][s] = b != 0;
                            } else {
                                panel.steps909[i][s] = b;
                            }
                        }
                    }
                }
            }
        }
    }

    // ---- out ----------------------------------------------------------------------------

    /// The message(s) the original sends for a model event; updates the pattern arrays for the
    /// edits.
    pub fn messages(&mut self, e: &Event) -> Vec<Msg> {
        let dev = |d: Device, sub: u8, data: &[u8]| Msg::Device {
            dev: d.id(),
            sub,
            data: data.to_vec(),
        };
        match e {
            Event::Press { id } => match id.as_str() {
                "transport.play" => vec![Msg::Global { id: 1, value: 1 }],
                "transport.stop" => vec![Msg::Global { id: 1, value: 0 }],
                _ => vec![],
            },
            Event::Pattern { row, pattern } => {
                self.row_pattern[*row] = *pattern;
                let d = ROW_DEVICES[*row];
                vec![dev(d, param_sub(d).unwrap(), &[1, *pattern])]
            }
            Event::Set { id, value } => self.set_messages(id, *value),
            Event::Step808 {
                instrument,
                step,
                on,
            } => self.drum_step(2, *step, *instrument, *on as u8),
            Event::Step909 {
                instrument,
                step,
                value,
            } => self.drum_step(3, *step, *instrument, *value),
            Event::Step303 {
                synth,
                pattern,
                step,
                field,
                value,
            } => {
                let (_, plen, slen) = layout(*synth);
                self.patterns[*synth]
                    [plen * *pattern as usize + 2 + slen * step + *field as usize] = *value;
                vec![dev(
                    ROW_DEVICES[*synth],
                    header_sub(ROW_DEVICES[*synth]) + 3,
                    &[*pattern, *step as u8, *field, *value],
                )]
            }
            Event::Clear303 { synth, pattern } => {
                let (_, plen, _) = layout(*synth);
                let at = plen * *pattern as usize;
                let mut bytes = self.patterns[*synth][at..at + plen].to_vec();
                for s in 0..16 {
                    bytes[2 + 6 * s..8 + 6 * s].copy_from_slice(&[0, 0, 0, 0, 0, 1]);
                }
                self.patterns[*synth][at..at + plen].copy_from_slice(&bytes);
                let mut data = vec![*pattern];
                data.extend(bytes);
                vec![dev(
                    ROW_DEVICES[*synth],
                    header_sub(ROW_DEVICES[*synth]) + 2,
                    &data,
                )]
            }
        }
    }

    fn drum_step(&mut self, row: usize, step: usize, inst: usize, value: u8) -> Vec<Msg> {
        let p = self.row_pattern[row];
        let (_, plen, slen) = layout(row);
        self.patterns[row][plen * p as usize + 2 + slen * step + inst] = value;
        let d = ROW_DEVICES[row];
        vec![Msg::Device {
            dev: d.id(),
            sub: header_sub(d) + 3,
            data: vec![p, step as u8, inst as u8, value],
        }]
    }

    fn set_messages(&mut self, id: &str, value: i32) -> Vec<Msg> {
        let global = |id: u8| {
            vec![Msg::Global {
                id,
                value: value as u32,
            }]
        };
        match id {
            "transport.mode" => return global(0),
            "transport.record" => return global(2),
            "transport.loop" => return global(3),
            "transport.tempo" => return global(4),
            "transport.position" | "transport.bar" => return global(8),
            "transport.loop_start" => return global(12),
            "transport.loop_end" => return global(16),
            "transport.shuffle" => return global(20),
            _ => {}
        }
        if let Some(t) = mixer_target(id) {
            let data = match t {
                MixerTarget::Global(i) => vec![i, value as u8],
                MixerTarget::Channel(c, k) => vec![c, k, value as u8],
            };
            let sub = if matches!(t, MixerTarget::Global(_)) {
                145
            } else {
                147
            };
            return vec![Msg::Device {
                dev: Device::Mixer.id(),
                sub,
                data,
            }];
        }
        // the pattern section's Shuffle and Steps: the pattern header `{p, field, v}`
        if let Some((sec, name)) = id.split_once('.')
            && let Some(r) = sec
                .strip_prefix("pattern")
                .and_then(|n| n.parse::<usize>().ok())
                .and_then(|n| n.checked_sub(1))
                .filter(|&r| r < 4)
            && let Some(field) = match name {
                "shuffle" => Some(0u8),
                "steps" => Some(1),
                _ => None,
            }
        {
            let p = self.row_pattern[r];
            let (_, plen, _) = layout(r);
            self.patterns[r][plen * p as usize + field as usize] = value as u8;
            let d = ROW_DEVICES[r];
            return vec![Msg::Device {
                dev: d.id(),
                sub: header_sub(d),
                data: vec![p, field, value as u8],
            }];
        }
        if let Some(&(d, c)) = self.controls.get(id) {
            return vec![Msg::Device {
                dev: d.id(),
                sub: param_sub(d).unwrap(),
                data: vec![c, value as u8],
            }];
        }
        vec![]
    }

    /// The player's command for a message. Record (msg 35 id 2) has none yet (task-057).
    pub fn command(m: &Msg) -> Option<Command> {
        match m {
            Msg::Global { id, value } => Some(match id {
                0 => Command::SongMode(*value != 0),
                1 if *value != 0 => Command::Play,
                1 => Command::Stop,
                3 => Command::LoopOn(*value != 0),
                4 => Command::Tempo(*value),
                8 => Command::Locate(*value),
                12 => Command::LoopStart(*value),
                16 => Command::LoopEnd(*value),
                20 => Command::Shuffle(*value as u8),
                _ => return None,
            }),
            Msg::Device { dev, sub, data } => {
                let d = Device::from_id(*dev)?;
                let b = |i: usize| data.get(i).copied();
                if Some(*sub) == param_sub(d) {
                    return Some(Command::DeviceParam {
                        device: d,
                        ctrl: b(0)?,
                        value: b(1)?,
                    });
                }
                match (d, *sub) {
                    (Device::Mixer, 145) => Some(Command::MixerGlobal {
                        id: b(0)?,
                        value: b(1)?,
                    }),
                    (Device::Mixer, 147) => Some(Command::MixerChannel {
                        channel: b(0)?,
                        field: b(1)?,
                        value: b(2)?,
                    }),
                    (d, s) if d.id() >= 1 && d.id() <= 4 && s == header_sub(d) => {
                        Some(Command::PatternField {
                            device: d,
                            pattern: b(0)?,
                            field: b(1)?,
                            value: b(2)?,
                        })
                    }
                    (d, s) if d.id() >= 1 && d.id() <= 4 && s == header_sub(d) + 2 => {
                        Some(Command::Pattern {
                            device: d,
                            pattern: b(0)?,
                            bytes: data[1..].into(),
                        })
                    }
                    (d, s) if d.id() >= 1 && d.id() <= 4 && s == header_sub(d) + 3 => {
                        Some(Command::StepField {
                            device: d,
                            pattern: b(0)?,
                            step: b(1)?,
                            field: b(2)?,
                            value: b(3)?,
                        })
                    }
                    _ => None,
                }
            }
        }
    }

    fn flush(&mut self) {
        while let Some(c) = self.backlog.pop_front() {
            if let Err(c) = self.player.send(c) {
                self.backlog.push_front(c);
                break;
            }
        }
    }

    /// Sends the panel's pending events to the player.
    pub fn forward(&mut self, panel: &mut Panel) {
        let events = std::mem::take(&mut panel.events);
        if events.is_empty() && self.backlog.is_empty() {
            return;
        }
        for e in &events {
            for m in self.messages(e) {
                if let Some(c) = Self::command(&m) {
                    self.backlog.push_back(c);
                }
                if let Some(log) = &mut self.log {
                    log.push(m);
                }
            }
        }
        self.flush_marked();
    }

    /// Flushes the backlog and sets `mark` to the first snapshot that can show the commands:
    /// the call that has already started when they are pushed does not drain them, so the
    /// snapshot after the next call is the first (`frames` of the snapshot read before the push,
    /// plus one call).
    fn flush_marked(&mut self) {
        let before = self.player.snapshot().frames;
        self.flush();
        self.mark = before + CALL_FRAMES as u64;
    }

    // ---- in -----------------------------------------------------------------------------

    /// Takes the engine's state into the panel.
    pub fn follow(&mut self, panel: &mut Panel) {
        let s = *self.player.snapshot();
        // the meters and the running lights follow every call
        for c in 0..4 {
            panel.meters.insert(
                format!("mixer{}.meter", c + 1),
                meter_state(s.channel_meters[c]),
            );
        }
        panel
            .meters
            .insert("master.meter_l".into(), meter_state(s.master_meters[0]));
        panel
            .meters
            .insert("master.meter_r".into(), meter_state(s.master_meters[1]));
        let live = |d: Device, i: usize| s.live[d as usize].as_slice().get(i).copied();
        for (id, d, i) in [
            ("delay.meter", Device::Delay, 5),
            ("dist.meter", Device::Distortion, 3),
            ("pcf.meter", Device::Pcf, 7),
            ("comp.meter", Device::Compressor, 4),
        ] {
            panel
                .meters
                .insert(id.into(), meter_state(live(d, i).unwrap_or(0)));
        }
        // the gain reduction: the centre LED is "0", the reduction lights the ones to its left
        let n = ((live(Device::Compressor, 3).unwrap_or(0) as u32 * 4 + 63) / 127) as usize;
        panel.gr_leds = [0; 9];
        panel.gr_leds[4] = 1;
        for k in 1..=n.min(4) {
            panel.gr_leds[4 - k] = if k <= 2 { 2 } else { 3 };
        }
        let step = |d: Device, i: usize| {
            live(d, i)
                .filter(|&b| s.playing && b < 16)
                .map(|b| b as usize)
        };
        panel.playing_step303 = [step(Device::Tb303_1, 9), step(Device::Tb303_2, 9)];
        panel.playing_step808 = step(Device::Tr808, 29);
        panel.playing_step909 = step(Device::Tr909, 30);

        // the values only from a call that ran the commands sent, and not of a waiting song
        if s.frames > self.mark && !s.pending {
            self.follow_values(panel, &s, &live);
        }
        self.fill_views(panel);
    }

    fn follow_values(
        &mut self,
        panel: &mut Panel,
        s: &rb_player::Snapshot,
        live: &dyn Fn(Device, usize) -> Option<u8>,
    ) {
        let t = &mut panel.transport;
        t.position = s.tick;
        t.playing = s.playing;
        t.song_mode = s.song_mode;
        t.looping = s.loop_on;
        t.loop_start = s.loop_start as i32;
        t.loop_end = s.loop_end as i32;
        t.tempo = s.tempo as i32;
        if !t.song_mode {
            t.recording = false;
        }
        panel.adopt("transport.shuffle", s.shuffle as i32);
        for (id, &(d, c)) in &self.controls {
            let i = if d == Device::Tr909 {
                tr909::internal_index(c as usize)
            } else {
                c as usize
            };
            if let Some(v) = live(d, i) {
                panel.adopt(id, v as i32);
            }
        }
        for (r, d) in ROW_DEVICES.into_iter().enumerate() {
            if let Some(p) = live(d, 1) {
                panel.adopt_pattern(r, p.min(31));
                self.row_pattern[r] = panel.rows[r].pattern;
            }
        }
        let m = &s.mixer;
        panel.adopt("master.fader", m.master as i32);
        panel.adopt("mixer.comp_id", m.comp_id as i32);
        panel.adopt("mixer.pcf_id", m.pcf_id as i32);
        for (c, ch) in m.channels.iter().enumerate() {
            let n = c + 1;
            panel.adopt(&format!("mixer{n}.mute"), ch.enabled as i32);
            panel.adopt(&format!("mixer{n}.level"), ch.level as i32);
            panel.adopt(&format!("mixer{n}.pan"), ch.pan as i32);
            panel.adopt(&format!("mixer{n}.delay"), ch.send as i32);
            panel.adopt(&format!("mixer{n}.dist"), ch.dist as i32);
        }
    }

    /// One turn of the UI loop: sends what the panel did, then takes what the engine did.
    pub fn sync(&mut self, panel: &mut Panel) {
        self.forward(panel);
        if !self.backlog.is_empty() {
            self.flush_marked();
        }
        self.player.collect_garbage();
        self.follow(panel);
    }
}
