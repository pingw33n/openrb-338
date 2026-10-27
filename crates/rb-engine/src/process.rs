//! The DLL's process globals that reach an export: state that is not part of a song and that
//! no song, stop, locate or msg 33 resets (`docs/export_state.md` section 7.3).
//!
//! These are the four drum counters and the delay line:
//!
//! - The drum counters live in BSS (0 when the DLL loads) and change only on drum hits, so an
//!   export depends on every snare and bass-drum hit since the process started.
//! - The delay line `flt_10054794` (131072 f32) and its gate `byte_10054790` are BSS too (0 when
//!   the DLL loads). Every delay object reads and writes the one line; it is refilled with 1e-6
//!   by the delay's slot 6 (0x100100E0: a song switch, msg 53, and msg 33 through slot 5
//!   0x1000F4F0) and once when a disabled delay runs (0x1000FAD1), so an export depends on what
//!   the delays wrote since the last refill.
//!
//! [`ProcessState`] is the engine's handle on them: the song's devices share one handle
//! ([`crate::frame::Devices::process`]), and a caller that reproduces a capture made in a
//! process that had played before sets them before the export, as
//! `tools/rbhost --drum-counters A,B,C,D` presets the counters
//! ([`ProcessState::set_drum_counters`], [`ProcessState::set_delay_line`]).
//!
//! The state is one handle per [`crate::frame::Engine`] (made by its `Devices`), not a true
//! process global: a second engine in the same OS process starts at zero, unlike the original,
//! where every song of the process shares the DLL's globals. To carry them from one engine to
//! the next, build the next engine's devices with the same handle
//! ([`crate::frame::Devices::implemented_with`], as the real-time player does), or copy
//! [`ProcessState::drum_counters`] and [`ProcessState::delay_line`] into the next engine's
//! handle.
//!
//! **Lock-free.** Every cell is an atomic (relaxed loads and stores, which compile to plain
//! moves), so a render in a real-time audio callback takes no lock (task-043). Only one thread
//! renders at a time; the atomics make the handle `Sync` without a lock, not a concurrent
//! design: a caller that reads the line while another thread renders gets a mix of old and
//! new cells.

use std::sync::Arc;
use std::sync::atomic::{AtomicBool, AtomicU16, AtomicU32, Ordering};

use crate::frame::GUARD;

/// The delay line's length, a power of two (`and 1FFFFh` at 0x1000FA05, 0x1000FA6D).
pub const DELAY_LEN: usize = 1 << 17;
/// The index mask.
pub const DELAY_MASK: u32 = DELAY_LEN as u32 - 1;

/// The global delay line and its gate.
#[derive(Debug, Clone, PartialEq)]
pub struct DelayLine {
    /// `flt_10054794`.
    pub buf: Box<[f32]>,
    /// `byte_10054790`: set by every refill, cleared by every enabled delay block (0x1000F95F);
    /// a disabled delay refills the line only while it is clear (0x1000FAD8).
    pub gate: bool,
}

impl Default for DelayLine {
    /// What the DLL has right after it is loaded: BSS, all zero.
    fn default() -> Self {
        DelayLine {
            buf: vec![0.0; DELAY_LEN].into_boxed_slice(),
            gate: false,
        }
    }
}

impl DelayLine {
    /// The refill of 0x100100E0 and 0x1000FADA: every cell 1e-6 (`358637BDh`), the gate set.
    pub fn refill(&mut self) {
        self.buf.fill(GUARD);
        self.gate = true;
    }
}

/// The four process-global drum counters, in the order of `rbhost --drum-counters A,B,C,D`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct DrumCounters {
    /// `word_1014CBD4`: the TR-808 snare's noise sequence position (`sub_1002A1E4`,
    /// pre-incremented, `& 31`).
    pub snare_808: u16,
    /// `word_1014E548`: the TR-909 snare's (`sub_1002DC54`).
    pub snare_909: u16,
    /// `word_1014CBD0`: the TR-808 bass drum's voice toggle (0x1002A0DA..0x1002A12A).
    pub bd_808: u16,
    /// `word_1014E544`: the TR-909 bass drum's (0x1002DB82 / 0x1002DC3B).
    pub bd_909: u16,
}

/// The delay line as the engine holds it: one atomic per cell (the f32 bits) and the gate.
#[derive(Debug)]
pub(crate) struct SharedDelayLine {
    buf: Box<[AtomicU32]>,
    gate: AtomicBool,
}

impl Default for SharedDelayLine {
    /// All zero, as [`DelayLine::default`].
    fn default() -> Self {
        SharedDelayLine {
            buf: (0..DELAY_LEN).map(|_| AtomicU32::new(0)).collect(),
            gate: AtomicBool::new(false),
        }
    }
}

impl SharedDelayLine {
    pub(crate) fn get(&self, i: u32) -> f32 {
        f32::from_bits(self.buf[i as usize].load(Ordering::Relaxed))
    }

    pub(crate) fn set(&self, i: u32, x: f32) {
        self.buf[i as usize].store(x.to_bits(), Ordering::Relaxed);
    }

    pub(crate) fn gate(&self) -> bool {
        self.gate.load(Ordering::Relaxed)
    }

    pub(crate) fn set_gate(&self, v: bool) {
        self.gate.store(v, Ordering::Relaxed);
    }

    /// [`DelayLine::refill`].
    pub(crate) fn refill(&self) {
        let g = GUARD.to_bits();
        for c in self.buf.iter() {
            c.store(g, Ordering::Relaxed);
        }
        self.set_gate(true);
    }
}

/// A shared handle on the process globals. Clones share the same state; a fresh handle holds
/// what the DLL has right after it is loaded (all zero).
#[derive(Debug, Clone, Default)]
pub struct ProcessState {
    drums: Arc<[AtomicU16; 4]>,
    delay: Arc<SharedDelayLine>,
}

