//! The distortion, an insert effect (`CDistInsert`, vtable 0x10033144, ctor 0x10010700;
//! `docs/initial_investigation.md` section 8.1).
//!
//! One object serves every channel whose distortion switch is on: the mixer block calls its
//! slot 15 once per routed channel, so with two channels routed its counter and its gain
//! smoothing advance twice per block. Every 16 calls the update 0x10010894 sets the gain
//! targets from the amount and the shape (a bilinear interpolation in the 5x5 grid
//! `flt_10151AB8`) and picks the waveshaper row `W[shape >> 2]`; shape 0 selects a hard clip
//! instead.
//!
//! The slots: 4 (stop, 0x100106C0) and 5 (reset, 0x1000F4F0) call slot 6, a no-op
//! (0x10012C00); 7 is a no-op; 8 (0x10010FE0) inserts the pattern copy's controllers 0..2;
//! 9 (0x100111F0) the song-mode switch; 10 (0x10010D60) the messages; 11 (0x10010FB0)
//! set-param; 15 (0x10010A50) the block; 16 (0x10010D40) the mixer update's meter check.

use crate::frame::{BLOCK, Block, BlockContext, InsertEffect, SongDevice};
use crate::seq::Track;
use crate::tables::Tables;

/// The controllers: enabled, amount, shape.
pub const ENABLED: usize = 0;
pub const AMOUNT: usize = 1;
pub const SHAPE: usize = 2;
/// Byte 3 of the parameter block is the meter byte (0x100108D9, 0x100108DE, 0x10010D4C).
pub const METER: usize = 3;

/// `sub_10019B30`: the parameter block's ctor values, for the live and the pattern copy.
pub const DEFAULTS: [u8; 4] = [1, 70, 0, 0];

/// `flt_10151AB8`: the output gain grid `GRID[shape][amount]`, sampled at 0, 1, .., 4 of
/// `x * (1/127) * 3.999`.
pub const GRID: [[f32; 5]; 5] = [
    [
        f32::from_bits(0x3F99_999A), // 1.2
        f32::from_bits(0x3ECC_CCCD), // 0.4
        f32::from_bits(0x3E8F_5C29), // 0.28
        f32::from_bits(0x3E4C_CCCD), // 0.2
        f32::from_bits(0x3E19_999A), // 0.15
    ],
    [
        f32::from_bits(0x3EB3_3333), // 0.35
        f32::from_bits(0x3E4C_CCCD), // 0.2
        f32::from_bits(0x3E19_999A), // 0.15
        f32::from_bits(0x3E0F_5C29), // 0.14
        f32::from_bits(0x3DF5_C28F), // 0.12
    ],
    [
        f32::from_bits(0x3E4C_CCCD), // 0.2
        f32::from_bits(0x3E19_999A), // 0.15
        f32::from_bits(0x3E05_1EB8), // 0.13
        f32::from_bits(0x3DF5_C28F), // 0.12
        f32::from_bits(0x3DF5_C28F), // 0.12
    ],
    [
        f32::from_bits(0x3E19_999A), // 0.15
        f32::from_bits(0x3DF5_C28F), // 0.12
        f32::from_bits(0x3DE1_47AE), // 0.11
        f32::from_bits(0x3DCC_CCCD), // 0.1
        f32::from_bits(0x3DCC_CCCD), // 0.1
    ],
    [
        f32::from_bits(0x3DF5_C28F), // 0.12
        f32::from_bits(0x3DCC_CCCD), // 0.1
        f32::from_bits(0x3DA3_D70A), // 0.08
        f32::from_bits(0x3DA3_D70A), // 0.08
        f32::from_bits(0x3DA3_D70A), // 0.08
    ],
];

