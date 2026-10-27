//! The song as the engine holds it after a song load, before an export starts.
//!
//! This is what ReBirth.dll receives from Rebirth.exe when a song is loaded
//! (`docs/song_load.md`): the global struct (song msg 36), one parameter and pattern block per
//! device (song msg 41), and the track events (song msg 37). The blocks keep the DLL's own
//! layouts, so a device's `params[c]` is the byte that controller `c` of its track sets, and every
//! byte the original sends is kept, including the constant ones. `crate::load` builds a [`Song`]
//! from an `.rbs` file; the `payload` methods give the bytes of the original's messages.

/// Engine ticks per quarter note.
pub const PPQ: u32 = 192;
/// Engine ticks per bar (the unit of the GLOB loop points).
pub const TICKS_PER_BAR: u32 = 4 * PPQ;
/// Engine ticks per RBS track position (a 32nd note): `tick = pos * 24`.
pub const TICKS_PER_POS: u32 = 24;
/// Track events at or beyond this tick (1000 bars) are dropped at load.
pub const MAX_TICK: u32 = 1000 * TICKS_PER_BAR;

/// The engine's devices. The discriminant is the device id of song msg 41 and the TRAK track
/// number (`docs/song_load.md` section 1).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[repr(u8)]
pub enum Device {
    Mixer = 0,
    Tb303_1 = 1,
    Tb303_2 = 2,
    Tr808 = 3,
    Tr909 = 4,
    Delay = 5,
    Distortion = 6,
    Pcf = 7,
    Compressor = 8,
}

impl Device {
    /// In id (= track) order.
    pub const ALL: [Device; 9] = [
        Device::Mixer,
        Device::Tb303_1,
        Device::Tb303_2,
        Device::Tr808,
        Device::Tr909,
        Device::Delay,
        Device::Distortion,
        Device::Pcf,
        Device::Compressor,
    ];

    pub fn id(self) -> u8 {
        self as u8
    }

    pub fn from_id(id: u8) -> Option<Device> {
        Device::ALL.get(id as usize).copied()
    }

    /// The `sub` byte of the device's data message (song msg 41), which each device's host-message
    /// handler takes as "here is my parameter and pattern block".
    pub fn data_sub(self) -> u8 {
        match self {
            Device::Mixer => 143,
            Device::Tb303_1 | Device::Tb303_2 => 39,
            Device::Tr808 => 56,
            Device::Tr909 => 73,
            Device::Delay => 86,
            Device::Distortion => 94,
            Device::Pcf => 116,
            Device::Compressor => 104,
        }
    }

    /// The length of the data block (msg 41 payload after `dev, sub`).
    pub fn data_len(self) -> usize {
        match self {
            Device::Mixer => Mixer::PAYLOAD_LEN,
            Device::Tb303_1 | Device::Tb303_2 => Tb303::PAYLOAD_LEN,
            Device::Tr808 => Tr808::PAYLOAD_LEN,
            Device::Tr909 => Tr909::PAYLOAD_LEN,
            Device::Delay => Delay::PAYLOAD_LEN,
            Device::Distortion => Distortion::PAYLOAD_LEN,
            Device::Pcf => Pcf::PAYLOAD_LEN,
            Device::Compressor => Compressor::PAYLOAD_LEN,
        }
    }
}

