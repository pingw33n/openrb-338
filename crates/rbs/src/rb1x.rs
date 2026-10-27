//! ReBirth 1.x songs: a Standard MIDI File (`MThd`) with the `.rbs` extension, read as
//! Rebirth.exe 2.01 reads it (`docs/song_load.md` section 12).
//!
//! The file is a format-1 MIDI file. Track 0 holds the text event `File version X.Y...`; one
//! track holds a sysex `F0 00 00 66 ...` whose data, 7 bits per byte, decodes to the 16 KB
//! [`Block`] (globals, mixer, effects, the 303 and 808 patterns, the song information); the other
//! tracks hold controller changes (`Bn cc v`), one MIDI track per device group. This module reads
//! the file into that raw form; `rb_engine::load::load_1x` turns it into the engine's messages
//! the way the exe does. `tools/rb1x.py` is the Python twin.

use crate::Error;

/// The decoded size of the song sysex (`sub_4304C5`: `0x4000` is the intended value of its
/// copy-protected size formula).
pub const BLOCK_LEN: usize = 0x4000;

/// Whether `data` is a 1.x song (a MIDI file).
pub fn is_1x(data: &[u8]) -> bool {
    data.starts_with(b"MThd")
}

fn err<T>(chunk: &str, offset: usize, msg: impl Into<String>) -> Result<T, Error> {
    Err(Error {
        chunk: chunk.to_string(),
        offset,
        msg: msg.into(),
    })
}

/// One MIDI event; `time` is the absolute time in file units (the exe forces the division to 8:
/// engine tick = `24 * time`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Event {
    pub time: u32,
    pub kind: Kind,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Kind {
    /// A control change `Bn ctrl value`, any channel (the exe looks at `status & 0xF0` only).
    Cc { ctrl: u8, value: u8 },
    /// A system exclusive `F0` (or `F7`) message: its data after the status byte.
    Sysex(Vec<u8>),
    /// A meta event `FF type len data`.
    Meta { kind: u8, data: Vec<u8> },
    /// Any other channel message (the exe skips it).
    Other { status: u8, data: Vec<u8> },
}

/// A parsed 1.x song: the MIDI header, the tracks and the file version.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Song1x {
    /// The MIDI format word (the exe accepts 1 only, 0x430CBC).
    pub format: u16,
    pub division: u16,
    pub tracks: Vec<Vec<Event>>,
    /// From the first `File version %s` text event of track 0 (`sub_431341`); (2, 0) when there
    /// is none (`sub_47D768(2, 0)`).
    pub version: (u32, u32),
}

fn vlq(b: &[u8], mut p: usize, base: usize) -> Result<(u32, usize), Error> {
    let mut v: u32 = 0;
    loop {
        let Some(&c) = b.get(p) else {
            return err("MTrk", base + p, "truncated variable-length number");
        };
        p += 1;
        v = (v << 7) | (c & 0x7F) as u32;
        if c & 0x80 == 0 {
            return Ok((v, p));
        }
    }
}

fn put_vlq(v: u32, out: &mut Vec<u8>) {
    let mut groups = vec![(v & 0x7F) as u8];
    let mut v = v >> 7;
    while v != 0 {
        groups.push((v & 0x7F) as u8 | 0x80);
        v >>= 7;
    }
    out.extend(groups.iter().rev());
}

/// The data bytes of a channel message (`byte_4D2A60[4 * ((status & 0x7F) >> 4)]`).
fn data_bytes(status: u8) -> usize {
    match status & 0xF0 {
        0xC0 | 0xD0 => 1,
        _ => 2,
    }
}

