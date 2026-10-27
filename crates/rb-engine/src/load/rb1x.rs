//! The import of a ReBirth 1.x song (a MIDI file), as Rebirth.exe 2.01 does it in `sub_430BA0`
//! (`docs/song_load.md` section 12): the song sysex gives msg 36 and the msg-41 blocks
//! (`sub_431747`), the controller tracks give the msg-37 events through nine track buffers
//! (`sub_4300E0`), with no validation. `tools/rb1x.py` is the Python twin.

use super::{BATCH_EVENTS, LoadError, Loaded, TrackBatch};
use crate::song::*;
use rbs::rb1x::{Block, Kind, Song1x};

/// The 909's msg-41 `4/74 {0, 0}` (0x431747's last device message): live enabled = 0.
pub(super) const TR909_OFF: [u8; 4] = [4, 74, 0, 0];

/// The oldest file version the exe imports: `CVersion(3, 1)` at 0x4DAD60, compared with
/// `sub_47DA1C` (0x425E01: file >= 3.1).
pub const MIN_VERSION: (u32, u32) = (3, 1);

/// 0x431920 and 0x430EFF: the PCF's channel 0, 1, 2 (303 #1, 303 #2, 808) is PCF id 1, 2, 3; 3
/// is off; any other value leaves the id at 0 (the mixer struct's default, `sub_4233F1`).
fn pcf_target(v: u8) -> u8 {
    match v {
        0..=2 => v + 1,
        _ => 0,
    }
}

/// `sub_43132C`: the 808 tom tunings of a 1.x song, `v/2 + 64`.
pub fn tom_tune(v: u8) -> u8 {
    v / 2 + 64
}

/// `sub_43067F` (the device) and `sub_430847` (the controller) of control change `cc` on MIDI
/// track `track`; `None` where the exe's switch gives -1.
pub fn map_cc(track: usize, cc: u8) -> Option<(Device, u8)> {
    let dev = match track {
        2 => Device::Tb303_1,
        3 => Device::Tb303_2,
        4 => Device::Tr808,
        5 => {
            return match cc {
                16 => Some((Device::Distortion, 1)),
                18 => Some((Device::Distortion, 0)),
                19 => Some((Device::Delay, 1)),
                20 => Some((Device::Delay, 3)),
                21 => Some((Device::Delay, 4)),
                22 => Some((Device::Delay, 0)),
                23 => Some((Device::Delay, 2)),
                24..=29 => Some((Device::Pcf, cc - 23)),
                30 => Some((Device::Pcf, 0)),
                _ => None,
            };
        }
        _ => return None,
    };
    let c = 6 * (track as u8 - 2);
    let last = if dev == Device::Tr808 { 42 } else { 22 };
    match cc {
        7 => Some((Device::Mixer, c + 6)),
        10 => Some((Device::Mixer, c + 7)),
        12 => Some((Device::Mixer, c + 8)),
        14 => Some((dev, 1)),
        15 => Some((dev, 0)),
        16.. if cc <= last => Some((dev, cc - 14)),
        _ => None,
    }
}

/// The nine track buffers of `sub_430BA0` (`sub_43005E`): `sub_4300E0` appends an event and
/// sends the buffer as one msg 37 (`sub_430163`) when it holds 1284; the rest go out at the end
/// (`sub_43035E`, tracks 0..8).
struct Buffers {
    pending: [Vec<TrackEvent>; 9],
    batches: [Vec<TrackBatch>; 9],
    sent: [u32; 9],
    order: Vec<(usize, usize)>,
}

impl Buffers {
    fn new() -> Self {
        Buffers {
            pending: Default::default(),
            batches: Default::default(),
            sent: [0; 9],
            order: Vec::new(),
        }
    }

    fn add(&mut self, track: Device, tick: u32, ctrl: u8, value: u8) {
        let t = track.id() as usize;
        self.pending[t].push(TrackEvent { tick, ctrl, value });
        if self.pending[t].len() == BATCH_EVENTS {
            self.flush(t);
        }
    }

