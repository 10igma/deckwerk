//! Festes synthetisches Referenz-Set (Spezifikation 10.1, Teil A).

use crate::track::{Elements, HatPattern, SectionKind, SectionSpec, TrackSpec};
use dw_core::Key;

/// Anzahl der Tracks im Referenz-Set.
pub const REFERENCE_SET_SIZE: usize = 40;

/// Positionen des ersten Beats in Sekunden, reihum verwendet.
const FIRST_BEATS: [f64; 11] = [0.0, 0.0125, 0.05, 0.137, 0.25, 0.3333, 0.5, 0.731, 1.0, 1.618, 2.0];

/// Das feste Referenz-Set: 40 Tracks von je 2–4 Minuten.
///
/// - BPM gleichmäßig über 120–160 verteilt, fast alle mit gebrochenen Werten (u. a. 133,37);
/// - alle 24 Tonarten, einige Tracks um +20 bzw. −15 Cent verstimmt;
/// - elf verschiedene Positionen des ersten Beats, alle vier `bar_offset`-Werte je zehnmal;
/// - sechs Strukturen, beide Hi-Hat-Muster, 44,1 und 48 kHz.
pub fn reference_set() -> Vec<TrackSpec> {
    (0..REFERENCE_SET_SIZE).map(reference_spec).collect()
}

/// Track `index` (0 … 39) des Referenz-Sets.
pub fn reference_spec(index: usize) -> TrackSpec {
    assert!(index < REFERENCE_SET_SIZE);
    // Reihenfolge der Tempi durchmischen, damit Tempo und Tonart nicht gemeinsam steigen.
    let j = (index * 7) % REFERENCE_SET_SIZE;
    let bpm = if j == 13 {
        133.37
    } else {
        ((120.0 + 40.0 * j as f64 / (REFERENCE_SET_SIZE - 1) as f64) * 100.0).round() / 100.0
    };
    let key = Key::from_index(((index * 5 + 3) % 24) as u8).expect("Index < 24");
    let tuning_cents = match index % 10 {
        4 => 20.0,
        9 => -15.0,
        _ => 0.0,
    };
    TrackSpec {
        name: format!("ref-{index:02}"),
        sample_rate: if index % 5 == 2 { 48_000 } else { 44_100 },
        bpm,
        key,
        first_beat_sec: FIRST_BEATS[index % FIRST_BEATS.len()],
        bar_offset: (index % 4) as u8,
        sections: structure(index % 6),
        hats: if (index / 2) % 2 == 0 { HatPattern::Offbeat } else { HatPattern::Sixteenth },
        tuning_cents,
        seed: 0xDEC0_0000 + index as u64,
    }
}

/// Sechs Strukturen mit 96–120 Takten.
fn structure(variant: usize) -> Vec<SectionSpec> {
    use SectionKind::*;
    let s = SectionSpec::new;
    match variant {
        0 => vec![s(Intro, 16), s(Main, 32), s(Breakdown, 16), s(Buildup, 8), s(Drop, 32), s(Outro, 16)],
        1 => vec![
            s(Intro, 8),
            s(Main, 16),
            s(Breakdown, 8),
            s(Buildup, 8),
            s(Drop, 32),
            s(Breakdown, 8),
            s(Buildup, 4),
            s(Drop, 16),
            s(Outro, 8),
        ],
        2 => vec![s(Intro, 16), s(Main, 24), s(Breakdown, 16), s(Buildup, 8), s(Drop, 24), s(Outro, 8)],
        // Intro ohne Kick (Fläche und Hi-Hats), früher erster Drop.
        3 => vec![
            SectionSpec::with_elements(
                Intro,
                8,
                Elements { pad: true, hats: true, ..Elements::default() },
            ),
            s(Buildup, 8),
            s(Drop, 32),
            s(Main, 16),
            s(Breakdown, 16),
            s(Buildup, 8),
            s(Drop, 16),
            s(Outro, 16),
        ],
        // Langes Intro, Drop direkt nach dem Breakdown (kein Aufbau).
        4 => vec![s(Intro, 32), s(Main, 32), s(Breakdown, 8), s(Drop, 32), s(Outro, 16)],
        // Abschnittsgrenzen im 4-Takt-Raster statt im 8-Takt-Raster.
        _ => vec![s(Intro, 12), s(Main, 32), s(Breakdown, 12), s(Buildup, 4), s(Drop, 32), s(Outro, 8)],
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashSet;

    #[test]
    fn reference_set_covers_the_required_range() {
        let set = reference_set();
        assert_eq!(set.len(), REFERENCE_SET_SIZE);
        assert_eq!(set, reference_set());

        let keys: HashSet<u8> = set.iter().map(|s| s.key.index()).collect();
        assert_eq!(keys.len(), 24);
        for offset in 0..4u8 {
            assert_eq!(set.iter().filter(|s| s.bar_offset == offset).count(), 10);
        }
        assert!(set.iter().all(|s| (120.0..=160.0).contains(&s.bpm)));
        assert!(set.iter().any(|s| s.bpm == 133.37));
        let fractional = set.iter().filter(|s| s.bpm.fract() != 0.0).count();
        assert!(fractional >= 36);
        let bpms: HashSet<u64> = set.iter().map(|s| (s.bpm * 100.0).round() as u64).collect();
        assert_eq!(bpms.len(), REFERENCE_SET_SIZE);
        assert!(set.iter().any(|s| s.sample_rate == 48_000));

        // Länge 2–4 Minuten (plus Vorlauf und Ausklang).
        for s in &set {
            let seconds = s.first_beat_sec + (s.bar_offset as u32 + 4 * s.total_bars()) as f64 * 60.0 / s.bpm + 1.0;
            assert!((120.0..=245.0).contains(&seconds), "{}: {seconds} s", s.name);
        }
    }
}
