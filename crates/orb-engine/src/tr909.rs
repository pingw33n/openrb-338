//! The TR-909 (`CTR909Device`, vtable 0x10034294, 0x1920 bytes, ctor `sub_1002A380`;
//! `docs/initial_investigation.md` sections 5 (flam) and 7.1..7.3).
//!
//! The step function (slot 1, 0x1002AAC0) plays the current pattern on the sequencer's ticks: on
//! each new step it computes the accent and triggers the step's instruments in descending column
//! order, each at the accent times 0.8 (step value 1 or 3) or 2.0 (value 2 or 4). A new step also
//! arms the flam: the first later call whose tick is past the flam tick retriggers every
//! instrument whose value is above 2 at the accent times 1.5. Each instrument plays on its own
//! voice of the classes in [`crate::drums`]; the bass drum alternates between two voices through
//! a process-global flag, and the snare's noise sample follows SEQ909 at a process-global counter
//! (both in [`ProcessState`]). The audio (slot 14, 0x1002B6F0) zeroes the bus and adds the voices
//! in a fixed order.
//!
//! The parameter blocks keep the DLL's internal order, which differs from the RBS and controller
//! order in bytes 20..22: the DLL holds the hi-hat level at 20, the rim shot's at 21 and the
//! clap's at 22 (msg 73's parse `sub_100196F4`, its inverse `sub_10019624`, set-param
//! 0x1002C6D0). Each field's doc names its offset.

#[cfg(test)]
mod tests;

use std::sync::Arc;

use crate::drums::{Dual, Filter, Pitched, Simple};
use crate::frame::{Block, BlockContext, ChannelDevice, SongDevice};
use crate::process::ProcessState;
use crate::samples::{Retired, Sample, SampleSet};
use crate::seq::TrackMut;
use crate::tables::data::{
    TR909_BD_ATTACK, TR909_SD_NOISE_SAMPLE, TR909_SD_NOISE_SEQ, TR909_SD_SAMPLE,
};

/// The parameter block (`+0x30` pattern-mode copy, `+0x1891` live) in the DLL's order:
/// controllers 0..29 (bytes 20..22 permuted), the step function's last step at byte 30 and two
/// unused bytes.
pub type Params = [u8; 33];
pub const ENABLED: usize = 0;
pub const PATTERN: usize = 1;
pub const ACCENT: usize = 2;
pub const BD_LEVEL: usize = 3;
pub const BD_TUNE: usize = 4;
pub const BD_ATTACK: usize = 5;
pub const BD_DECAY: usize = 6;
pub const SD_LEVEL: usize = 7;
pub const SD_TUNE: usize = 8;
pub const SD_TONE: usize = 9;
pub const SD_SNAPPY: usize = 10;
/// The toms' level, tuning and decay: low at 11, mid at 14, high at 17.
pub const LT_LEVEL: usize = 11;
pub const MT_LEVEL: usize = 14;
pub const HT_LEVEL: usize = 17;
/// Byte 20 of the DLL's block: the hi-hat level (controller 22).
pub const HH_LEVEL: usize = 20;
/// Byte 21: the rim shot's level (controller 20).
pub const RS_LEVEL: usize = 21;
/// Byte 22: the clap's level (controller 21).
pub const CP_LEVEL: usize = 22;
pub const CH_DECAY: usize = 23;
pub const OH_DECAY: usize = 24;
pub const CC_LEVEL: usize = 25;
pub const CC_TUNE: usize = 26;
pub const RC_LEVEL: usize = 27;
pub const RC_TUNE: usize = 28;
/// Byte 29: the flam interval (controller 29).
pub const FLAM: usize = 29;
/// Byte 30 of the live block (`+0x18AF`): the last step the step function played.
const LAST_STEP: usize = 30;
/// The last controller set-param takes (0x1002C6D7: `cmp edx, 1Dh; ja`; case 29 at 0x1002C80B,
/// which Hex-Rays drops, stores the flam interval).
pub const LAST_CTRL: usize = FLAM;
/// The controllers whose tick-0 events slot 8 inserts (0x1002D4A6: `i < 30`).
const INIT_CTRLS: usize = 30;
/// One pattern: shuffle flag, length, 16 steps of 12 columns.
pub const PATTERN_LEN: usize = 194;
pub const PATTERNS: usize = 32;
/// The step columns: 0 the accent, then the instruments (triggered from 11 down to 1).
pub const AC: usize = 0;
pub const BD: usize = 1;
pub const SD: usize = 2;
pub const LT: usize = 3;
pub const MT: usize = 4;
pub const HT: usize = 5;
pub const RS: usize = 6;
pub const CP: usize = 7;
pub const CH: usize = 8;
pub const OH: usize = 9;
pub const CC: usize = 10;
pub const RC: usize = 11;

