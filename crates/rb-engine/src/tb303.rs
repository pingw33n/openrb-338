//! The TB-303 (`CTB303Device`, vtable 0x10033FD4, 0xD04 bytes, ctor `sub_10023920`;
//! `docs/initial_investigation.md` section 6).
//!
//! The device keeps the DLL's fields; each field's doc names its offset. The step function
//! (slot 1, 0x100253A0) plays the current pattern on the sequencer's ticks: note-on 0x10023CD4,
//! note-off 0x10023C74. The audio (slot 14, 0x10023F10) runs one of two sound engines, chosen by
//! the song's sound-model byte (`song+0x16D`, tested at 0x10023F2D):
//! - zero, "ReBirth 2.0 sound": `sub_100248C4`, control values once per 64-sample block, the
//!   filter coefficients interpolated between two resonance rows (`sub_10023800`) and ramped
//!   every 2 samples;
//! - non-zero, "Vintage": the code inline in 0x10023F10, control values every 4 samples from one
//!   resonance row.
//!
//! Both feed the same per-sample voice: the oscillator, a one-pole pre-filter, the 2-pole
//! resonant low-pass and a DC blocker. The sample loops keep every state in x87 registers or
//! qword locals and store an f32 only at the block end; the output samples and the accent
//! envelope (`+0xCBC`) are the only f32 stores inside the loops.

#[cfg(test)]
mod tests;

use crate::frame::{Block, BlockContext, ChannelDevice, SongDevice};
use crate::seq::Track;
use crate::tables::Tables;

/// The parameter block (`+0x30` pattern-mode copy, `+0xC7C` live): controllers 0..8, then the
/// step function's last step at byte 9 and two unused bytes.
pub type Params = [u8; 12];
/// Live/param byte indices.
pub const ENABLED: usize = 0;
pub const PATTERN: usize = 1;
pub const TUNE: usize = 2;
pub const CUTOFF: usize = 3;
pub const RESONANCE: usize = 4;
pub const ENVMOD: usize = 5;
pub const DECAY: usize = 6;
pub const ACCENT: usize = 7;
pub const WAVEFORM: usize = 8;
/// Byte 9 of the live block (`+0xC85`): the last step the step function played.
const LAST_STEP: usize = 9;
/// One pattern: shuffle flag, length, 16 steps of `{note, slide, accent, up, down, rest}`.
pub const PATTERN_LEN: usize = 98;
pub const PATTERNS: usize = 32;

/// The parameter block of `sub_10018C20` (ctor, both copies): enabled, pattern 0, tune 64,
/// cutoff 20, resonance 90, envmod 20, decay 100, accent 64, saw.
pub const DEFAULT_PARAMS: Params = [1, 0, 0x40, 0x14, 0x5A, 0x14, 0x64, 0x40, 0, 0, 0, 0];

/// The tune table `word_10033F90` (i16[32]), indexed by `tune >> 2`, in semitones.
const TUNE_TABLE: [i16; 32] = [
    -12, -12, -11, -10, -9, -8, -7, -6, -5, -4, -3, -2, -1, 0, 0, 0, 0, 0, 0, 1, 2, 3, 4, 5, 6, 7,
    8, 9, 10, 11, 12, 12,
];

// Note-on (0x10023CD4).
/// `dbl_10153B28`: 1/12700.
const ACC_RESO_SCALE: f64 = f64::from_bits(0x3F14_A429_9AE3_DC33);
/// `dbl_10153B30`.
const ACC_RESO_BASE: f64 = f64::from_bits(0x3FEF_AE14_7AE1_47AE); // 0.99
/// `dbl_10153B38`.
const ACC_RESO_MUL: f64 = f64::from_bits(0x3FEF_F7CE_D916_872B); // 0.999
/// `dbl_10153B40`: 3/127.
const ACC_AMOUNT_SCALE: f64 = f64::from_bits(0x3F98_3060_C183_060C);
/// `dbl_10153B48`.
const DECAY_ONE: f64 = 1.0;
/// `dbl_10153B50`.
const DECAY_SPAN: f64 = f64::from_bits(0x3F33_A92A_3055_3261); // 0.0003
/// `dbl_10153B58`.
const DECAY_BASE: f64 = f64::from_bits(0x3FEF_FFEB_074A_771D); // 0.99999
/// `flt_10153B74`: f32 1/127.
const DECAY_SCALE: f32 = f32::from_bits(0x3C01_0204);
/// The accented envelope decay (`mov eax, 3F7FEBAFh` at 0x10023DB0).
const ACC_DECAY: f32 = f32::from_bits(0x3F7F_EBAF); // 0.99969