// The constants of the update and the block, decoded from their bytes.
/// `dbl_10151B20`: the grid coordinate's scale.
const GRID_SCALE: f64 = f64::from_bits(0x400F_FDF3_B645_A1CB); // 3.999
/// `dbl_10151B28`, `dbl_10151B30`: the input gain `(T[amount] + 0.0625) * 2048`.
const IN_OFFSET: f64 = 0.0625;
const IN_SCALE: f64 = 2048.0;
/// `dbl_10151B38`, `dbl_10151B40`: the output gain `r * 5 * (1/256)`.
const OUT_SCALE: f64 = 5.0;
const OUT_SCALE2: f64 = 0.00390625;
/// `flt_10151B4C`: the meter's decay per update.
const METER_DECAY: f32 = f32::from_bits(0x3F6B_851F); // 0.92
/// `flt_10151B50`: f32 1/127.
const C127: f32 = f32::from_bits(0x3C01_0204);
/// `flt_10151B54`: the meter byte scale.
const METER_SCALE: f32 = 127.0;
/// `dbl_10151B58`: the clip threshold's numerator.
const ONE: f64 = 1.0;
/// `dbl_10151B60`, `dbl_10151B68`: the gain smoothing's keep and take factors.
const KEEP: f64 = f64::from_bits(0x3FEC_CCCC_CCCC_CCCD); // 0.9
const TAKE: f64 = f64::from_bits(0x3FB9_9999_9999_999A); // 0.1
/// `dbl_10151B70`, `dbl_10151B78`: the clip gain `16 * T[amount] + 1`.
const CLIP_SCALE: f64 = 16.0;
const CLIP_OFFSET: f64 = 1.0;
/// `dbl_10151B80`: the lower threshold's sign.
const NEG: f64 = -1.0;
/// The amount from which the clip threshold stops falling (`cmp eax, 5Ah` at 0x10010B0F).
const CLIP_KNEE: u8 = 90;
/// `flt_10151B8C`, `flt_10151B90`, `flt_10151B94`: the shaper's lower bound, its input offset
/// and its upper bound. Above 254 the output is `W[254]` (`fld [ebp+3F8h]`), not `W[255]`.
const SHAPER_LO: f32 = 0.0;
const SHAPER_MID: f32 = 128.0;
const SHAPER_HI: f32 = 254.0;

fn st(x: f64) -> f32 {
    x as f32
}

/// `|x|` as 0x10010AA0 takes it (the sign flipped when the bits are above 0x80000000).
fn fabs(x: f32) -> f32 {
    if x.to_bits() > 0x8000_0000 { -x } else { x }
}

/// An amount or shape byte clamped to 127. The exe's track load accepts only 0..127 for them
/// (`docs/song_load.md` section 5), and the DLL does not check: a larger byte would read past the grid
/// (`flt_10151AB8`), the level curve (`flt_10054550`) and the shaper (`flt_100D47A4`) into
/// adjacent memory, which the engine cannot reproduce, so it clamps instead.
fn clamp_param(x: u8) -> u8 {
    x.min(127)
}

/// The grid coordinate of a parameter byte (0x10010909..0x10010920 for the shape,
/// 0x10010926..0x1001097D for the amount): `(x * f32(1/127)) * 3.999`.
fn coord(x: u8) -> f64 {
    (x as f64 * C127 as f64) * GRID_SCALE
}

/// The output gain's grid value at `(shape, amount)`, 0x10010966..0x10010A34: with
/// `s = coord(shape)`, `a = coord(amount)`, their truncations `i`, `j` and fractions `fs`,
/// `fa`:
/// ```text
/// top = (G[i+1][j+1] - G[i+1][j]) * fa + G[i+1][j]
/// bot = (G[i][j+1] - G[i][j]) * fa + G[i][j]
/// r = fs * (top - bot) + bot
/// ```
pub fn grid(shape: u8, amount: u8) -> f64 {
    let s = coord(shape);
    let a = coord(amount);
    // `fistp` truncating; the low 16 bits are kept (`mov bp, word`, `mov bl/bh`)
    let i = s as i64 as u16 as usize;
    let j = a as i64 as u16 as usize;
    let fs = s - i as f64;
    let fa = a - j as f64;
    let g = |r: usize, c: usize| GRID[r][c] as f64;
    let top = (g(i + 1, j + 1) - g(i + 1, j)) * fa + g(i + 1, j);
    let bot = (g(i, j + 1) - g(i, j)) * fa + g(i, j);
    fs * (top - bot) + bot
}

