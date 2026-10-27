//! The TR-808 (`CTR808Device`, vtable 0x10034154, 0x1910 bytes, ctor `sub_10026F70`;
//! `docs/initial_investigation.md` sections 7.1..7.3).
//!
//! The step function (slot 1, 0x10027690) plays the current pattern on the sequencer's ticks:
//! on each new step it computes the accent and triggers the step's instruments in ascending
//! column order (`sub_10027894`). Each instrument plays on its own voice of the classes in
//! [`crate::drums`]; the bass drum alternates between two voices through a process-global flag,
//! and the snare's noise sample follows a fixed sequence indexed by a process-global counter
//! (both in [`ProcessState`]). The audio (slot 14, 0x100280B0) zeroes the bus and adds the voices
//! in a fixed order.
//!
//! The device keeps the DLL's fields; each field's doc names its offset.

#[cfg(test)]
mod tests;

use std::sync::Arc;

use crate::drums::{Dual, Filter, Pitched, Simple};
use crate::frame::{Block, BlockContext, ChannelDevice, SongDevice};
use crate::process::ProcessState;
use crate::samples::{Sample, SampleSet};
use crate::seq::Track;
use crate::tables::data::{
    TR808_DECAY, TR808_SD_NOISE_SAMPLE, TR808_SD_NOISE_SEQ, TR808_SD_SAMPLE, TR808_TONE,
};

/// The parameter block (`+0x30` pattern-mode copy, `+0x1890` live): controllers 0..28 in RBS42
/// order, then the step function's last step at byte 29 and two unused bytes.
pub type Params = [u8; 32];
pub const ENABLED: usize = 0;
pub const PATTERN: usize = 1;
pub const ACCENT: usize = 2;
pub const BD_LEVEL: usize = 3;
pub const BD_TONE: usize = 4;
pub const BD_DECAY: usize = 5;
pub const SD_LEVEL: usize = 6;
pub const SD_TONE: usize = 7;
pub const SD_SNAPPY: usize = 8;
/// The toms' level, tuning and selector: low at 9, mid at 12, high at 15.
pub const LT_LEVEL: usize = 9;
pub const MT_LEVEL: usize = 12;
pub const HT_LEVEL: usize = 15;
pub const RS_LEVEL: usize = 18;
pub const RS_SELECT: usize = 19;
pub const CP_LEVEL: usize = 20;
pub const CP_SELECT: usize = 21;
pub const CB_LEVEL: usize = 22;
pub const CY_LEVEL: usize = 23;
pub const CY_TONE: usize = 24;
pub const CY_DECAY: usize = 25;
pub const OH_LEVEL: usize = 26;
pub const OH_DECAY: usize = 27;
pub const CH_LEVEL: usize = 28;
/// The last controller.
pub const LAST_CTRL: usize = CH_LEVEL;
/// Byte 29 of the live block (`+0x18AD`): the last step the step function played.
const LAST_STEP: usize = 29;
/// One pattern: shuffle flag, length, 16 steps of 12 columns.
pub const PATTERN_LEN: usize = 194;
pub const PATTERNS: usize = 32;
/// The step columns: 0 the accent, then the instruments in trigger order.
pub const AC: usize = 0;
pub const BD: usize = 1;
pub const SD: usize = 2;
pub const LT: usize = 3;
pub const MT: usize = 4;
pub const HT: usize = 5;
pub const RS: usize = 6;
pub const CP: usize = 7;
pub const CB: usize = 8;
pub const CY: usize = 9;
pub const OH: usize = 10;
pub const CH: usize = 11;

/// The parameter block of `sub_10018FB0` (ctor, both copies).
pub const DEFAULT_PARAMS: Params = [
    1, 0, 40, 100, 64, 64, 100, 64, 64, 100, 64, 0, 100, 64, 0, 100, 64, 0, 100, 0, 100, 0, 100,
    100, 0, 100, 100, 100, 100, 0, 0, 0,
];

/// The device's sample slots (`+0x18D8`, the mod's table: slot `n` at `+4n`), with the built-in
/// kit slot `n` = resource `999 + n` (`sub_1001A030`).
pub mod slot {
    pub const BD: usize = 1;
    pub const CB: usize = 11;
    pub const CH: usize = 12;
    pub const CL: usize = 13;
    pub const CP: usize = 14;
    pub const CY: usize = 15;
    pub const HC: usize = 16;
    pub const HT: usize = 17;
    pub const LC: usize = 18;
    pub const LT: usize = 19;
    pub const MA: usize = 20;
    pub const MC: usize = 21;
    pub const MT: usize = 22;
    pub const OH: usize = 23;
    pub const RS: usize = 24;
}

