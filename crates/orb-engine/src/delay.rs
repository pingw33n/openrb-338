//! The delay, the song's send effect (`CDelayEffect`, vtable 0x10032CB4, ctor 0x1000F5A0;
//! `docs/initial_investigation.md` section 8.4).
//!
//! Every block the mixer hands it the send bus (`song+0x160C`, the channels' `bus * sendGain`
//! on top of the 1e-6 guard); the delay adds the line's output into the master and writes
//! `fb * out + send` back into the line, then refills the send bus with the guard. The line is
//! the process-global `flt_10054794` ([`crate::process::DelayLine`]), shared by every delay of
//! the process; the write position, the length and the gains are the object's own.
//!
//! The slots: 5 (reset, 0x1000F4F0) calls slot 6 (0x100100E0, the line refilled); 7 is a no-op
//! (0x1000F4E0); 8 (0x10010020) inserts the pattern copy's controllers 0..4 at the given tick;
//! 9 (0x100100B0) the song-mode switch; 10 (0x1000FB40) the messages; 11 (0x1000FDC0) set-param;
//! 14 (0x1000F940) the block.

use crate::frame::{BLOCK, Block, BlockContext, GUARD, SendEffect, SongDevice};
use crate::process::{DELAY_MASK, ProcessState};
use crate::seq::TrackMut;

/// The controllers: enabled, steps (1..32), mode (0 triplets, else sixteenths), feedback, pan.
pub const ENABLED: usize = 0;
pub const STEPS: usize = 1;
pub const MODE: usize = 2;
pub const FEEDBACK: usize = 3;
pub const PAN: usize = 4;
/// Byte 5 of the parameter block is the meter byte the update writes (0x1000F8F4, 0x1000F938).
pub const METER: usize = 5;

/// `sub_10019AB0`: the parameter block's ctor values, for the live and the pattern copy.
pub const DEFAULTS: [u8; 6] = [1, 3, 1, 80, 64, 0];

/// `flt_101517C0`: the 1e-6 added to the pan targets.
const OFFSET: f32 = GUARD;
/// `dbl_101517B8`: 1/127, the feedback scale.
const FB_SCALE: f64 = f64::from_bits(0x3F80_2040_8102_0408);
/// `flt_101517C8`: the meter's decay per update.
const METER_DECAY: f32 = f32::from_bits(0x3F6B_851F); // 0.92
/// `flt_101517CC`: the meter byte scale.
const METER_SCALE: f32 = 127.0;
/// `dbl_101517D0`, `dbl_101517D8`: the gain smoothing's keep and take factors.
const KEEP: f64 = f64::from_bits(0x3FEC_CCCC_CCCC_CCCD); // 0.9
const TAKE: f64 = f64::from_bits(0x3FB9_9999_9999_999A); // 0.1
/// The samples per step at 1 BPM*1000 per beat fraction: 60 s * 44100 * 1000 / 100
/// (`26460000 * steps`, built by shifts and adds at 0x1000F804..0x1000F834).
const STEP_SAMPLES: u32 = 26_460_000;
/// The longest delay (`cmp eax, 1FFFFh` at 0x1000F890).
const MAX_LENGTH: u32 = 0x1FFFF;

/// The delay length in samples, the update's 0x1000F7E5..0x1000F89E: `q = 3*T/100` for mode 0
/// (triplets), `4*T/100` otherwise (32-bit `lea`/`shl`, signed `idiv`), then
/// `min((26460000*steps mod 2^32) / q, 0x1FFFF)` (unsigned `div`). A q of 0 (a tempo below
/// 34 milli-BPM, which no song has) would fault in the original; here it gives the maximum.
pub fn length(tempo: u32, steps: u8, mode: u8) -> u32 {
    let t = tempo as i32;
    let q = if mode == 0 {
        t.wrapping_mul(3) / 100
    } else {
        t.wrapping_mul(4) / 100
    };
    let n = STEP_SAMPLES.wrapping_mul(steps as u32);
    n.checked_div(q as u32)
        .unwrap_or(MAX_LENGTH)
        .min(MAX_LENGTH)
}