/// The song's global struct (DLL `song+0x158`), as song msg 36 sets it at load
/// (`docs/song_load.md` section 3). Each byte offset is also the msg 35 id that sets the field.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Globals {
    /// Byte 0: 0 pattern mode, 1 song mode. In song mode a locate chases the tracks.
    pub song_mode: u8,
    /// Byte 1: play (0 = stopped at load).
    pub play: u8,
    /// Byte 2: 0 at load; meaning not traced (msg 35 id 2, DLL `0x10021194`).
    pub byte2: u8,
    /// Byte 3.
    pub loop_on: u8,
    /// Bytes 4..7: BPM * 1000.
    pub tempo: u32,
    /// Bytes 8..11: the tick to locate to (0 at load).
    pub locate: u32,
    /// Bytes 12..15: in ticks (bar * 768), not scaled.
    pub loop_start: u32,
    /// Bytes 16..19: in ticks.
    pub loop_end: u32,
    /// Byte 20: 0..127.
    pub shuffle: u8,
    /// Byte 21, DLL `song+0x16D`: the sound model. Zero is "ReBirth 2.0 sound", non-zero is
    /// "Vintage" (the 303 engine of `0x10023F10` and the pan-dependent mixer send).
    pub vintage: u8,
    /// Byte 22, DLL `song+0x16E`: 1 at load; no reader found in the DLL.
    pub byte22: u8,
}

impl Globals {
    pub const PAYLOAD_LEN: usize = 23;

    /// Whether the Vintage sound model is selected (the DLL tests the byte against zero,
    /// 0x10023F2D and 0x10012C9F).
    pub fn is_vintage(&self) -> bool {
        self.vintage != 0
    }

    /// The msg 36 payload (multi-byte values big-endian).
    pub fn payload(&self) -> [u8; Self::PAYLOAD_LEN] {
        let mut b = [0; Self::PAYLOAD_LEN];
        b[0] = self.song_mode;
        b[1] = self.play;
        b[2] = self.byte2;
        b[3] = self.loop_on;
        b[4..8].copy_from_slice(&self.tempo.to_be_bytes());
        b[8..12].copy_from_slice(&self.locate.to_be_bytes());
        b[12..16].copy_from_slice(&self.loop_start.to_be_bytes());
        b[16..20].copy_from_slice(&self.loop_end.to_be_bytes());
        b[20] = self.shuffle;
        b[21] = self.vintage;
        b[22] = self.byte22;
        b
    }
}

/// One mixer channel. Channel c is 0 TB-303 #1, 1 TB-303 #2, 2 TR-808, 3 TR-909.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct MixerChannel {
    /// Not automatable.
    pub enabled: u8,
    pub level: u8,
    pub pan: u8,
    pub delay_send: u8,
    /// The distortion insert on this channel (0/1).
    pub dist: u8,
}

/// The mixer block (`docs/song_load.md` section 4.1; DLL unpack `sub_10019DE0`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Mixer {
    /// Not automatable.
    pub master: u8,
    /// 0 off, 1 the stereo compressor on the master, 2..5 the mono compressor on channel 0..3.
    pub comp_id: u8,
    /// 0 off, 1..4 the PCF on channel 0..3.
    pub pcf_id: u8,
    pub channels: [MixerChannel; 4],
}

impl Mixer {
    pub const PAYLOAD_LEN: usize = 29;
    /// Mixer track controller of the compressor id.
    pub const CTRL_COMP_ID: u8 = 1;
    /// Mixer track controller of the PCF id.
    pub const CTRL_PCF_ID: u8 = 2;

    /// The mixer track controller of channel `c`'s field `k` (0 level, 1 pan, 2 delay send,
    /// 3 dist): `6 + 6c + k`. The DLL's setParam (`0x10015C00`) decodes `(ctrl-5)/6` and
    /// `(ctrl-5)%6`.
    pub fn channel_ctrl(c: usize, k: usize) -> u8 {
        (6 + 6 * c + k) as u8
    }

    /// The msg 41 `0/143` payload: master, comp id, PCF id, two zero bytes, then per channel
    /// enabled, level, pan, delay send, dist and a zero byte.
    pub fn payload(&self) -> [u8; Self::PAYLOAD_LEN] {
        let mut b = [0; Self::PAYLOAD_LEN];
        b[0] = self.master;
        b[1] = self.comp_id;
        b[2] = self.pcf_id;
        for (c, ch) in self.channels.iter().enumerate() {
            let o = 5 + 6 * c;
            b[o..o + 5].copy_from_slice(&[ch.enabled, ch.level, ch.pan, ch.delay_send, ch.dist]);
        }
        b
    }
}

