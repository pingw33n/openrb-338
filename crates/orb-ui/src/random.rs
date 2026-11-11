//! The randomization of a 303 pattern by the GENERATE menu's RANDOM row (`generate.rs`): the
//! chosen step fields of all 16 steps get new random values. DOWN, UP, ACCENT and SLIDE are each
//! set on about one step in four (UP and DOWN never on one step); NOTE makes about three steps
//! in four a note (the rest are rests), each a note of the synth's chosen scale and key.

use crate::model::Step303;
use crate::scale;

/// The RANDOM row's five buttons, in their order: the step fields they randomize.
pub const PARAMS: [&str; 5] = ["DOWN", "UP", "ACCENT", "SLIDE", "NOTE"];
pub const DOWN: usize = 0;
pub const UP: usize = 1;
pub const ACCENT: usize = 2;
pub const SLIDE: usize = 3;
pub const NOTE: usize = 4;

/// The share of steps a flag is set on, and the share of steps NOTE makes a note.
pub const FLAG_CHANCE: f64 = 0.25;
pub const NOTE_CHANCE: f64 = 0.75;

/// A small random number generator (xorshift64*): enough for musical randomness, no crate.
#[derive(Clone, Copy, Debug)]
pub struct Rng(u64);

impl Rng {
    /// A generator from `seed` (any value; 0 is replaced).
    pub fn new(seed: u64) -> Self {
        Self(if seed == 0 { 0x9e37_79b9_7f4a_7c15 } else { seed })
    }

    /// A generator seeded from the clock.
    pub fn from_time() -> Self {
        let t = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map_or(1, |d| d.as_nanos() as u64);
        Self::new(t)
    }

    pub fn next_u64(&mut self) -> u64 {
        let mut x = self.0;
        x ^= x >> 12;
        x ^= x << 25;
        x ^= x >> 27;
        self.0 = x;
        x.wrapping_mul(0x2545_f491_4f6c_dd1d)
    }

    /// A number in 0.0..1.0.
    pub fn unit(&mut self) -> f64 {
        (self.next_u64() >> 11) as f64 / (1u64 << 53) as f64
    }

    /// True with probability `p`.
    pub fn chance(&mut self, p: f64) -> bool {
        self.unit() < p
    }

    /// A number in 0..n (n > 0).
    pub fn below(&mut self, n: usize) -> usize {
        (self.unit() * n as f64) as usize % n
    }
}

/// Randomizes the fields `on` (indexed by `PARAMS`) of all the steps. NOTE needs the scale's
/// notes `mask` (`scale::mask`) and is left out without it. The other fields stay as they were,
/// except that setting UP clears DOWN on its step and setting DOWN clears UP.
pub fn randomize(steps: &mut [Step303; 16], on: [bool; 5], mask: Option<u16>, rng: &mut Rng) {
    let notes: Vec<u8> = mask
        .map(|m| (0..=12u8).filter(|&n| scale::key_in(m, n as usize)).collect())
        .unwrap_or_default();
    for st in steps.iter_mut() {
        match (on[DOWN], on[UP]) {
            (true, true) => {
                // one draw for both: never up and down on one step
                let r = rng.unit();
                st.down = r < FLAG_CHANCE;
                st.up = !st.down && r < 2.0 * FLAG_CHANCE;
            }
            (true, false) => {
                st.down = rng.chance(FLAG_CHANCE);
                st.up &= !st.down;
            }
            (false, true) => {
                st.up = rng.chance(FLAG_CHANCE);
                st.down &= !st.up;
            }
            (false, false) => {}
        }
        if on[ACCENT] {
            st.accent = rng.chance(FLAG_CHANCE);
        }
        if on[SLIDE] {
            st.slide = rng.chance(FLAG_CHANCE);
        }
        if on[NOTE] && !notes.is_empty() {
            st.note = notes[rng.below(notes.len())];
            st.rest = !rng.chance(NOTE_CHANCE);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::scale::{KEYS, SCALES, mask};

    fn c_major() -> u16 {
        let s = SCALES.iter().position(|(n, _)| *n == "MAJOR").unwrap();
        let k = KEYS.iter().position(|n| *n == "C").unwrap();
        mask(s, k)
    }

    /// A pattern with every field set, so a field left alone is easy to see.
    fn full() -> [Step303; 16] {
        [Step303 {
            note: 1,
            slide: true,
            accent: true,
            up: true,
            down: false,
            rest: false,
        }; 16]
    }

    #[test]
    fn notes_come_from_the_scale_and_about_three_in_four_are_notes() {
        let mut rng = Rng::new(7);
        let mut notes = 0;
        for _ in 0..100 {
            let mut p = full();
            randomize(&mut p, [false, false, false, false, true], Some(c_major()), &mut rng);
            for st in p {
                assert!(scale::key_in(c_major(), st.note as usize), "note {}", st.note);
                assert!(st.note <= 12);
                notes += !st.rest as usize;
                // the flags stay as they were
                assert!(st.slide && st.accent && st.up && !st.down);
            }
        }
        let share = notes as f64 / 1600.0;
        assert!((0.70..0.80).contains(&share), "{share}");
    }

    #[test]
    fn note_needs_a_scale() {
        let mut p = full();
        randomize(&mut p, [false, false, false, false, true], None, &mut Rng::new(1));
        assert_eq!(p, full());
    }

    #[test]
    fn up_and_down_are_never_on_one_step_and_each_is_on_about_one_in_four() {
        let mut rng = Rng::new(3);
        let (mut ups, mut downs) = (0, 0);
        for _ in 0..100 {
            let mut p = full();
            randomize(&mut p, [true, true, false, false, false], None, &mut rng);
            for st in p {
                assert!(!(st.up && st.down));
                ups += st.up as usize;
                downs += st.down as usize;
                assert!(st.slide && st.accent && st.note == 1 && !st.rest);
            }
        }
        for n in [ups, downs] {
            let share = n as f64 / 1600.0;
            assert!((0.20..0.30).contains(&share), "{share}");
        }
        // DOWN alone clears UP where it sets DOWN
        let mut p = full();
        randomize(&mut p, [true, false, false, false, false], None, &mut rng);
        assert!(p.iter().all(|st| !(st.up && st.down)));
        assert!(p.iter().any(|st| st.down));
    }

    #[test]
    fn accent_and_slide_change_only_themselves() {
        let mut rng = Rng::new(11);
        let mut p = full();
        randomize(&mut p, [false, false, true, true, false], None, &mut rng);
        assert!(p.iter().any(|st| !st.accent) && p.iter().any(|st| !st.slide));
        assert!(p.iter().all(|st| st.note == 1 && st.up && !st.down && !st.rest));
    }
}