// The step function and the triggers (0x10153F40.., 0x10154200..).
/// `flt_10153F48`: the accent step.
const ACC_STEP: f32 = 0.03125;
/// `dbl_10153F40`.
const ACC_BASE: f64 = 1.0;
/// `flt_10153F4C`, `flt_10154200`, `flt_10154224`, `flt_1015422C`: f32 1/127.
const K: f32 = f32::from_bits(0x3C01_0204);
/// `flt_10153F50`, `flt_10154228`: the level scale of BD, CY, RS, CP, CB, OH and CH.
const LEVEL_SCALE: f32 = f32::from_bits(0x3BA3_D70A); // 0.005
/// `flt_10154230`: the snare's level scale.
const SD_SCALE: f32 = f32::from_bits(0x3CCC_CCCD); // 0.025
/// `dbl_10154218`: `1 - c` of the bass drum.
const ONE: f64 = 1.0;
/// The bass drum's fade of its other voice (`mov eax, 3F7D70A4h` at 0x1002A0E5).
const BD_FADE: f32 = f32::from_bits(0x3F7D_70A4); // 0.99
/// The open hi-hat's multiplier (`mov esi, 3F7F7CEEh` at 0x10027E4F).
const OH_DECAY_M: f32 = f32::from_bits(0x3F7F_7CEE); // 0.998
/// `mov ecx, 3F800000h` at 0x100278BF: the multiplier of every other instrument.
const NO_DECAY: f32 = 1.0;

fn st(x: f64) -> f32 {
    x as f32
}

/// A TR-808.
#[derive(Debug, Clone)]
pub struct Tr808 {
    /// `+4` (CDevice): the tick of the bar the pattern started at (song mode).
    pub pat_start: i32,
    /// `+0x30`: the pattern-mode parameter copy.
    pub params: Params,
    /// `+0x50`: the 32 patterns, 194 bytes each, as msg 56 sends them.
    pub patterns: Box<[u8; PATTERNS * PATTERN_LEN]>,
    /// `+0x1890`: the live parameters.
    pub live: Params,
    /// `+0x18D0`: the playing pattern.
    pub cur_pattern: u8,
    /// `+0x18D1`: the enable flag latched at the bar line.
    pub enabled: u8,
    /// `+0x18D2`: set by a pattern wrap and by play; forces the next step to trigger.
    pub wrapped: bool,
    /// `+0x18D4`: the pattern position of the last step call.
    pub last_pos: i32,
    /// `+0x18DC`: the SIMPLE voices of RS/CL, CP/MA, CB, OH and CH.
    pub simple: [Simple; 5],
    /// `+0x18F0`: the PITCHED voices of LT/LC, MT/MC and HT/HC.
    pub pitched: [Pitched; 3],
    /// `+0x18FC`: the FILTER voices of the two bass drums and CY.
    pub filter: [Filter; 3],
    /// `+0x1908`: the snare's DUAL voice.
    pub dual: Dual,
    /// `+0x190C`: the accent of the last step played.
    pub accent: f32,
    /// The song's global byte 0 (`sub_1001FCB0`), followed through slot 9.
    pub song_mode: bool,
    /// `+0x18D8`: the sample table.
    samples: Arc<SampleSet>,
    /// The process globals (the BD toggle `word_1014CBD0`, the snare counter `word_1014CBD4`).
    process: ProcessState,
    /// T808 (`word_1014CBD8`, built by the first 808 ctor, [`crate::tables::tr808_tom_step`]).
    tom_step: [u16; 128],
}

const SIMPLE_RS: usize = 0;
const SIMPLE_CP: usize = 1;
const SIMPLE_CB: usize = 2;
const SIMPLE_OH: usize = 3;
const SIMPLE_CH: usize = 4;
const FILTER_BD_A: usize = 0;
const FILTER_BD_B: usize = 1;
const FILTER_CY: usize = 2;