macro_rules! param_block {
    ($(#[$m:meta])* $name:ident, $len:expr, [$($(#[$cm:meta])* $c:ident = $i:expr),* $(,)?]) => {
        $(#[$m])*
        #[derive(Debug, Clone, PartialEq, Eq)]
        pub struct $name {
            /// The block as the DLL receives it; controller `c` of the track sets `params[c]`.
            pub params: [u8; $len],
        }

        impl $name {
            pub const PAYLOAD_LEN: usize = $len;
            $($(#[$cm])* pub const $c: usize = $i;)*

            pub fn payload(&self) -> [u8; $len] {
                self.params
            }
        }
    };
}

param_block!(
    /// The delay (`docs/song_load.md` section 4.2). Byte 5 is 0.
    Delay, 6, [
        ENABLED = 0,
        /// 1..32.
        STEPS = 1,
        /// 0 = triplet steps (1/3 beat), 1 = sixteenths (1/4 beat).
        MODE = 2,
        FEEDBACK = 3,
        PAN = 4,
    ]
);

param_block!(
    /// The PCF (`docs/song_load.md` section 4.3). Byte 7 is 0.
    Pcf, 8, [
        ENABLED = 0,
        FREQUENCY = 1,
        RESONANCE = 2,
        AMOUNT = 3,
        WAVE = 4,
        DECAY = 5,
        /// 0 band-pass, 1 low-pass.
        MODE = 6,
    ]
);

param_block!(
    /// The distortion (`docs/song_load.md` section 4.4). Byte 3 is 0.
    Distortion, 4, [ENABLED = 0, AMOUNT = 1, SHAPE = 2]
);

param_block!(
    /// The compressor (`docs/song_load.md` section 4.5). Bytes 3 and 4 are always 127 and 0
    /// (the exe's defaults, `sub_42311F`); no file field or controller sets them.
    Compressor, 5, [ENABLED = 0, AMOUNT = 1, THRESHOLD = 2]
);

/// A TB-303 step as the DLL stores it: six bytes, each 0 or 1 except the note.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct Step303 {
    /// 0..12.
    pub note: u8,
    pub slide: u8,
    pub accent: u8,
    /// Transpose up.
    pub up: u8,
    /// Transpose down.
    pub down: u8,
    /// 1 = rest: the note-on plays only when this is 0 (DLL 0x10023E04).
    pub rest: u8,
}

/// A TB-303 pattern: 98 bytes in the DLL.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct Pattern303 {
    /// 0 = shuffle off (the step function then uses the unshuffled tick, 0x100253D0).
    pub shuffle: u8,
    pub length: u8,
    pub steps: [Step303; 16],
}

impl Pattern303 {
    pub const LEN: usize = 98;
}

/// A TB-303 (`docs/song_load.md` section 4.6; DLL unpack `sub_10018EF0`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Tb303 {
    /// Controllers 0..8 (enabled, pattern, tune, cutoff, resonance, envmod, decay, accent,
    /// waveform 0 saw / 1 square), then three zero bytes.
    pub params: [u8; 12],
    pub patterns: [Pattern303; 32],
}

impl Tb303 {
    pub const PAYLOAD_LEN: usize = 12 + 32 * Pattern303::LEN;
    pub const ENABLED: usize = 0;
    pub const PATTERN: usize = 1;
    pub const TUNE: usize = 2;
    pub const CUTOFF: usize = 3;
    pub const RESONANCE: usize = 4;
    pub const ENVMOD: usize = 5;
    pub const DECAY: usize = 6;
    pub const ACCENT: usize = 7;
    pub const WAVEFORM: usize = 8;

    /// The msg 41 `1/39` (or `2/39`) payload.
    pub fn payload(&self) -> Vec<u8> {
        let mut b = Vec::with_capacity(Self::PAYLOAD_LEN);
        b.extend_from_slice(&self.params);
        for p in &self.patterns {
            b.extend([p.shuffle, p.length]);
            for s in &p.steps {
                b.extend([s.note, s.slide, s.accent, s.up, s.down, s.rest]);
            }
        }
        b
    }
}

/// A drum pattern: 194 bytes in the DLL; `steps[s][i]` is step s of instrument column i.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct DrumPattern {
    pub shuffle: u8,
    pub length: u8,
    pub steps: [[u8; 12]; 16],
}

