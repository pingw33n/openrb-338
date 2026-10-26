//! The engine's lookup tables (`docs/initial_investigation.md` sections 4, 6, 7, 8 and 10 item 4).
//!
//! The original builds these at run time, in constructors and static initializers of
//! ReBirth.dll; every function here follows its builder instruction by instruction: the same
//! operand order, f64 arithmetic (the x87 runs at 53-bit precision), an f32 rounding exactly where
//! the builder stores a float, and truncation where it converts to an integer. The transcendentals
//! (x87 `fcos`, `fsin`, `f2xm1`/`fscale` for `exp`, and the CRT `pow` at 0x1002DDF0) are
//! approximated by the pure-Rust `libm` crate (pinned `=0.2.16`, task-048), so the tables are the
//! same on every platform. Every generated table is bit-identical to the original's (f32 bits),
//! checked against dumps of the running DLL in `tests.rs`, so no table is embedded. Across the
//! f32 stores the nearest f64 value lies at least 1,358 f64 ulps from an f32 rounding midpoint
//! (the test `store_margins`), so the tables do not hinge on the last ulp of the math library;
//! the one fragile entry is `T808[64]` (see [`tr808_tom_step`]).
//!
//! The static tables, which the original keeps as initialized data, are in [`data`].
//!
//! The f64 and f32 constants below are the builders' operands. A `C_xxxxxxxx` constant is data at
//! that address; the others are immediates of `mov` instructions, with their addresses in their
//! docs. The tests check every one against the DLL's bytes.

pub mod data;

#[cfg(test)]
mod tests;

use libm::{cos, exp, pow, sin};

/// An `fstp dword`: the f64 value rounded to f32. Tests record how close it came to an f32
/// rounding midpoint.
#[inline]
fn st(x: f64) -> f32 {
    #[cfg(test)]
    tests::record_store(x);
    x as f32
}

/// A level, send or amount knob value 0..127 mapped to a gain.
pub type LevelCurve = [f32; 128];
/// `PAN[a][b] = [L, R]` for level `a` and pan `b`.
pub type PanTable = [[[f32; 2]; 128]; 128];
/// The TB-303 phase increment per sample, indexed by `pitch >> 4`.
pub type PitchTable = [f32; 6144];
/// The TB-303 filter `FILT[reso >> 4][cutoff] = [c0, c1, c2, c3]`.
pub type Tb303Filter = [[[f32; 4]; 1024]; 8];
/// A PCF filter `[res >> 4][env] = [c0, c1, c2]`.
pub type PcfFilter = [[[f32; 3]; 512]; 8];
/// The distortion waveshaper `W[shape >> 2][x]`.
pub type DistShaper = [[f32; 256]; 32];

/// A 2-pole cutoff smoother `y = (a*y1 + x*g) - b2*y2`, built by a static initializer.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Smoother {
    pub b2: f32,
    pub a: f32,
    pub g: f32,
}

/// Every computed table, built once.
pub struct Tables {
    pub level: LevelCurve,
    pub pan: Box<PanTable>,
    pub tb303_pitch: Box<PitchTable>,
    pub tb303_filter: Box<Tb303Filter>,
    pub pcf_lowpass: Box<PcfFilter>,
    pub pcf_bandpass: Box<PcfFilter>,
    pub dist_shaper: Box<DistShaper>,
    pub tr808_tom_step: [u16; 128],
    pub tr909_tom_step: [u16; 128],
    pub tr909_decay: [f32; 16],
    pub tb303_smoother: Smoother,
    pub pcf_smoother: Smoother,
}

impl Tables {
    pub fn new() -> Self {
        let level = level_curve();
        Tables {
            pan: pan_table(&level),
            level,
            tb303_pitch: tb303_pitch(),
            tb303_filter: tb303_filter(),
            pcf_lowpass: pcf_lowpass(),
            pcf_bandpass: pcf_bandpass(),
            dist_shaper: dist_shaper(),
            tr808_tom_step: tr808_tom_step(),
            tr909_tom_step: tr909_tom_step(),
            tr909_decay: tr909_decay(),
            tb303_smoother: tb303_smoother(),
            pcf_smoother: pcf_smoother(),
        }
    }
}

impl Default for Tables {
    fn default() -> Self {
        Self::new()
    }
}

fn boxed<T: Copy, const N: usize>(v: T) -> Box<[T; N]> {
    vec![v; N].into_boxed_slice().try_into().ok().unwrap()
}