/// The parameter block of `sub_100195A0` (ctor, both copies), in the DLL's order.
pub const DEFAULT_PARAMS: Params = [
    1, 0, 32, 100, 64, 64, 127, 100, 64, 64, 64, 100, 64, 127, 100, 64, 127, 100, 64, 127, 100,
    100, 100, 32, 127, 100, 64, 100, 64, 64, 0, 0, 0,
];

/// The byte of the DLL's block that controller (and RBS byte) `c` maps to: 20..22 are permuted.
pub const fn internal_index(c: usize) -> usize {
    match c {
        20 => RS_LEVEL,
        21 => CP_LEVEL,
        22 => HH_LEVEL,
        _ => c,
    }
}

/// The device's sample slots (`+0x18DC`, the mod's table: slot `n` at `+4n`), with the built-in
/// kit slot `n` = resource `1000 + n` (`sub_1001A030`).
pub mod slot {
    /// bd1..bd4 (25..28), sd1..sd5 (29..33), sdn1..sdn3 (34..36).
    pub const BD1: usize = 25;
    pub const LT: usize = 37;
    pub const MT: usize = 38;
    pub const HT: usize = 39;
    pub const RS: usize = 40;
    /// `hc`, the hand clap.
    pub const CP: usize = 41;
    pub const CH: usize = 42;
    pub const OH: usize = 43;
    pub const CC: usize = 44;
    pub const RC: usize = 45;
}

// The step function (0x10154348..) and the triggers (0x101545D0.., 0x101545F8.., 0x10154638..).
/// `dbl_10154348`.
const ACC_BASE: f64 = 1.0;
/// `flt_10154368`: the accent step.
const ACC_STEP: f32 = 0.03125;
/// `dbl_10154358`: the gain of step values 1 and 3.
const GAIN_ON: f64 = f64::from_bits(0x3FE9_9999_9999_999A); // 0.8
/// `dbl_10154360`: the gain of step values 2 and 4.
const GAIN_ACCENT: f64 = 2.0;
/// `dbl_10154350`: the gain of the flam's retrigger.
const GAIN_FLAM: f64 = 1.5;
/// `mov ebx, 0AAE60h` at 0x1002ABBC: the flam interval's divisor.
const FLAM_DIV: i32 = 700_000;
/// `flt_10154644`, `flt_1015464C`: f32 1/127.
const K: f32 = f32::from_bits(0x3C01_0204);
/// `flt_101545D8`, `flt_10154600`: the level scale of the SIMPLE and PITCHED instruments.
const LEVEL_SCALE: f32 = f32::from_bits(0x3882_0610); // 6.2e-5
/// `flt_101545D4`: the rim shot's and the open hi-hat's factor.
const C_RS_OH: f32 = f32::from_bits(0x3F8C_CCCD); // 1.1
/// `flt_101545E4`: the clap's factor.
const C_CP: f32 = f32::from_bits(0x4013_3333); // 2.3
/// `flt_101545DC`: the closed hi-hat's factor.
const C_CH: f32 = f32::from_bits(0x3F4C_CCCD); // 0.8
/// `mov eax, 3F7EF9DBh` at 0x1002D92A: the open hi-hat's multiplier after a closed hi-hat.
const CHOKE: f32 = f32::from_bits(0x3F7E_F9DB); // 0.996
/// `flt_1015460C`, `flt_10154608`, `dword_10154604`, `flt_101545FC`: the toms' and cymbals' kv.
const KV_LT: f32 = f32::from_bits(0x3F99_999A); // 1.2
const KV_MT: f32 = f32::from_bits(0x3F8C_CCCD); // 1.1
const KV_HT: f32 = f32::from_bits(0x3FB3_3333); // 1.4
const KV_CY: f32 = f32::from_bits(0x3F19_999A); // 0.6
/// `dbl_10154638`: `1 - c` of the bass drum.
const ONE: f64 = 1.0;
/// `mov eax, 3F7D70A4h` at 0x1002DB59: the bass drum's fade of its other voice.
const BD_FADE: f32 = f32::from_bits(0x3F7D_70A4); // 0.99
/// `flt_10154648`: the snare's snappy scale.
const SNAPPY_SCALE: f32 = f32::from_bits(0x3C23_D70A); // 0.01
/// `mov [esp+..], 3F7F7CEEh` at 0x1002DC6B: the snare's noise multiplier.
const SD_DECAY_M: f32 = f32::from_bits(0x3F7F_7CEE); // 0.998
/// `add ebx, 1Eh` at 0x1002DD1F: the snare's hold is `tone + 30` blocks.
const SD_HOLD: u32 = 30;
/// The decay argument the step function passes for RS, CP, crash and ride (`mov ecx, 7Fh`).
const FIXED_DECAY: u8 = 127;