// The amp's input and feedback constants (immediates).
/// Note-on, 2.0 sound (0x10023E3D: `esi` = 1.0, `edi` = 0).
const AMP_ON_20: (f32, f32) = (1.0, 0.0);
/// Note-on and note-off, Vintage (0x10023E25, 0x10023C9B).
const AMP_VINTAGE: (f32, f32) = (f32::from_bits(0x3DCC_CCCD), f32::from_bits(0x3F66_6666)); // 0.1, 0.9
/// Note-off, 2.0 sound (0x10023CB3).
const AMP_OFF_20: (f32, f32) = (f32::from_bits(0x3D81_0625), f32::from_bits(0x3F6F_DF3B)); // 0.063, 0.937

// The audio. The Vintage engine's constants are at 0x10153B78..0x10153BDC, the 2.0 engine's at
// 0x10153BE0..0x10153C50; the values are the same except the resonance scale.
/// `dbl_10153B78` / `dbl_10153BE0`.
const ONE: f64 = 1.0;
/// `dbl_10153B80` / `dbl_10153BE8`.
const CUT_SCALE: f64 = f64::from_bits(0x3F62_B4C5_B45E_7F8E); // 0.0022834645669291337
/// `dbl_10153B88` / `dbl_10153BF0`.
const CUT_BASE: f64 = f64::from_bits(0x3FDD_70A3_D70A_3D71); // 0.46
/// `dbl_10153B90` / `dbl_10153BF8`.
const EM_SCALE: f64 = f64::from_bits(0x3F68_82F1_67EE_957D); // 0.0029921259842519685
/// `dbl_10153B98` / `dbl_10153C00`.
const EM_BASE: f64 = f64::from_bits(0x3FB4_7AE1_47AE_147B); // 0.08
/// `dbl_10153BA0` / `dbl_10153C10`.
const KA_SLOPE: f64 = f64::from_bits(0x3FB1_1111_1111_1111); // 1/15
/// `dbl_10153BA8` / `dbl_10153C18`.
const KA_BASE: f64 = f64::from_bits(0x3FBA_06D3_A06D_3A07); // 0.10166666666666667
/// `dbl_10153BB0` / `dbl_10153C20`.
const ENV_BIAS: f64 = f64::from_bits(0x3FD9_9999_9999_999A); // 0.4
/// `flt_10153BB8` / `flt_10153C30`: the denormal offset added to the envelope and the amp.
const OFFSET: f32 = f32::from_bits(0x3586_37BD);
/// `flt_10153BBC`: the Vintage resonance scale, f32 1/127.
const RESO_VINTAGE: f32 = f32::from_bits(0x3C01_0204);
/// `dbl_10153C08`: the 2.0 resonance scale, f64 1/127.
const RESO_20: f64 = f64::from_bits(0x3F80_2040_8102_0408);
/// `flt_10153C3C`: the 2.0 resonance row position scale (`7 * x * 127 < 7`).
const RESO_ROW: f32 = f32::from_bits(0x3C00_FEB7); // 0.0078732288
/// `flt_10153A70`: rows per unit in `sub_10023800`.
const ROWS: f32 = 7.0;
/// `flt_10153BC0`, `flt_10153BC4` (and `flt_10153C34`, `flt_10153C38`): the cutoff clamp.
const C_MAX: f32 = 1.0;
const C_MIN: f32 = 0.0;
/// `flt_10153BC8`, `flt_10153A6C`: the cutoff index scale.
const C_SCALE: f32 = 1023.0;
/// `dbl_10153C28`: the 2.0 ramp step, 1/32.
const RAMP: f64 = f64::from_bits(0x3FA0_0000_0000_0000);
/// `flt_10153BC0`: the phase wraps above 1 ...
const PHASE_MAX: f32 = 1.0;
/// ... by `flt_10153BCC`.
const PHASE_WRAP: f32 = 2.0;
/// `flt_10153BD0`: the DC blocker's feedback.
const DC: f32 = f32::from_bits(0x3F7C_60E3); // 0.985853374
/// `flt_10153BD4`: the square wave's threshold, `flt_10153BD8`/`flt_10153BDC` its levels.
const SQ_THRESH: f32 = f32::from_bits(0x3D3B_8000); // 0.0457763671875
const SQ_HI: f32 = f32::from_bits(0x3F0C_A000); // 0.54931640625
const SQ_LO: f32 = f32::from_bits(0xBF0C_A000);

