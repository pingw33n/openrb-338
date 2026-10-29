//! The PCF, an insert effect (`CPCFInsert`, vtable 0x10033AD4, ctor 0x10016490;
//! `docs/initial_investigation.md` section 8.2).
//!
//! A filter whose cutoff follows an envelope triggered by one of 55 static step patterns. Every
//! block (0x100166D0) the pattern step is computed from the song's tick, a new step with a
//! non-zero velocity triggers the envelope, a 2-pole smoother moves the cutoff base towards
//! `4 * frequency`, and 16 groups of 4 samples run the envelope and a low-pass or band-pass
//! 2-pole filter whose coefficients come from the tables at 0x1010C270 (LP) and 0x10100270
//! (BP). The filter state, the envelope and the trigger level stay in x87 registers for the
//! whole block; they are stored as f32 at the block end.
//!
//! The wave byte indexes the pattern data unchecked: a wave above 54 reads the bytes after the
//! 55 patterns ([`PCF_DATA`]). For 73 of the waves 55..127 the resolution is not 8, 16 or 32,
//! which leaves the step quotient at a stale stack slot ([`Pcf::quotient`]). For 17 of them
//! (55, 111 and 113..127) the pattern length is 0 and the step divides by zero. That exception
//! aborts the whole render call ([`InsertEffect::faulted`], [`Fault`]); it is why the PCF
//! with wave 127 silences the song (task-039).
//!
//! The slots: 4 (stop, 0x100106C0) and 5 (reset, 0x1000F4F0) call slot 6, a no-op
//! (0x10012C00); 7 is a no-op (0x1000F4E0); 8 (0x10017200) inserts the pattern copy's
//! controllers 0..6; 9 (0x10017290) the song-mode switch; 10 (0x10016C50) the messages;
//! 11 (0x10016EF0) set-param; 15 (0x100166D0) the block; 16 (0x10016C30) the mixer update's
//! meter check.

use crate::frame::{BLOCK, Block, BlockContext, Fault, InsertEffect, SongDevice};
use crate::seq::TrackMut;
use crate::tables::data::{PCF_DATA, PCF_DECAY};
use crate::tables::{PcfFilter, Smoother};

/// The controllers: enabled, frequency, resonance, amount, wave (the pattern), decay, mode
/// (1 low-pass, anything else band-pass, `cmp dl, 1` at 0x10016921).
pub const ENABLED: usize = 0;
pub const FREQUENCY: usize = 1;
pub const RESONANCE: usize = 2;
pub const AMOUNT: usize = 3;
pub const WAVE: usize = 4;
pub const DECAY: usize = 5;
pub const MODE: usize = 6;
/// Byte 7 of the parameter block is the meter byte the update writes (0x1001667A, 0x100166BF).
pub const METER: usize = 7;

/// `sub_10019C10`: the parameter block's ctor values, for the live and the pattern copy.
pub const DEFAULTS: [u8; 8] = [1, 70, 64, 40, 24, 90, 1, 0];

/// The offsets in [`PCF_DATA`] (at 0x10033310) of the lengths, the resolutions and the steps.
const LENGTHS: usize = 0;
const RESOLUTIONS: usize = 0x10033380 - 0x10033310;
const STEPS: usize = 0x100333F0 - 0x10033310;

// The constants of the block and the update, decoded from their bytes.
/// `dbl_10152288`: 1.0, the attack's complement `1 - r`.
const ONE: f64 = 1.0;
/// `flt_10152294`: 0.0, the envelope's lower clamp and the instant attack's keep factor.
const ZERO: f32 = 0.0;
/// `flt_10152298`, `flt_1015229C`: the cutoff target `freq * 512 * (1/128)`.
const FREQ_SCALE: f32 = 512.0;
const FREQ_SCALE2: f32 = f32::from_bits(0x3C00_0000); // 1/128
/// `flt_101522A0`: the envelope's upper bound, 511.
const TOP: f32 = f32::from_bits(0x43FF_8000);
/// `flt_101522A4`, `flt_101522A8`: the slow attack's keep and take factors.
const SLOW_KEEP: f32 = f32::from_bits(0x3F7E_B852); // 0.995
const SLOW_TAKE: f32 = f32::from_bits(0x3BA3_D700); // 0.0049999952
/// `flt_101522AC`: the instant attack's take factor.
const INSTANT_TAKE: f32 = 1.0;
/// `flt_1015227C`: the meter's decay per update.
const METER_DECAY: f32 = f32::from_bits(0x3F6B_851F); // 0.92
/// `flt_10152280`: the meter byte scale.
const METER_SCALE: f32 = 127.0;