// ------------------------------------------------------------------ level curve and pan

/// `T[127] = 1` (0x100046B4).
const T_TOP: f32 = 1.0;

/// The level curve `T`, `sub_100046B0` (song ctor, guard `byte_100E0128`; at 0x10054550):
/// `T[i] = f32(T[i+1] - step*T[i+1])` for i = 126 down to 1, `T[0] = 0`.
pub fn level_curve() -> LevelCurve {
    let mut t = [0.0f32; 128];
    t[127] = T_TOP;
    let step = data::LEVEL_STEP as f64;
    for i in (1..=126).rev() {
        let next = t[i + 1] as f64;
        t[i] = st(next - step * next);
    }
    t[0] = 0.0;
    t
}

/// `PAN[a][b]`, `sub_100046B0` (at 0x10034550): with `d = b - 64` and `q = d*d >> 5`,
/// `L = T[d > 0 ? clamp(a - q, 0, 127) : a]` and `R = T[d < 0 ? clamp(a - q, 0, 127) : a]`
/// (`sub_10004660` is the clamp).
pub fn pan_table(t: &LevelCurve) -> Box<PanTable> {
    let mut pan: Box<PanTable> = boxed([[0.0; 2]; 128]);
    for a in 0..128i32 {
        for b in 0..128i32 {
            let d = b - 64;
            let q = (d * d) >> 5;
            let cut = (a - q).clamp(0, 127) as usize;
            let l = if d > 0 { cut } else { a as usize };
            let r = if d < 0 { cut } else { a as usize };
            pan[a as usize][b as usize] = [t[l], t[r]];
        }
    }
    pan
}

// ------------------------------------------------------------------ TB-303

/// `pow(2.0, 1/12)`: 2.0 high dword at 0x10023A8E, 1/12 at 0x10023A93 (low) / 0x10023A9B (high).
const PITCH_SEMI_EXP: f64 = f64::from_bits(0x3FB5_5555_5555_5555);
const C_10153A90: f64 = 1.0;
const C_10153A98: f64 = 55.0;
const C_10153A80: f64 = 2.0;
/// 1/44100.
const C_10153AB0: f64 = f64::from_bits(0x3EF7_C6F8_C751_F177);
/// `pow(2.0, 1/768)`: 1/768 at 0x10023AF0 (low) / 0x10023B02 (high).
const PITCH_STEP_EXP: f64 = f64::from_bits(0x3F55_5555_5555_5555);
/// 3072.0, high dword at 0x10023B0E.
const PITCH_ORIGIN: f64 = 3072.0;

/// The TB-303 pitch table, 303 ctor `sub_10023920` (guard `byte_10124704`; at 0x10125364):
/// `k = (((1*s)*(s*s))*55*2 * (1/44100)) / pow(p, 3072)` with `s = pow(2, 1/12)` (the cube by
/// the compiler's squaring loop) and `p = pow(2, 1/768)`, then `tab[i] = f32(pow(p, i) * k)`.
pub fn tb303_pitch() -> Box<PitchTable> {
    let s = pow(2.0, PITCH_SEMI_EXP);
    // exponent 3 by squaring (0x10023AC3..0x10023AE3): acc = 1*s, then acc *= s*s
    let cube = (C_10153A90 * s) * (s * s);
    let v18 = (cube * C_10153A98) * C_10153A80;
    let p = pow(2.0, PITCH_STEP_EXP);
    let k = (v18 * C_10153AB0) / pow(p, PITCH_ORIGIN);
    let mut tab: Box<PitchTable> = boxed(0.0);
    for (i, e) in tab.iter_mut().enumerate() {
        *e = st(pow(p, i as f64) * k);
    }
    tab
}