/// `sub_4718F6`: the leading digits of the text before the first `.` and after it.
fn parse_version(text: &[u8]) -> Option<(u32, u32)> {
    let text = text.strip_prefix(b"File version ")?;
    let word: Vec<u8> = text
        .iter()
        .copied()
        .take_while(|c| !c.is_ascii_whitespace())
        .collect();
    let dot = word.iter().position(|&c| c == b'.')?;
    let num = |s: &[u8]| -> u32 {
        s.iter()
            .take_while(|c| c.is_ascii_digit())
            .fold(0u32, |a, &c| {
                a.saturating_mul(10).saturating_add((c - b'0') as u32)
            })
    };
    let (major, minor) = (num(&word[..dot]), num(&word[dot + 1..]));
    (major < 100 && minor < 100).then_some((major, minor))
}

impl Song1x {
    /// Reads a MIDI file. Errors are what makes the exe throw `XBadFormat`: not a MIDI file, a
    /// truncated track, a bad version text.
    pub fn parse(data: &[u8]) -> Result<Song1x, Error> {
        if !is_1x(data) || data.len() < 14 {
            return err("MThd", 0, "not a MIDI file");
        }
        let hlen = u32::from_be_bytes(data[4..8].try_into().unwrap()) as usize;
        if hlen < 6 || 8 + hlen > data.len() {
            return err("MThd", 4, "bad header length");
        }
        let format = u16::from_be_bytes([data[8], data[9]]);
        let ntracks = u16::from_be_bytes([data[10], data[11]]) as usize;
        let division = u16::from_be_bytes([data[12], data[13]]);
        let mut p = 8 + hlen;
        let mut tracks = Vec::new();
        // the running status survives the track seek (sub_4A0CA3 does not reset it)
        let mut status = 0u8;
        while p + 8 <= data.len() && tracks.len() < ntracks {
            let tag = &data[p..p + 4];
            let len = u32::from_be_bytes(data[p + 4..p + 8].try_into().unwrap()) as usize;
            let base = p + 8;
            let Some(body) = data.get(base..base.saturating_add(len)) else {
                return err("MTrk", p, "track past the end of the file");
            };
            p = base + len;
            if tag != b"MTrk" {
                continue;
            }
            let mut evs = Vec::new();
            let (mut q, mut t) = (0usize, 0u32);
            while q < body.len() {
                let (d, nq) = vlq(body, q, base)?;
                q = nq;
                t = t.wrapping_add(d);
                let Some(&b) = body.get(q) else {
                    return err("MTrk", base + q, "truncated event");
                };
                if b & 0x80 != 0 {
                    status = b;
                    q += 1;
                }
                let take = |q: usize, n: usize| -> Result<Vec<u8>, Error> {
                    body.get(q..q + n).map(<[u8]>::to_vec).ok_or_else(|| Error {
                        chunk: "MTrk".into(),
                        offset: base + q,
                        msg: "truncated event".into(),
                    })
                };
                match status {
                    0xFF => {
                        let Some(&kind) = body.get(q) else {
                            return err("MTrk", base + q, "truncated meta event");
                        };
                        let (n, nq) = vlq(body, q + 1, base)?;
                        let d = take(nq, n as usize)?;
                        q = nq + n as usize;
                        evs.push(Event {
                            time: t,
                            kind: Kind::Meta { kind, data: d },
                        });
                        if kind == 0x2F {
                            break;
                        }
                    }
                    0xF0 | 0xF7 => {
                        let (n, nq) = vlq(body, q, base)?;
                        let d = take(nq, n as usize)?;
                        q = nq + n as usize;
                        evs.push(Event {
                            time: t,
                            kind: Kind::Sysex(d),
                        });
                    }
                    0x80..=0xEF => {
                        let n = data_bytes(status);
                        let d = take(q, n)?;
                        q += n;
                        let kind = if status & 0xF0 == 0xB0 {
                            Kind::Cc {
                                ctrl: d[0],
                                value: d[1],
                            }
                        } else {
                            Kind::Other { status, data: d }
                        };
                        evs.push(Event { time: t, kind });
                    }
                    _ => return err("MTrk", base + q, "data byte without a status"),
                }
            }
            tracks.push(evs);
        }
        if tracks.len() < ntracks {
            return err("MThd", 10, format!("{} tracks of {ntracks}", tracks.len()));
        }
        let mut version = (2, 0);
        if let Some(t0) = tracks.first() {
            for e in t0 {
                if let Kind::Meta { kind: 2, data } = &e.kind {
                    let text = &data[..data.len().min(200)];
                    version = parse_version(text).ok_or_else(|| Error {
                        chunk: "MTrk".into(),
                        offset: 0,
                        msg: format!("bad version text {:?}", String::from_utf8_lossy(text)),
                    })?;
                    break;
                }
                if matches!(e.kind, Kind::Meta { kind: 0x2F, .. }) {
                    break;
                }
            }
        }
        Ok(Song1x {
            format,
            division,
            tracks,
            version,
        })
    }