/// The waveshaper lookup of 0x10010BF9..0x10010C85 on `v = gIn*x + 128`: `W[0]` below 0,
/// `W[254]` above 254, else the linear interpolation `(v - i) * (W[i+1] - W[i]) + W[i]` with
/// `i = trunc(v)`.
pub fn shape(w: &[f32; 256], v: f64) -> f64 {
    if v < SHAPER_LO as f64 {
        w[0] as f64
    } else if v > SHAPER_HI as f64 {
        w[254] as f64
    } else {
        let i = v as i64 as usize;
        (v - i as f64) * (w[i + 1] as f64 - w[i] as f64) + w[i] as f64
    }
}

/// The clip of shape 0 (0x10010B4E..0x10010BA8): `th` if `y > th`, else `-th` if `y < -th`,
/// else `y`.
fn clip(y: f64, th: f64, nth: f64) -> f64 {
    if y > th {
        th
    } else if y < nth {
        nth
    } else {
        y
    }
}

/// One step of the per-call gain smoothing (0x10010AC2..0x10010AF6): `f32(g*0.9 + t*0.1)`;
/// the block reads the stored f32.
pub fn smooth(gain: f32, target: f32) -> f32 {
    st(gain as f64 * KEEP + target as f64 * TAKE)
}

/// The distortion object.
#[derive(Debug, Clone)]
pub struct Distortion {
    /// `+0x39`: the live parameters, which the audio reads.
    pub live: [u8; 4],
    /// `+0x3D`: the pattern-mode copy.
    pub pattern: [u8; 4],
    /// `+0x30`: the call counter (word); the update runs when `counter & 15 == 0`.
    pub counter: u16,
    /// `+0x34`: the counter at the last mixer update (slot 16).
    pub last_counter: u16,
    /// `+0x38`: shape 0, the hard clip.
    pub clip: bool,
    /// `+0x44`, `+0x48`: the smoothed input and output gains.
    pub gain_in: f32,
    pub gain_out: f32,
    /// `+0x54`, `+0x58`: their targets.
    pub target_in: f32,
    pub target_out: f32,
    /// `+0x50`: the peak meter of the input (display only).
    pub peak: f32,
    /// `+0x5C`: the waveshaper row, `shape >> 2`.
    pub row: u16,
}

impl Default for Distortion {
    fn default() -> Self {
        Self::new()
    }
}

impl Distortion {
    /// The ctor's state (0x10010700: the parameter blocks at their defaults, the clip flag,
    /// the row, the gains, the targets and the meter 0).
    pub fn new() -> Self {
        Distortion {
            live: DEFAULTS,
            pattern: DEFAULTS,
            counter: 0,
            last_counter: 0,
            clip: false,
            gain_in: 0.0,
            gain_out: 0.0,
            target_in: 0.0,
            target_out: 0.0,
            peak: 0.0,
            row: 0,
        }
    }

    /// The update 0x10010894.
    fn param_update(&mut self, t: &Tables) {
        // 0x1001089B: the meter byte (an integer compare of the float bits), then the decay
        if self.peak.to_bits() as i32 > 1.0f32.to_bits() as i32 {
            self.live[METER] = 0x7F;
        } else {
            self.live[METER] = (self.peak as f64 * METER_SCALE as f64) as i64 as u8;
        }
        self.peak = st(self.peak as f64 * METER_DECAY as f64);
        let shape = self.live[SHAPE];
        let amount = self.live[AMOUNT];
        self.clip = shape == 0;
        // 0x10010932: (T[amount] + 0.0625) * 2048
        self.target_in = st((t.level[amount as usize] as f64 + IN_OFFSET) * IN_SCALE);
        // 0x10010A0E..0x10010A27: (shape * 32) / 128, a signed division by shifts
        self.row = (shape as u16) >> 2;
        // 0x10010A36: r * 5 * (1/256)
        self.target_out = st(grid(shape, amount) * OUT_SCALE * OUT_SCALE2);
    }
}