const SNARE_808: usize = 0;
const SNARE_909: usize = 1;
const BD_808: usize = 2;
const BD_909: usize = 3;

impl ProcessState {
    pub fn new() -> Self {
        Self::default()
    }

    /// The counters as they are now.
    pub fn drum_counters(&self) -> DrumCounters {
        let g = |i: usize| self.drums[i].load(Ordering::Relaxed);
        DrumCounters {
            snare_808: g(SNARE_808),
            snare_909: g(SNARE_909),
            bd_808: g(BD_808),
            bd_909: g(BD_909),
        }
    }

    /// Presets the counters (rbhost `--drum-counters`).
    pub fn set_drum_counters(&self, c: DrumCounters) {
        let s = |i: usize, v: u16| self.drums[i].store(v, Ordering::Relaxed);
        s(SNARE_808, c.snare_808);
        s(SNARE_909, c.snare_909);
        s(BD_808, c.bd_808);
        s(BD_909, c.bd_909);
    }

    /// A copy of the delay line and its gate as they are now.
    pub fn delay_line(&self) -> DelayLine {
        let d = &self.delay;
        DelayLine {
            buf: (0..DELAY_LEN as u32).map(|i| d.get(i)).collect(),
            gate: d.gate(),
        }
    }

    /// Presets the delay line and its gate (a capture's export-start state, e.g. rbhost's
    /// `delay_buffer` and `delay_gate` dumps). Panics unless `line.buf` has [`DELAY_LEN`] cells.
    pub fn set_delay_line(&self, line: DelayLine) {
        assert_eq!(line.buf.len(), DELAY_LEN);
        for (i, &x) in line.buf.iter().enumerate() {
            self.delay.set(i as u32, x);
        }
        self.delay.set_gate(line.gate);
    }

    /// The delay's slot 6, 0x100100E0: the line refilled with 1e-6 and the gate set. The
    /// effect of msg 53 when it replaces a current song (that song's delay runs it) and of
    /// msg 33 (slot 5, 0x1000F4F0, calls slot 6).
    pub fn refill_delay_line(&self) {
        self.delay.refill();
    }

    /// Whether two handles share the same state.
    pub fn same(&self, other: &ProcessState) -> bool {
        Arc::ptr_eq(&self.drums, &other.drums) && Arc::ptr_eq(&self.delay, &other.delay)
    }

    pub(crate) fn delay(&self) -> &SharedDelayLine {
        &self.delay
    }

    /// The 808 snare's hit: `c = (c + 1) & 31` is stored and returned (0x1002A23F..0x1002A27D:
    /// the word is incremented as a 32-bit value, masked, stored as a word).
    pub(crate) fn next_snare_808(&self) -> u16 {
        let c = (self.drums[SNARE_808].load(Ordering::Relaxed) as u32 + 1) & 31;
        self.drums[SNARE_808].store(c as u16, Ordering::Relaxed);
        c as u16
    }

    pub(crate) fn bd_808(&self) -> u16 {
        self.drums[BD_808].load(Ordering::Relaxed)
    }

    pub(crate) fn set_bd_808(&self, v: u16) {
        self.drums[BD_808].store(v, Ordering::Relaxed);
    }

    /// The 909 snare's hit: `c = (c + 1) & 31` is stored and returned (0x1002DCB8..0x1002DCED:
    /// the word is incremented as a 32-bit value, masked, stored as a word).
    pub(crate) fn next_snare_909(&self) -> u16 {
        let c = (self.drums[SNARE_909].load(Ordering::Relaxed) as u32 + 1) & 31;
        self.drums[SNARE_909].store(c as u16, Ordering::Relaxed);
        c as u16
    }

    /// The 909 bass drum's toggle (`test edx, 0FFFFh` at 0x1002DB51).
    pub(crate) fn bd_909(&self) -> u16 {
        self.drums[BD_909].load(Ordering::Relaxed)
    }

    /// 0x1002DB82 (0) and 0x1002DC3B (1).
    pub(crate) fn set_bd_909(&self, v: u16) {
        self.drums[BD_909].store(v, Ordering::Relaxed);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn clones_share_the_counters() {
        let a = ProcessState::new();
        let b = a.clone();
        assert_eq!(a.drum_counters(), DrumCounters::default());
        let c = DrumCounters {
            snare_808: 31,
            snare_909: 2,
            bd_808: 1,
            bd_909: 0,
        };
        b.set_drum_counters(c);
        assert_eq!(a.drum_counters(), c);
        // 31 + 1 wraps to 0
        assert_eq!(a.next_snare_808(), 0);
        assert_eq!(b.next_snare_808(), 1);
        a.set_bd_808(0);
        assert_eq!(b.bd_808(), 0);
    }

    #[test]
    fn clones_share_the_delay_line() {
        let a = ProcessState::new();
        let b = a.clone();
        let fresh = a.delay_line();
        assert_eq!(fresh.buf.len(), DELAY_LEN);
        assert!(fresh.buf.iter().all(|&x| x == 0.0) && !fresh.gate);
        b.refill_delay_line();
        let l = a.delay_line();
        assert!(l.buf.iter().all(|&x| x.to_bits() == 0x3586_37BD) && l.gate);
        let mut p = DelayLine::default();
        p.buf[5] = 0.5;
        a.set_delay_line(p.clone());
        assert_eq!(b.delay_line(), p);
    }

    #[test]
    fn a_preset_counter_above_31_is_masked_on_the_next_hit() {
        let a = ProcessState::new();
        a.set_drum_counters(DrumCounters {
            snare_808: 40,
            ..Default::default()
        });
        assert_eq!(a.next_snare_808(), 41 & 31);
    }
}