fn st(x: f64) -> f32 {
    x as f32
}

/// A TR-909.
#[derive(Debug, Clone)]
pub struct Tr909 {
    /// `+4` (CDevice): the tick of the bar the pattern started at (song mode).
    pub pat_start: i32,
    /// `+0x30`: the pattern-mode parameter copy (the DLL's order).
    pub params: Params,
    /// `+0x51`: the 32 patterns, 194 bytes each, as msg 73 sends them.
    pub patterns: Box<[u8; PATTERNS * PATTERN_LEN]>,
    /// `+0x1891`: the live parameters (the DLL's order).
    pub live: Params,
    /// `+0x18D3`: the playing pattern.
    pub cur_pattern: u8,
    /// `+0x18D4`: the enable flag latched at the bar line.
    pub enabled: u8,
    /// `+0x18D5`: set by a pattern wrap and by play; forces the next step to trigger.
    pub wrapped: bool,
    /// `+0x18D8`: the pattern position of the last step call.
    pub last_pos: i32,
    /// `+0x18E0`: the SIMPLE voices of RS, CP, CH and OH.
    pub simple: [Simple; 4],
    /// `+0x18F0`: the PITCHED voices of LT, MT, HT, crash and ride.
    pub pitched: [Pitched; 5],
    /// `+0x1904`: the FILTER voices of the two bass drums.
    pub filter: [Filter; 2],
    /// `+0x190C`: the snare's DUAL voice.
    pub dual: Dual,
    /// `+0x1910`: a new step armed the flam (cleared when it fires).
    pub flam_pending: bool,
    /// `+0x1914`: the tick the flam waits to pass.
    pub flam_tick: i32,
    /// `+0x1918`: the accent of the last step played.
    pub accent: f32,
    /// `+0x191C`: the gain of the instrument being triggered (accent times 0.8, 2.0 or 1.5).
    pub gain: f32,
    /// The song's global byte 0 (`sub_1001FCB0`), followed through slot 9.
    pub song_mode: bool,
    /// `+0x18DC`: the sample table.
    samples: Arc<SampleSet>,
    /// The process globals (the BD toggle `word_1014E544`, the snare counter `word_1014E548`).
    process: ProcessState,
    /// T909 (`word_1014E54C`, built by the first 909 ctor, [`crate::tables::tr909_tom_step`]).
    tom_step: [u16; 128],
    /// DT (`flt_1014E64C`, the same ctor, [`crate::tables::tr909_decay`]).
    decay: [f32; 16],
}

const SIMPLE_RS: usize = 0;
const SIMPLE_CP: usize = 1;
const SIMPLE_CH: usize = 2;
const SIMPLE_OH: usize = 3;
const FILTER_BD_A: usize = 0;
const FILTER_BD_B: usize = 1;