    fn flush(&mut self, t: usize) {
        if self.pending[t].is_empty() {
            return;
        }
        let events = std::mem::take(&mut self.pending[t]);
        let first = self.sent[t];
        self.sent[t] += events.len() as u32;
        self.order.push((t, self.batches[t].len()));
        self.batches[t].push(TrackBatch {
            track: t as u8,
            first,
            last: self.sent[t],
            events,
        });
    }
}

/// The 16 pattern lengths bytes of a device record (`rec[7..23]`): pattern 2k is the high
/// nibble + 1 (`sub_466B80`), 2k + 1 the low nibble + 1 (`sub_466B93`).
fn lengths(rec: &[u8]) -> [u8; 32] {
    std::array::from_fn(|p| {
        let b = rec[7 + p / 2];
        if p % 2 == 0 { b >> 4 } else { b & 15 }.wrapping_add(1)
    })
}

/// The device record of device j (0 303 #1, 1 303 #2, 2 808): 32 bytes at `0x100 + 32j`.
fn record(b: &[u8], j: usize) -> &[u8] {
    &b[0x100 + 32 * j..0x120 + 32 * j]
}

/// The pattern area of device j: 4096 bytes at `352 + 4096j`.
fn area(b: &[u8], j: usize) -> &[u8] {
    &b[352 + 4096 * j..352 + 4096 * (j + 1)]
}

fn globals(blk: &Block) -> Globals {
    let b = blk.bytes();
    let start = blk.u32_at(10);
    let len = blk.u32_at(14);
    Globals {
        song_mode: (b[0x12] != 0) as u8,
        play: 0,
        byte2: 0,
        loop_on: (b[0x2E] != 0) as u8,
        tempo: blk.u32_at(0x2A),
        locate: 0,
        // imul ecx, 300h (0x431855, 0x431861)
        loop_start: start.wrapping_mul(768),
        loop_end: start.wrapping_add(len).wrapping_mul(768),
        // sub_420A81's default
        shuffle: 64,
        // mov [ebp+var_2B], 1 (0x43186A): the Vintage sound model
        vintage: 1,
        byte22: 1,
    }
}

fn mixer(b: &[u8]) -> Mixer {
    let ch = |j: usize| {
        let r = record(b, j);
        MixerChannel {
            enabled: r[0x17],
            level: r[2],
            pan: r[3],
            delay_send: r[4],
            dist: (b[0x37] as usize == j) as u8,
        }
    };
    Mixer {
        master: b[9],
        comp_id: 0,
        pcf_id: pcf_target(b[0x40]),
        // channel 3 (the 909): sub_423560's defaults
        channels: [
            ch(0),
            ch(1),
            ch(2),
            MixerChannel {
                enabled: 1,
                level: 100,
                pan: 64,
                delay_send: 0,
                dist: 0,
            },
        ],
    }
}

fn tb303(b: &[u8], j: usize) -> Tb303 {
    let rec = record(b, j);
    let p = area(b, j);
    let mut params = [0u8; 12];
    params[0] = (rec[5] == 0) as u8;
    params[1] = rec[1];
    params[2..9].copy_from_slice(&p[16..23]);
    let len = lengths(rec);
    let patterns = std::array::from_fn(|pat| Pattern303 {
        shuffle: 0,
        length: len[pat],
        steps: std::array::from_fn(|s| {
            let st = &p[30 + 68 * pat + 4 * s..34 + 68 * pat + 4 * s];
            let fl = st[2];
            let (mut note, mut up) = (st[0], fl >> 2 & 1);
            if note > 12 {
                up = 1;
                note -= 12;
            }
            Step303 {
                note,
                slide: fl & 1,
                accent: fl >> 1 & 1,
                up,
                down: fl >> 3 & 1,
                rest: st[3],
            }
        }),
    });
    Tb303 { params, patterns }
}

