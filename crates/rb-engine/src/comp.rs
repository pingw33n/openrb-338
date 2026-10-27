//! The compressor, an insert effect (`CCompInsert`, vtable 0x10032B24, ctor 0x1000E990;
//! `docs/initial_investigation.md` section 8.3).
//!
//! It sits on one channel (mono, slot 15, 0x1000EC70) or on the master (stereo, slot 14,
//! 0x1000EDE0, linked: one detector over both buses). Every 16 calls the update 0x1000EB44
//! sets the gain target's factors from the amount and the threshold. Per group of 4 samples
//! the detector takes `|x1 + x3|`, the envelope attacks towards it or, after a hold of 300
//! groups, releases, never below the floor, and the gain ramps linearly in 4 steps towards
//! `X/env + off`. The envelope and the gain stay in x87 registers for the whole block and are
//! stored as f32 at its end.
//!
//! The slots: 4 (stop, 0x100106C0) and 5 (reset, 0x1000F4F0) call slot 6, a no-op
//! (0x10012C00); 7 is a no-op (0x1000F4E0); 8 (0x1000F420) inserts the pattern copy's
//! controllers 0..2; 9 (0x1000F4B0) the song-mode switch; 10 (0x1000EFD0) the messages;
//! 11 (0x1000F220) set-param; 14 the stereo block; 15 the mono block; 16 (0x1000EFB0) the
//! mixer update's meter check.

use crate::frame::{Block, BlockContext, InsertEffect, SongDevice};
use crate::seq::Track;

/// The controllers: enabled, amount, threshold.
pub const ENABLED: usize = 0;
pub const AMOUNT: usize = 1;
pub const THRESHOLD: usize = 2;
/// Bytes 3 and 4 of the parameter block are the meter bytes the update writes: the gain
/// (`min(G*18, 127)`, 0x1000EBA0) and the input peak (0x1000EC2C, 0x1000EC6B). The exe sends
/// 127 and 0 there (`docs/song_load.md` section 4.5), which the first update overwrites.
pub const GAIN_METER: usize = 3;
pub const PEAK_METER: usize = 4;

/// `sub_10019B90`: the parameter block's ctor values, for the live and the pattern copy.
pub const DEFAULTS: [u8; 5] = [1, 70, 64, 127, 0];

// The ctor's values (0x1000EA0C..0x1000EA3B), decoded from their bytes.
/// `+0x54`: the attack factor (0x3C23D70A).
pub const ATTACK: f32 = f32::from_bits(0x3C23_D70A); // 0.01
/// `+0x58`: the release factor (0x3F7F9724).
pub const RELEASE: f32 = f32::from_bits(0x3F7F_9724); // 0.9984
/// `+0x66`: the hold reload, in groups.
pub const HOLD: u16 = 300;
/// `+0x60`: the floor before the first update.
const FLOOR_INIT: f32 = 1000.0;

// The constants of the update and the blocks.
/// `dbl_10151628`: the gain meter's scale.
const GAIN_METER_SCALE: f64 = 18.0;
/// `flt_10151660`: the meter bytes' full scale.
const METER_FULL: f32 = 127.0;
/// `flt_10151664`: f32 1/127.
const C127: f32 = f32::from_bits(0x3C01_0204);
/// `dbl_10151630`: the amount's share of the offset `1 - af*0.99`.
const OFF_SCALE: f64 = f64::from_bits(0x3FEF_AE14_7AE1_47AE); // 0.99
/// `dbl_10151638`: 1.0.
const ONE: f64 = 1.0;
/// `dbl_10151640`: the threshold's share of the gain factor, 1/32.
const TH_SCALE: f64 = 0.03125;
/// `dbl_10151648`: the gain factor's scale.
const X_SCALE: f64 = 0.3662109375;
/// `flt_10151668`, `dbl_10151650`, `dbl_10151658`: the floor `th * f32(1/127) * 80000 / 32768
/// + 0.01220703125`.
const FLOOR_SCALE: f32 = 80000.0;
const FLOOR_SCALE2: f64 = 1.0 / 32768.0;
const FLOOR_OFFSET: f64 = 0.01220703125;
/// `flt_10151670`: the peak's decay per update.
const METER_DECAY: f32 = f32::from_bits(0x3F6B_851F); // 0.92
/// `dbl_10151678` (mono) and `dbl_10151690` (stereo): the ramp's step factor, 1/4.
const RAMP: f64 = 0.25;
/// `flt_10151688` (`flt_101516A0`): the detector's sign test, 0.
const ZERO: f32 = 0.0;
/// `flt_1015168C` (`flt_101516A4`): the release's denormal guard, 1e-6.
const GUARD: f32 = f32::from_bits(0x3586_37BD);

fn st(x: f64) -> f32 {
    x as f32
}