impl Tr909 {
    /// The ctor `sub_1002A380`: the parameter defaults in both copies, every pattern 16 empty
    /// steps (`sub_10018D20`, `sub_100197C0`), pattern 0, the enable latch 1, the flam unarmed,
    /// the voices silent.
    pub fn new(samples: Arc<SampleSet>, process: ProcessState) -> Self {
        let mut patterns = Box::new([0u8; PATTERNS * PATTERN_LEN]);
        for p in patterns.chunks_mut(PATTERN_LEN) {
            p[1] = 16;
        }
        Tr909 {
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
            flam_pending: false,
            flam_tick: 0,
            accent: 0.0,
            gain: 0.0,
            song_mode: false,
            samples,
            process,
            tom_step: crate::tables::tr909_tom_step(),
            decay: crate::tables::tr909_decay(),
        }
    }

    fn pattern(&self, p: u8) -> &[u8] {
        let at = p as usize * PATTERN_LEN;
        &self.patterns[at..at + PATTERN_LEN]
    }

    /// The sample in slot `n`.
    fn sample(&self, n: usize) -> Option<Arc<Sample>> {
        Some(self.samples.shared(1000 + n as u16))
    }

    fn bar_of(now: i32) -> i32 {
        768 * (now / 768)
    }

    /// `DT[(d << 4) >> 7]`, the index the triggers compute from a decay byte.
    fn dt(&self, d: u8) -> f32 {
        self.decay[((d as usize) << 4) >> 7]
    }

    /// Instrument `i` (the switches at 0x1002AD2D, 0x1002B03C and 0x1002B359, cases 1..11) at
    /// the gain in [`Tr909::gain`].
    fn trigger(&mut self, i: usize) {
        let lv = self.live;
        match i {
            BD => self.bass(lv[BD_LEVEL], lv[BD_DECAY], lv[BD_TUNE], lv[BD_ATTACK]),
            SD => self.snare(lv[SD_LEVEL], lv[SD_TONE], lv[SD_TUNE], lv[SD_SNAPPY]),
            LT => self.pitched(
                i,
                slot::LT,
                lv[LT_LEVEL],
                lv[LT_LEVEL + 1],
                lv[LT_LEVEL + 2],
            ),
            MT => self.pitched(
                i,
                slot::MT,
                lv[MT_LEVEL],
                lv[MT_LEVEL + 1],
                lv[MT_LEVEL + 2],
            ),
            HT => self.pitched(
                i,
                slot::HT,
                lv[HT_LEVEL],
                lv[HT_LEVEL + 1],
                lv[HT_LEVEL + 2],
            ),
            CC => self.pitched(i, slot::CC, lv[CC_LEVEL], lv[CC_TUNE], FIXED_DECAY),
            RC => self.pitched(i, slot::RC, lv[RC_LEVEL], lv[RC_TUNE], FIXED_DECAY),
            RS => self.simple(i, slot::RS, lv[RS_LEVEL], FIXED_DECAY),
            CP => self.simple(i, slot::CP, lv[CP_LEVEL], FIXED_DECAY),
            CH => self.simple(i, slot::CH, lv[HH_LEVEL], lv[CH_DECAY]),
            OH => self.simple(i, slot::OH, lv[HH_LEVEL], lv[OH_DECAY]),
            _ => {}
        }
    }

    /// `sub_1002D8B4(sample, level, decay, i)`: RS, CP, CH and OH on their SIMPLE voices,
    /// `g = f32((((6.2e-5f*L)*L)*c)*gain)`, hold 0. The multiplier is `DT[d]` (RS, CP with
    /// d = 127: 1.0; OH), or for CH `f32(DT[d]^8)` (three squarings in the x87); CH first sets
    /// the open hi-hat's multiplier to 0.996 (the choke).
    fn simple(&mut self, i: usize, sample: usize, level: u8, decay: u8) {
        let dt = self.dt(decay);
        let (voice, c, m) = match i {
            RS => (SIMPLE_RS, C_RS_OH, dt),
            CP => (SIMPLE_CP, C_CP, dt),
            CH => {
                // 0x1002D935: fld f32 DT; fmul st, st; fmul st, st; fmul st, st
                let x = dt as f64;
                let x2 = x * x;
                let x4 = x2 * x2;
                let x8 = x4 * x4;
                // 0x1002D958: sub_1001E3D0(0.996) on the OH voice
                self.simple[SIMPLE_OH].set_m(CHOKE);
                (SIMPLE_CH, C_CH, st(x8))
            }
            _ => (SIMPLE_OH, C_RS_OH, dt),
        };
        let l = level as f64;
        let g = st((((LEVEL_SCALE as f64 * l) * l) * c as f64) * self.gain as f64);
        let s = self.sample(sample);
        self.simple[voice].trigger(s, g, m, 0);
    }