fn tr808(b: &[u8]) -> Tr808 {
    let rec = record(b, 2);
    let p = area(b, 2);
    let mut params = [0u8; 32];
    params[0] = (rec[5] == 0) as u8;
    params[1] = rec[1];
    params[2..29].copy_from_slice(&p[16..43]);
    for i in [10, 13, 16] {
        params[i] = tom_tune(p[i + 14]);
    }
    let len = lengths(rec);
    let patterns = std::array::from_fn(|pat| {
        // sub_43285C: four 2-bit values per byte, the high ones first
        let src = &p[48 + 64 * pat..112 + 64 * pat];
        let v: Vec<u8> = src
            .iter()
            .flat_map(|&x| [x >> 6 & 3, x >> 4 & 3, x >> 2 & 3, x & 3])
            .collect();
        DrumPattern {
            shuffle: 0,
            length: len[pat],
            steps: std::array::from_fn(|s| std::array::from_fn(|i| v[s + 16 * i])),
        }
    });
    Tr808 { params, patterns }
}

/// The 909 the song ctor leaves (`crate::tr909::Tr909::new`): the defaults, every pattern 16
/// empty steps. The import sends no 909 block.
fn tr909_ctor() -> Tr909 {
    Tr909 {
        params: crate::tr909::DEFAULT_PARAMS,
        patterns: [DrumPattern {
            shuffle: 0,
            length: 16,
            steps: [[0; 12]; 16],
        }; 32],
    }
}

/// Imports a 1.x song as `sub_430BA0` does.
pub fn load_1x(song: &Song1x) -> Result<Loaded, LoadError> {
    let bad = |m: String| Err(LoadError::Rb1x(m));
    if song.format != 1 {
        // 0x430CBC
        return bad(format!(
            "MIDI format {} (the exe imports format 1)",
            song.format
        ));
    }
    if song.version < MIN_VERSION {
        return bad(format!(
            "file version {}.{} (the exe imports 3.1 and later)",
            song.version.0, song.version.1
        ));
    }
    let mut buf = Buffers::new();
    // 0x430CFA: the 909 off at tick 0
    buf.add(Device::Tr909, 0, 0, 0);
    let mut block = None;
    for (ti, track) in song.tracks.iter().enumerate().skip(1) {
        for e in track {
            // 0x430D72: the division is forced to 8, so tick = 192 / 8 * time (imul, u32)
            let tick = e.time.wrapping_mul(24);
            match &e.kind {
                &Kind::Cc { ctrl: 17, value } if ti == 5 => {
                    // 0x430E47: the distortion's channel
                    buf.add(Device::Mixer, tick, 9, (value == 0) as u8);
                    buf.add(Device::Mixer, tick, 15, (value == 1) as u8);
                    buf.add(Device::Mixer, tick, 21, (value == 2) as u8);
                    buf.add(Device::Mixer, tick, 27, 0);
                }
                &Kind::Cc { ctrl: 31, value } if ti == 5 => {
                    // 0x430ED2: the PCF's channel
                    buf.add(Device::Mixer, tick, 2, pcf_target(value));
                }
                &Kind::Cc { ctrl, value } => {
                    let Some((dev, c)) = map_cc(ti, ctrl) else {
                        // 0x430FEF reads var_3C[255]: no defined behavior
                        return bad(format!(
                            "MIDI track {ti}: controller {ctrl} has no mapping (the exe would \
                             index its track buffers with -1)"
                        ));
                    };
                    let v = if dev == Device::Tr808 && matches!(c, 10 | 13 | 16) {
                        tom_tune(value)
                    } else {
                        value
                    };
                    buf.add(dev, tick, c, v);
                }
                Kind::Sysex(data) => {
                    // a sysex that is not the song's makes the exe skip its length again
                    // (0x4304C5's else branch), which this model does not follow
                    let b = Block::decode(data).map_err(|e| LoadError::Rb1x(e.to_string()))?;
                    if block.is_some() {
                        return bad("a second song sysex".into());
                    }
                    block = Some(b);
                }
                // other events are skipped (sub_4A0D77); FF 2F ends the track
                _ => {}
            }
        }
    }
    let Some(block) = block else {
        return bad("no song sysex".into());
    };
    for t in 0..9 {
        buf.flush(t);
    }
    let b = &block.bytes()[..];
    let tracks = std::array::from_fn(|t| {
        buf.batches[t]
            .iter()
            .flat_map(|x| x.events.iter().copied())
            .collect()
    });
    let out = Song {
        form: LoadForm::Rb1x,
        // 0x425E17: the built-in mod
        mod_name: "Standard ReBirth".into(),
        globals: globals(&block),
        mixer: mixer(b),
        delay: Delay {
            params: [
                b[0x35],
                b[0x2F] & 0x7F,
                (b[0x2F] & 0x80 != 0) as u8,
                b[0x33],
                b[0x34],
                0,
            ],
        },
        pcf: Pcf {
            params: [
                b[0x3F], b[0x39], b[0x3A], b[0x3B], b[0x3C], b[0x3D], b[0x3E], 0,
            ],
        },
        dist: Distortion {
            params: [b[0x38], b[0x36], 0, 0],
        },
        // no compressor block: the ctor's (= the exe's defaults, docs/song_load.md 4.9)
        comp: Compressor {
            params: crate::comp::DEFAULTS,
        },
        tb303: [tb303(b, 0), tb303(b, 1)],
        tr808: tr808(b),
        tr909: tr909_ctor(),
        tracks,
    };
    Ok(Loaded {
        song: out,
        batches: buf.batches,
        order: buf.order,
    })
}

