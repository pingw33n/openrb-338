//! The randomization of a 303 pattern by the GENERATE menu's RANDOM row (`generate.rs`): the
//! chosen step fields of all 16 steps get new random values. DOWN, UP, ACCENT and SLIDE are each
//! set on about one step in four (UP and DOWN never on one step); NOTE makes about three steps
//! in four a note (the rest are rests), each a note of the synth's chosen scale and key.
//!
//! The ALTER row (`alter`) makes nothing new: it moves the chosen fields of the steps to other
//! steps, all along one chain through the 16 steps (step 3's to step 15, step 15's to another,
//! and so on back to step 3), so every step's values land somewhere else. The chosen fields of
//! a step move together.

use crate::model::Step303;
use crate::scale;

/// The RANDOM and ALTER rows' five buttons, in their order: the step fields they act on.
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

/// A random chain through the 16 steps (Sattolo's shuffle): `to[i]` is the step that step `i`'s
/// values move to. It is one cycle, so no step keeps its own values.
pub fn chain(rng: &mut Rng) -> [usize; 16] {
    let mut to: [usize; 16] = std::array::from_fn(|i| i);
    for i in (1..16).rev() {
        let j = rng.below(i);
        to.swap(i, j);
    }
    to
}

/// Moves the fields `on` (indexed by `PARAMS`) of every step along one random chain (`chain`):
/// the chosen fields of a step move together, the others stay. NOTE moves the note with its
/// rest flag. When UP or DOWN alone moves onto a step that has the other, the one moved in
/// stays and the other is cleared (never both on one step).
pub fn alter(steps: &mut [Step303; 16], on: [bool; 5], rng: &mut Rng) {
    if !on.iter().any(|&o| o) {
        return;
    }
    let to = chain(rng);
    let old = *steps;
    for (from, &dest) in to.iter().enumerate() {
        let (o, n) = (&old[from], &mut steps[dest]);
        if on[DOWN] {
            n.down = o.down;
        }
        if on[UP] {
            n.up = o.up;
        }
        match (on[DOWN], on[UP]) {
            (true, false) if n.down => n.up = false,
            (false, true) if n.up => n.down = false,
            _ => {}
        }
        if on[ACCENT] {
            n.accent = o.accent;
        }
        if on[SLIDE] {
            n.slide = o.slide;
        }
        if on[NOTE] {
            n.note = o.note;
            n.rest = o.rest;
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

    /// A pattern whose every step has its own note value (i, so a moved note is easy to see;
    /// alter does not look at the value) and a flag pattern by step.
    fn varied() -> [Step303; 16] {
        std::array::from_fn(|i| Step303 {
            note: i as u8,
            slide: i % 3 == 0,
            accent: i % 4 == 0,
            up: i % 5 == 1,
            down: i % 5 == 3,
            rest: i % 7 == 6,
        })
    }

    #[test]
    fn the_chain_moves_every_step_somewhere_else_in_one_cycle() {
        let mut rng = Rng::new(21);
        for _ in 0..50 {
            let to = chain(&mut rng);
            let mut seen = [false; 16];
            let mut i = 0;
            for _ in 0..16 {
                assert!(!seen[i]);
                seen[i] = true;
                assert_ne!(to[i], i);
                i = to[i];
            }
            assert_eq!(i, 0, "one cycle through all 16 steps");
        }
    }

    #[test]
    fn alter_moves_the_chosen_fields_together_and_keeps_the_others() {
        let old = varied();
        let mut p = old;
        alter(&mut p, [false, false, true, false, true], &mut Rng::new(4));
        assert_ne!(p, old);
        // the same notes, rests and accents, in other places, moved together
        let mut a: Vec<_> = old.iter().map(|s| (s.note, s.rest, s.accent)).collect();
        let mut b: Vec<_> = p.iter().map(|s| (s.note, s.rest, s.accent)).collect();
        a.sort();
        b.sort();
        assert_eq!(a, b);
        for (i, (o, n)) in old.iter().zip(p.iter()).enumerate() {
            assert_ne!(n.note, o.note, "step {i} got another step's note");
            assert_eq!((n.slide, n.up, n.down), (o.slide, o.up, o.down), "step {i}");
        }
    }

    #[test]
    fn alter_never_puts_up_and_down_on_one_step() {
        let mut rng = Rng::new(8);
        for on in [[true, false, false, false, false], [false, true, false, false, false]] {
            for _ in 0..50 {
                let mut p = varied();
                alter(&mut p, on, &mut rng);
                assert!(p.iter().all(|s| !(s.up && s.down)));
            }
        }
        let mut p = varied();
        alter(&mut p, [false; 5], &mut rng);
        assert_eq!(p, varied(), "nothing chosen, nothing moves");
    }
}