fn st(x: f64) -> f32 {
    x as f32
}

/// `|x|` as 0x1001671D takes it: the sign flipped (`fchs`) when the bits are above 0x80000000
/// unsigned, so -0.0 stays -0.0.
fn fabs(x: f32) -> f32 {
    if x.to_bits() > 0x8000_0000 { -x } else { x }
}

/// A u16 of [`PCF_DATA`] at byte offset `o`.
fn data_u16(o: usize) -> u16 {
    u16::from_le_bytes([PCF_DATA[o], PCF_DATA[o + 1]])
}

/// A wave byte clamped to 127. The exe's track load accepts only 0..127 for it
/// (`docs/song_load.md` section 5) and the DLL does not check it: a wave of 128 or more would
/// read the pattern data from BSS (above 0x10034400, run-time tables), which the engine cannot
/// reproduce, so it clamps instead. Waves 55..127 read past the 55 patterns into the bytes that
/// follow them, which [`PCF_DATA`] holds.
fn clamp_wave(x: u8) -> usize {
    x.min(127) as usize
}

/// The pattern length and resolution of `wave` (0x1001675F..0x10016774: u16 reads of
/// `word_10033310[wave]` and `word_10033380[wave]`).
pub fn pattern(wave: u8) -> (u16, u16) {
    let w = clamp_wave(wave);
    (data_u16(LENGTHS + 2 * w), data_u16(RESOLUTIONS + 2 * w))
}

/// The step velocity of `wave` at `step` (`byte_100333F0[wave*32 + step]`, 0x10016854). The
/// step is `q % len`, so non-negative for a non-negative tick; an index outside [`PCF_DATA`]
/// (a negative tick, which no song position gives) reads 0, no trigger.
pub fn velocity(wave: u8, step: i32) -> u8 {
    let i = STEPS as i64 + clamp_wave(wave) as i64 * 32 + step as i64;
    usize::try_from(i)
        .ok()
        .and_then(|i| PCF_DATA.get(i))
        .copied()
        .unwrap_or(0)
}

/// The step quotient of 0x1001677B..0x100167AE: the tick divided (signed `idiv`) by the ticks
/// per step of the resolution (24 for 32, 48 for 16, 96 for 8 steps per bar), `None` for any
/// other resolution, where the original leaves the stack slot (`var_1C`) as it was.
pub fn quotient(tick: i32, res: u16) -> Option<i32> {
    match res {
        32 => Some(tick / 24),
        16 => Some(tick / 48),
        8 => Some(tick / 96),
        _ => None,
    }
}

/// One step of the cutoff smoother (0x100167B2..0x10016825): with `f = ((freq*512)*(1/128))*g`,
/// `s = (A*y1 + f) - b2*y2`.
pub fn smooth_cutoff(k: &Smoother, freq: u8, y1: f32, y2: f32) -> f64 {
    let f = (freq as f64 * FREQ_SCALE as f64) * FREQ_SCALE2 as f64 * k.g as f64;
    (k.a as f64 * y1 as f64 + f) - k.b2 as f64 * y2 as f64
}

/// The trigger level of a step with velocity `v` (0x10016887..0x100168CC): `(v*amount) >> 5`,
/// limited so that it plus the cutoff base stays at most 511 (`511 - base` when the sum is
/// above it).
pub fn trigger_level(v: u8, amount: u8, base: f32) -> f64 {
    let raw = ((v as i32 * amount as i32) >> 5) as f64;
    if raw + base as f64 > TOP as f64 {
        TOP as f64 - base as f64
    } else {
        raw
    }
}

/// The filter table index of an envelope value (0x10016968..0x100169C4): 0 below 0 (or a NaN),
/// 511 above 511, else truncated.
pub fn env_index(env: f64) -> usize {
    let v = if env < ZERO as f64 || env.is_nan() {
        ZERO as f64
    } else if env > TOP as f64 {
        TOP as f64
    } else {
        env
    };
    v as i64 as usize
}