/// The engine-side model of a 1.x song.
pub fn convert_1x(song: &Song1x) -> Result<Song, LoadError> {
    Ok(load_1x(song)?.song)
}

#[cfg(test)]
mod tests {
    use super::*;
    use rbs::rb1x::{BLOCK_LEN, Event};

    fn cc(time: u32, ctrl: u8, value: u8) -> Event {
        Event {
            time,
            kind: Kind::Cc { ctrl, value },
        }
    }

    fn block() -> Block {
        let mut b = Box::new([0u8; BLOCK_LEN]);
        b[..5].copy_from_slice(&[0xBC, 3, 1, 0, 0]);
        b[5..9].copy_from_slice(b"TinY");
        b[9] = 90; // master
        b[10..14].copy_from_slice(&2u32.to_be_bytes()); // loop start bar
        b[14..18].copy_from_slice(&4u32.to_be_bytes()); // loop length
        b[0x12] = 1; // song mode
        b[0x2A..0x2E].copy_from_slice(&128_000u32.to_be_bytes());
        b[0x2E] = 1;
        b[0x2F] = 0x83; // delay: 3 steps, sixteenths
        b[0x33] = 70;
        b[0x34] = 60;
        b[0x35] = 1;
        b[0x36] = 50; // dist amount
        b[0x37] = 1; // dist on 303 #2
        b[0x38] = 1;
        b[0x39..0x3F].copy_from_slice(&[11, 12, 13, 14, 15, 1]);
        b[0x3F] = 1;
        b[0x40] = 2; // PCF on the 808
        for j in 0..3 {
            let r = 0x100 + 32 * j;
            b[r + 1] = j as u8 + 4; // pattern
            b[r + 2..r + 5].copy_from_slice(&[100, 64, 20]);
            b[r + 5] = (j == 1) as u8; // 303 #2 muted
            b[r + 7] = 0x3F; // lengths 4, 16
            b[r + 0x17] = 1;
            let p = 352 + 4096 * j;
            b[p + 16..p + 43].copy_from_slice(&std::array::from_fn::<u8, 27, _>(|i| i as u8 * 4));
        }
        // 303 #1 pattern 0 step 0: note 13 (an octave up), slide + down; step 1 a rest
        b[352 + 30..352 + 34].copy_from_slice(&[13, 0, 0b1001, 0]);
        b[352 + 34..352 + 38].copy_from_slice(&[3, 0, 0b0110, 1]);
        // 808 pattern 0: bytes 0x80 (AC step 0), then 0x40 at byte 4 (BD step 1)
        b[352 + 8192 + 48] = 0x40;
        b[352 + 8192 + 48 + 4] = 0x10;
        Block(b)
    }