impl SongDevice for Distortion {
    /// Slot 9, 0x100111F0: entering song mode saves the live block into the pattern copy,
    /// leaving it restores the live block from the copy (all four bytes).
    fn set_song_mode(&mut self, on: bool) {
        if on {
            self.pattern = self.live;
        } else {
            self.live = self.pattern;
        }
    }

    /// Msg 94 (0x10010F77): the block goes into the pattern copy (`sub_10019B70`, four bytes),
    /// and into the live block only in pattern mode (`sub_1001FCB0` zero).
    fn data(&mut self, payload: &[u8], song_mode: bool) {
        self.pattern.copy_from_slice(&payload[..4]);
        for c in [AMOUNT, SHAPE] {
            self.pattern[c] = clamp_param(self.pattern[c]);
        }
        if !song_mode {
            self.live = self.pattern;
        }
    }

    /// Slot 11, 0x10010FB0: controllers 0..2 set the live block (any controller also sets a UI
    /// change flag at `+0x4C + ctrl`, not modelled; the load accepts only 0..2 on track 6).
    fn set_param(&mut self, ctrl: u8, value: u8) {
        match ctrl as usize {
            ENABLED => self.live[ENABLED] = value,
            c @ (AMOUNT | SHAPE) => self.live[c] = clamp_param(value),
            _ => {}
        }
    }

    /// Slot 8, 0x10010FE0, with tick 0 from the song ctor: controllers 0..2 of the pattern copy
    /// inserted into the distortion's track (`sub_1001F900`; controller 3 is skipped).
    fn init_track(&mut self, track: &mut Track) {
        for c in 0..=SHAPE {
            track.insert(0, c as u8, self.pattern[c]);
        }
    }
}

impl InsertEffect for Distortion {
    /// Slot 15, 0x10010A50: nothing when disabled (the bus passes unchanged and the counter
    /// stands).
    fn process(&mut self, bus: &mut Block, ctx: &BlockContext) {
        if self.live[ENABLED] == 0 {
            return;
        }
        if self.counter & 15 == 0 {
            self.param_update(ctx.tables);
        }
        self.counter = self.counter.wrapping_add(1);
        // 0x10010A94: the input's peak at the meter index
        let v = fabs(bus[ctx.meter_index]);
        if v > self.peak {
            self.peak = v;
        }
        self.gain_in = smooth(self.gain_in, self.target_in);
        self.gain_out = smooth(self.gain_out, self.target_out);
        if self.clip {
            // 0x10010B01: g = 16*T[a] + 1, th = 1/(16*T[min(a, 90)] + 1); the gains are unused
            let t = &ctx.tables.level;
            let a = self.live[AMOUNT];
            let g = CLIP_SCALE * t[a as usize] as f64 + CLIP_OFFSET;
            let h = if a < CLIP_KNEE {
                g
            } else {
                CLIP_SCALE * t[CLIP_KNEE as usize] as f64 + CLIP_OFFSET
            };
            let th = ONE / h;
            let nth = th * NEG;
            for x in bus.iter_mut() {
                *x = st(clip(g * *x as f64, th, nth));
            }
        } else {
            // 0x10010BD3: the row's lookup, out = f32(y * gOut)
            let w = &ctx.tables.dist_shaper[self.row as usize];
            let gi = self.gain_in as f64;
            let go = self.gain_out as f64;
            for x in bus.iter_mut().take(BLOCK) {
                let v = gi * *x as f64 + SHAPER_MID as f64;
                *x = st(shape(w, v) * go);
            }
        }
    }

    /// Slot 16, 0x10010D40, run by every mixer update: the meter byte is cleared when no call
    /// came since the last update.
    fn update(&mut self, _ctx: &BlockContext) {
        if self.counter == self.last_counter {
            self.live[METER] = 0;
        }
        self.last_counter = self.counter;
    }
}

#[cfg(test)]
mod tests;
