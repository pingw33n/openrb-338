//! The scale and key of each bass synth, chosen in the GENERATE menu (`generate.rs`), and the
//! rule that keeps the two synths in harmony: BAZE 1 (synth 0) is the primary one, and BAZE 2
//! (synth 1) may only use a scale and key whose every note is also in BAZE 1's.
//!
//! Notes are pitch classes 0 to 11 (C to B); a set of them is a 12-bit mask, bit n for pitch
//! class n. The 303's keys `key0` to `key12` are the notes C to the C an octave up.

/// The scales of the list: the name shown and the notes as semitones above the key note.
pub const SCALES: [(&str, &[u8]); 13] = [
    ("MAJOR", &[0, 2, 4, 5, 7, 9, 11]),
    ("MINOR", &[0, 2, 3, 5, 7, 8, 10]),
    ("HARM MINOR", &[0, 2, 3, 5, 7, 8, 11]),
    ("MEL MINOR", &[0, 2, 3, 5, 7, 9, 11]),
    ("DORIAN", &[0, 2, 3, 5, 7, 9, 10]),
    ("PHRYGIAN", &[0, 1, 3, 5, 7, 8, 10]),
    ("LYDIAN", &[0, 2, 4, 6, 7, 9, 11]),
    ("MIXOLYDIAN", &[0, 2, 4, 5, 7, 9, 10]),
    ("LOCRIAN", &[0, 1, 3, 5, 6, 8, 10]),
    ("MAJ PENTA", &[0, 2, 4, 7, 9]),
    ("MIN PENTA", &[0, 3, 5, 7, 10]),
    ("BLUES", &[0, 3, 5, 6, 7, 10]),
    ("CHROMATIC", &[0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11]),
];

/// The key notes of the list, pitch classes 0 to 11.
pub const KEYS: [&str; 12] = [
    "C", "C#", "D", "D#", "E", "F", "F#", "G", "G#", "A", "A#", "B",
];

/// The notes of scale `scale` on key note `key`, as a pitch-class mask.
pub fn mask(scale: usize, key: usize) -> u16 {
    SCALES[scale]
        .1
        .iter()
        .fold(0, |m, &s| m | 1 << ((s as usize + key) % 12))
}

/// Whether the 303's key `n` (0 to 12, C to C) is a note of the pitch-class mask `m`.
pub fn key_in(m: u16, n: usize) -> bool {
    m & (1 << (n % 12)) != 0
}

/// One synth's choice: nothing, a scale, or a scale and its key note.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct ScaleKey {
    pub scale: Option<usize>,
    pub key: Option<usize>,
}

impl ScaleKey {
    /// The notes of the scale once both the scale and the key are chosen.
    pub fn mask(&self) -> Option<u16> {
        Some(mask(self.scale?, self.key?))
    }
}

/// The choices of both synths, kept in harmony.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct ScaleKeys(pub [ScaleKey; 2]);

impl ScaleKeys {
    pub fn get(&self, synth: usize) -> ScaleKey {
        self.0[synth]
    }

    /// Whether a synth's fields can be used: BAZE 1's always, BAZE 2's once BAZE 1 has both a
    /// scale and a key.
    pub fn enabled(&self, synth: usize) -> bool {
        synth == 0 || self.0[0].mask().is_some()
    }

    /// Whether `scale` on `key` may be chosen for `synth`: any for BAZE 1; for BAZE 2 only one
    /// whose every note is in BAZE 1's scale.
    pub fn fits(&self, synth: usize, scale: usize, key: usize) -> bool {
        if synth == 0 {
            return true;
        }
        self.0[0]
            .mask()
            .is_some_and(|primary| mask(scale, key) & !primary == 0)
    }

    /// The scales the list offers `synth`: those with at least one key that fits.
    pub fn scale_options(&self, synth: usize) -> Vec<usize> {
        if !self.enabled(synth) {
            return vec![];
        }
        (0..SCALES.len())
            .filter(|&s| (0..12).any(|k| self.fits(synth, s, k)))
            .collect()
    }

    /// The key notes the list offers `synth` for its chosen scale (none before a scale).
    pub fn key_options(&self, synth: usize) -> Vec<usize> {
        match self.0[synth].scale {
            Some(s) if self.enabled(synth) => (0..12).filter(|&k| self.fits(synth, s, k)).collect(),
            _ => vec![],
        }
    }

    /// Chooses a scale from the list; false (and no change) if the list does not offer it. A key
    /// already chosen stays if it still fits.
    pub fn set_scale(&mut self, synth: usize, scale: usize) -> bool {
        if !self.scale_options(synth).contains(&scale) {
            return false;
        }
        let c = &mut self.0[synth];
        c.scale = Some(scale);
        if let Some(k) = c.key
            && !self.fits(synth, scale, k)
        {
            self.0[synth].key = None;
        }
        self.revalidate();
        true
    }