impl Tr808 {
    /// The ctor `sub_10026F70`: the parameter defaults in both copies, every pattern 16 empty
    /// steps (`sub_10018D20`, `sub_100197C0`), pattern 0, the enable latch 1, the voices silent.
    pub fn new(samples: Arc<SampleSet>, process: ProcessState) -> Self {
        let mut patterns = Box::new([0u8; PATTERNS * PATTERN_LEN]);
        for p in patterns.chunks_mut(PATTERN_LEN) {
            p[1] = 16;
        }
        Tr808 {
            pat_start: 0,
            params: DEFAULT_PARAMS,
            patterns,
            live: DEFAULT_PARAMS,
            cur_pattern: 0,
            enabled: 1,
            wrapped: false,
            last_pos: 0,
            simple: Default::default(),
            pitched: Default::default(),
            filter: Default::default(),
            dual: Default::default(),
            accent: 0.0,
            song_mode: false,
            samples,
            process,
            tom_step: crate::tables::tr808_tom_step(),
        }
    }

    fn pattern(&self, p: u8) -> &[u8] {
        let at = p as usize * PATTERN_LEN;
        &self.patterns[at..at + PATTERN_LEN]
    }

    /// The sample in slot `n`.
    fn sample(&self, n: usize) -> Option<Arc<Sample>> {
        Some(self.samples.shared(999 + n as u16))
    }

    fn bar_of(now: i32) -> i32 {
        768 * (now / 768)
    }

    /// `((L*k)*L)*scale*accent` as the SIMPLE instruments, BD and CY compute it (f64, unrounded).
    fn level(&self, l: u8, scale: f32) -> f64 {
        let l = l as f64;
        (((K as f64 * l) * l) * scale as f64) * self.accent as f64
    }

    /// Instrument `i` (`sub_10027894`, cases 1..11).
    fn trigger(&mut self, i: usize) {
        let lv = self.live;
        match i {
            BD => self.bass_or_cymbal(true),
            CY => self.bass_or_cymbal(false),
            SD => self.snare(),
            LT | MT | HT => self.tom(i),
            RS | CP | CB => {
                let (voice, level, sample) = match i {
                    RS => (
                        SIMPLE_RS,
                        lv[RS_LEVEL],
                        if lv[RS_SELECT] != 0 {
                            slot::RS
                        } else {
                            slot::CL
                        },
                    ),
                    CP => (
                        SIMPLE_CP,
                        lv[CP_LEVEL],
                        if lv[CP_SELECT] != 0 {
                            slot::CP
                        } else {
                            slot::MA
                        },
                    ),
                    _ => (SIMPLE_CB, lv[CB_LEVEL], slot::CB),
                };
                let g = st(self.level(level, LEVEL_SCALE));
                let s = self.sample(sample);
                self.simple[voice].trigger(s, g, NO_DECAY, 0);
            }
            OH => {
                // 0x10027DDB: level >> 1, hold 4*decay, m 0.998
                let g = st(self.level(lv[OH_LEVEL] >> 1, LEVEL_SCALE));
                let s = self.sample(slot::OH);
                self.simple[SIMPLE_OH].trigger(s, g, OH_DECAY_M, 4 * lv[OH_DECAY] as u32);
            }
            CH => {
                // 0x10027E90: (2*level)/3, then every SIMPLE voice's hold is cleared (the choke)
                let g = st(self.level(((2 * lv[CH_LEVEL] as i32) / 3) as u8, LEVEL_SCALE));
                let s = self.sample(slot::CH);
                self.simple[SIMPLE_CH].trigger(s, g, NO_DECAY, 0);
                for v in &mut self.simple {
                    v.set_hold(0);
                }
            }
            _ => {}
        }
    }

    /// `sub_1002A0B4`: the bass drum (`bd`) or the cymbal on a FILTER voice.
    fn bass_or_cymbal(&mut self, bd: bool) {
        let lv = self.live;
        let (level, decay, tone, sample) = if bd {
            (lv[BD_LEVEL], lv[BD_DECAY], lv[BD_TONE], slot::BD)
        } else {
            (lv[CY_LEVEL], lv[CY_DECAY], lv[CY_TONE], slot::CY)
        };
        let voice = if bd {
            // 0x1002A0DA: the global toggle picks the voice and fades the other one
            if self.process.bd_808() != 0 {
                self.filter[FILTER_BD_A].set_m(BD_FADE);
                self.process.set_bd_808(0);
                FILTER_BD_B
            } else {
                self.filter[FILTER_BD_B].set_m(BD_FADE);
                self.process.set_bd_808(1);
                FILTER_BD_A
            }
        } else {
            FILTER_CY
        };
        let c = TR808_TONE[(tone >> 4) as usize];
        let m = TR808_DECAY[(decay >> 3) as usize];
        let x = self.level(level, LEVEL_SCALE);
        let g = if bd { st((ONE - c as f64) * x) } else { st(x) };
        let s = self.sample(sample);
        self.filter[voice].trigger(s, g, m, 0, c);
    }