fn st(x: f64) -> f32 {
    x as f32
}

/// `x^(2^n)` by repeated `fmul st, st`, from the stored f32 and stored back as f32.
fn square_n(x: f32, n: u32) -> f32 {
    let mut v = x as f64;
    for _ in 0..n {
        v = v * v;
    }
    st(v)
}

/// The cutoff clamp of both engines: `> 1` gives 1 (0x10024152), then `< 0` gives 0
/// (0x10024173).
fn clamp_c(c: f64) -> f64 {
    let c = if c > C_MAX as f64 { C_MAX as f64 } else { c };
    if c < C_MIN as f64 { C_MIN as f64 } else { c }
}

/// The filter coefficients of one sample pair: `c0`, `c1` the 2-pole feedback, `g` the input
/// gain (`c2` times the accent and amp gain), `c3` the pre-filter's feedback.
#[derive(Debug, Clone, Copy, PartialEq)]
struct Coefs {
    c0: f64,
    c1: f64,
    g: f64,
    c3: f64,
}

/// The per-sample state the loops keep in registers.
#[derive(Debug, Clone, Copy, PartialEq)]
struct Voice {
    ph: f64,
    s: f64,
    y1: f64,
    y2: f64,
    o: f64,
}

impl Voice {
    /// One sample (0x100241A5..0x1002429A for the Vintage saw; the same expression in every
    /// loop): the phase step and wrap, the oscillator, `s = c3*s + x`,
    /// `y = (s*g + y1*c0) - y2*c1`, `o = (y - y1) + o*DC`; the output is `f32(o)`.
    #[inline]
    fn sample(&mut self, inc: f64, square: bool, k: &Coefs) -> f32 {
        let mut ph = self.ph + inc;
        if ph > PHASE_MAX as f64 {
            ph -= PHASE_WRAP as f64;
        }
        self.ph = ph;
        let x = if !square {
            ph
        } else if ph <= SQ_THRESH as f64 {
            SQ_LO as f64
        } else {
            SQ_HI as f64
        };
        self.s = self.s * k.c3 + x;
        let y = (self.s * k.g + self.y1 * k.c0) - self.y2 * k.c1;
        self.o = (y - self.y1) + self.o * DC as f64;
        self.y2 = self.y1;
        self.y1 = y;
        st(self.o)
    }
}