    fn song(tracks: Vec<Vec<Event>>) -> Song1x {
        let mut all = vec![
            vec![Event {
                time: 0,
                kind: Kind::Meta {
                    kind: 2,
                    data: b"File version 3.1.0".to_vec(),
                },
            }],
            vec![Event {
                time: 0,
                kind: Kind::Sysex(block().encode()),
            }],
        ];
        all.extend(tracks);
        Song1x::parse(
            &Song1x {
                format: 1,
                division: 32,
                tracks: all,
                version: (3, 1),
            }
            .write(),
        )
        .unwrap()
    }

    #[test]
    fn block_fields() {
        let s = convert_1x(&song(vec![])).unwrap();
        let g = &s.globals;
        assert_eq!(
            (g.song_mode, g.loop_on, g.tempo, g.loop_start, g.loop_end),
            (1, 1, 128_000, 1536, 4608)
        );
        assert_eq!((g.shuffle, g.vintage, g.byte22), (64, 1, 1));
        assert_eq!(s.mixer.payload()[..5], [90, 0, 3, 0, 0]);
        assert_eq!(s.mixer.channels[1].dist, 1);
        assert_eq!(s.mixer.channels[0].dist, 0);
        assert_eq!(s.mixer.channels[3].level, 100);
        assert_eq!(s.delay.params, [1, 3, 1, 70, 60, 0]);
        assert_eq!(s.dist.params, [1, 50, 0, 0]);
        assert_eq!(s.pcf.params, [1, 11, 12, 13, 14, 15, 1, 0]);
        assert_eq!(s.tb303[0].params[..3], [1, 4, 0]);
        assert_eq!(s.tb303[1].params[0], 0);
        assert_eq!(s.tb303[0].patterns[0].length, 4);
        assert_eq!(s.tb303[0].patterns[1].length, 16);
        let st = s.tb303[0].patterns[0].steps;
        assert_eq!(
            (st[0].note, st[0].up, st[0].slide, st[0].down),
            (1, 1, 1, 1)
        );
        assert_eq!(
            (st[1].note, st[1].accent, st[1].up, st[1].rest),
            (3, 1, 1, 1)
        );
        // params[2 + i] = p[16 + i] = 4i; the LT tuning (10) is p[24] = 32 -> 32/2 + 64
        assert_eq!(s.tr808.params[10], 80);
        assert_eq!(s.tr808.params[11], 36);
        // value k of the unpacked pattern is step k % 16 of column k / 16: byte 0 = 0x40 sets
        // value 0 (AC, step 0), byte 4 = 0x10 sets value 17 (BD, step 1)
        let p = s.tr808.patterns[0].steps;
        assert_eq!((p[0][0], p[1][1], p[1][0], p[0][1]), (1, 1, 0, 0));
        assert_eq!(s.comp.params, crate::comp::DEFAULTS);
        assert_eq!(s.form, LoadForm::Rb1x);
    }