/// `|x|` as 0x1000ECBA takes it: the sign flipped (`fchs`) when the bits are above 0x80000000
/// unsigned.
fn fabs(x: f32) -> f32 {
    if x.to_bits() > 0x8000_0000 { -x } else { x }
}

/// The parameters the update 0x1000EB44 derives from the amount and the threshold:
/// `af = amount * f32(1/127)`, `off = 1 - af*0.99`, `X = (af * (th/32 + 1)) * 0.3662109375`,
/// `floor = ((th * f32(1/127)) * 80000) * (1/32768) + 0.01220703125`, each stored as f32.
pub fn params(amount: u8, threshold: u8) -> (f32, f32, f32) {
    let af = amount as f64 * C127 as f64;
    let th = threshold as f64;
    let off = ONE - af * OFF_SCALE;
    let x = (af * (th * TH_SCALE + ONE)) * X_SCALE;
    let floor = ((th * C127 as f64) * FLOOR_SCALE as f64) * FLOOR_SCALE2 + FLOOR_OFFSET;
    (st(x), st(off), st(floor))
}

/// The detector and envelope of one group: `env` attacks (`env + (s - env)*atk`, the hold
/// reloaded) when `s > env`, else it holds while the hold counter runs down and then releases
/// (`env*rel + 1e-6`); it never goes below the floor (0x1000ED14..0x1000ED60).
pub fn envelope(env: f64, s: f64, hold: &mut u16, floor: f32) -> f64 {
    let e = if s > env {
        *hold = HOLD;
        env + (s - env) * ATTACK as f64
    } else if *hold == 0 {
        env * RELEASE as f64 + GUARD as f64
    } else {
        *hold -= 1;
        env
    };
    if e < floor as f64 || e.is_nan() {
        floor as f64
    } else {
        e
    }
}

/// `|s|` as the detector takes it: negated when below 0 (`fcom 0.0`, C0; a NaN too).
fn detect(s: f64) -> f64 {
    if s < ZERO as f64 || s.is_nan() { -s } else { s }
}

/// The compressor object.
#[derive(Debug, Clone)]
pub struct Compressor {
    /// `+0x38`: the live parameters, which the audio reads.
    pub live: [u8; 5],
    /// `+0x3D`: the pattern-mode copy.
    pub pattern: [u8; 5],
    /// `+0x30`: the call counter (word); the update runs when `counter & 15 == 0`.
    pub counter: u16,
    /// `+0x34`: the counter at the last mixer update (slot 16).
    pub last_counter: u16,
    /// `+0x48`: the gain factor X.
    pub x: f32,
    /// `+0x4C`: the gain offset.
    pub off: f32,
    /// `+0x50`: the envelope.
    pub env: f32,
    /// `+0x5C`: the gain G; 0 at the ctor, so the compressor fades in.
    pub gain: f32,
    /// `+0x60`: the envelope's floor.
    pub floor: f32,
    /// `+0x64`: the hold counter (word), reloaded with `+0x66` ([`HOLD`]).
    pub hold: u16,
    /// `+0x68`: the peak meter of the (left) input (display only).
    pub peak: f32,
}

impl Default for Compressor {
    fn default() -> Self {
        Self::new()
    }
}

impl Compressor {
    /// The ctor's state (0x1000E990): the parameter blocks at their defaults, counter, hold,
    /// peak and gain 0, X, offset and envelope 1.0, floor 1000, attack 0.01, release
    /// 0.9984, hold reload 300, change flags `+0x42..+0x46` 0.
    pub fn new() -> Self {
        Compressor {
            live: DEFAULTS,
            pattern: DEFAULTS,
            counter: 0,
            last_counter: 0,
            x: 1.0,
            off: 1.0,
            env: 1.0,
            gain: 0.0,
            floor: FLOOR_INIT,
            hold: 0,
            peak: 0.0,
        }
    }

    /// The update 0x1000EB44: the gain meter, the parameters, then the peak meter.
    fn param_update(&mut self) {
        // 0x1000EB48: trunc(min(G*18, 127)) into byte 3
        let g = self.gain as f64 * GAIN_METER_SCALE;
        let g = if g > METER_FULL as f64 {
            METER_FULL as f64
        } else {
            g
        };
        self.live[GAIN_METER] = g as i64 as u8;
        (self.x, self.off, self.floor) = params(self.live[AMOUNT], self.live[THRESHOLD]);
        // 0x1000EC23: the peak meter (an integer compare of the float bits), then the decay
        if self.peak.to_bits() as i32 > 1.0f32.to_bits() as i32 {
            self.live[PEAK_METER] = 0x7F;
        } else {
            self.live[PEAK_METER] = (METER_FULL as f64 * self.peak as f64) as i64 as u8;
        }
        self.peak = st(self.peak as f64 * METER_DECAY as f64);
    }

