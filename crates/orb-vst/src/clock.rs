//! Host transport to the engine's tempo, play flag and position.
//!
//! This is OpenRB's clock, not ReBirth's MIDI clock (that path was not traced).
//! The host's tempo becomes BPM×1000, rounded. The host's beat position becomes
//! a tick at 192 ticks per quarter note, truncated toward zero. A jump of more
//! than one 16th note (48 ticks) relocates; smaller drift is left to the
//! engine so a locate does not retrigger every buffer.

use orb_engine::frame::Engine;
use orb_engine::song::PPQ;

/// One 16th note. A host jump larger than this relocates.
const JUMP: i64 = 48;

/// What the plugin reads from the host at the start of a buffer.
pub struct Host {
    pub playing: bool,
    /// Beats per minute, when the host reports one.
    pub tempo_bpm: Option<f64>,
    /// Position in quarter notes, when the host reports one.
    pub pos_beats: Option<f64>,
}

/// Remembers whether the previous buffer was playing, so play and halt run on
/// the edges.
pub struct Clock {
    was_playing: bool,
}

impl Clock {
    pub fn new() -> Self {
        Clock { was_playing: false }
    }

    /// `follow` is the "Follow host" parameter. When it is off the sequencer
    /// is halted and the host transport is ignored, so live notes are not cut
    /// by pattern steps.
    pub fn apply(&mut self, engine: &mut Engine, host: Host, follow: bool) {
        if !follow {
            if engine.playing() {
                engine.halt();
            }
            self.was_playing = false;
            return;
        }
        if let Some(bpm) = host.tempo_bpm {
            engine.set_tempo(milli_bpm(bpm));
        }
        let tick = host.pos_beats.map(beats_to_tick);
        if host.playing {
            if !self.was_playing {
                if let Some(t) = tick {
                    engine.locate(t);
                }
                engine.play();
            } else if let Some(t) = tick {
                let cur = engine.frame.seq.tick_pos() as i64;
                if (cur - t as i64).abs() > JUMP {
                    engine.locate(t);
                }
            }
        } else if self.was_playing {
            engine.halt();
            if let Some(t) = tick {
                engine.locate(t);
            }
        }
        self.was_playing = host.playing;
    }
}

/// BPM×1000, the engine's tempo unit, rounded and clamped to 1..=500000.
pub fn milli_bpm(bpm: f64) -> u32 {
    if !bpm.is_finite() || bpm <= 0.0 {
        return 1;
    }
    let m = (bpm * 1000.0).round();
    if m >= 500_000.0 {
        500_000
    } else if m < 1.0 {
        1
    } else {
        m as u32
    }
}

/// Quarter notes to an engine tick. Negative and non-finite positions are 0.
/// The result stops at the song-end tick so a locate cannot step past it.
pub fn beats_to_tick(beats: f64) -> u32 {
    if !beats.is_finite() || beats <= 0.0 {
        return 0;
    }
    let t = beats * PPQ as f64;
    if t >= 767_231.0 { 767_231 } else { t as u32 }
}

#[cfg(test)]
mod tests {
    use super::*;
    use orb_engine::frame::{Devices, Engine};
    use orb_engine::tables::Tables;
    use std::sync::Arc;

    fn engine() -> Engine {
        Engine::new(Arc::new(Tables::new()), Devices::placeholders())
    }

    #[test]
    fn tempo_and_ticks() {
        assert_eq!(milli_bpm(140.0), 140_000);
        assert_eq!(milli_bpm(120.4), 120_400);
        assert_eq!(milli_bpm(0.0), 1);
        assert_eq!(beats_to_tick(1.0), 192);
        assert_eq!(beats_to_tick(0.5), 96);
        assert_eq!(beats_to_tick(-1.0), 0);
    }

    #[test]
    fn play_locates_then_runs_and_stop_does_not_jump_to_the_bar() {
        let mut engine = engine();
        let mut clock = Clock::new();
        clock.apply(
            &mut engine,
            Host {
                playing: true,
                tempo_bpm: Some(120.0),
                pos_beats: Some(2.0),
            },
            true,
        );
        assert!(engine.playing());
        assert_eq!(engine.frame.seq.globals.tempo, 120_000);
        assert_eq!(engine.frame.seq.tick_pos(), 384);
        clock.apply(
            &mut engine,
            Host {
                playing: false,
                tempo_bpm: Some(120.0),
                pos_beats: Some(2.25),
            },
            true,
        );
        assert!(!engine.playing());
        // halt, then locate to the host tick. Not the bar start (384).
        assert_eq!(engine.frame.seq.tick_pos(), 432);
    }

    #[test]
    fn follow_off_halts() {
        let mut engine = engine();
        let mut clock = Clock::new();
        clock.apply(
            &mut engine,
            Host {
                playing: true,
                tempo_bpm: Some(100.0),
                pos_beats: Some(0.0),
            },
            true,
        );
        assert!(engine.playing());
        clock.apply(
            &mut engine,
            Host {
                playing: true,
                tempo_bpm: Some(100.0),
                pos_beats: Some(0.0),
            },
            false,
        );
        assert!(!engine.playing());
    }
}