    /// Chooses a key note from the list; false (and no change) if the list does not offer it.
    pub fn set_key(&mut self, synth: usize, key: usize) -> bool {
        if !self.key_options(synth).contains(&key) {
            return false;
        }
        self.0[synth].key = Some(key);
        self.revalidate();
        true
    }

    /// After a change of BAZE 1: BAZE 2's choice is cleared when it no longer fits.
    fn revalidate(&mut self) {
        let second = self.0[1];
        let ok = match (second.scale, second.key) {
            (None, _) => true,
            (Some(s), None) => !self.key_options_for(s).is_empty(),
            (Some(s), Some(k)) => self.enabled(1) && self.fits(1, s, k),
        };
        if !ok {
            self.0[1] = ScaleKey::default();
        }
    }

    fn key_options_for(&self, scale: usize) -> Vec<usize> {
        if !self.enabled(1) {
            return vec![];
        }
        (0..12).filter(|&k| self.fits(1, scale, k)).collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn s(name: &str) -> usize {
        SCALES.iter().position(|(n, _)| *n == name).unwrap()
    }

    fn k(name: &str) -> usize {
        KEYS.iter().position(|n| *n == name).unwrap()
    }

    fn notes(m: u16) -> Vec<&'static str> {
        (0..12).filter(|&n| key_in(m, n)).map(|n| KEYS[n]).collect()
    }

    #[test]
    fn the_scales_have_their_notes() {
        assert_eq!(notes(mask(s("MAJOR"), k("C"))), ["C", "D", "E", "F", "G", "A", "B"]);
        assert_eq!(notes(mask(s("MINOR"), k("A"))), ["C", "D", "E", "F", "G", "A", "B"]);
        assert_eq!(notes(mask(s("MAJ PENTA"), k("G"))), ["D", "E", "G", "A", "B"]);
        assert_eq!(notes(mask(s("BLUES"), k("C"))), ["C", "D#", "F", "F#", "G", "A#"]);
        assert_eq!(mask(s("CHROMATIC"), k("F#")), 0xfff);
        // the upper C key (12) is the same note as the lower one
        let m = mask(s("MAJOR"), k("C"));
        assert!(key_in(m, 0) && key_in(m, 12) && !key_in(m, 1));
    }

    #[test]
    fn baze_2_waits_for_baze_1() {
        let mut sk = ScaleKeys::default();
        assert!(sk.enabled(0) && !sk.enabled(1));
        assert!(sk.scale_options(1).is_empty());
        assert!(!sk.set_scale(1, s("MAJOR")));
        // a key needs a scale first
        assert!(sk.key_options(0).is_empty());
        assert!(!sk.set_key(0, k("C")));
        assert!(sk.set_scale(0, s("MAJOR")));
        assert!(!sk.enabled(1));
        assert!(sk.set_key(0, k("C")));
        assert!(sk.enabled(1));
    }

    #[test]
    fn baze_2_is_offered_only_what_fits_baze_1() {
        let mut sk = ScaleKeys::default();
        sk.set_scale(0, s("MAJOR"));
        sk.set_key(0, k("C"));
        let names: Vec<&str> = sk.scale_options(1).into_iter().map(|i| SCALES[i].0).collect();
        assert_eq!(
            names,
            [
                "MAJOR",
                "MINOR",
                "DORIAN",
                "PHRYGIAN",
                "LYDIAN",
                "MIXOLYDIAN",
                "LOCRIAN",
                "MAJ PENTA",
                "MIN PENTA"
            ]
        );
        assert!(!sk.set_scale(1, s("BLUES")));
        assert!(sk.set_scale(1, s("MINOR")));
        assert_eq!(sk.key_options(1), [k("A")]);
        sk.set_scale(1, s("MAJ PENTA"));
        assert_eq!(sk.key_options(1), [k("C"), k("F"), k("G")]);
        assert!(!sk.set_key(1, k("D")));
        assert!(sk.set_key(1, k("G")));
        assert_eq!(sk.get(1).mask(), Some(mask(s("MAJ PENTA"), k("G"))));
    }

    #[test]
    fn a_change_of_baze_1_clears_baze_2_when_it_no_longer_fits() {
        let mut sk = ScaleKeys::default();
        sk.set_scale(0, s("MAJOR"));
        sk.set_key(0, k("C"));
        sk.set_scale(1, s("MINOR"));
        sk.set_key(1, k("A"));
        // C minor lacks A minor's E, A and B: BAZE 2 is cleared
        sk.set_scale(0, s("MINOR"));
        assert_eq!(sk.get(0), ScaleKey { scale: Some(s("MINOR")), key: Some(k("C")) });
        assert_eq!(sk.get(1), ScaleKey::default());
        // a chromatic BAZE 1 lets everything fit, and BAZE 2 stays when BAZE 1 goes back
        sk.set_scale(0, s("CHROMATIC"));
        sk.set_scale(1, s("MIN PENTA"));
        sk.set_key(1, k("A"));
        sk.set_scale(0, s("MAJOR"));
        assert_eq!(sk.get(1).mask(), Some(mask(s("MIN PENTA"), k("A"))));
    }
}