/// A TB-303.
#[derive(Debug, Clone, PartialEq)]
pub struct Tb303 {
    /// `+4` (CDevice): the tick of the bar the pattern started at (song mode).
    pub pat_start: i32,
    /// `+0x30`: the pattern-mode parameter copy.
    pub params: Params,
    /// `+0x3C`: the 32 patterns, 98 bytes each, as msg 39 sends them.
    pub patterns: Box<[u8; PATTERNS * PATTERN_LEN]>,
    /// `+0xC7C`: the live parameters.
    pub live: Params,
    /// `+0xC94`: the playing pattern.
    pub cur_pattern: u8,
    /// `+0xC95`: the enable flag latched at the bar line.
    pub enabled: u8,
    /// `+0xC96`: set by a pattern wrap and by play; forces the next step to trigger.
    pub wrapped: bool,
    /// `+0xC98`: the tick position (in the pattern) of the pending gate-off, -1 for none.
    pub gate_off: i32,
    /// `+0xC9C`: the pattern position of the last step call.
    pub last_pos: i32,
    /// `+0xCA0`: note-on without slide sets it; the 2.0 engine then starts the block's
    /// coefficients without a ramp and with `g = 0`.
    pub retrig: bool,
    /// `+0xCA1`: the slide flag of the last step; a sliding step skips the note-off.
    pub slide: bool,
    /// `+0xCA4`: the oscillator phase, -1..1.
    pub phase: f32,
    /// `+0xCA8`: the envelope (1 at note-on, decays by `env_decay`).
    pub env: f32,
    /// `+0xCAC`: the envelope decay per control step.
    pub env_decay: f32,
    /// `+0xCB0`: the target pitch, `+0xCB4` the gliding pitch (both 16 * table index).
    pub pitch_target: u32,
    pub pitch: u32,
    /// `+0xCB8`: the accent amount, `+0xCBC` the accent envelope.
    pub acc_amount: f32,
    pub acc_env: f32,
    /// `+0xCC0`, `+0xCC4`: the cutoff smoother's last two outputs.
    pub cs1: f32,
    pub cs2: f32,
    /// `+0xCC8`, `+0xCCC`: the amp's input and feedback; `+0xCD0` the gate (0 or 1);
    /// `+0xCD4` the amp.
    pub amp_in: f32,
    pub amp_fb: f32,
    pub gate: f32,
    pub amp: f32,
    /// `+0xCDC`, `+0xCE0`: the 2-pole's last two outputs; `+0xCE4` the pre-filter;
    /// `+0xCE8` the DC blocker.
    pub y1: f32,
    pub y2: f32,
    pub s: f32,
    pub o: f32,
    /// `+0xCEC`: the accent envelope's feedback.
    pub acc_coef: f32,
    /// `+0xCF4`, `+0xCF8`, `+0xCFC`, `+0xD00`: the 2.0 engine's coefficients at the block end
    /// (`c0`, `c1`, `c2*G`, `c3`).
    pub coefs: [f32; 4],
    /// The song's global byte 0 (`sub_1001FCB0`), followed through slot 9.
    pub song_mode: bool,
    /// The song's byte `+0x16D` (`sub_10013DA0()+0x15`), followed through msg 36.
    pub vintage: bool,
}

impl Default for Tb303 {
    fn default() -> Self {
        Self::new()
    }
}

impl Tb303 {
    /// The ctor `sub_10023920`: the parameter defaults in both copies, every pattern 16 steps of
    /// rests (`sub_10018D20`, `sub_10018D30`), the state zero except `cs1 = cs2 = 0.5`, the
    /// enable flag 1, no gate-off pending.
    pub fn new() -> Self {
        let mut patterns = Box::new([0u8; PATTERNS * PATTERN_LEN]);
        for p in patterns.chunks_mut(PATTERN_LEN) {
            p[1] = 16;
            for s in 0..16 {
                p[2 + 6 * s + 5] = 1;
            }
        }
        Tb303 {
            pat_start: 0,
            params: DEFAULT_PARAMS,
            patterns,
            live: DEFAULT_PARAMS,
            cur_pattern: 0,
            enabled: 1,
            wrapped: false,
            gate_off: -1,
            last_pos: 0,
            retrig: false,
            slide: false,
            phase: 0.0,
            env: 0.0,
            env_decay: 0.0,
            pitch_target: 0,
            pitch: 0,
            acc_amount: 0.0,
            acc_env: 0.0,
            cs1: 0.5,
            cs2: 0.5,
            amp_in: 0.0,
            amp_fb: 0.0,
            gate: 0.0,
            amp: 0.0,
            y1: 0.0,
            y2: 0.0,
            s: 0.0,
            o: 0.0,
            acc_coef: 0.0,
            coefs: [0.0; 4],
            song_mode: false,
            vintage: false,
        }
    }