    /// `sub_1002DA24(sample, level, tune, decay, i)`: a tom or cymbal on its PITCHED voice,
    /// `g = f32(((L*(L*6.2e-5f))*kv)*gain)`, `m = DT[decay]`, hold 0, `step = T909[tune]`.
    fn pitched(&mut self, i: usize, sample: usize, level: u8, tune: u8, decay: u8) {
        let (voice, kv) = match i {
            LT => (0, KV_LT),
            MT => (1, KV_MT),
            HT => (2, KV_HT),
            CC => (3, KV_CY),
            _ => (4, KV_CY),
        };
        let l = level as f64;
        let g = st(((l * (l * LEVEL_SCALE as f64)) * kv as f64) * self.gain as f64);
        let m = self.dt(decay);
        // `mov dx, word_1014E54C[eax*2]`
        let step = self.tom_step[tune as usize & 127] as u32;
        let s = self.sample(sample);
        self.pitched[voice].trigger(s, g, m, 0, step);
    }

    /// `sub_1002DB24(level, decay, tune, attack)`: the bass drum on a FILTER voice. The sample
    /// is bd1..bd4 by `tune >> 5`; the global toggle picks the voice and fades the other one;
    /// `g = f32(((((L*k)*L)*k)*gain)*(1 - c))` with `c = C8[attack >> 4]`, `m = DT[decay]`.
    fn bass(&mut self, level: u8, decay: u8, tune: u8, attack: u8) {
        // 0x1002DB3E: a signed divide by 32 of a byte
        let sample = slot::BD1 + (tune as usize >> 5);
        let voice = if self.process.bd_909() != 0 {
            self.filter[FILTER_BD_A].set_m(BD_FADE);
            self.process.set_bd_909(0);
            FILTER_BD_B
        } else {
            self.filter[FILTER_BD_B].set_m(BD_FADE);
            self.process.set_bd_909(1);
            FILTER_BD_A
        };
        let l = level as f64;
        let x = (l * (l * K as f64)) * K as f64;
        // 0x1002DBBC: `(attack << 3) >> 7`
        let c = TR909_BD_ATTACK[((attack as usize) << 3) >> 7];
        let g = st((x * self.gain as f64) * (ONE - c as f64));
        let m = self.dt(decay);
        let s = self.sample(sample);
        self.filter[voice].trigger(s, g, m, 0, c);
    }

    /// `sub_1002DC54(level, tone, tune, snappy, i)`: the snare on the DUAL voice. A is
    /// sd1..sd5 by `(tune*5) >> 7`, B the noise sample of SEQ909 at the global counter;
    /// `g1 = f32((((L*k)*L)*k)*gain)`, `g2 = f32(snappy*0.01f)`, `m = 0.998`, hold `tone + 30`.
    fn snare(&mut self, level: u8, tone: u8, tune: u8, snappy: u8) {
        let g2 = st(snappy as f64 * SNAPPY_SCALE as f64);
        let l = level as f64;
        let g1 = st(((l * (l * K as f64)) * K as f64) * self.gain as f64);
        let a = slot::BD1 + TR909_SD_SAMPLE[((tune as usize) * 5) >> 7] as usize;
        let c = self.process.next_snare_909();
        let b = slot::BD1 + TR909_SD_NOISE_SAMPLE[TR909_SD_NOISE_SEQ[c as usize] as usize] as usize;
        let (sa, sb) = (self.sample(a), self.sample(b));
        self.dual
            .trigger(sa, g1, SD_DECAY_M, tone as u32 + SD_HOLD, sb, g2);
    }

