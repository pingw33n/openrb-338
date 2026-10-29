//! The RBS 4.2 song file format (see `docs/RBS42.txt`): reader and writer.
//!
//! The model mirrors `tools/rbs.py`. The field meanings follow `docs/song_load.md` section 10
//! (the errata of RBS42.txt): a TRAK position is in 32nd notes (engine tick = position * 24),
//! 303 step flag bit 4 set means rest, delay mode 0 is triplets, PCF ids are 1..4, comp id 1 is
//! the master, USRI is 712 bytes.
//!
//! Bytes the model does not name (reserved bytes, the USRI tail, junk after a string's
//! terminator, the version signature) are carried in each chunk's private raw copy and written
//! back as read, so `Song::parse` followed by `Song::write` reproduces the input exactly. A field
//! is rewritten only when its value differs from what the raw bytes decode to. Unknown chunks
//! directly under the `RB40` catalog are kept and written back at the same position.
//!
//! # Round trip and leniency
//!
//! The reader is lenient on purpose, because user files will be rendered: RB40 items in any order
//! (RBS42.txt line 78: the order at RB40 level is not fixed), non-minimal (non-canonical) delta
//! encodings, a TRAK with zero events or a first event after position 0, and bytes after the last
//! event of a TRAK (kept in the model). The writer is canonical by design: the RB40 order is HEAD,
//! GLOB, USRI, DEVL, TRKL, deltas are minimal. So `write(parse(x)) == x` byte for byte for
//! canonical files (all shipped and corpus songs are), and any other file is normalized: it
//! re-parses to the same model but its bytes change. Unknown chunks inside DEVL or TRKL are an
//! error (RBS42.txt fixes their order); unknown chunks directly under RB40 are kept.
//!
//! # Field meanings
//!
//! Delay `mode` (`DELAY_PARAMS[2]`): 0 = triplets (1/3 beat), 1 = sixteenths (1/4 beat). TB-303
//! `waveform`: 0 = saw, 1 = square. PCF `mode`: 0 = bandpass, 1 = lowpass. Mixer `comp_id`: 0 off,
//! 1 master, 2..5 = 303 #1, 303 #2, 808, 909. Mixer `pcf_id`: 0 off, 1..4 = 303 #1, 303 #2, 808,
//! 909. Mixer track controllers 6.. are `<channel>_<level|pan|delay_send|dist>` at 6 + 6 * channel
//! + k (so the 909 delay send and dist are 0x1A, 0x1B).

use std::fmt;

pub mod rb1x;

/// Names of the nine TRAK tracks, in file order.
pub const TRACK_NAMES: [&str; 9] = [
    "mixer", "tb303_1", "tb303_2", "tr808", "tr909", "delay", "dist", "pcf", "comp",
];
/// Names of the four mixer channels (index 0..3).
pub const CHANNELS: [&str; 4] = ["tb303_1", "tb303_2", "tr808", "tr909"];

pub const DELAY_PARAMS: [&str; 5] = ["enabled", "steps", "mode", "feedback", "pan"];
pub const PCF_PARAMS: [&str; 7] = [
    "enabled",
    "frequency",
    "resonance",
    "amount",
    "wave",
    "decay",
    "mode",
];
pub const DIST_PARAMS: [&str; 3] = ["enabled", "amount", "shape"];
pub const COMP_PARAMS: [&str; 3] = ["enabled", "amount", "threshold"];
pub const TB303_PARAMS: [&str; 9] = [
    "enabled",
    "pattern",
    "tune",
    "cutoff",
    "resonance",
    "env_mod",
    "decay",
    "accent",
    "waveform",
];
pub const TR808_PARAMS: [&str; 29] = [
    "enabled",
    "pattern",
    "accent_level",
    "bass_level",
    "bass_tone",
    "bass_decay",
    "snare_level",
    "snare_tone",
    "snare_snappy",
    "low_tom_level",
    "low_tom_tuning",
    "low_tom_selector",
    "mid_tom_level",
    "mid_tom_tuning",
    "mid_tom_selector",
    "hi_tom_level",
    "hi_tom_tuning",
    "hi_tom_selector",
    "rim_shot_level",
    "rim_shot_selector",
    "clap_level",
    "clap_selector",
    "cow_bell_level",
    "cymbal_level",
    "cymbal_tone",
    "cymbal_decay",
    "open_hihat_level",
    "open_hihat_decay",
    "closed_hihat_level",
];
pub const TR909_PARAMS: [&str; 30] = [
    "enabled",
    "pattern",
    "accent_level",
    "bass_level",
    "bass_tune",
    "bass_attack",
    "bass_decay",
    "snare_level",
    "snare_tune",
    "snare_tone",
    "snare_snappy",
    "low_tom_level",
    "low_tom_tune",
    "low_tom_decay",
    "mid_tom_level",
    "mid_tom_tune",
    "mid_tom_decay",
    "hi_tom_level",
    "hi_tom_tune",
    "hi_tom_decay",
    "rim_shot_level",
    "clap_level",
    "hihat_level",
    "closed_hihat_decay",
    "open_hihat_decay",
    "crash_level",
    "crash_tune",
    "ride_level",
    "ride_tune",
    "flam_interval",
];
/// Step columns of the 808 patterns, in file order (0 off, 1 on).
pub const TR808_COLUMNS: [&str; 12] = [
    "accent",
    "bass",
    "snare",
    "low_tom",
    "mid_tom",
    "hi_tom",
    "rim_shot",
    "clap",
    "cow_bell",
    "cymbal",
    "open_hihat",
    "closed_hihat",
];
/// Step columns of the 909 patterns, in file order (0 off, 1 on, 2 accent, 3 flam).
pub const TR909_COLUMNS: [&str; 12] = [
    "accent",
    "bass",
    "snare",
    "low_tom",
    "mid_tom",
    "hi_tom",
    "rim_shot",
    "clap",
    "closed_hihat",
    "open_hihat",
    "crash",
    "ride",
];