    fn pattern(&self, p: u8) -> &[u8] {
        let at = p as usize * PATTERN_LEN;
        &self.patterns[at..at + PATTERN_LEN]
    }

    /// Note-off `sub_10023C74`: skipped while the last step slides; else the gate closes and the
    /// amp gets its release constants.
    pub fn note_off(&mut self) {
        if self.slide {
            return;
        }
        self.gate = 0.0;
        (self.amp_in, self.amp_fb) = if self.vintage {
            AMP_VINTAGE
        } else {
            AMP_OFF_20
        };
    }

    /// Note-on 0x10023CD4 with the step's six bytes. A rest step still sets the coefficients,
    /// the pitch and the slide flag; only a played step opens the gate.
    pub fn note_on(&mut self, step: [u8; 6]) {
        let [note, slide, accent, up, down, rest] = step;
        let n = if self.vintage { 2 } else { 6 };
        // 0x10023CF2: fild reso; fmul 1/12700; fadd 0.99; fmul 0.999; fstp
        let c = st((self.live[RESONANCE] as f64 * ACC_RESO_SCALE + ACC_RESO_BASE) * ACC_RESO_MUL);
        self.acc_coef = square_n(c, n);
        if accent == 0 {
            self.acc_amount = 0.0;
            // 0x10023D72: 0.99999 - (1 - decay*f32(1/127))^2 * 0.0003
            let d = DECAY_ONE - self.live[DECAY] as f64 * DECAY_SCALE as f64;
            self.env_decay = st(DECAY_BASE - d * d * DECAY_SPAN);
        } else {
            self.env_decay = ACC_DECAY;
            self.acc_amount = st(self.live[ACCENT] as f64 * ACC_AMOUNT_SCALE);
        }
        self.env_decay = square_n(self.env_decay, n);
        if rest == 0 {
            self.gate = 1.0;
            (self.amp_in, self.amp_fb) = if self.vintage { AMP_VINTAGE } else { AMP_ON_20 };
            if !self.slide {
                self.env = 1.0;
                self.retrig = true;
            }
        }
        // 0x10023E61: ((tune + (int16)(note + 12*up - 12*down)) * 64 + 3072) * 16
        let semis = (note as i32 + 12 * up as i32 - 12 * down as i32) as i16 as i32;
        let tune = TUNE_TABLE[(self.live[TUNE] >> 2) as usize] as i32;
        self.pitch_target = (((tune << 6) + (semis << 6) + 0xC00) << 4) as u32;
        if !self.slide {
            self.pitch = self.pitch_target;
        }
        // 0x10023ED5: dword_10124708 counts note-ons; nothing reads it
        self.slide = slide != 0;
    }

    /// The live block from the pattern-mode copy (0x10025CD4, 0x10026B86).
    fn params_live(&mut self) {
        self.live = self.params;
    }

    /// `patStart = 768 * (now / 768)` (0x100255EE, 0x100255BE).
    fn bar_of(now: i32) -> i32 {
        768 * (now / 768)
    }

    /// The glide `(11*pitch + target) / 12` in u32 (0x10023F9F, 0x100248EB) and the phase
    /// increment `PITCH[pitch >> 4]`.
    fn glide(&mut self, t: &Tables) -> f64 {
        self.pitch = self.pitch.wrapping_mul(11).wrapping_add(self.pitch_target) / 12;
        t.tb303_pitch[(self.pitch >> 4) as usize] as f64
    }

    /// The cutoff smoother (0x10023F73..0x10023FEC; 0x100248E7..0x1002495F):
    /// `cs = (A*cs1 + cut*g) - b2*cs2`, `cs1` stored as f32 and the old `cs1` moved to `cs2`.
    fn smooth_cutoff(&mut self, t: &Tables) -> f64 {
        let k = &t.tb303_smoother;
        let cut = self.live[CUTOFF] as f64 * CUT_SCALE + CUT_BASE;
        let cs = (k.a as f64 * self.cs1 as f64 + cut * k.g as f64) - k.b2 as f64 * self.cs2 as f64;
        self.cs2 = self.cs1;
        self.cs1 = st(cs);
        cs
    }