/// One step of the per-block gain smoothing (0x1000F9BB..0x1000FA02): `g*0.9 + t*0.1`, kept
/// unrounded for the block's loop; the f32 rounding is what is stored.
pub fn smooth(gain: f32, target: f32) -> f64 {
    gain as f64 * KEEP + target as f64 * TAKE
}

/// A pan byte clamped to 127. The exe's track load accepts only 0..127 (`docs/song_load.md`
/// section 5), and the DLL does not check: a larger byte would read past the `PAN[127]` row
/// (`flt_10054150`) into adjacent memory, which the engine cannot reproduce, so it clamps.
fn clamp_pan(x: u8) -> u8 {
    x.min(127)
}

fn st(x: f64) -> f32 {
    x as f32
}

/// `|x|` as 0x1000F998 takes it: the sign flipped (`fchs`) when the bits are above 0x80000000
/// unsigned, so -0.0 stays -0.0.
fn fabs(x: f32) -> f32 {
    if x.to_bits() > 0x8000_0000 { -x } else { x }
}

/// The delay object. The fields at `+0x48`, `+0x4C` and `+0x50` (a tempo average over 25
/// updates, 0x1000F75B..0x1000F7BA) are not modelled: the update overwrites `+0x4C` with the
/// current tempo (0x1000F7E2) before it reads it, so the average reaches nothing.
#[derive(Debug, Clone)]
pub struct Delay {
    process: ProcessState,
    /// `+0x34`: the live parameters, which the audio reads.
    pub live: [u8; 6],
    /// `+0x3A`: the pattern-mode copy.
    pub pattern: [u8; 6],
    /// `+0x30`: the block counter (word); the update runs when `counter & 15 == 0`.
    pub counter: u16,
    /// `+0x54`, `+0x58`: the pan gains' targets.
    pub target: [f32; 2],
    /// `+0x5C`, `+0x60`: the smoothed pan gains.
    pub gain: [f32; 2],
    /// `+0x64`: the feedback.
    pub feedback: f32,
    /// `+0x68`: the delay in samples.
    pub length: u32,
    /// `+0x6C`: the write position, 0 at the ctor and never reset.
    pub write: u32,
    /// `+0x70`: the peak meter of the send bus (display only).
    pub peak: f32,
}

impl Delay {
    /// The ctor's state (0x1000F5A0: the parameter blocks at their defaults, every gain, the
    /// length, the write position and the meter 0).
    pub fn new(process: ProcessState) -> Self {
        Delay {
            process,
            live: DEFAULTS,
            pattern: DEFAULTS,
            counter: 0,
            target: [0.0; 2],
            gain: [0.0; 2],
            feedback: 0.0,
            length: 0,
            write: 0,
            peak: 0.0,
        }
    }

    /// The update 0x1000F754, every 16 enabled blocks: the length, the pan targets
    /// `f32(1e-6 + PAN[127][pan])`, the feedback `f32(fb * (1/127))`, and the meter byte.
    fn param_update(&mut self, ctx: &BlockContext) {
        let p = self.live;
        self.length = length(ctx.tempo, p[STEPS], p[MODE]);
        // 0x1000F8AB: fld 1e-6; fadd PAN[127][pan][k] (`flt_10054150`, the level-127 row)
        let pg = ctx.tables.pan[127][p[PAN] as usize];
        self.target = [
            st(OFFSET as f64 + pg[0] as f64),
            st(OFFSET as f64 + pg[1] as f64),
        ];
        // 0x1000F8D7: fild (qword) fb; fmul 1/127
        self.feedback = st(p[FEEDBACK] as f64 * FB_SCALE);
        // 0x1000F8EC: the meter byte (an integer compare of the float bits), then the decay
        if self.peak.to_bits() as i32 > 1.0f32.to_bits() as i32 {
            self.live[METER] = 0x7F;
        } else {
            self.live[METER] = (self.peak as f64 * METER_SCALE as f64) as i64 as u8;
        }
        self.peak = st(self.peak as f64 * METER_DECAY as f64);
    }
}

impl SongDevice for Delay {
    fn live(&self) -> &[u8] {
        &self.live
    }

    /// Slot 5, 0x1000F4F0: slot 6.
    fn reset(&mut self) {
        self.silence();
    }

    /// Slot 6, 0x100100E0: the global line refilled with 1e-6, the gate set.
    fn silence(&mut self) {
        self.process.refill_delay_line();
    }