    #[test]
    fn tracks_map_and_flush() {
        let mut t2: Vec<Event> = (0..1300).map(|i| cc(i, 14, 1)).collect();
        t2.push(cc(2000, 7, 99));
        let t5 = vec![cc(10, 17, 2), cc(10, 31, 3), cc(11, 20, 77)];
        let t4 = vec![cc(5, 24, 100)]; // 808 LT tuning (ctrl 10)
        let x = load_1x(&song(vec![t2, vec![], t4, t5])).unwrap();
        // 303 #1 flushed once at 1284 during the walk, before the mixer's and the rest
        assert_eq!(x.order[0], (1, 0));
        assert_eq!(x.batches[1][0].events.len(), 1284);
        assert_eq!((x.batches[1][1].first, x.batches[1][1].last), (1284, 1300));
        let m = &x.song.tracks[0];
        assert_eq!(
            m[0],
            TrackEvent {
                tick: 48_000,
                ctrl: 6,
                value: 99
            }
        );
        assert_eq!(
            m[1..5]
                .iter()
                .map(|e| (e.ctrl, e.value))
                .collect::<Vec<_>>(),
            [(9, 0), (15, 0), (21, 1), (27, 0)]
        );
        assert_eq!((m[5].ctrl, m[5].value, m[5].tick), (2, 0, 240));
        assert_eq!(
            x.song.tracks[5],
            [TrackEvent {
                tick: 264,
                ctrl: 3,
                value: 77
            }]
        );
        assert_eq!(
            x.song.tracks[3],
            [TrackEvent {
                tick: 120,
                ctrl: 10,
                value: 114
            }]
        );
        assert_eq!(
            x.song.tracks[4],
            [TrackEvent {
                tick: 0,
                ctrl: 0,
                value: 0
            }]
        );
        // the stream: 36, seven blocks, the 909's 4/74, the batches, 32
        let msgs = x.messages(7);
        assert_eq!(msgs[0][..3], [78, 7, 36]);
        assert_eq!(msgs[8], [78, 7, 41, 4, 74, 0, 0]);
        assert_eq!(msgs[9][3], 1);
        assert_eq!(msgs.last().unwrap(), &vec![78, 7, 32]);
        assert_eq!(msgs.len(), 9 + x.order.len() + 1);
    }

    /// `Engine::load` of a 1.x song: the 909's live enabled byte is 0 (msg 74) while its
    /// pattern-mode copy keeps the ctor's 1, and the compressor keeps its ctor block.
    #[test]
    fn engine_load_switches_the_909_off() {
        use crate::frame::{Devices, Engine};
        use crate::samples::{COUNT, Sample, SampleSet};
        let s = convert_1x(&song(vec![])).unwrap();
        let kit =
            SampleSet::from_samples((0..COUNT).map(|_| Sample::from_audio(&[0; 4])).collect());
        let mut e = Engine::new(
            std::sync::Arc::new(crate::tables::Tables::new()),
            Devices::implemented(Some(std::sync::Arc::new(kit))),
        );
        e.load(&s);
        let d = &mut e.frame.devices;
        assert_eq!(d.device_mut(Device::Tr909).live()[0], 0);
        // (bytes 3 and 4 are the meters, which the ctor's mixer update already wrote)
        assert_eq!(
            d.device_mut(Device::Compressor).live()[..3],
            crate::comp::DEFAULTS[..3]
        );
        // song mode: the track's tick-0 event (0, 0, 0) replaced the ctor's, so the chase at
        // msg 32 keeps it at 0
        assert_eq!(s.globals.song_mode, 1);
        assert!(
            e.frame
                .seq
                .events(4)
                .iter()
                .any(|x| x.tick == 0 && x.ctrl == 0 && x.value == 0)
        );
    }

    #[test]
    fn refusals() {
        let mut s = song(vec![vec![cc(0, 99, 1)]]);
        assert!(matches!(load_1x(&s), Err(LoadError::Rb1x(_))));
        s = song(vec![]);
        s.version = (3, 0);
        assert!(load_1x(&s).is_err());
        s.version = (3, 1);
        s.format = 0;
        assert!(load_1x(&s).is_err());
        s.format = 1;
        s.tracks.remove(1);
        assert!(load_1x(&s).is_err());
    }

    #[test]
    fn cc_map_matches_the_exe_tables() {
        assert_eq!(map_cc(2, 7), Some((Device::Mixer, 6)));
        assert_eq!(map_cc(4, 12), Some((Device::Mixer, 20)));
        assert_eq!(map_cc(3, 22), Some((Device::Tb303_2, 8)));
        assert_eq!(map_cc(3, 23), None);
        assert_eq!(map_cc(4, 42), Some((Device::Tr808, 28)));
        assert_eq!(map_cc(4, 43), None);
        assert_eq!(map_cc(5, 17), None);
        assert_eq!(map_cc(5, 30), Some((Device::Pcf, 0)));
        assert_eq!(map_cc(5, 24), Some((Device::Pcf, 1)));
        assert_eq!(map_cc(6, 7), None);
        assert_eq!(map_cc(1, 7), None);
    }
}