    fn voice(&self) -> Voice {
        Voice {
            ph: self.phase as f64,
            s: self.s as f64,
            y1: self.y1 as f64,
            y2: self.y2 as f64,
            o: self.o as f64,
        }
    }

    fn store_voice(&mut self, v: &Voice) {
        self.phase = st(v.ph);
        self.s = st(v.s);
        self.y1 = st(v.y1);
        self.y2 = st(v.y2);
        self.o = st(v.o);
    }

    /// `sub_10023800`: the coefficients at cutoff `c` (an f32), between the resonance rows
    /// `l = trunc(rf*7)` and `l + 1`, each `f32(lo + (hi - lo)*frac)`.
    fn interpolate(t: &Tables, c: f32, rf: f64) -> [f32; 4] {
        let idx = (c as f64 * C_SCALE as f64) as i64 as i32 as usize;
        let r7 = rf * ROWS as f64;
        let l = (r7 as i64 as u16) as usize;
        let frac = r7 - l as f64;
        let lo = &t.tb303_filter[l][idx];
        let hi = &t.tb303_filter[l + 1][idx];
        std::array::from_fn(|k| {
            let d = hi[k] as f64 - lo[k] as f64;
            st(lo[k] as f64 + d * frac)
        })
    }

    /// The 2.0 sound, `sub_100248C4`.
    fn process_20(&mut self, bus: &mut Block, t: &Tables) {
        let cs = self.smooth_cutoff(t);
        let inc = self.glide(t);
        // 0x1002494C: amp = (amp + 1e-6)*fb + gate*in, once per block
        let amp0 = self.amp as f64 + OFFSET as f64;
        self.amp = st(amp0);
        let amp = amp0 * self.amp_fb as f64 + self.gate as f64 * self.amp_in as f64;
        let e0 = OFFSET as f64 + self.env as f64;
        self.env = st(e0);
        // 0x100249BB: acc = coef*acc + E*((1 - coef)*amount)
        let acc_coef = self.acc_coef as f64;
        let acc = acc_coef * self.acc_env as f64 + e0 * ((ONE - acc_coef) * self.acc_amount as f64);
        self.acc_env = st(acc);
        let g = amp * (acc + ONE);
        let em = self.live[ENVMOD] as f64 * EM_SCALE + EM_BASE;
        let r = self.live[RESONANCE] as f64 * RESO_20;
        let ka = KA_BASE - r * KA_SLOPE;
        let acc32 = self.acc_env as f64;
        let rf = self.live[RESONANCE] as f64 * RESO_ROW as f64;
        if self.retrig {
            // 0x10024ABC: the coefficients at the unchanged envelope, c2*G starting at 0
            let c = clamp_c(ka * acc32 + ((e0 - ENV_BIAS) * em + cs));
            let k = Self::interpolate(t, st(c), rf);
            self.coefs = [k[0], k[1], 0.0, k[3]];
            self.retrig = false;
        }
        // 0x10024B65: the targets at the decayed envelope
        let e1 = e0 * self.env_decay as f64;
        let c = clamp_c(ka * acc32 + ((e1 - ENV_BIAS) * em + cs));
        let n = Self::interpolate(t, st(c), rf);
        let [k0, k1, k2, k3] = self.coefs.map(|x| x as f64);
        // 0x10024BE8: the ramp steps, (target - start) * 1/32
        let d2 = (n[2] as f64 * g - k2) * RAMP;
        let d0 = (n[0] as f64 - k0) * RAMP;
        let d1 = (n[1] as f64 - k1) * RAMP;
        let d3 = (n[3] as f64 - k3) * RAMP;
        let mut k = Coefs {
            c0: k0,
            c1: k1,
            g: k2,
            c3: k3,
        };
        let square = self.live[WAVEFORM] != 0;
        let mut v = self.voice();
        for pair in bus.as_chunks_mut::<2>().0 {
            pair[0] = v.sample(inc, square, &k);
            pair[1] = v.sample(inc, square, &k);
            k.c0 += d0;
            k.c1 += d1;
            k.g += d2;
            k.c3 += d3;
        }
        self.coefs = [st(k.c0), st(k.c1), st(k.g), st(k.c3)];
        self.store_voice(&v);
        self.env = st(e1);
        self.amp = st(amp);
    }