/// A malformed input or an unwritable model: the chunk, the absolute file offset, the reason.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Error {
    pub chunk: String,
    pub offset: usize,
    pub msg: String,
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "chunk {} at offset {}: {}",
            self.chunk, self.offset, self.msg
        )
    }
}

impl std::error::Error for Error {}

fn err<T>(chunk: &str, offset: usize, msg: impl Into<String>) -> Result<T, Error> {
    Err(Error {
        chunk: chunk.trim_end().to_string(),
        offset,
        msg: msg.into(),
    })
}

/// A tag for messages: trailing spaces trimmed, non-printable bytes escaped.
fn tag_str(tag: &[u8; 4]) -> String {
    let n = tag.iter().rposition(|&c| c != b' ').map_or(0, |i| i + 1);
    tag[..n]
        .iter()
        .flat_map(|&c| std::ascii::escape_default(c))
        .map(char::from)
        .collect()
}

// ------------------------------------------------------------------ IFF helpers

struct RawChunk<'a> {
    tag: [u8; 4],
    /// Absolute file offset of the chunk body.
    start: usize,
    body: &'a [u8],
}

/// Splits `data[pos..end]` into chunks. `parent` names the container for errors.
fn read_chunks<'a>(
    data: &'a [u8],
    mut pos: usize,
    end: usize,
    parent: &str,
) -> Result<Vec<RawChunk<'a>>, Error> {
    let mut out = Vec::new();
    while pos < end {
        if pos + 8 > end {
            return err(parent, pos, "truncated chunk header");
        }
        let tag: [u8; 4] = data[pos..pos + 4].try_into().unwrap();
        let size = u32::from_be_bytes(data[pos + 4..pos + 8].try_into().unwrap()) as usize;
        let start = pos + 8;
        if start.checked_add(size).is_none_or(|e| e > end) {
            return err(
                &tag_str(&tag),
                pos,
                format!("chunk of {size} bytes overruns its parent"),
            );
        }
        out.push(RawChunk {
            tag,
            start,
            body: &data[start..start + size],
        });
        pos = start + size;
        if size & 1 == 1 {
            if pos >= end {
                return err(&tag_str(&tag), pos, "missing pad byte");
            }
            if data[pos] != 0 {
                return err(&tag_str(&tag), pos, "nonzero pad byte");
            }
            pos += 1;
        }
    }
    Ok(out)
}

fn chunk(tag: &[u8; 4], body: &[u8]) -> Vec<u8> {
    let mut v = Vec::with_capacity(body.len() + 9);
    v.extend_from_slice(tag);
    v.extend_from_slice(&(body.len() as u32).to_be_bytes());
    v.extend_from_slice(body);
    if body.len() & 1 == 1 {
        v.push(0);
    }
    v
}

fn cat(kind: &[u8; 4], children: &[Vec<u8>]) -> Vec<u8> {
    let mut body = kind.to_vec();
    for c in children {
        body.extend_from_slice(c);
    }
    chunk(b"CAT ", &body)
}

// ------------------------------------------------------------- field handling

fn get_u32(raw: &[u8], off: usize) -> u32 {
    u32::from_be_bytes(raw[off..off + 4].try_into().unwrap())
}

fn get_z(raw: &[u8], off: usize, size: usize) -> String {
    let b = &raw[off..off + size];
    let n = b.iter().position(|&c| c == 0).unwrap_or(size);
    b[..n].iter().map(|&c| c as char).collect() // latin-1
}

fn put_z(raw: &mut [u8], off: usize, size: usize, s: &str, chunk: &str) -> Result<(), Error> {
    if get_z(raw, off, size) == s {
        return Ok(());
    }
    let mut b = Vec::new();
    for c in s.chars() {
        match u8::try_from(c as u32) {
            Ok(x) => b.push(x),
            Err(_) => return err(chunk, 0, format!("string {s:?} is not latin-1")),
        }
    }
    if b.len() > size - 1 {
        return err(chunk, 0, format!("string {s:?} longer than {}", size - 1));
    }
    b.resize(size, 0);
    raw[off..off + size].copy_from_slice(&b);
    Ok(())
}

/// The raw bytes of a chunk, or a zeroed chunk of `size` for a model built from scratch.
fn raw_or(raw: &[u8], size: usize) -> Vec<u8> {
    if raw.is_empty() {
        vec![0; size]
    } else {
        raw.to_vec()
    }
}

fn check_size(name: &str, start: usize, body: &[u8], want: usize) -> Result<(), Error> {
    if body.len() != want {
        return err(
            name,
            start - 8,
            format!("size {} but {} expected", body.len(), want),
        );
    }
    Ok(())
}

// --------------------------------------------------------------------- chunks

#[derive(Debug, Clone, PartialEq)]
pub struct Head {
    pub major: u8,
    pub minor: u8,
    /// HEAD byte 7. Rebirth.exe compares the whole version (major, minor, revision): its "4.0"
    /// and "4.1" paths need exactly revision 0 (docs/song_load.md section 2).
    pub revision: u8,
    pub copyright: String,
    raw: Vec<u8>,
}

impl Head {
    pub const SIZE: usize = 256;

    pub fn version(&self) -> String {
        format!("{}.{}", self.major, self.minor)
    }

    fn parse(c: &RawChunk) -> Result<Self, Error> {
        check_size("HEAD", c.start, c.body, Self::SIZE)?;
        let raw = c.body;
        Ok(Head {
            major: raw[5],
            minor: raw[6],
            revision: raw[7],
            copyright: get_z(raw, 9, 129),
            raw: raw.to_vec(),
        })
    }