impl DrumPattern {
    pub const LEN: usize = 194;
}

/// A drum machine whose parameter block is `P` bytes.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DrumMachine<const P: usize> {
    /// Controller c sets `params[c]`; the bytes after the last controller are 0.
    pub params: [u8; P],
    pub patterns: [DrumPattern; 32],
}

impl<const P: usize> DrumMachine<P> {
    pub const PAYLOAD_LEN: usize = P + 32 * DrumPattern::LEN;

    pub fn payload(&self) -> Vec<u8> {
        let mut b = Vec::with_capacity(Self::PAYLOAD_LEN);
        b.extend_from_slice(&self.params);
        for p in &self.patterns {
            b.extend([p.shuffle, p.length]);
            for s in &p.steps {
                b.extend_from_slice(s);
            }
        }
        b
    }
}

/// The TR-808 (`docs/song_load.md` section 4.7): controllers 0..28 in RBS42 order, then three
/// zero bytes. Step columns AC, BD, SD, LT, MT, HT, RS, CP, CB, CY, OH, CH, each 0 or 1.
pub type Tr808 = DrumMachine<32>;

/// The TR-909 (`docs/song_load.md` section 4.8): controllers 0..29 in RBS42 order, then three
/// zero bytes. Step columns AC, BD, SD, LT, MT, HT, RS, CP, CH, OH, CC, RC, each 0 off, 1 on,
/// 2 accented, 3 flam (the DLL step function `0x1002AAC0` reads them raw).
pub type Tr909 = DrumMachine<33>;

/// A track event in engine ticks, as msg 37 delivers it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TrackEvent {
    /// A multiple of 24 below [`MAX_TICK`].
    pub tick: u32,
    pub ctrl: u8,
    pub value: u8,
}

/// Which reader of the exe built the song, which decides the device messages of the load.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum LoadForm {
    /// An RBS file (`sub_434D05`): the nine msg-41 blocks in [`LoadForm::device_blocks`] order.
    #[default]
    Rbs,
    /// A ReBirth 1.x MIDI file (`sub_430BA0`, `docs/song_load.md` section 12): no compressor
    /// and no 909 block (both keep the constructor's state), and after the 808 block the 909's
    /// `4/74 {0, 0}`, which sets its live enabled byte to 0 (DLL 0x1002BF12, like slot 11).
    Rb1x,
}

impl LoadForm {
    /// The devices whose msg-41 data block the load sends, in the exe's order.
    pub fn device_blocks(self) -> &'static [Device] {
        use Device as D;
        match self {
            LoadForm::Rbs => &[
                D::Mixer,
                D::Delay,
                D::Pcf,
                D::Distortion,
                D::Compressor,
                D::Tb303_1,
                D::Tb303_2,
                D::Tr808,
                D::Tr909,
            ],
            // sub_431747
            LoadForm::Rb1x => &[
                D::Mixer,
                D::Delay,
                D::Distortion,
                D::Pcf,
                D::Tb303_1,
                D::Tb303_2,
                D::Tr808,
            ],
        }
    }
}

/// Everything the engine holds for a song after the load.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Song {
    /// The reader that built it; [`LoadForm::Rb1x`] songs carry the constructor's compressor and
    /// 909 blocks, which the load does not send.
    pub form: LoadForm,
    /// The mod named in GLOB ("Standard ReBirth" for the built-in sounds). The message stream does
    /// not carry it; it selects the sample set the song is opened with (msg 52, m-2).
    pub mod_name: String,
    pub globals: Globals,
    pub mixer: Mixer,
    pub delay: Delay,
    pub pcf: Pcf,
    pub dist: Distortion,
    pub comp: Compressor,
    pub tb303: [Tb303; 2],
    pub tr808: Tr808,
    pub tr909: Tr909,
    /// By track number (= [`Device`] id), sorted by tick as in the file; each track holds only
    /// the events the load accepted.
    pub tracks: [Vec<TrackEvent>; 9],
}