    /// The MIDI file again, without running status (used by tests to build songs).
    pub fn write(&self) -> Vec<u8> {
        let mut out = b"MThd".to_vec();
        out.extend(6u32.to_be_bytes());
        out.extend(self.format.to_be_bytes());
        out.extend((self.tracks.len() as u16).to_be_bytes());
        out.extend(self.division.to_be_bytes());
        for t in &self.tracks {
            let mut body = Vec::new();
            let mut last = 0u32;
            for e in t {
                put_vlq(e.time.wrapping_sub(last), &mut body);
                last = e.time;
                match &e.kind {
                    Kind::Cc { ctrl, value } => body.extend([0xB0, *ctrl, *value]),
                    Kind::Sysex(d) => {
                        body.push(0xF0);
                        put_vlq(d.len() as u32, &mut body);
                        body.extend(d);
                    }
                    Kind::Meta { kind, data } => {
                        body.extend([0xFF, *kind]);
                        put_vlq(data.len() as u32, &mut body);
                        body.extend(data);
                    }
                    Kind::Other { status, data } => {
                        body.push(*status);
                        body.extend(data);
                    }
                }
            }
            out.extend(b"MTrk");
            out.extend((body.len() as u32).to_be_bytes());
            out.extend(body);
        }
        out
    }
}

/// The decoded song sysex (`sub_431747` reads it; offsets in `docs/song_load.md` section 12).
#[derive(Clone, PartialEq, Eq)]
pub struct Block(pub Box<[u8; BLOCK_LEN]>);

impl std::fmt::Debug for Block {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Block({:02x?}..)", &self.0[..16])
    }
}

impl Block {
    /// `sub_4304C5`: the sysex data (after `F0`) must start `00 00 66` (`sub_4A14AD`), with
    /// byte 5 non-zero, byte 6 equal to 1 and byte 7 zero (counting `F0` as byte 0); then
    /// 8-byte groups give 7 bytes each, the 8th holding their high bits (0x40 for the first,
    /// `sub_4663F0`), until 0x4000 bytes (`sub_4665CD`; missing input reads as zero). The
    /// decoded block must carry `TinY` at 5 (`sub_431747`).
    pub fn decode(sysex: &[u8]) -> Result<Block, Error> {
        let v: Vec<u8> = std::iter::once(0xF0).chain(sysex.iter().copied()).collect();
        if v.len() < 8 || v[1..4] != [0, 0, 0x66] || v[5] == 0 || v[6] != 1 || v[7] != 0 {
            return err("sysex", 0, "not a ReBirth song sysex");
        }
        let src = &v[8..];
        let mut out = Box::new([0u8; BLOCK_LEN]);
        let mut n = 0;
        let mut p = 0;
        while n < BLOCK_LEN {
            let g: [u8; 8] = std::array::from_fn(|k| src.get(p + k).copied().unwrap_or(0));
            for k in 0..7 {
                if n < BLOCK_LEN {
                    out[n] = g[k] | if g[7] & (0x40 >> k) != 0 { 0x80 } else { 0 };
                    n += 1;
                }
            }
            p += 8;
        }
        if &out[5..9] != b"TinY" {
            return err("sysex", 5, "no TinY tag in the decoded block");
        }
        Ok(Block(out))
    }