/// `pow(1602.0, 1/1023)`: 1602.0 at 0x100234BC (high) / 0x100234B7 (low), 1/1023 at
/// 0x100234C1 (low) / 0x100234C9 (high).
const FILT_SPAN: f64 = 1602.0;
const FILT_SPAN_EXP: f64 = f64::from_bits(0x3F50_0401_0040_1004);
/// 1023.0, high dword at 0x100234DA.
const FILT_TOP: f64 = 1023.0;
/// f32 1.78 widened: 0x10023507 (low) / 0x10023513 (high); the same at 0x100172FB / 0x10017307
/// (PCF LP) and 0x100175AB / 0x100175B7 (PCF BP).
const Q_BASE: f64 = f64::from_bits(0x3FFC_7AE1_4000_0000);
const C_101539C8: f64 = 0.4;
const C_101539D8: f64 = 0.52;
/// f32 1/7.
const C_10153A58: f32 = f32::from_bits(0x3E12_4925);
const C_101539E0: f64 = f64::from_bits(0x3FC5_5555_5555_5555);
const C_101539E8: f64 = 0.5;
const C_101539F0: f64 = 0.6;
const C_101539F8: f64 = 128.0;
const C_10153A00: f64 = 1.0 / 1024.0;
const C_10153A08: f64 = 0.000125;
const C_10153A10: f64 = 0.3;
const C_10153A5C: f32 = 1.0;
const C_10153A60: f32 = 0.0;
const C_10153A18: f64 = 1.0;
/// The decimal literal 6.283185308.
const C_10153A20: f64 = f64::from_bits(0x4019_21FB_5452_4550);
const C_101539A8: f64 = 1.0;
/// -3.141592654.
const C_10153A28: f64 = f64::from_bits(0xC009_21FB_5452_4550);
const C_10153A30: f64 = 2.0;
/// -6.283185308.
const C_10153A38: f64 = f64::from_bits(0xC019_21FB_5452_4550);
const C_10153A40: f64 = 0.85;
const C_10153A48: f64 = 0.15;
const C_10153A64: f32 = 4.0;
const C_10153A68: f32 = 3.0;
const C_10153A50: f64 = 0.8;

/// The TB-303 filter table FILT, `sub_100234A0` (every 303 ctor, guard `byte_10124700`; at
/// 0x1012B364). See `docs/initial_investigation.md` section 6.2 for the formula.
pub fn tb303_filter() -> Box<Tb303Filter> {
    let k = pow(FILT_SPAN, FILT_SPAN_EXP);
    let base = C_101539C8 / pow(k, FILT_TOP);
    let mut tab: Box<Tb303Filter> = boxed([[0.0; 4]; 1024]);
    for (r, row) in tab.iter_mut().enumerate() {
        let rf = r as f64;
        let pm = pow(Q_BASE, rf) * C_101539D8;
        let t = rf * C_10153A58 as f64;
        let m94 = rf * C_101539E0 + C_101539E8;
        let aw = t * C_101539C8 + C_101539F0;
        for (i, e) in row.iter_mut().enumerate() {
            let fi = i as f64;
            let w = (pow(k, fi) * base) * aw;
            let u = (fi * C_101539F8) * C_10153A00;
            let mut q = ((((u * fi) * C_101539F8) * C_10153A00) * C_10153A08) - C_10153A10;
            if q > C_10153A5C as f64 {
                q = C_10153A5C as f64;
            }
            if q < C_10153A60 as f64 {
                q = C_10153A60 as f64;
            }
            let d = q * pm + (C_10153A18 - q) * m94;
            let inv_d = C_101539A8 / d;
            let cs = cos(w * C_10153A20);
            let e1 = exp((w * C_10153A28) * inv_d);
            let c0 = e1 * (cs * C_10153A30);
            let w2 = w * C_10153A38;
            let c1 = exp(w2 * inv_d);
            let one_c0 = C_10153A18 - c0;
            let c3 = exp((t * C_10153A40 + C_10153A48) * w2);
            let num = (C_10153A18 - c3) * (c1 + one_c0);
            let dq = if d > C_10153A64 as f64 {
                C_10153A68 as f64
            } else {
                d
            };
            let c2 = num / ((dq + C_10153A5C as f64) * C_10153A50);
            *e = [st(c0), st(c1), st(c2), st(c3)];
        }
    }
    tab
}

/// The smoothers' angle w = f32 0.0628 widened (0x10153DE0, 0x10152490).
const SMOOTH_W: f64 = f64::from_bits(0x3FB0_15BF_8C18_0485);
/// -w/2 (0x10153DE8, 0x10152498).
const SMOOTH_HALF_NEG_W: f64 = f64::from_bits(0xBFA0_15BF_8C18_0485);
/// 2.0 (0x10153DF0, 0x101524A0).
const SMOOTH_TWO: f64 = 2.0;
/// -w (0x10153DF8, 0x101524A8).
const SMOOTH_NEG_W: f64 = f64::from_bits(0xBFB0_15BF_8C18_0485);
/// 1.0 (0x10153E00, 0x101524B0).
const SMOOTH_ONE: f64 = 1.0;

