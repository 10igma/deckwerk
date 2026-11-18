//! Tonarten, Camelot-/Open-Key-Schreibweise und harmonische Verträglichkeit (Spezifikation 5.3, 5.11).

use serde::{Deserialize, Serialize};

/// Eine von 24 Tonarten. Index 0–11 = Dur (C … H), 12–23 = Moll (c … h).
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug, Serialize, Deserialize)]
pub struct Key(u8);

const NAMES_MAJOR: [&str; 12] = [
    "C-Dur", "Des-Dur", "D-Dur", "Es-Dur", "E-Dur", "F-Dur", "Fis-Dur", "G-Dur", "As-Dur", "A-Dur",
    "B-Dur", "H-Dur",
];
const NAMES_MINOR: [&str; 12] = [
    "c-Moll", "cis-Moll", "d-Moll", "es-Moll", "e-Moll", "f-Moll", "fis-Moll", "g-Moll", "as-Moll",
    "a-Moll", "b-Moll", "h-Moll",
];

impl Key {
    /// `pitch_class`: 0 = C, 1 = Cis/Des, … 11 = H.
    pub fn new(pitch_class: u8, minor: bool) -> Self {
        Key(pitch_class % 12 + if minor { 12 } else { 0 })
    }

    pub fn from_index(index: u8) -> Option<Self> {
        (index < 24).then_some(Key(index))
    }

    pub fn index(self) -> u8 {
        self.0
    }

    pub fn pitch_class(self) -> u8 {
        self.0 % 12
    }

    pub fn is_minor(self) -> bool {
        self.0 >= 12
    }

    /// Camelot-Zahl 1–12 und Buchstabe ('A' = Moll, 'B' = Dur).
    pub fn camelot(self) -> (u8, char) {
        // Moll wird über die Paralleltonart (kleine Terz höher) auf dieselbe Zahl gelegt.
        let major_pc = if self.is_minor() { (self.pitch_class() + 3) % 12 } else { self.pitch_class() };
        let number = ((major_pc as u32 * 7) % 12 + 7) % 12 + 1;
        (number as u8, if self.is_minor() { 'A' } else { 'B' })
    }

    /// Open-Key-Zahl 1–12 und Buchstabe ('m' = Moll, 'd' = Dur).
    pub fn open_key(self) -> (u8, char) {
        let (n, letter) = self.camelot();
        ((n + 4) % 12 + 1, if letter == 'A' { 'm' } else { 'd' })
    }

    pub fn from_camelot(number: u8, letter: char) -> Option<Self> {
        if !(1..=12).contains(&number) {
            return None;
        }
        let minor = match letter.to_ascii_uppercase() {
            'A' => true,
            'B' => false,
            _ => return None,
        };
        (0..24).map(Key).find(|k| k.camelot() == (number, if minor { 'A' } else { 'B' }))
    }

    pub fn name_de(self) -> &'static str {
        if self.is_minor() {
            NAMES_MINOR[self.pitch_class() as usize]
        } else {
            NAMES_MAJOR[self.pitch_class() as usize]
        }
    }

    pub fn camelot_string(self) -> String {
        let (n, l) = self.camelot();
        format!("{n}{l}")
    }

    pub fn open_key_string(self) -> String {
        let (n, l) = self.open_key();
        format!("{n}{l}")
    }

    pub fn transpose(self, semitones: i32) -> Key {
        let pc = (self.pitch_class() as i32 + semitones).rem_euclid(12) as u8;
        Key::new(pc, self.is_minor())
    }
}

/// Harmonische Verträglichkeit K für den Wechsel von `a` nach `b` (Tabelle in 5.11).
pub fn harmonic_compat(a: Key, b: Key) -> f32 {
    let (na, la) = a.camelot();
    let (nb, lb) = b.camelot();
    // Schritte im Uhrzeigersinn von a nach b, 0–11.
    let up = (nb as i32 - na as i32).rem_euclid(12);
    let same_letter = la == lb;
    match (up, same_letter) {
        (0, true) => 1.0,
        (0, false) => 0.9,
        (1, true) | (11, true) => 0.9,
        (1, false) | (11, false) => 0.6,
        (2, true) => 0.6,
        (7, true) => 0.5,
        _ => 0.1,
    }
}

/// Tonhöhenverschiebung in Halbtönen bei einer Tempoänderung ohne Key Lock.
pub fn semitones_for_rate(rate: f64) -> f64 {
    12.0 * rate.log2()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn camelot_table_matches_spec() {
        let c_major = Key::new(0, false);
        let a_minor = Key::new(9, true);
        assert_eq!(c_major.camelot(), (8, 'B'));
        assert_eq!(a_minor.camelot(), (8, 'A'));
        assert_eq!(c_major.open_key(), (1, 'd'));
        assert_eq!(a_minor.open_key(), (1, 'm'));
        assert_eq!(Key::new(8, true).camelot(), (1, 'A')); // as-Moll
        assert_eq!(Key::new(11, false).camelot(), (1, 'B')); // H-Dur
        assert_eq!(Key::new(1, true).camelot(), (12, 'A')); // cis-Moll
        assert_eq!(Key::new(4, false).camelot(), (12, 'B')); // E-Dur
        assert_eq!(Key::new(5, true).camelot(), (4, 'A')); // f-Moll
        assert_eq!(Key::new(8, true).open_key(), (6, 'm'));
        assert_eq!(Key::new(2, true).open_key(), (12, 'm')); // d-Moll = 7A = 12m
    }

    #[test]
    fn camelot_roundtrip() {
        for i in 0..24 {
            let k = Key::from_index(i).unwrap();
            let (n, l) = k.camelot();
            assert_eq!(Key::from_camelot(n, l), Some(k));
        }
    }

    #[test]
    fn transposing_one_semitone_moves_seven_steps() {
        for i in 0..24 {
            let k = Key::from_index(i).unwrap();
            let (n, l) = k.camelot();
            let (n2, l2) = k.transpose(1).camelot();
            assert_eq!(l, l2);
            assert_eq!((n as i32 + 7 - 1).rem_euclid(12) + 1, n2 as i32);
        }
    }

    #[test]
    fn compat_table() {
        let k = |n, l| Key::from_camelot(n, l).unwrap();
        assert_eq!(harmonic_compat(k(8, 'A'), k(8, 'A')), 1.0);
        assert_eq!(harmonic_compat(k(8, 'A'), k(7, 'A')), 0.9);
        assert_eq!(harmonic_compat(k(8, 'A'), k(9, 'A')), 0.9);
        assert_eq!(harmonic_compat(k(8, 'A'), k(8, 'B')), 0.9);
        assert_eq!(harmonic_compat(k(8, 'A'), k(9, 'B')), 0.6);
        assert_eq!(harmonic_compat(k(8, 'B'), k(7, 'A')), 0.6);
        assert_eq!(harmonic_compat(k(8, 'A'), k(10, 'A')), 0.6);
        assert_eq!(harmonic_compat(k(8, 'A'), k(3, 'A')), 0.5);
        assert_eq!(harmonic_compat(k(8, 'A'), k(2, 'A')), 0.1);
        assert_eq!(harmonic_compat(k(12, 'A'), k(1, 'A')), 0.9);
    }
}