    /// The Vintage sound, inline in 0x10023F10 (0x10023F38..0x10024897).
    fn process_vintage(&mut self, bus: &mut Block, t: &Tables) {
        let r = self.live[RESONANCE] as f64 * RESO_VINTAGE as f64;
        let cs = self.smooth_cutoff(t);
        let inc = self.glide(t);
        // 0x10023FDA, 0x10024002: the amp and the envelope get the offset, are stored, and the
        // loop runs on the unrounded sums
        let mut amp = self.amp as f64 + OFFSET as f64;
        self.amp = st(amp);
        let mut e = OFFSET as f64 + self.env as f64;
        self.env = st(e);
        let ka = KA_BASE - r * KA_SLOPE;
        let em = self.live[ENVMOD] as f64 * EM_SCALE + EM_BASE;
        let acc_in = (ONE - self.acc_coef as f64) * self.acc_amount as f64;
        let acc_coef = self.acc_coef as f64;
        let decay = self.env_decay as f64;
        let row = &t.tb303_filter[(self.live[RESONANCE] >> 4) as usize];
        let square = self.live[WAVEFORM] != 0;
        let mut v = self.voice();
        for quad in bus.as_chunks_mut::<4>().0 {
            // 0x100240DD
            let e1 = e * decay;
            let acc = acc_coef * self.acc_env as f64 + e * acc_in;
            self.acc_env = st(acc);
            amp = amp * self.amp_fb as f64 + self.gate as f64 * self.amp_in as f64;
            let c = clamp_c(((e1 - ENV_BIAS) * em + cs) + acc * ka);
            let g = (acc + ONE) * amp;
            e = e1;
            let f = &row[(c * C_SCALE as f64) as i64 as i32 as usize];
            let k = Coefs {
                c0: f[0] as f64,
                c1: f[1] as f64,
                g: g * f[2] as f64,
                c3: f[3] as f64,
            };
            for o in quad.iter_mut() {
                *o = v.sample(inc, square, &k);
            }
        }
        // 0x10024841
        self.store_voice(&v);
        self.env = st(e);
        self.amp = st(amp);
    }
}