/// The envelope's per-group state and factors in the block's registers.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Envelope {
    /// The trigger level E, multiplied by the decay every group.
    pub e: f64,
    /// The envelope.
    pub env: f64,
    /// The keep and take factors of `env = env*keep + (E + base)*take`.
    pub keep: f64,
    pub take: f64,
}

impl Envelope {
    /// One group (0x1001694D..0x10016962): `E *= dec`, then
    /// `env = env*keep + (E + base)*take`; returns the new envelope.
    pub fn step(&mut self, dec: f32, base: f32) -> f64 {
        self.e *= dec as f64;
        self.env = self.env * self.keep + (self.e + base as f64) * self.take;
        self.env
    }
}

/// The filter's two last outputs (`y1` the latest).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Filter {
    pub y1: f64,
    pub y2: f64,
}

impl Filter {
    /// One sample: `y = (x*c2 + c0*y1) - c1*y2` (0x100169E3..0x10016A10).
    pub fn step(&mut self, x: f32, c: &[f32; 3]) -> f64 {
        let [c0, c1, c2] = c.map(|v| v as f64);
        let y = (x as f64 * c2 + c0 * self.y1) - c1 * self.y2;
        self.y2 = self.y1;
        self.y1 = y;
        y
    }
}

/// Runs the 16 groups of a block over `bus` with the table row `tab`: per group the envelope
/// step, the index from it, then four filter steps. The low-pass outputs `y`, the band-pass
/// `y - y_prev` (0x10016BA4, `fsubr`). Returns the envelope of the last group.
pub fn run(
    bus: &mut Block,
    tab: &[[f32; 3]; 512],
    bandpass: bool,
    env: &mut Envelope,
    f: &mut Filter,
    dec: f32,
    base: f32,
) {
    for g in 0..BLOCK / 4 {
        let c = &tab[env_index(env.step(dec, base))];
        for x in &mut bus[4 * g..4 * g + 4] {
            let prev = f.y1;
            let y = f.step(*x, c);
            *x = st(if bandpass { y - prev } else { y });
        }
    }
}

/// The PCF object.
#[derive(Debug, Clone)]
pub struct Pcf {
    /// `+0x38`: the live parameters, which the audio reads.
    pub live: [u8; 8],
    /// `+0x40`: the pattern-mode copy.
    pub pattern: [u8; 8],
    /// `+0x30`: the call counter (word); the meter update runs when `counter & 15 == 0`.
    pub counter: u16,
    /// `+0x34`: the counter at the last mixer update (slot 16).
    pub last_counter: u16,
    /// `+0x50`, `+0x54`: the low-pass state (`y1`, `y2`); zeroed by every band-pass block.
    pub lp: [f32; 2],
    /// `+0x58`, `+0x5C`: the band-pass state; zeroed by every low-pass block.
    pub bp: [f32; 2],
    /// `+0x68`: the trigger level E.
    pub trigger: f32,
    /// `+0x6C`: the cutoff base, the smoother's output.
    pub base: f32,
    /// `+0x70`: the envelope.
    pub env: f32,
    /// `+0x74`: the last step, -1 at the ctor (`sub_100178D0`).
    pub step: i32,
    /// `+0x78`: the attack's keep factor `r` (0 instant, 0.995 slow).
    pub attack: f32,
    /// `+0x7C`: the decay per group, `PCF_DECAY[decay >> 3]`, set at every new step.
    pub decay: f32,
    /// `+0x80`, `+0x84`: the cutoff smoother's last two outputs.
    pub smooth: [f32; 2],
    /// `+0x88`: the peak meter of the input (display only).
    pub peak: f32,
    /// The block's stack slot `var_1C` (`esp+0x48`, entry `esp - 0x1C`), which holds the step
    /// quotient when the resolution is 8, 16 or 32 and is left as it was otherwise. The 73
    /// waves of 55..127 whose resolution is not 8, 16 or 32 reach it; 17 of them fault before
    /// they use it. What the original finds there is what the code before the PCF left at that
    /// stack address ([`BlockContext::stack_word`], [`crate::frame::Frame::stack_word`]; the
    /// PCF's own quotient does not survive to the next block). On the 808 and 909 channels it is
    /// the return address of the audio slot's last call ([C] task-069: 0x1002B8B4 on the 909 in
    /// every block). The 303 audio does not write there, so on the 303 channels it is the last
    /// of ([C] task-072, every PCF block of six rbhost sessions): the render call's lock (0),
    /// the mixer update's compressor id, the clock's tempo, an enabled compressor on channel 0
    /// (its release factor 0x3F7F9724), or from the block before the 909's return address, a
    /// compressor on channel 3 or the enabled delay's last output pointer. The addresses are
    /// the values at the preferred base 0x10000000, which the corpus and rbhost assume, as for
    /// [`PCF_DATA`]. Where the original leaves a heap address (the delay after a partial
    /// block) the stack word is `None` and the engine keeps the last quotient this PCF computed
    /// there (0 before the first), a stand-in.
    pub quotient: i32,
    /// Set by a block that divided by zero, read and cleared by [`InsertEffect::faulted`].
    fault: Option<Fault>,
}