/// The static initializers `sub_10026BB0` (303, at 0x1012470C) and `sub_10017840` (PCF, at
/// 0x10100260) compute the same constants: `A = exp(-w/2) * (cos(w)*2)`, `b2 = exp(-w)`,
/// `g = (1 - A) + b2`, stored as f32 in the order b2, A, g.
fn smoother() -> Smoother {
    let cs = cos(SMOOTH_W);
    let e1 = exp(SMOOTH_HALF_NEG_W);
    let a = e1 * (cs * SMOOTH_TWO);
    let b2 = exp(SMOOTH_NEG_W);
    let g = (SMOOTH_ONE - a) + b2;
    Smoother {
        b2: st(b2),
        a: st(a),
        g: st(g),
    }
}

/// The TB-303 cutoff smoother (`sub_10026BB0`, at 0x1012470C).
pub fn tb303_smoother() -> Smoother {
    smoother()
}

/// The PCF cutoff smoother (`sub_10017840`, at 0x10100260).
pub fn pcf_smoother() -> Smoother {
    smoother()
}

// ------------------------------------------------------------------ PCF

/// The PCF base `K` = 1.0145453214645386: 0x100172CE (low) / 0x100172D3 (high), and in the loop
/// 0x100173A1 / 0x100173AA; BP 0x1001757E / 0x10017583 and 0x10017651 / 0x1001765A.
const PCF_K: f64 = f64::from_bits(0x3FF0_3B93_E000_0000);
/// 511.0, high dword at 0x100172DD (BP 0x1001758D).
const PCF_TOP: f64 = 511.0;

/// The constants of one PCF builder, in the order the builder reads them.
struct PcfConsts {
    c04: f64,
    c052: f64,
    c17: f32,
    c16: f64,
    c05: f64,
    c06: f64,
    c128: f64,
    c512: f64,
    cq: f64,
    c03: f64,
    one_f: f32,
    zero_f: f32,
    one: f64,
    two_pi: f64,
    one_d: f64,
    neg_pi: f64,
    two: f64,
    neg_two_pi: f64,
    four_f: f32,
    three_f: f32,
    c08: f64,
}

/// `sub_100172C4` (LP) data: 0x10152340, 0x10152350, 0x101523C0, 0x10152358, 0x10152360,
/// 0x10152368, 0x10152370, 0x10152378, 0x10152380, 0x10152388, 0x101523C4, 0x101523C8,
/// 0x10152390, 0x10152398, 0x10152328, 0x101523A0, 0x101523A8, 0x101523B0, 0x101523D0,
/// 0x101523D4, 0x101523B8.
const PCF_LP: PcfConsts = PcfConsts {
    c04: 0.4,
    c052: 0.52,
    c17: f32::from_bits(0x3E12_4925),
    c16: f64::from_bits(0x3FC5_5555_5555_5555),
    c05: 0.5,
    c06: 0.6,
    c128: 128.0,
    c512: 1.0 / 512.0,
    cq: 0.000125,
    c03: 0.3,
    one_f: 1.0,
    zero_f: 0.0,
    one: 1.0,
    two_pi: f64::from_bits(0x4019_21FB_5452_4550),
    one_d: 1.0,
    neg_pi: f64::from_bits(0xC009_21FB_5452_4550),
    two: 2.0,
    neg_two_pi: f64::from_bits(0xC019_21FB_5452_4550),
    four_f: 4.0,
    three_f: 3.0,
    c08: 0.8,
};
/// LP: `(c1 + (1 - c0)) * 2f` (0x101523CC).
const C_101523CC: f32 = 2.0;