    /// `sub_1002A1E4`: the snare on the DUAL voice; the noise sample follows SEQ808 at the
    /// global counter.
    fn snare(&mut self) {
        let lv = self.live;
        let sk = lv[SD_SNAPPY] as f64 * K as f64;
        let l = lv[SD_LEVEL] as f64;
        let g1 = st((((l * K as f64) * l) * SD_SCALE as f64) * self.accent as f64);
        let g2 = st(sk * sk);
        let a = TR808_SD_SAMPLE[((lv[SD_TONE] as usize) * 5) >> 7] as usize + 1;
        let c = self.process.next_snare_808();
        let b = TR808_SD_NOISE_SAMPLE[TR808_SD_NOISE_SEQ[c as usize] as usize] as usize + 1;
        let (sa, sb) = (self.sample(a), self.sample(b));
        self.dual.trigger(sa, g1, NO_DECAY, 0, sb, g2);
    }

    /// `sub_1002A004`: a tom (selector set) or conga on its PITCHED voice.
    fn tom(&mut self, i: usize) {
        let base = LT_LEVEL + 3 * (i - LT);
        let (level, tune, select) = (self.live[base], self.live[base + 1], self.live[base + 2]);
        let sample = match (i, select != 0) {
            (LT, true) => slot::LT,
            (LT, false) => slot::LC,
            (MT, true) => slot::MT,
            (MT, false) => slot::MC,
            (_, true) => slot::HT,
            (_, false) => slot::HC,
        };
        let l = level as f64;
        let g = st((((l * K as f64) * l) * K as f64) * self.accent as f64);
        // `mov dl, byte ptr word_1014CBD8[eax*2]; mov dh, ...+1`: the u16 entry
        let step = self.tom_step[tune as usize & 127] as u32;
        let s = self.sample(sample);
        self.pitched[i - LT].trigger(s, g, NO_DECAY, 0, step);
    }

    /// The live parameters from the pattern-mode copy.
    fn params_live(&mut self) {
        self.live = self.params;
    }
}

impl SongDevice for Tr808 {
    fn live(&self) -> &[u8] {
        &self.live
    }

    /// Slot 1, 0x10027690.
    fn step(&mut self, tick: i32, shuffled: i32, _tempo: i32) {
        let pat = self.cur_pattern;
        let (shuffle, len) = {
            let p = self.pattern(pat);
            (p[0], p[1])
        };
        let t = if shuffle != 0 { shuffled } else { tick };
        let base = if self.song_mode { self.pat_start } else { 0 };
        let period = 48 * len as u32;
        if period == 0 {
            // `div` by zero in the original; no file has a pattern of length 0
            return;
        }
        // 0x10027713: unsigned (t - base) % (48*len); the step is the signed quotient by 48
        let v = (t.wrapping_sub(base) as u32 % period) as i32;
        let step = (v / 48) & 0xFF;
        if self.last_pos.wrapping_sub(v) > 24 {
            self.wrapped = true;
        }
        self.last_pos = v;
        if step == self.live[LAST_STEP] as i32 && !self.wrapped {
            return;
        }
        // 0x10027772
        self.live[LAST_STEP] = step as u8;
        self.wrapped = false;
        let mut pat = pat;
        let q = tick / 48;
        let s16 = if q < 0 { -((-q) & 15) } else { q & 15 };
        if s16 == 0 {
            // the bar line: the pending pattern and the enable flag
            let p = self.live[PATTERN];
            if p != self.cur_pattern {
                self.cur_pattern = p;
                self.pat_start = 768 * (tick / 768);
                pat = p;
            }
            self.enabled = self.live[ENABLED];
        }
        if self.enabled == 0 {
            return;
        }
        let at = 2 + 12 * step as usize;
        let row: [u8; 12] = {
            let p = self.pattern(pat);
            std::array::from_fn(|i| p.get(at + i).copied().unwrap_or(0))
        };
        // 0x10027821: f32(AC * accent level * 0.03125f + 1.0)
        self.accent = st((row[AC] as f64 * self.live[ACCENT] as f64) * ACC_STEP as f64 + ACC_BASE);
        for (i, &v) in row.iter().enumerate().skip(1) {
            if v != 0 {
                self.trigger(i);
            }
        }
    }

