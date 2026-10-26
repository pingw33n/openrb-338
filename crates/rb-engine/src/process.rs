//! The DLL's process globals that reach an export: state that is not part of a song and that
//! no song, stop, locate or msg 33 resets (`docs/export_state.md` section 7.3).
//!
//! So far these are the four drum counters. They live in BSS (0 when the DLL loads) and change
//! only on drum hits, so an export depends on every snare and bass-drum hit since the process
//! started. [`ProcessState`] is the engine's handle on them: the song's devices share one
//! handle ([`crate::frame::Devices::process`]), and a caller that reproduces a capture made in a
//! process that had played before sets the counters before the export, as
//! `tools/rbhost --drum-counters A,B,C,D` presets them.
//!
//! The state is one handle per [`crate::frame::Engine`] (made by its `Devices`), not a true
//! process global: a second engine in the same OS process starts at zero, unlike the original,
//! where every song of the process shares the DLL's counters. To carry them from one engine to
//! the next, copy [`ProcessState::drum_counters`] into the next engine's handle.

use std::sync::Arc;
use std::sync::atomic::{AtomicU16, Ordering};

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

/// A shared handle on the process globals. Clones share the same state; a fresh handle holds
/// what the DLL has right after it is loaded (all zero).
#[derive(Debug, Clone, Default)]
pub struct ProcessState {
    drums: Arc<[AtomicU16; 4]>,
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
    fn a_preset_counter_above_31_is_masked_on_the_next_hit() {
        let a = ProcessState::new();
        a.set_drum_counters(DrumCounters {
            snare_808: 40,
            ..Default::default()
        });
        assert_eq!(a.next_snare_808(), 41 & 31);
    }
}