    /// The live parameters from the pattern-mode copy.
    fn params_live(&mut self) {
        self.live = self.params;
    }

    /// The pattern-mode block in RBS (controller) order (`sub_10019624`).
    pub fn params_rbs_order(&self) -> Params {
        std::array::from_fn(|c| self.params[internal_index(c)])
    }
}

impl SongDevice for Tr909 {
    fn live(&self) -> &[u8] {
        &self.live
    }

    fn pattern_params(&self) -> &[u8] {
        &self.params
    }

    /// Slot 1, 0x1002AAC0.
    fn step(&mut self, tick: i32, shuffled: i32, tempo: i32) {
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
        // 0x1002AB49: unsigned (t - base) % (48*len); the step is the signed quotient by 48
        let v = (t.wrapping_sub(base) as u32 % period) as i32;
        if self.last_pos.wrapping_sub(v) > 24 {
            self.wrapped = true;
        }
        self.last_pos = v;
        let step = (v / 48) & 0xFFFF;
        let mut pat = pat;
        let flam = if step == self.live[LAST_STEP] as i32 && !self.wrapped {
            // 0x1002AB87: the same step: only an armed flam whose tick has passed (the
            // unshuffled tick, signed) goes on
            if !(self.flam_pending && tick > self.flam_tick) {
                return;
            }
            self.flam_pending = false;
            true
        } else {
            // 0x1002ABB0
            self.live[LAST_STEP] = step as u8;
            // imul (32 bits), cdq, idiv 700000
            let d = (self.live[FLAM] as i32).wrapping_mul(tempo) / FLAM_DIV;
            self.flam_tick = tick.wrapping_add(d).wrapping_add(1);
            self.wrapped = false;
            self.flam_pending = true;
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
            false
        };
        // 0x1002AC74
        if self.enabled == 0 {
            return;
        }
        let at = 2 + 12 * step as usize;
        let row: [u8; 12] = {
            let p = self.pattern(pat);
            std::array::from_fn(|i| p.get(at + i).copied().unwrap_or(0))
        };
        // 0x1002ACAF: f32(AC * accent level * 0.03125f + 1.0)
        self.accent = st((row[AC] as f64 * self.live[ACCENT] as f64) * ACC_STEP as f64 + ACC_BASE);
        for i in (1..=11).rev() {
            let v = row[i];
            let k = if flam {
                // 0x1002B320: values above 2 retrigger at x1.5
                if v <= 2 {
                    continue;
                }
                GAIN_FLAM
            } else {
                match v {
                    1 | 3 => GAIN_ON,
                    2 | 4 => GAIN_ACCENT,
                    _ => continue,
                }
            };
            self.gain = st(self.accent as f64 * k);
            self.trigger(i);
        }
    }

    /// Slot 2, 0x1002B670: the argument is not read; the bar of the song's position, the
    /// pending pattern and the enable flag.
    fn relocate(&mut self, _tick: i32, now: i32) {
        self.pat_start = Self::bar_of(now);
        if self.live[PATTERN] != self.cur_pattern {
            self.cur_pattern = self.live[PATTERN];
        }
        self.enabled = self.live[ENABLED];
    }

    /// Slot 3, 0x1002B6C0.
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

    /// Slot 6, 0x1002DD40: slot 1 (0x1001EB30) on every voice.
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

    /// Slot 9, 0x1002D510: entering song mode saves the live block into the pattern-mode copy,
    /// leaving it restores it (all 33 bytes).
    fn set_song_mode(&mut self, on: bool) {
        if on {
            self.params = self.live;
        } else {
            self.params_live();
        }
        self.song_mode = on;
    }

    /// Msg 73 (0x1002BE90 → `sub_100199A0`): the parameter block (RBS order, permuted into the
    /// DLL's by `sub_100196F4`) into the pattern-mode copy, and the patterns; in pattern mode
    /// the block also goes live (bytes 30..32 included).
    fn data(&mut self, payload: &[u8], song_mode: bool) {
        for (c, &b) in payload[..33].iter().enumerate() {
            self.params[internal_index(c)] = b;
        }
        self.patterns
            .copy_from_slice(&payload[33..33 + PATTERNS * PATTERN_LEN]);
        if !song_mode {
            self.params_live();
        }
    }