    /// The sysex data (after `F0`) that decodes to this block: the inverse of [`Block::decode`]
    /// with header bytes `00 00 66 00 15 01 00` as in the collection's files, ending in `F7`.
    pub fn encode(&self) -> Vec<u8> {
        let mut out = vec![0, 0, 0x66, 0, 0x15, 1, 0];
        for g in self.0.chunks(7) {
            let mut hi = 0u8;
            for (k, &b) in g.iter().enumerate() {
                out.push(b & 0x7F);
                if b & 0x80 != 0 {
                    hi |= 0x40 >> k;
                }
            }
            out.extend(std::iter::repeat_n(0, 7 - g.len()));
            out.push(hi);
        }
        out.push(0xF7);
        out
    }

    pub fn bytes(&self) -> &[u8; BLOCK_LEN] {
        &self.0
    }

    /// The big-endian u32 at `off` (`sub_465FCA`).
    pub fn u32_at(&self, off: usize) -> u32 {
        u32::from_be_bytes(self.0[off..off + 4].try_into().unwrap())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn block() -> Block {
        let mut b = Box::new([0u8; BLOCK_LEN]);
        b[..5].copy_from_slice(&[0xBC, 3, 1, 0, 0]);
        b[5..9].copy_from_slice(b"TinY");
        for (i, x) in b.iter_mut().enumerate().skip(9) {
            *x = (i * 37 % 251) as u8;
        }
        Block(b)
    }

    #[test]
    fn sysex_round_trip() {
        let b = block();
        let sx = b.encode();
        assert_eq!(sx.len(), 7 + 2341 * 8 + 1);
        assert_eq!(Block::decode(&sx).unwrap(), b);
    }

    #[test]
    fn bad_sysex_header_is_an_error() {
        let mut sx = block().encode();
        sx[5] = 0;
        assert!(Block::decode(&sx).is_err());
        assert!(Block::decode(&[0, 0]).is_err());
    }

    #[test]
    fn midi_round_trip_and_running_status() {
        let s = Song1x {
            format: 1,
            division: 32,
            tracks: vec![
                vec![Event {
                    time: 0,
                    kind: Kind::Meta {
                        kind: 2,
                        data: b"File version 3.1.0 (c)1997".to_vec(),
                    },
                }],
                vec![
                    Event {
                        time: 0,
                        kind: Kind::Cc {
                            ctrl: 7,
                            value: 100,
                        },
                    },
                    Event {
                        time: 200,
                        kind: Kind::Cc { ctrl: 14, value: 3 },
                    },
                ],
            ],
            version: (3, 1),
        };
        let data = s.write();
        assert_eq!(Song1x::parse(&data).unwrap(), s);
        // the same track with running status: 00 B0 07 64, 81 48 0E 03
        let mut rs = data[..data.len() - 17].to_vec();
        rs.extend(b"MTrk\0\0\0\x08\x00\xB0\x07\x64\x81\x48\x0E\x03");
        assert_eq!(Song1x::parse(&rs).unwrap(), s);
    }

    #[test]
    fn version_text() {
        assert_eq!(parse_version(b"File version 3.1.0 (c)1997"), Some((3, 1)));
        assert_eq!(parse_version(b"File version 3.2"), Some((3, 2)));
        assert_eq!(parse_version(b"File version 300.1"), None);
        assert_eq!(parse_version(b"File version 3"), None);
        assert_eq!(parse_version(b"Something"), None);
    }

    #[test]
    fn truncated_files_are_errors_not_panics() {
        let data = Song1x {
            format: 1,
            division: 32,
            tracks: vec![vec![Event {
                time: 5,
                kind: Kind::Sysex(block().encode()),
            }]],
            version: (2, 0),
        }
        .write();
        for n in 0..data.len() {
            let _ = Song1x::parse(&data[..n]);
        }
    }
}