/// `sub_10017574` (BP) data: 0x101523F0, 0x10152400, 0x10152478, 0x10152408, 0x10152410,
/// 0x10152418, 0x10152420, 0x10152428, 0x10152430, 0x10152438, 0x1015247C, 0x10152480,
/// 0x10152440, 0x10152448, 0x101523D8, 0x10152450, 0x10152458, 0x10152460, 0x10152488,
/// 0x10152484, 0x10152470.
const PCF_BP: PcfConsts = PcfConsts {
    c04: 0.4,
    c052: 0.52,
    c17: f32::from_bits(0x3E12_4925),
    c16: f64::from_bits(0x3FC5_5555_5555_5555),
    c05: 0.5,
    c06: 0.6,
    c128: 128.0,
    c512: 1.0 / 512.0,
    cq: 0.000125,
    c03: 0.3,
    one_f: 1.0,
    zero_f: 0.0,
    one: 1.0,
    two_pi: f64::from_bits(0x4019_21FB_5452_4550),
    one_d: 1.0,
    neg_pi: f64::from_bits(0xC009_21FB_5452_4550),
    two: 2.0,
    neg_two_pi: f64::from_bits(0xC019_21FB_5452_4550),
    four_f: 4.0,
    three_f: 3.0,
    c08: 0.8,
};
/// BP: `cos(W*pi)`, pi = 3.141592654 (0x10152468).
const C_10152468: f64 = f64::from_bits(0x4009_21FB_5452_4550);

/// One PCF table; `c2` is the only difference between LP and BP.
fn pcf_filter(c: &PcfConsts, bandpass: bool) -> Box<PcfFilter> {
    let base = c.c04 / pow(PCF_K, PCF_TOP);
    let mut tab: Box<PcfFilter> = boxed([[0.0; 3]; 512]);
    for (r, row) in tab.iter_mut().enumerate() {
        let rf = r as f64;
        let pm = pow(Q_BASE, rf) * c.c052;
        let t = rf * c.c17 as f64;
        let m94 = rf * c.c16 + c.c05;
        let aw = t * c.c04 + c.c06;
        for (i, e) in row.iter_mut().enumerate() {
            let fi = i as f64;
            let w = (pow(PCF_K, fi) * base) * aw;
            let u = (fi * c.c128) * c.c512;
            let mut q = ((((u * fi) * c.c128) * c.c512) * c.cq) - c.c03;
            if q > c.one_f as f64 {
                q = c.one_f as f64;
            }
            if q < c.zero_f as f64 {
                q = c.zero_f as f64;
            }
            let d = q * pm + (c.one - q) * m94;
            let inv_d = c.one_d / d;
            let cs = cos(w * c.two_pi);
            let e1 = exp((w * c.neg_pi) * inv_d);
            let c0 = e1 * (cs * c.two);
            let c1 = exp((w * c.neg_two_pi) * inv_d);
            let num = if bandpass {
                // 0x10017789..0x100177A9: cos(W*pi) * sqrt((1f - c0) + c1) * 3f
                let cos_pi = cos(w * C_10152468);
                (cos_pi * ((c.one_f as f64 - c0) + c1).sqrt()) * c.three_f as f64
            } else {
                (c1 + (c.one - c0)) * C_101523CC as f64
            };
            let dq = if d > c.four_f as f64 {
                c.three_f as f64
            } else {
                d
            };
            let c2 = num / ((dq + c.one_f as f64) * c.c08);
            *e = [st(c0), st(c1), st(c2)];
        }
    }
    tab
}

/// The PCF low-pass table, `sub_100172C4` (PCF ctor `sub_10016490`, guard `byte_1010026C`; at
/// 0x1010C270).
pub fn pcf_lowpass() -> Box<PcfFilter> {
    pcf_filter(&PCF_LP, false)
}

/// The PCF band-pass table, `sub_10017574` (same guard; at 0x10100270).
pub fn pcf_bandpass() -> Box<PcfFilter> {
    pcf_filter(&PCF_BP, true)
}

// ------------------------------------------------------------------ distortion

const C_10151BFC: f32 = 0.0;
/// f32 1/961.
const C_10151C00: f32 = f32::from_bits(0x3A88_642A);
/// f32 0.3926987648, pi/8 with pi cut short: not f32(pi/8) = 0x3EC90FDB.
const C_10151C04: f32 = f32::from_bits(0x3EC9_0FD0);
/// f32 0.0030679591, pi/1024 cut the same way: not f32(pi/1024) = 0x3B490FDB.
const C_10151C08: f32 = f32::from_bits(0x3B49_0FD0);
const C_10151C0C: f32 = 256.0;
const C_10151BE0: f64 = 1.0;
const C_10151BF0: f64 = 1.0 / 128.0;
const C_10151BE8: f64 = 0.7;