impl SongDevice for Tb303 {
    /// Slot 1, 0x100253A0.
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
        // 0x10025407: unsigned (t - base) % (48*len)
        let v = (t.wrapping_sub(base) as u32 % period) as i32;
        if self.last_pos.wrapping_sub(v) > 24 {
            if self.gate_off >= 0 {
                self.note_off();
                self.gate_off = -1;
            }
            self.wrapped = true;
        }
        self.last_pos = v;
        let step = (v / 48) & 0xFFFF;
        if self.gate_off >= 0 {
            // 0x1002547E
            if self.enabled == 0 {
                self.slide = false;
            }
            if v >= self.gate_off {
                self.note_off();
                self.gate_off = -1;
            }
            return;
        }
        if step == self.live[LAST_STEP] as i32 && !self.wrapped {
            return;
        }
        // 0x100254D3
        self.gate_off = v + 24;
        self.wrapped = false;
        let mut pat = pat;
        let q = tick / 48;
        let s16 = if q < 0 { -((-q) & 15) } else { q & 15 };
        if s16 == 0 {
            // the bar line: the pending pattern and the enable flag
            let p = self.live[PATTERN];
            if p != self.cur_pattern {
                self.pat_start = 768 * (tick / 768);
                self.cur_pattern = p;
                pat = p;
            }
            self.enabled = self.live[ENABLED];
        }
        if self.enabled != 0 {
            let at = 2 + 6 * step as usize;
            let p = self.pattern(pat);
            let s: [u8; 6] = std::array::from_fn(|i| p.get(at + i).copied().unwrap_or(0));
            self.note_on(s);
        } else {
            self.note_off();
        }
        self.live[LAST_STEP] = step as u8;
    }

    /// Slot 2, 0x100255E0: the argument is not read; the bar of the song's position, the pending
    /// pattern and the enable flag.
    fn relocate(&mut self, _tick: i32, now: i32) {
        self.pat_start = Self::bar_of(now);
        if self.live[PATTERN] != self.cur_pattern {
            self.cur_pattern = self.live[PATTERN];
        }
        self.enabled = self.live[ENABLED];
    }

    /// Slot 3, 0x100255B0.
    fn start(&mut self, now: i32) {
        self.wrapped = true;
        self.pat_start = Self::bar_of(now);
    }

    /// Slot 4, 0x100106C0: slot 6.
    fn stop(&mut self) {
        self.silence();
    }

    /// Slot 5, 0x10026CC0, then slot 6. The cutoff smoother restarts at 0.5; the 2.0
    /// coefficients, the retrigger flag, the patterns and the pattern position are kept.
    fn reset(&mut self) {
        self.wrapped = false;
        self.slide = false;
        self.pitch_target = 0;
        self.pitch = 0;
        self.last_pos = 0;
        self.acc_coef = 0.0;
        self.o = 0.0;
        self.s = 0.0;
        self.y2 = 0.0;
        self.y1 = 0.0;
        self.amp = 0.0;
        self.gate = 0.0;
        self.amp_fb = 0.0;
        self.amp_in = 0.0;
        self.acc_env = 0.0;
        self.acc_amount = 0.0;
        self.env_decay = 0.0;
        self.env = 0.0;
        self.phase = 0.0;
        self.cs2 = 0.5;
        self.cs1 = 0.5;
        self.gate_off = -1;
        self.silence();
    }

    /// Slot 6, 0x10025630: the slide flag cleared, then note-off.
    fn silence(&mut self) {
        self.slide = false;
        self.note_off();
    }

    /// Slot 9, 0x10026B60: entering song mode saves the live block into the pattern-mode copy,
    /// leaving it restores it.
    fn set_song_mode(&mut self, on: bool) {
        if on {
            self.params = self.live;
        } else {
            self.params_live();
        }
        self.song_mode = on;
    }

    /// Msg 39 (0x10025CB0 → `sub_10018EF0`): the parameter block into the pattern-mode copy
    /// and the patterns; in pattern mode the block also goes live (bytes 9..11 included).
    fn data(&mut self, payload: &[u8], song_mode: bool) {
        self.params.copy_from_slice(&payload[..12]);
        self.patterns
            .copy_from_slice(&payload[12..12 + PATTERNS * PATTERN_LEN]);
        if !song_mode {
            self.params_live();
        }
    }

    /// Slot 11, 0x10026180: controllers 0..8 set the live byte. (The "changed" flags at
    /// `+0xC88 + ctrl`, read only by the panel update 0x10026700, are not modelled; the load
    /// accepts no other controller on a 303 track.)
    fn set_param(&mut self, ctrl: u8, value: u8) {
        if (ctrl as usize) <= WAVEFORM {
            self.live[ctrl as usize] = value;
        }
    }

    fn set_sound_model(&mut self, vintage: bool) {
        self.vintage = vintage;
    }

    /// Slots 7 (0x10026C40) and 8 (0x10025650) from the song ctor: controllers 0 and 1 of the
    /// track are quantised to 32 * 24 ticks (a bar, `sub_10021560` → `sub_10014600`), and a
    /// tick-0 event of every controller 0..8 with the ctor's value is inserted
    /// (`sub_1001F900` → `sub_10014750`).
    fn init_track(&mut self, track: &mut Track) {
        track.set_quant(ENABLED as u8, 32);
        track.set_quant(PATTERN as u8, 32);
        for c in 0..=WAVEFORM {
            track.insert(0, c as u8, self.params[c]);
        }
    }
}

impl ChannelDevice for Tb303 {
    /// Slot 14, 0x10023F10.
    fn process(&mut self, bus: &mut Block, ctx: &BlockContext) {
        if self.vintage {
            self.process_vintage(bus, ctx.tables);
        } else {
            self.process_20(bus, ctx.tables);
        }
    }
}