    /// Slot 11, 0x1002C6D0: controllers 0..29 set the live byte (20..22 permuted), every
    /// controller also the panel's "changed" flag at `+0x18B2 + ctrl` (not modelled).
    fn set_param(&mut self, ctrl: u8, value: u8) {
        if (ctrl as usize) <= LAST_CTRL {
            self.live[internal_index(ctrl as usize)] = value;
        }
    }

    /// Msg 74 `{ctrl, v}` (0x1002BF12, sender `sub_445C41`): controllers 0..29 as slot 11, with
    /// the same permutation of 20..22 (0x1002C359, 0x1002C38D, 0x1002C3C1 store `+0x18A6`,
    /// `+0x18A7`, `+0x18A5`), 30 nothing (0x1002C555: the last-step byte stays), 31 the unused
    /// live byte 31 (`+0x18B0`, 0x1002C557); then the changed flag `+0x18B2 + ctrl` is cleared
    /// (not modelled).
    fn param_message(&mut self, ctrl: u8, value: u8) {
        match ctrl {
            31 => self.live[31] = value,
            _ => self.set_param(ctrl, value),
        }
    }

    /// Slots 7 (0x10026C40, the 303's) and 8 (0x1002D480) from the song ctor: controllers 0 and
    /// 1 of the track are quantised to a bar, and a tick-0 event of every controller 0..29 with
    /// the pattern-mode copy's value in RBS order (`sub_10019624`) is inserted (`sub_1001F900`).
    fn init_track(&mut self, track: &mut TrackMut) {
        track.set_quant(ENABLED as u8, 32);
        track.set_quant(PATTERN as u8, 32);
        let p = self.params_rbs_order();
        for (c, &v) in p.iter().enumerate().take(INIT_CTRLS) {
            track.insert(0, c as u8, v);
        }
    }
}

impl ChannelDevice for Tr909 {
    fn playing_pattern(&self) -> Option<u8> {
        Some(self.cur_pattern)
    }

    /// `sub_1002B8C0`: slot 6 (`[eax+18h]`), then the table `+0x18DC` (0x1002B8D0).
    fn set_samples(&mut self, samples: &Arc<SampleSet>, retired: &mut Retired) {
        self.silence();
        for v in &mut self.simple {
            v.retire(retired);
        }
        for v in &mut self.filter {
            v.retire(retired);
        }
        for v in &mut self.pitched {
            v.retire(retired);
        }
        self.dual.retire(retired);
        let old = std::mem::replace(&mut self.samples, samples.clone());
        if let Err(old) = retired.keep_set(old) {
            // no room: dropped here (`Retired::SETS` has room for both drum machines)
            drop(old);
        }
    }

    fn patterns_mut(&mut self) -> Option<&mut [u8]> {
        Some(&mut self.patterns[..])
    }

    /// Slot 14 ends with an unconditional `call sub_1001E9A0` at 0x1002B8AF from `esp0 - 0x18`
    /// (four register pushes, `push esi` 0x1002B6F8, `push edi` 0x1002B8A5), whose return
    /// address 0x1002B8B4 stays at `esp0 - 0x1C` (measured, task-069). The value holds at the
    /// DLL's preferred base 0x10000000 (it moves with the load address), which the corpus and
    /// rbhost assume.
    fn stack_word(&self) -> Option<i32> {
        Some(0x1002_B8B4)
    }

    /// Slot 14, 0x1002B6F0: the bus is zeroed, then the voices add in this order: BD A, BD B,
    /// SD, LT, MT, HT, RS, CP, CH, OH, crash, ride (every per-instrument output is null in the
    /// export, so all go to bus 0).
    fn process(&mut self, bus: &mut Block, _ctx: &BlockContext) {
        bus.fill(0.0);
        for v in &mut self.filter {
            v.process(bus);
        }
        self.dual.process(bus);
        let [lt, mt, ht, cc, rc] = &mut self.pitched;
        lt.process(bus);
        mt.process(bus);
        ht.process(bus);
        for v in &mut self.simple {
            v.process(bus);
        }
        cc.process(bus);
        rc.process(bus);
    }
}