impl Default for Pcf {
    fn default() -> Self {
        Self::new()
    }
}

impl Pcf {
    /// The ctor's state (0x10016490: the parameter blocks at their defaults, `+0x50..+0x88` 0,
    /// then the last step -1 and the change flags `+0x48..+0x4F` 0).
    pub fn new() -> Self {
        Pcf {
            live: DEFAULTS,
            pattern: DEFAULTS,
            counter: 0,
            last_counter: 0,
            lp: [0.0; 2],
            bp: [0.0; 2],
            trigger: 0.0,
            base: 0.0,
            env: 0.0,
            step: -1,
            attack: 0.0,
            decay: 0.0,
            smooth: [0.0; 2],
            peak: 0.0,
            quotient: 0,
            fault: None,
        }
    }

    /// The meter update 0x10016664: the peak as an integer above 0x3F800000 gives byte 0x7F,
    /// else `trunc(peak*127)`; then `peak = f32(peak*0.92)` (the peak is not clamped).
    fn meter_update(&mut self) {
        if self.peak.to_bits() as i32 > 1.0f32.to_bits() as i32 {
            self.live[METER] = 0x7F;
        } else {
            self.live[METER] = (self.peak as f64 * METER_SCALE as f64) as i64 as u8;
        }
        self.peak = st(self.peak as f64 * METER_DECAY as f64);
    }

    /// The block 0x100166D0 up to the filter: the step, the smoother, the trigger. Returns the
    /// envelope's starting state, or `None` when the length is 0 (the `idiv` at 0x10016811
    /// faults; what was stored before it, the counter, the peak and `+0x84`, stands).
    fn control(&mut self, ctx: &BlockContext) -> Option<Envelope> {
        let p = self.live;
        let (len, res) = pattern(p[WAVE]);
        if let Some(q) = quotient(ctx.tick, res) {
            self.quotient = q;
        } else if let Some(w) = ctx.stack_word {
            // the slot as the channel's audio slot left it
            self.quotient = w;
        }
        // 0x100167B2..0x10016825: the smoother; `+0x84` takes the old `+0x80` before the
        // division, `+0x80` and the base the new value after it
        let s = smooth_cutoff(
            &ctx.tables.pcf_smoother,
            p[FREQUENCY],
            self.smooth[0],
            self.smooth[1],
        );
        self.smooth[1] = self.smooth[0];
        if len == 0 {
            return None;
        }
        let step = self.quotient % len as i32;
        self.smooth[0] = st(s);
        self.base = st(s);
        let slow = |r: f32| (r as f64, ONE - r as f64);
        let mut e = self.trigger as f64;
        let (mut keep, mut take) = slow(self.attack);
        if step != self.step {
            // 0x10016845: a new step sets the decay, and a non-zero velocity triggers
            self.step = step;
            // `decay >> 3` indexes 16 entries; the load accepts only 0..127 for the byte and a
            // larger one would read past `dword_100332D0`, so it is clamped
            self.decay = PCF_DECAY[(p[DECAY] >> 3).min(15) as usize];
            let v = velocity(p[WAVE], step);
            if v != 0 {
                e = trigger_level(v, p[AMOUNT], self.base);
                if v & 1 != 0 {
                    self.attack = SLOW_KEEP;
                    (keep, take) = (SLOW_KEEP as f64, SLOW_TAKE as f64);
                } else {
                    self.attack = 0.0;
                    (keep, take) = (ZERO as f64, INSTANT_TAKE as f64);
                }
            }
        }
        Some(Envelope {
            e,
            env: self.env as f64,
            keep,
            take,
        })
    }
}