impl Song {
    /// The data block of device `dev` (the msg 41 payload after `dev, sub`).
    pub fn device_payload(&self, dev: Device) -> Vec<u8> {
        match dev {
            Device::Mixer => self.mixer.payload().to_vec(),
            Device::Tb303_1 => self.tb303[0].payload(),
            Device::Tb303_2 => self.tb303[1].payload(),
            Device::Tr808 => self.tr808.payload(),
            Device::Tr909 => self.tr909.payload(),
            Device::Delay => self.delay.payload().to_vec(),
            Device::Distortion => self.dist.payload().to_vec(),
            Device::Pcf => self.pcf.payload().to_vec(),
            Device::Compressor => self.comp.payload().to_vec(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn units() {
        assert_eq!(TICKS_PER_BAR, 768);
        assert_eq!(MAX_TICK, 768_000);
        assert_eq!(32 * TICKS_PER_POS, TICKS_PER_BAR);
    }

    #[test]
    fn payload_lengths() {
        assert_eq!(Tb303::PAYLOAD_LEN, 3148);
        assert_eq!(Tr808::PAYLOAD_LEN, 6240);
        assert_eq!(Tr909::PAYLOAD_LEN, 6241);
        let lens: Vec<_> = Device::ALL.iter().map(|d| d.data_len()).collect();
        assert_eq!(lens, [29, 3148, 3148, 6240, 6241, 6, 4, 8, 5]);
        let subs: Vec<_> = Device::ALL.iter().map(|d| d.data_sub()).collect();
        assert_eq!(subs, [143, 39, 39, 56, 73, 86, 94, 116, 104]);
        for d in Device::ALL {
            assert_eq!(Device::from_id(d.id()), Some(d));
        }
        assert_eq!(Device::from_id(9), None);
    }

    #[test]
    fn mixer_payload_layout() {
        let mut m = Mixer {
            master: 100,
            comp_id: 3,
            pcf_id: 4,
            channels: [MixerChannel::default(); 4],
        };
        for (c, ch) in m.channels.iter_mut().enumerate() {
            let c = c as u8;
            *ch = MixerChannel {
                enabled: 1,
                level: 10 + c,
                pan: 20 + c,
                delay_send: 30 + c,
                dist: c & 1,
            };
        }
        let p = m.payload();
        assert_eq!(&p[..5], &[100, 3, 4, 0, 0]);
        assert_eq!(&p[5..11], &[1, 10, 20, 30, 0, 0]);
        assert_eq!(&p[23..29], &[1, 13, 23, 33, 1, 0]);
        assert_eq!(Mixer::channel_ctrl(0, 0), 6);
        assert_eq!(Mixer::channel_ctrl(3, 2), 0x1A);
        assert_eq!(Mixer::channel_ctrl(3, 3), 0x1B);
    }

    #[test]
    fn globals_payload_layout() {
        let g = Globals {
            song_mode: 1,
            play: 0,
            byte2: 0,
            loop_on: 1,
            tempo: 0x0001_D4C0,
            locate: 0,
            loop_start: 768,
            loop_end: 0x0102_0304,
            shuffle: 50,
            vintage: 1,
            byte22: 1,
        };
        let p = g.payload();
        assert_eq!(
            p,
            [
                1, 0, 0, 1, 0, 1, 0xD4, 0xC0, 0, 0, 0, 0, 0, 0, 3, 0, 1, 2, 3, 4, 50, 1, 1
            ]
        );
        assert!(g.is_vintage());
    }
}