    /// Slot 2, 0x10028030: the argument is not read; the bar of the song's position, the
    /// pending pattern and the enable flag.
    fn relocate(&mut self, _tick: i32, now: i32) {
        self.pat_start = Self::bar_of(now);
        if self.live[PATTERN] != self.cur_pattern {
            self.cur_pattern = self.live[PATTERN];
        }
        self.enabled = self.live[ENABLED];
    }

    /// Slot 3, 0x10028080.
    fn start(&mut self, now: i32) {
        self.wrapped = true;
        self.pat_start = Self::bar_of(now);
    }

    /// Slot 4, 0x100106C0: slot 6.
    fn stop(&mut self) {
        self.silence();
    }

    /// Slot 5, 0x1000F4F0: slot 6.
    fn reset(&mut self) {
        self.silence();
    }

    /// Slot 6, 0x1002A2D0: slot 1 (0x1001EB30) on every voice.
    fn silence(&mut self) {
        for v in &mut self.simple {
            v.silence();
        }
        for v in &mut self.filter {
            v.silence();
        }
        for v in &mut self.pitched {
            v.silence();
        }
        self.dual.silence();
    }

    /// Slot 9, 0x10029E30: entering song mode saves the live block into the pattern-mode copy,
    /// leaving it restores it (all 32 bytes).
    fn set_song_mode(&mut self, on: bool) {
        if on {
            self.params = self.live;
        } else {
            self.params_live();
        }
        self.song_mode = on;
    }

    /// Msg 56 (0x1002884C → `sub_10019490`): the parameter block into the pattern-mode copy
    /// and the patterns; in pattern mode the block also goes live (bytes 29..31 included).
    fn data(&mut self, payload: &[u8], song_mode: bool) {
        self.params.copy_from_slice(&payload[..32]);
        self.patterns
            .copy_from_slice(&payload[32..32 + PATTERNS * PATTERN_LEN]);
        if !song_mode {
            self.params_live();
        }
    }

    /// Slot 11, 0x10029050: controllers 0..28 set the live byte (and a "changed" flag at
    /// `+0x18B0 + ctrl`, read only by the panel update 0x100291A0, not modelled).
    fn set_param(&mut self, ctrl: u8, value: u8) {
        if (ctrl as usize) <= LAST_CTRL {
            self.live[ctrl as usize] = value;
        }
    }

    /// Slots 7 (0x10026C40, the 303's) and 8 (0x10029DA0) from the song ctor: controllers 0 and
    /// 1 of the track are quantised to a bar, and a tick-0 event of every controller 0..28 with
    /// the pattern-mode copy's value is inserted (`sub_1001F900`).
    fn init_track(&mut self, track: &mut Track) {
        track.set_quant(ENABLED as u8, 32);
        track.set_quant(PATTERN as u8, 32);
        for c in 0..=LAST_CTRL {
            track.insert(0, c as u8, self.params[c]);
        }
    }
}

impl ChannelDevice for Tr808 {
    fn playing_pattern(&self) -> Option<u8> {
        Some(self.cur_pattern)
    }

    /// Slot 14, 0x100280B0: the bus is zeroed, then the voices add in this order: BD A, BD B,
    /// SD, LT, MT, HT, RS/CL, CP/MA, CB, OH, CH, CY (every per-instrument output is null in the
    /// export, so all go to bus 0).
    fn process(&mut self, bus: &mut Block, _ctx: &BlockContext) {
        bus.fill(0.0);
        let [a, b, cy] = &mut self.filter;
        a.process(bus);
        b.process(bus);
        self.dual.process(bus);
        for v in &mut self.pitched {
            v.process(bus);
        }
        for v in &mut self.simple {
            v.process(bus);
        }
        cy.process(bus);
    }
}