/// The distortion waveshaper W, `sub_10011210` (dist ctor, guard `byte_100D47A0`; at
/// 0x100D47A4). Per row t, with `inc = (t*t * (1/961)f) * (pi/8)f + (pi/1024)f` and `ph`, `y`
/// and `z` starting at 0 and advancing by `inc`, 1/128 and 1/128 (all f64) for e = 128..255:
/// `v = ((sin(ph)*256f)*(1 - y) + (y*z)*256f) * 0.7`, `W[t][e] = f32(v)`, then
/// `W[t][256 - e] = f32(-v)` (so `W[t][128]` is -0.0), and finally `W[t][0] = -W[t][255]`.
pub fn dist_shaper() -> Box<DistShaper> {
    let mut tab: Box<DistShaper> = boxed([0.0; 256]);
    for (t, row) in tab.iter_mut().enumerate() {
        let tt = (t * t) as f64;
        let inc = (tt * C_10151C00 as f64) * C_10151C04 as f64 + C_10151C08 as f64;
        let mut ph = C_10151BFC as f64;
        let mut y = C_10151BFC as f64;
        let mut z = C_10151BFC as f64;
        for e in 128..256 {
            let s = sin(ph) * C_10151C0C as f64;
            let v = ((s * (C_10151BE0 - y)) + ((y * z) * C_10151C0C as f64)) * C_10151BE8;
            row[e] = st(v);
            row[256 - e] = st(-v);
            y += C_10151BF0;
            z += C_10151BF0;
            ph += inc;
        }
        row[0] = -row[255];
    }
    tab
}

// ------------------------------------------------------------------ drums

/// The tom step table of an 808 or 909 ctor: `k = 256 / s^64` with `s = pow(2, e)` raised by
/// the compiler's squaring loop, then `tab[i] = trunc(pow(pow(2, e), i) * k)` (truncating
/// `fistp`, the low 16 bits stored).
fn tom_step(e2: f64, one: f64, top: f64) -> [u16; 128] {
    let s = pow(2.0, e2);
    // exponent 64 by squaring: s^2, s^4, ..., s^64, then acc = 1 * s^64
    let mut sq = s;
    for _ in 0..6 {
        sq = sq * sq;
    }
    let acc = one * sq;
    let k = top / acc;
    let p = pow(2.0, e2);
    let mut tab = [0u16; 128];
    for (i, e) in tab.iter_mut().enumerate() {
        *e = (pow(p, i as f64) * k) as i64 as u16;
    }
    tab
}

/// 808: `pow(2.0, 1/192)`, 2.0 high dword at 0x10027066, 1/192 at 0x1002706B (low) /
/// 0x10027073 (high); squaring count 64 at 0x10027093.
const T808_EXP: f64 = f64::from_bits(0x3F75_5555_5555_5555);
const C_10153EA0: f64 = 1.0;
const C_10153EA8: f64 = 256.0;
/// 909: `pow(2.0, 1/144)`, 2.0 high dword at 0x1002A476, 1/144 at 0x1002A47B (low) /
/// 0x1002A483 (high); squaring count 64 at 0x1002A4A3.
const T909_EXP: f64 = f64::from_bits(0x3F7C_71C7_1C71_C71C);
const C_101542A0: f64 = 1.0;
const C_101542B0: f64 = 256.0;

/// T808, the 808 ctor `sub_10026F70`, first instance (guard `word_1014CCD8`; at 0x1014CBD8).
///
/// `T808[64]` is `trunc(pow(s, 64) * (256 / s^64))`: it is 256 only because `pow` gives the
/// squared value bit for bit (the product is exactly 256.0); one ulp less would truncate to 255.
pub fn tr808_tom_step() -> [u16; 128] {
    tom_step(T808_EXP, C_10153EA0, C_10153EA8)
}

/// T909, the 909 ctor `sub_1002A380`, first instance (guard `word_1014E68C`; at 0x1014E54C).
pub fn tr909_tom_step() -> [u16; 128] {
    tom_step(T909_EXP, C_101542A0, C_101542B0)
}

const C_1015431C: f32 = 1.0;
const C_101542A8: f64 = 2.5e-5;

/// DT, the 909 ctor, the same block (at 0x1014E64C): `x = 1f`, then for i = 15 down to 0
/// `DT[i] = f32(x); x = x - 2.5e-5` with x kept in f64.
pub fn tr909_decay() -> [f32; 16] {
    let mut tab = [0.0f32; 16];
    let mut x = C_1015431C as f64;
    for e in tab.iter_mut().rev() {
        *e = st(x);
        x -= C_101542A8;
    }
    tab
}