    /// Slot 9, 0x100100B0: entering song mode saves the live block into the pattern copy,
    /// leaving it restores the live block from the copy (all six bytes).
    fn set_song_mode(&mut self, on: bool) {
        if on {
            self.pattern = self.live;
        } else {
            self.live = self.pattern;
        }
    }

    /// Msg 86 (0x1000FD7D): the block goes into the pattern copy (`sub_10019B00`, six bytes),
    /// and into the live block only in pattern mode (`sub_1001FCB0` zero).
    fn data(&mut self, payload: &[u8], song_mode: bool) {
        self.pattern.copy_from_slice(&payload[..6]);
        self.pattern[PAN] = clamp_pan(self.pattern[PAN]);
        if !song_mode {
            self.live = self.pattern;
        }
    }

    /// Slot 11, 0x1000FDC0: controllers 0..4 set the live block (any controller also sets a
    /// UI change flag at `+0x40 + ctrl`, not modelled; the load accepts only 0..4 on track 5).
    fn set_param(&mut self, ctrl: u8, value: u8) {
        match ctrl as usize {
            PAN => self.live[PAN] = clamp_pan(value),
            c @ ENABLED..=FEEDBACK => self.live[c] = value,
            _ => {}
        }
    }

    /// Slot 8, 0x10010020, with tick 0 from the song ctor: controllers 0..4 of the pattern copy
    /// inserted into the delay's track (`sub_1001F900`; controller 5 is skipped). Slot 7 is a
    /// no-op, so the track keeps its quantisation.
    fn init_track(&mut self, track: &mut TrackMut) {
        for c in 0..=PAN {
            track.insert(0, c as u8, self.pattern[c]);
        }
    }
}

impl SendEffect for Delay {
    /// Enabled, the sample loop keeps its right-output pointer in a local at the channel slots'
    /// `esp - 0x1C` (0x1000FA9A, `esp+0x10`; the delay is called from `esp0 - 0xC` of the mixer
    /// block, 0x10013865), last the block's last right sample, `out_end`; disabled (0x1000FAD1)
    /// it leaves `prev`.
    fn stack_word(&self, out_end: Option<i32>, prev: Option<i32>) -> Option<i32> {
        if self.live[ENABLED] != 0 {
            out_end
        } else {
            prev
        }
    }

    /// Slot 14, 0x1000F940.
    fn process(&mut self, send: &mut Block, l: &mut Block, r: &mut Block, ctx: &BlockContext) {
        if self.live[ENABLED] == 0 {
            // 0x1000FAD1: the line refilled once (while the gate is clear), the send bus
            // refilled, the meter byte and the peak reset; nothing reaches the master
            let line = self.process.delay();
            if !line.gate() {
                line.refill();
            }
            send.fill(GUARD);
            self.live[METER] = 0;
            self.peak = GUARD;
            return;
        }
        // 0x1000F95F (the update below touches no cell of the line)
        self.process.delay().set_gate(false);
        if self.counter & 15 == 0 {
            self.param_update(ctx);
        }
        self.counter = self.counter.wrapping_add(1);
        // 0x1000F98B: the send bus's peak at the meter index
        let v = fabs(send[ctx.meter_index]);
        if v > self.peak {
            self.peak = v;
        }
        // 0x1000F9BB: the smoothed gains, stored rounded, used unrounded
        let gl = smooth(self.gain[0], self.target[0]);
        let gr = smooth(self.gain[1], self.target[1]);
        self.gain = [st(gl), st(gr)];
        let fb = self.feedback as f64;
        let buf = self.process.delay();
        let mut rd = self.write.wrapping_sub(self.length) & DELAY_MASK;
        let mut w = self.write;
        // 0x1000FA34: per sample, the read before the write (they meet when the length is 0)
        for i in 0..BLOCK {
            let x = buf.get(rd) as f64;
            l[i] = st(x * gl + l[i] as f64);
            r[i] = st(r[i] as f64 + x * gr);
            let s = send[i] as f64;
            send[i] = GUARD;
            buf.set(w, st(x * fb + s));
            rd = (rd + 1) & DELAY_MASK;
            w = (w + 1) & DELAY_MASK;
        }
        self.write = w;
    }
}

#[cfg(test)]
mod tests;