impl SongDevice for Pcf {
    fn live(&self) -> &[u8] {
        &self.live
    }

    /// Slot 9, 0x10017290: entering song mode saves the live block into the pattern copy,
    /// leaving it restores the live block from the copy (all eight bytes).
    fn set_song_mode(&mut self, on: bool) {
        if on {
            self.pattern = self.live;
        } else {
            self.live = self.pattern;
        }
    }

    /// Msg 116 (0x10016EA5): the block goes into the pattern copy (`sub_10019C80`, eight
    /// bytes), and into the live block only in pattern mode (`sub_1001FCB0` zero).
    fn data(&mut self, payload: &[u8], song_mode: bool) {
        self.pattern.copy_from_slice(&payload[..8]);
        if !song_mode {
            self.live = self.pattern;
        }
    }

    /// Slot 11, 0x10016EF0: controllers 0..6 set the live block (any controller also sets a UI
    /// change flag at `+0x48 + ctrl`, not modelled; the load accepts only 0..6 on track 7).
    fn set_param(&mut self, ctrl: u8, value: u8) {
        if (ctrl as usize) <= MODE {
            self.live[ctrl as usize] = value;
        }
    }

    /// Slot 8, 0x10017200, with tick 0 from the song ctor: controllers 0..6 of the pattern copy
    /// inserted into the PCF's track (`sub_1001F900`; controller 7 is skipped).
    fn init_track(&mut self, track: &mut TrackMut) {
        for c in 0..=MODE {
            track.insert(0, c as u8, self.pattern[c]);
        }
    }
}

impl InsertEffect for Pcf {
    /// The quotient when the block stored one (enabled, resolution 8, 16 or 32: 0x10016792,
    /// 0x100167A0, 0x100167AE, `var_1C` at the entry `esp - 0x1C`), else `prev`.
    fn stack_word(&self, prev: Option<i32>) -> Option<i32> {
        let (_, res) = pattern(self.live[WAVE]);
        if self.live[ENABLED] != 0 && quotient(0, res).is_some() {
            Some(self.quotient)
        } else {
            prev
        }
    }

    /// Slot 15, 0x100166D0: nothing when disabled (the bus passes unchanged and the counter
    /// stands).
    fn process(&mut self, bus: &mut Block, ctx: &BlockContext) {
        if self.live[ENABLED] == 0 {
            return;
        }
        if self.counter & 15 == 0 {
            self.meter_update();
        }
        self.counter = self.counter.wrapping_add(1);
        // 0x10016711: the input's peak at the meter index
        let v = fabs(bus[ctx.meter_index]);
        if v > self.peak {
            self.peak = v;
        }
        let Some(mut env) = self.control(ctx) else {
            self.fault = Some(Fault::PcfZeroLength {
                wave: self.live[WAVE],
            });
            return;
        };
        // `(res*8) >> 7` picks one of 8 rows; the load accepts only 0..127 for the byte and a
        // larger one would read past the table, so it is clamped
        let row = ((self.live[RESONANCE] as usize * 8) >> 7).min(7);
        let lowpass = self.live[MODE] == 1;
        let (tab, state): (&PcfFilter, [f32; 2]) = if lowpass {
            (&ctx.tables.pcf_lowpass, self.lp)
        } else {
            (&ctx.tables.pcf_bandpass, self.bp)
        };
        let mut f = Filter {
            y1: state[0] as f64,
            y2: state[1] as f64,
        };
        run(
            bus, &tab[row], !lowpass, &mut env, &mut f, self.decay, self.base,
        );
        // 0x10016A80 / 0x10016C0B: the state, E and the envelope stored; the other filter's
        // state zeroed
        let s = [st(f.y1), st(f.y2)];
        if lowpass {
            self.lp = s;
            self.bp = [0.0; 2];
        } else {
            self.bp = s;
            self.lp = [0.0; 2];
        }
        self.trigger = st(env.e);
        self.env = st(env.env);
    }

    /// Slot 16, 0x10016C30, run by every mixer update: the meter byte is cleared when no call
    /// came since the last update.
    fn update(&mut self, _ctx: &BlockContext) {
        if self.counter == self.last_counter {
            self.live[METER] = 0;
        }
        self.last_counter = self.counter;
    }

    fn faulted(&mut self) -> Option<Fault> {
        self.fault.take()
    }
}

#[cfg(test)]
mod tests;