    /// The head of both blocks: the update every 16 calls, the counter, the peak of `bus` at
    /// the meter index. Returns false when the compressor is disabled.
    fn begin(&mut self, bus: &Block, ctx: &BlockContext) -> bool {
        if self.live[ENABLED] == 0 {
            return false;
        }
        if self.counter & 15 == 0 {
            self.param_update();
        }
        self.counter = self.counter.wrapping_add(1);
        let v = fabs(bus[ctx.meter_index]);
        if v > self.peak {
            self.peak = v;
        }
        true
    }

    /// The 16 groups of a block over `buses` (one, or L and R linked): the detector sums
    /// samples 1 and 3 of each bus in order (`((L1 + L3) + R1) + R3`), then each sample k of
    /// the group is multiplied by the gain after k+1 ramp steps.
    fn run(&mut self, buses: &mut [&mut Block]) {
        let x = self.x as f64;
        let off = self.off as f64;
        let mut env = self.env as f64;
        let mut g = self.gain as f64;
        for i in (0..crate::frame::BLOCK).step_by(4) {
            let mut s = buses[0][i + 1] as f64 + buses[0][i + 3] as f64;
            for b in &buses[1..] {
                s = s + b[i + 1] as f64 + b[i + 3] as f64;
            }
            env = envelope(env, detect(s), &mut self.hold, self.floor);
            let d = ((x / env + off) - g) * RAMP;
            for k in 0..4 {
                g += d;
                for b in buses.iter_mut() {
                    b[i + k] = st(b[i + k] as f64 * g);
                }
            }
        }
        self.env = st(env);
        self.gain = st(g);
    }
}

impl SongDevice for Compressor {
    fn live(&self) -> &[u8] {
        &self.live
    }

    /// Slot 9, 0x1000F4B0: entering song mode saves the live block into the pattern copy,
    /// leaving it restores the live block from the copy (all five bytes).
    fn set_song_mode(&mut self, on: bool) {
        if on {
            self.pattern = self.live;
        } else {
            self.live = self.pattern;
        }
    }

    /// Msg 104 (0x1000F1DF): the block goes into the pattern copy (`sub_10019BE0`, five
    /// bytes), and into the live block only in pattern mode (`sub_1001FCB0` zero).
    fn data(&mut self, payload: &[u8], song_mode: bool) {
        self.pattern.copy_from_slice(&payload[..5]);
        if !song_mode {
            self.live = self.pattern;
        }
    }

    /// Slot 11, 0x1000F220: controllers 0..2 set the live block (any controller also sets a UI
    /// change flag at `+0x42 + ctrl`, not modelled; the load accepts only 0..2 on track 8).
    fn set_param(&mut self, ctrl: u8, value: u8) {
        if (ctrl as usize) <= THRESHOLD {
            self.live[ctrl as usize] = value;
        }
    }

    /// Slot 8, 0x1000F420, with tick 0 from the song ctor: controllers 0..2 of the pattern copy
    /// inserted into the compressor's track (`sub_1001F900`; controllers 3 and 4 are skipped).
    fn init_track(&mut self, track: &mut Track) {
        for c in 0..=THRESHOLD {
            track.insert(0, c as u8, self.pattern[c]);
        }
    }
}

impl InsertEffect for Compressor {
    /// Slot 15 copies the release factor `+0x58` (only the ctor writes it, so always
    /// [`RELEASE`]) to its local at the entry `esp - 0x1C` when enabled (0x1000ECE9,
    /// `esp+0x8` after the 0x24-byte prologue), else leaves `prev`.
    fn stack_word(&self, prev: Option<i32>) -> Option<i32> {
        if self.live[ENABLED] != 0 {
            Some(RELEASE.to_bits() as i32)
        } else {
            prev
        }
    }

    /// Slot 15, 0x1000EC70: the mono compressor on a channel bus.
    fn process(&mut self, bus: &mut Block, ctx: &BlockContext) {
        if self.begin(bus, ctx) {
            self.run(&mut [bus]);
        }
    }

    /// Slot 14, 0x1000EDE0: the linked stereo compressor on the master (`l` is the first
    /// argument, whose sample the peak meter reads).
    fn process_stereo(&mut self, l: &mut Block, r: &mut Block, ctx: &BlockContext) {
        if self.begin(l, ctx) {
            self.run(&mut [l, r]);
        }
    }

    /// Slot 16, 0x1000EFB0, run by every mixer update: both meter bytes are cleared when no
    /// call came since the last update.
    fn update(&mut self, _ctx: &BlockContext) {
        if self.counter == self.last_counter {
            self.live[GAIN_METER] = 0;
            self.live[PEAK_METER] = 0;
        }
        self.last_counter = self.counter;
    }
}

#[cfg(test)]
mod tests;