    fn build(&self) -> Result<Vec<u8>, Error> {
        let mut raw = if self.raw.is_empty() {
            let mut v = vec![0x5b, 0x54, 0x5b, 0x54, 0xbc, 4, 2, 0, 0];
            v.resize(Self::SIZE, 0);
            v
        } else {
            self.raw.clone()
        };
        raw[5] = self.major;
        raw[6] = self.minor;
        raw[7] = self.revision;
        put_z(&mut raw, 9, 129, &self.copyright, "HEAD")?;
        Ok(raw)
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct Glob {
    /// 0 pattern mode, 1 song mode.
    pub song_mode: u8,
    pub loop_on: u8,
    /// BPM * 1000.
    pub tempo: u32,
    /// Bar * 768.
    pub loop_start: u32,
    pub loop_end: u32,
    pub shuffle: u8,
    pub mod_name: String,
    pub mod_ftp: String,
    pub mod_www: String,
    /// 0 ReBirth 2.0 sound, 1 vintage sound.
    pub vintage: u8,
    raw: Vec<u8>,
}

impl Glob {
    pub const SIZE: usize = 512;

    fn parse(c: &RawChunk) -> Result<Self, Error> {
        check_size("GLOB", c.start, c.body, Self::SIZE)?;
        let r = c.body;
        Ok(Glob {
            song_mode: r[0],
            loop_on: r[1],
            tempo: get_u32(r, 2),
            loop_start: get_u32(r, 6),
            loop_end: get_u32(r, 10),
            shuffle: r[14],
            mod_name: get_z(r, 15, 65),
            mod_ftp: get_z(r, 80, 201),
            mod_www: get_z(r, 281, 201),
            vintage: r[482],
            raw: r.to_vec(),
        })
    }

    fn build(&self) -> Result<Vec<u8>, Error> {
        let mut r = raw_or(&self.raw, Self::SIZE);
        r[0] = self.song_mode;
        r[1] = self.loop_on;
        r[2..6].copy_from_slice(&self.tempo.to_be_bytes());
        r[6..10].copy_from_slice(&self.loop_start.to_be_bytes());
        r[10..14].copy_from_slice(&self.loop_end.to_be_bytes());
        r[14] = self.shuffle;
        put_z(&mut r, 15, 65, &self.mod_name, "GLOB")?;
        put_z(&mut r, 80, 201, &self.mod_ftp, "GLOB")?;
        put_z(&mut r, 281, 201, &self.mod_www, "GLOB")?;
        r[482] = self.vintage;
        Ok(r)
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct Usri {
    pub window_title: String,
    /// 0x0d is a newline.
    pub text: String,
    pub web_page: String,
    pub show_on_open: u8,
    /// Real files hold 712 bytes (RBS42.txt says 512); bytes 544.. are junk the engine ignores.
    raw: Vec<u8>,
}

impl Usri {
    pub const SIZE: usize = 712;

    fn parse(c: &RawChunk) -> Result<Self, Error> {
        check_size("USRI", c.start, c.body, Self::SIZE)?;
        let r = c.body;
        Ok(Usri {
            window_title: get_z(r, 0, 41),
            text: get_z(r, 41, 401),
            web_page: get_z(r, 442, 101),
            show_on_open: r[543],
            raw: r.to_vec(),
        })
    }

    fn build(&self) -> Result<Vec<u8>, Error> {
        let mut r = raw_or(&self.raw, Self::SIZE);
        put_z(&mut r, 0, 41, &self.window_title, "USRI")?;
        put_z(&mut r, 41, 401, &self.text, "USRI")?;
        put_z(&mut r, 442, 101, &self.web_page, "USRI")?;
        r[543] = self.show_on_open;
        Ok(r)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct MixChannel {
    pub enabled: u8,
    pub level: u8,
    pub pan: u8,
    pub delay_send: u8,
    /// Distortion insert on/off.
    pub dist: u8,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Mixer {
    pub master: u8,
    /// 0 off, 1 master, 2..5 = channel 0..3.
    pub comp_id: u8,
    /// 0 off, 1..4 = channel 0..3.
    pub pcf_id: u8,
    pub channels: [MixChannel; 4],
    raw: Vec<u8>,
}

impl Mixer {
    pub const SIZE: usize = 64;

    fn parse(c: &RawChunk) -> Result<Self, Error> {
        check_size("MIXR", c.start, c.body, Self::SIZE)?;
        let r = c.body;
        let ch = |i: usize| {
            let o = 16 + 12 * i;
            MixChannel {
                enabled: r[o],
                level: r[o + 1],
                pan: r[o + 2],
                delay_send: r[o + 3],
                dist: r[o + 4],
            }
        };
        Ok(Mixer {
            master: r[0],
            comp_id: r[1],
            pcf_id: r[2],
            channels: [ch(0), ch(1), ch(2), ch(3)],
            raw: r.to_vec(),
        })
    }

    /// The 64-byte MIXR body as it is written. A version 4.0 file stores 7-byte channels at the
    /// same offsets (16 + 12 * channel), whose bytes 5 and 6 (PCF and compressor flags) only
    /// this gives access to (docs/song_load.md section 4.1).
    pub fn bytes(&self) -> Vec<u8> {
        self.build()
    }

    fn build(&self) -> Vec<u8> {
        let mut r = raw_or(&self.raw, Self::SIZE);
        r[0] = self.master;
        r[1] = self.comp_id;
        r[2] = self.pcf_id;
        for (i, c) in self.channels.iter().enumerate() {
            let o = 16 + 12 * i;
            r[o..o + 5].copy_from_slice(&[c.enabled, c.level, c.pan, c.delay_send, c.dist]);
        }
        r
    }
}

/// A plain parameter chunk (DELY, PCF, DIST, COMP): one byte per parameter, in the order of the
/// matching `*_PARAMS` name table.
#[derive(Debug, Clone, PartialEq)]
pub struct Params {
    pub values: Vec<u8>,
    raw: Vec<u8>,
}

impl Params {
    fn parse(name: &str, c: &RawChunk, size: usize, count: usize) -> Result<Self, Error> {
        check_size(name, c.start, c.body, size)?;
        Ok(Params {
            values: c.body[..count].to_vec(),
            raw: c.body.to_vec(),
        })
    }

    fn build(&self, name: &str, size: usize, count: usize) -> Result<Vec<u8>, Error> {
        if self.values.len() != count {
            return err(
                name,
                0,
                format!("{} values but {count} expected", self.values.len()),
            );
        }
        let mut r = raw_or(&self.raw, size);
        r[..count].copy_from_slice(&self.values);
        Ok(r)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Step303 {
    /// 0..12.
    pub note: u8,
    /// Flags bit 0.
    pub slide: u8,
    /// Bit 1.
    pub accent: u8,
    /// Bit 2 (transpose up).
    pub up: u8,
    /// Bit 3 (transpose down).
    pub down: u8,
    /// Bit 4: SET = rest (docs/song_load.md 4.6).
    pub rest: u8,
    /// Bits 5..7, unused by the original.
    pub flags_hi: u8,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Pattern303 {
    pub shuffle: u8,
    pub length: u8,
    pub steps: [Step303; 16],
}

#[derive(Debug, Clone, PartialEq)]
pub struct Tb303 {
    /// Indexed as `TB303_PARAMS`.
    pub params: [u8; 9],
    pub patterns: Vec<Pattern303>,
    raw: Vec<u8>,
}

impl Tb303 {
    pub const SIZE: usize = 1097;

    fn parse(c: &RawChunk) -> Result<Self, Error> {
        check_size("303 ", c.start, c.body, Self::SIZE)?;
        let r = c.body;
        let patterns = (0..32)
            .map(|p| {
                let o = 9 + 34 * p;
                let steps = std::array::from_fn(|s| {
                    let (note, fl) = (r[o + 2 + 2 * s], r[o + 3 + 2 * s]);
                    Step303 {
                        note,
                        slide: fl & 1,
                        accent: fl >> 1 & 1,
                        up: fl >> 2 & 1,
                        down: fl >> 3 & 1,
                        rest: fl >> 4 & 1,
                        flags_hi: fl >> 5,
                    }
                });
                Pattern303 {
                    shuffle: r[o],
                    length: r[o + 1],
                    steps,
                }
            })
            .collect();
        Ok(Tb303 {
            params: r[..9].try_into().unwrap(),
            patterns,
            raw: r.to_vec(),
        })
    }

    fn build(&self) -> Result<Vec<u8>, Error> {
        if self.patterns.len() != 32 {
            return err("303 ", 0, "must hold exactly 32 patterns");
        }
        let mut r = raw_or(&self.raw, Self::SIZE);
        r[..9].copy_from_slice(&self.params);
        for (p, pat) in self.patterns.iter().enumerate() {
            let o = 9 + 34 * p;
            r[o] = pat.shuffle;
            r[o + 1] = pat.length;
            for (s, st) in pat.steps.iter().enumerate() {
                if st.flags_hi > 7 {
                    return err("303 ", 0, "flags_hi above 7");
                }
                r[o + 2 + 2 * s] = st.note;
                r[o + 3 + 2 * s] = st.slide
                    | st.accent << 1
                    | st.up << 2
                    | st.down << 3
                    | st.rest << 4
                    | st.flags_hi << 5;
            }
        }
        Ok(r)
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct PatternDrum {
    pub shuffle: u8,
    pub length: u8,
    /// `steps[s][i]`, i over `TR808_COLUMNS` / `TR909_COLUMNS`.
    pub steps: [[u8; 12]; 16],
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DrumKind {
    Tr808,
    Tr909,
}

impl DrumKind {
    pub fn size(self) -> usize {
        match self {
            DrumKind::Tr808 => 6238,
            DrumKind::Tr909 => 6239,
        }
    }
    fn base(self) -> usize {
        match self {
            DrumKind::Tr808 => 30,
            DrumKind::Tr909 => 31,
        }
    }
    fn count(self) -> usize {
        match self {
            DrumKind::Tr808 => TR808_PARAMS.len(),
            DrumKind::Tr909 => TR909_PARAMS.len(),
        }
    }
    fn name(self) -> &'static str {
        match self {
            DrumKind::Tr808 => "808 ",
            DrumKind::Tr909 => "909 ",
        }
    }
}

/// TR-808 (6238 bytes) or TR-909 (6239 bytes).
#[derive(Debug, Clone, PartialEq)]
pub struct Drums {
    pub kind: DrumKind,
    /// Indexed as `TR808_PARAMS` / `TR909_PARAMS`.
    pub params: Vec<u8>,
    pub patterns: Vec<PatternDrum>,
    raw: Vec<u8>,
}

impl Drums {
    fn parse(kind: DrumKind, c: &RawChunk) -> Result<Self, Error> {
        check_size(kind.name(), c.start, c.body, kind.size())?;
        let r = c.body;
        let patterns = (0..32)
            .map(|p| {
                let o = kind.base() + 194 * p;
                let steps =
                    std::array::from_fn(|s| r[o + 2 + 12 * s..o + 14 + 12 * s].try_into().unwrap());
                PatternDrum {
                    shuffle: r[o],
                    length: r[o + 1],
                    steps,
                }
            })
            .collect();
        Ok(Drums {
            kind,
            params: r[..kind.count()].to_vec(),
            patterns,
            raw: r.to_vec(),
        })
    }

    fn build(&self) -> Result<Vec<u8>, Error> {
        let name = self.kind.name();
        if self.patterns.len() != 32 {
            return err(name, 0, "must hold exactly 32 patterns");
        }
        if self.params.len() != self.kind.count() {
            return err(name, 0, "wrong parameter count");
        }
        let mut r = raw_or(&self.raw, self.kind.size());
        r[..self.params.len()].copy_from_slice(&self.params);
        for (p, pat) in self.patterns.iter().enumerate() {
            let o = self.kind.base() + 194 * p;
            r[o] = pat.shuffle;
            r[o + 1] = pat.length;
            for (s, st) in pat.steps.iter().enumerate() {
                r[o + 2 + 12 * s..o + 14 + 12 * s].copy_from_slice(st);
            }
        }
        Ok(r)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Event {
    /// Absolute, in 32nd notes (32 per bar); engine tick = pos * 24.
    pub pos: u32,
    pub ctrl: u8,
    pub value: u8,
}

/// Encodes a delta in the fewest 7-bit groups: up to 4 bytes as RBS42.txt, 5 for a delta of
/// 2^28 or more (which only a file with such a delta gives, see [`decode_delta`]).
fn encode_delta(v: u32, out: &mut Vec<u8>) -> Result<(), Error> {
    let mut groups = vec![(v & 0x7f) as u8];
    let mut v = v >> 7;
    while v != 0 {
        groups.push(0x80 | (v & 0x7f) as u8);
        v >>= 7;
    }
    out.extend(groups.iter().rev());
    Ok(())
}

/// Decodes a delta; returns the value and the next index. RBS42.txt's ReadDeltaPos allows 4
/// bytes, but the exe's reader `sub_4A09D1` has no limit: it reads while bit 7 is set and keeps
/// `v = (v << 7) | (c & 0x7f)` in 32 bits (`shl eax, 7` at 0x4A0A1D), so the bits shifted out
/// are lost (docs/song_load.md section 5).
fn decode_delta(body: &[u8], mut p: usize, base: usize) -> Result<(u32, usize), Error> {
    let at = |p: usize| -> Result<u8, Error> {
        match body.get(p) {
            Some(&c) => Ok(c),
            None => err("TRAK", base + p, "truncated event"),
        }
    };
    let mut c = at(p)?;
    p += 1;
    let mut v = c as u32;
    if c & 0x80 != 0 {
        v &= 0x7f;
        loop {
            c = at(p)?;
            p += 1;
            v = (v << 7) | (c & 0x7f) as u32;
            if c & 0x80 == 0 {
                break;
            }
        }
    }
    Ok((v, p))
}

#[derive(Debug, Clone, PartialEq, Default)]
pub struct Track {
    pub events: Vec<Event>,
    /// Bytes after the last event, if any.
    trailer: Vec<u8>,
}

impl Track {
    fn parse(c: &RawChunk) -> Result<Self, Error> {
        let body = c.body;
        if body.len() < 4 {
            return err("TRAK", c.start - 8, "chunk shorter than its event count");
        }
        let n = get_u32(body, 0) as usize;
        let (mut pos, mut p) = (0u32, 4usize);
        let mut events = Vec::with_capacity(n.min(body.len()));
        for _ in 0..n {
            let (d, q) = decode_delta(body, p, c.start)?;
            // u32, as the exe's `abs + delta` (0x436CE5); the engine takes the deltas back with
            // a wrapping subtraction (orb_engine::load::track_batches)
            pos = pos.wrapping_add(d);
            if q + 2 > body.len() {
                return err("TRAK", c.start + q, "truncated event");
            }
            events.push(Event {
                pos,
                ctrl: body[q],
                value: body[q + 1],
            });
            p = q + 2;
        }
        Ok(Track {
            events,
            trailer: body[p..].to_vec(),
        })
    }

    /// The TRAK body. Each delta is `pos - prev` modulo 2^32, the inverse of the reader's u32
    /// add, so every parsed track writes back its own deltas, one whose positions wrap past
    /// 2^32 (a file's huge deltas) included. A position below the previous one is written as
    /// such a wrap, not refused: the callers that build tracks from a model (orb-ui's
    /// `document.rs`) check that their lists are sorted.
    fn build(&self) -> Result<Vec<u8>, Error> {
        let mut out = (self.events.len() as u32).to_be_bytes().to_vec();
        let mut prev = 0u32;
        for e in &self.events {
            encode_delta(e.pos.wrapping_sub(prev), &mut out)?;
            out.extend([e.ctrl, e.value]);
            prev = e.pos;
        }
        out.extend(&self.trailer);
        Ok(out)
    }
}

/// An unknown chunk directly under `RB40`, kept as read.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Unknown {
    /// Index among the RB40 items at which it was found (and is written back).
    pub index: usize,
    pub tag: [u8; 4],
    pub body: Vec<u8>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Song {
    pub head: Head,
    pub glob: Glob,
    pub usri: Usri,
    pub mixer: Mixer,
    pub delay: Params,
    pub pcf: Params,
    pub dist: Params,
    pub comp: Params,
    pub tb303: [Tb303; 2],
    pub tr808: Drums,
    pub tr909: Drums,
    /// By `TRACK_NAMES`.
    pub tracks: [Track; 9],
    pub unknown: Vec<Unknown>,
}

fn is_cat(c: &RawChunk, kind: &[u8; 4]) -> bool {
    &c.tag == b"CAT " && c.body.len() >= 4 && &c.body[..4] == kind
}

impl Song {
    /// An empty 4.2 song for a writer to fill: every parameter 0, 32 empty patterns of 16 steps
    /// in each pattern device, no automation, the built-in sounds ("Standard ReBirth").
    pub fn empty() -> Song {
        let params = |n: usize| Params {
            values: vec![0; n],
            raw: Vec::new(),
        };
        let drums = |kind: DrumKind| Drums {
            kind,
            params: vec![0; kind.count()],
            patterns: (0..32)
                .map(|_| PatternDrum {
                    shuffle: 0,
                    length: 16,
                    steps: [[0; 12]; 16],
                })
                .collect(),
            raw: Vec::new(),
        };
        let tb303 = || Tb303 {
            params: [0; 9],
            patterns: (0..32)
                .map(|_| Pattern303 {
                    shuffle: 0,
                    length: 16,
                    steps: [Step303 {
                        note: 0,
                        slide: 0,
                        accent: 0,
                        up: 0,
                        down: 0,
                        rest: 1,
                        flags_hi: 0,
                    }; 16],
                })
                .collect(),
            raw: Vec::new(),
        };
        let channel = MixChannel {
            enabled: 1,
            level: 0,
            pan: 64,
            delay_send: 0,
            dist: 0,
        };
        Song {
            head: Head {
                major: 4,
                minor: 2,
                revision: 0,
                copyright: String::new(),
                raw: Vec::new(),
            },
            glob: Glob {
                song_mode: 0,
                loop_on: 0,
                tempo: 140_000,
                loop_start: 0,
                loop_end: 4 * 768,
                shuffle: 0,
                mod_name: "Standard ReBirth".into(),
                mod_ftp: String::new(),
                mod_www: String::new(),
                vintage: 0,
                raw: Vec::new(),
            },
            usri: Usri {
                window_title: String::new(),
                text: String::new(),
                web_page: String::new(),
                show_on_open: 0,
                raw: Vec::new(),
            },
            mixer: Mixer {
                master: 0,
                comp_id: 0,
                pcf_id: 0,
                channels: [channel; 4],
                raw: Vec::new(),
            },
            delay: params(DELAY_PARAMS.len()),
            pcf: params(PCF_PARAMS.len()),
            dist: params(DIST_PARAMS.len()),
            comp: params(COMP_PARAMS.len()),
            tb303: [tb303(), tb303()],
            tr808: drums(DrumKind::Tr808),
            tr909: drums(DrumKind::Tr909),
            tracks: Default::default(),
            unknown: Vec::new(),
        }
    }

    pub fn parse(data: &[u8]) -> Result<Song, Error> {
        let top = read_chunks(data, 0, data.len(), "file")?;
        if top.len() != 1 || !is_cat(&top[0], b"RB40") {
            return err("file", 0, "not an RB40 song");
        }
        let (start, size) = (top[0].start, top[0].body.len());
        let items = read_chunks(data, start + 4, start + size, "RB40")?;
        let (mut head, mut glob, mut usri, mut devl, mut trkl) = (None, None, None, None, None);
        let mut unknown = Vec::new();
        for (index, c) in items.iter().enumerate() {
            let dup = || err(&tag_str(&c.tag), c.start - 8, "duplicate chunk");
            match &c.tag {
                b"HEAD" if head.is_some() => dup()?,
                b"GLOB" if glob.is_some() => dup()?,
                b"USRI" if usri.is_some() => dup()?,
                b"HEAD" => head = Some(Head::parse(c)?),
                b"GLOB" => glob = Some(Glob::parse(c)?),
                b"USRI" => usri = Some(Usri::parse(c)?),
                _ if is_cat(c, b"DEVL") && devl.is_some() => dup()?,
                _ if is_cat(c, b"TRKL") && trkl.is_some() => dup()?,
                _ if is_cat(c, b"DEVL") => devl = Some(c),
                _ if is_cat(c, b"TRKL") => trkl = Some(c),
                _ => unknown.push(Unknown {
                    index,
                    tag: c.tag,
                    body: c.body.to_vec(),
                }),
            }
        }
        let missing = |n: &str| Error {
            chunk: n.into(),
            offset: start - 8,
            msg: "missing chunk".into(),
        };
        let head = head.ok_or_else(|| missing("HEAD"))?;
        let glob = glob.ok_or_else(|| missing("GLOB"))?;
        let usri = usri.ok_or_else(|| missing("USRI"))?;
        let devl = devl.ok_or_else(|| missing("DEVL"))?;
        let trkl = trkl.ok_or_else(|| missing("TRKL"))?;

        let d = read_chunks(data, devl.start + 4, devl.start + devl.body.len(), "DEVL")?;
        let want: [&[u8; 4]; 9] = [
            b"MIXR", b"DELY", b"PCF ", b"DIST", b"COMP", b"303 ", b"303 ", b"808 ", b"909 ",
        ];
        if d.len() != 9 || d.iter().zip(want).any(|(c, w)| &c.tag != w) {
            return err(
                "DEVL",
                devl.start - 8,
                "chunks are not MIXR DELY PCF DIST COMP 303 303 808 909 in order",
            );
        }
        let t = read_chunks(data, trkl.start + 4, trkl.start + trkl.body.len(), "TRKL")?;
        if t.len() != 9 || t.iter().any(|c| &c.tag != b"TRAK") {
            return err("TRKL", trkl.start - 8, "must hold 9 TRAK chunks");
        }
        let tracks = [
            Track::parse(&t[0])?,
            Track::parse(&t[1])?,
            Track::parse(&t[2])?,
            Track::parse(&t[3])?,
            Track::parse(&t[4])?,
            Track::parse(&t[5])?,
            Track::parse(&t[6])?,
            Track::parse(&t[7])?,
            Track::parse(&t[8])?,
        ];
        Ok(Song {
            head,
            glob,
            usri,
            mixer: Mixer::parse(&d[0])?,
            delay: Params::parse("DELY", &d[1], 8, DELAY_PARAMS.len())?,
            pcf: Params::parse("PCF ", &d[2], 12, PCF_PARAMS.len())?,
            dist: Params::parse("DIST", &d[3], 8, DIST_PARAMS.len())?,
            comp: Params::parse("COMP", &d[4], 8, COMP_PARAMS.len())?,
            tb303: [Tb303::parse(&d[5])?, Tb303::parse(&d[6])?],
            tr808: Drums::parse(DrumKind::Tr808, &d[7])?,
            tr909: Drums::parse(DrumKind::Tr909, &d[8])?,
            tracks,
            unknown,
        })
    }

    pub fn write(&self) -> Result<Vec<u8>, Error> {
        let devl = cat(
            b"DEVL",
            &[
                chunk(b"MIXR", &self.mixer.build()),
                chunk(b"DELY", &self.delay.build("DELY", 8, DELAY_PARAMS.len())?),
                chunk(b"PCF ", &self.pcf.build("PCF ", 12, PCF_PARAMS.len())?),
                chunk(b"DIST", &self.dist.build("DIST", 8, DIST_PARAMS.len())?),
                chunk(b"COMP", &self.comp.build("COMP", 8, COMP_PARAMS.len())?),
                chunk(b"303 ", &self.tb303[0].build()?),
                chunk(b"303 ", &self.tb303[1].build()?),
                chunk(b"808 ", &self.tr808.build()?),
                chunk(b"909 ", &self.tr909.build()?),
            ],
        );
        let mut traks = Vec::new();
        for t in &self.tracks {
            traks.push(chunk(b"TRAK", &t.build()?));
        }
        let trkl = cat(b"TRKL", &traks);
        let mut items = vec![
            chunk(b"HEAD", &self.head.build()?),
            chunk(b"GLOB", &self.glob.build()?),
            chunk(b"USRI", &self.usri.build()?),
            devl,
            trkl,
        ];
        let mut unk: Vec<&Unknown> = self.unknown.iter().collect();
        unk.sort_by_key(|u| u.index);
        for u in unk {
            items.insert(u.index.min(items.len()), chunk(&u.tag, &u.body));
        }
        Ok(cat(b"RB40", &items))
    }
}

// ---------------------------------------------------------- controller names

fn mixer_name(id: u8) -> Option<String> {
    match id {
        1 => Some("comp_id".into()),
        2 => Some("pcf_id".into()),
        6..=29 => {
            let i = (id - 6) as usize;
            Some(format!(
                "{}_{}",
                CHANNELS[i / 6],
                ["level", "pan", "delay_send", "dist"].get(i % 6)?
            ))
        }
        _ => None,
    }
}

/// The name of controller `id` of the track `track` (an index into `TRACK_NAMES`), if it has one.
pub fn controller_name(track: usize, id: u8) -> Option<String> {
    let table: &[&str] = match track {
        0 => return mixer_name(id),
        1 | 2 => &TB303_PARAMS,
        3 => &TR808_PARAMS,
        4 => &TR909_PARAMS,
        5 => &DELAY_PARAMS,
        6 => &DIST_PARAMS,
        7 => &PCF_PARAMS,
        8 => &COMP_PARAMS,
        _ => return None,
    };
    table.get(id as usize).map(|s| s.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn silent() -> Vec<u8> {
        let p = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../../tests/songs/shipped/Silent Default Song.rbs");
        std::fs::read(p).unwrap()
    }

    /// The RB40 items of a song as (tag, whole chunk bytes).
    fn items(data: &[u8]) -> Vec<([u8; 4], Vec<u8>)> {
        let top = read_chunks(data, 0, data.len(), "file").unwrap();
        let (s, n) = (top[0].start, top[0].body.len());
        read_chunks(data, s + 4, s + n, "RB40")
            .unwrap()
            .iter()
            .map(|c| (c.tag, chunk(&c.tag, c.body)))
            .collect()
    }

    fn assemble(items: &[Vec<u8>]) -> Vec<u8> {
        cat(b"RB40", items)
    }

    #[test]
    fn reordered_items_parse_and_write_canonically() {
        let data = silent();
        let mut it: Vec<Vec<u8>> = items(&data).into_iter().map(|(_, b)| b).collect();
        it.swap(0, 1); // GLOB, HEAD, ...
        let odd = assemble(&it);
        assert_ne!(odd, data);
        let song = Song::parse(&odd).unwrap();
        assert_eq!(song.write().unwrap(), data);
        assert_eq!(Song::parse(&song.write().unwrap()).unwrap(), song);
    }

    /// Replaces the body of the mixer TRAK (the first) in a song file.
    fn with_mixer_track(data: &[u8], body: &[u8]) -> Vec<u8> {
        let mut it = items(data);
        let trkl = it.iter_mut().find(|(_, b)| &b[8..12] == b"TRKL").unwrap();
        let top = read_chunks(&trkl.1, 0, trkl.1.len(), "t").unwrap();
        let traks = read_chunks(top[0].body, 4, top[0].body.len(), "TRKL").unwrap();
        let mut ch: Vec<Vec<u8>> = traks.iter().map(|c| chunk(b"TRAK", c.body)).collect();
        ch[0] = chunk(b"TRAK", body);
        trkl.1 = cat(b"TRKL", &ch);
        assemble(&it.into_iter().map(|(_, b)| b).collect::<Vec<_>>())
    }

    #[test]
    fn noncanonical_delta_and_trailer_are_normalized_but_kept() {
        // count 1, delta 0x80 0x80 0x01 (= 1, three bytes), ctrl 6, value 9, then 2 trailing bytes.
        let body = [0, 0, 0, 1, 0x80, 0x80, 0x01, 6, 9, 0xAA, 0xBB];
        let odd = with_mixer_track(&silent(), &body);
        let song = Song::parse(&odd).unwrap();
        assert_eq!(
            song.tracks[0].events,
            [Event {
                pos: 1,
                ctrl: 6,
                value: 9
            }]
        );
        let out = song.write().unwrap();
        assert_ne!(out, odd);
        let again = Song::parse(&out).unwrap();
        assert_eq!(again, song);
        assert_eq!(again.write().unwrap(), out);
        // the canonical body: count, delta 1, ctrl, value, trailer kept
        assert!(
            out.windows(9)
                .any(|w| w == [0, 0, 0, 1, 1, 6, 9, 0xAA, 0xBB])
        );
    }

    #[test]
    fn a_delta_of_any_length_reads_as_the_exe_does() {
        // A 5-byte delta (8f ff fa 94 00, from a third-party song): 0xFFFE8A00, as sub_4A09D1's
        // 32-bit shift gives; then 6 bytes, whose top bits are shifted out (0x81 x5, 0x02 is
        // 2^35 + .. + 2 modulo 2^32). Both read, and the 5-byte one writes back unchanged.
        let body = [0, 0, 0, 2, 0, 6, 9, 0x8f, 0xff, 0xfa, 0x94, 0x00, 7, 1];
        let odd = with_mixer_track(&silent(), &body);
        let song = Song::parse(&odd).unwrap();
        let ev = |pos, ctrl, value| Event { pos, ctrl, value };
        assert_eq!(song.tracks[0].events, [ev(0, 6, 9), ev(0xFFFE_8A00, 7, 1)]);
        assert_eq!(song.write().unwrap(), odd);

        let body = [0, 0, 0, 1, 0x81, 0x81, 0x81, 0x81, 0x81, 0x02, 6, 9];
        let song = Song::parse(&with_mixer_track(&silent(), &body)).unwrap();
        let v = (0..6).fold(0u32, |v, i| (v << 7) | [1, 1, 1, 1, 1, 2][i]);
        assert_eq!(song.tracks[0].events, [ev(v, 6, 9)]);
        let back = Song::parse(&song.write().unwrap()).unwrap();
        assert_eq!(back, song);
    }

    #[test]
    fn a_track_that_wraps_past_2_32_writes_back_its_deltas() {
        // deltas 0xFFFFFFF0 (8f ff ff ff 70) then 0x20: positions 0xFFFFFFF0 and 0x10
        let body = [0, 0, 0, 2, 0x8f, 0xff, 0xff, 0xff, 0x70, 6, 9, 0x20, 7, 1];
        let odd = with_mixer_track(&silent(), &body);
        let song = Song::parse(&odd).unwrap();
        let ev = |pos, ctrl, value| Event { pos, ctrl, value };
        assert_eq!(
            song.tracks[0].events,
            [ev(0xFFFF_FFF0, 6, 9), ev(0x10, 7, 1)]
        );
        assert_eq!(song.write().unwrap(), odd);

        // every backward step is a wrap, whatever its delta: 0x10 then 0 (delta 0xFFFFFFF0),
        // 0x80000000 then 0 (delta 2^31) and 5 then 4 (delta 2^32 - 1) all write and read back
        for evs in [
            vec![ev(0x10, 6, 9), ev(0, 7, 1)],
            vec![ev(0x8000_0000, 6, 9), ev(0, 7, 1)],
            vec![ev(5, 6, 9), ev(4, 7, 1)],
        ] {
            let mut s = song.clone();
            s.tracks[0].events = evs.clone();
            let bytes = s.write().unwrap();
            let back = Song::parse(&bytes).unwrap();
            assert_eq!(back.tracks[0].events, evs);
            assert_eq!(back.write().unwrap(), bytes);
        }
        // 0x10 then delta 0xFFFFFFF0, as a file holds it, round-trips byte for byte
        let body = [0, 0, 0, 2, 0x10, 6, 9, 0x8f, 0xff, 0xff, 0xff, 0x70, 7, 1];
        let odd = with_mixer_track(&silent(), &body);
        let song = Song::parse(&odd).unwrap();
        assert_eq!(song.tracks[0].events, [ev(0x10, 6, 9), ev(0, 7, 1)]);
        assert_eq!(song.write().unwrap(), odd);
    }

    #[test]
    fn the_empty_song_writes_and_reads_back() {
        let s = Song::empty();
        let bytes = s.write().unwrap();
        let back = Song::parse(&bytes).unwrap();
        assert_eq!(back.head.version(), "4.2");
        assert_eq!(back.glob.mod_name, "Standard ReBirth");
        assert_eq!(back.tb303[1].patterns.len(), 32);
        assert_eq!(back.tr909.patterns[31].length, 16);
        assert_eq!(back.write().unwrap(), bytes);
    }

    #[test]
    fn empty_track_is_accepted() {
        let odd = with_mixer_track(&silent(), &[0, 0, 0, 0]);
        let song = Song::parse(&odd).unwrap();
        assert!(song.tracks[0].events.is_empty());
        assert_eq!(song.write().unwrap(), odd);
    }

    #[test]
    fn wrong_model_shapes_are_errors_not_panics() {
        let base = Song::parse(&silent()).unwrap();
        let mut s = base.clone();
        s.delay.values.push(1);
        assert!(s.write().is_err());
        let mut s = base.clone();
        s.pcf.values.clear();
        assert!(s.write().is_err());
        let mut s = base.clone();
        s.tb303[0].patterns.pop();
        assert!(s.write().is_err());
        let mut s = base.clone();
        s.tr909.patterns.pop();
        assert!(s.write().is_err());
        let mut s = base.clone();
        s.tr808.params.pop();
        assert!(s.write().is_err());
        let mut s = base.clone();
        s.glob.mod_name = "x".repeat(65);
        assert!(s.write().is_err());
        // A position below the one before is no error since task-129: the writer stores it as
        // a u32 wrap, which reads back the same; the order of a model's tracks is checked where
        // they are built (orb-ui `document::set_engine_tracks`).
        let mut s = base;
        s.tracks[0].events = vec![
            Event {
                pos: 5,
                ctrl: 0,
                value: 0,
            },
            Event {
                pos: 1,
                ctrl: 0,
                value: 0,
            },
        ];
        let back = Song::parse(&s.write().unwrap()).unwrap();
        assert_eq!(back.tracks[0].events, s.tracks[0].events);
    }

    #[test]
    fn error_tags_are_trimmed_and_escaped() {
        assert_eq!(tag_str(b"PCF "), "PCF");
        assert_eq!(tag_str(&[0xff, b'A', 0, b' ']), "\\xffA\\x00");
        let e = err::<()>("303 ", 7, "x").unwrap_err();
        assert_eq!(e.chunk, "303");
    }
}
