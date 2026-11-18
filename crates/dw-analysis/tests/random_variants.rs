//! Belastungsprobe mit 240 zufällig zusammengestellten Tracks (anderes Tempo, andere
//! Struktur und Besetzung als im festen Referenz-Set). Läuft nicht im normalen Testlauf:
//!
//! `cargo test --release -p dw-analysis --test random_variants -- --ignored --nocapture`

mod common;

use dw_core::Key;
use dw_testgen::{Elements, HatPattern, SectionKind, SectionSpec, TrackSpec};
use rayon::prelude::*;

/// Xorshift, nur für die Auswahl der Parameter.
struct Dice(u64);

impl Dice {
    fn next(&mut self) -> u64 {
        self.0 ^= self.0 << 13;
        self.0 ^= self.0 >> 7;
        self.0 ^= self.0 << 17;
        self.0
    }

    fn unit(&mut self) -> f64 {
        (self.next() >> 11) as f64 / (1u64 << 53) as f64
    }

    fn below(&mut self, n: usize) -> usize {
        (self.next() % n as u64) as usize
    }
}

fn random_spec(index: usize) -> TrackSpec {
    let mut dice = Dice(0x1234_5678_9ABC_DEF1 ^ (index as u64).wrapping_mul(0x9E37_79B9_7F4A_7C15));
    for _ in 0..8 {
        dice.next();
    }
    let bpm = ((110.0 + 60.0 * dice.unit()) * 100.0).round() / 100.0;
    let key = Key::from_index(dice.below(24) as u8).unwrap();
    let lengths = [4u32, 8, 8, 16, 16, 32];
    let none = Elements::default();

    // Intro mit Kick und Hi-Hats, nur Kick oder ganz ohne Kick.
    let intro_bars = lengths[1 + dice.below(5)];
    let mut sections = vec![match dice.below(3) {
        0 => SectionSpec::with_elements(SectionKind::Intro, intro_bars, Elements { pad: true, hats: true, ..none }),
        1 => SectionSpec::with_elements(SectionKind::Intro, intro_bars, Elements { kick: true, ..none }),
        _ => SectionSpec::new(SectionKind::Intro, intro_bars),
    }];
    let kinds = [SectionKind::Main, SectionKind::Breakdown, SectionKind::Buildup, SectionKind::Drop];
    let mut last = SectionKind::Intro;
    for _ in 0..3 + dice.below(5) {
        let mut kind = kinds[dice.below(4)];
        if kind == last {
            let at = kinds.iter().position(|k| *k == kind).unwrap();
            kind = kinds[(at + 1 + dice.below(3)) % 4];
        }
        let bars = if kind == SectionKind::Buildup { lengths[dice.below(3)] } else { lengths[dice.below(6)] };
        sections.push(SectionSpec::new(kind, bars));
        last = kind;
    }
    sections.push(SectionSpec::new(SectionKind::Outro, lengths[1 + dice.below(4)]));

    TrackSpec {
        name: format!("rnd-{index:03}"),
        sample_rate: if dice.below(4) == 0 { 48_000 } else { 44_100 },
        bpm,
        key,
        first_beat_sec: dice.unit() * 2.0,
        bar_offset: dice.below(4) as u8,
        sections,
        hats: if dice.below(2) == 0 { HatPattern::Offbeat } else { HatPattern::Sixteenth },
        tuning_cents: if dice.below(4) == 0 { (dice.unit() * 60.0 - 30.0) as f32 } else { 0.0 },
        seed: 5000 + index as u64,
    }
}

#[test]
#[ignore = "dauert rund eine Minute"]
fn random_variants_meet_the_targets() {
    let specs: Vec<TrackSpec> = (0..240).map(random_spec).collect();
    let results: Vec<common::TrackResult> = specs.par_iter().map(common::evaluate).collect();
    let report = common::format_report(&results);
    // Nur Kopf, Fehlfälle und Zusammenfassung zeigen.
    for line in report.lines() {
        if !line.starts_with("rnd-") || line.contains("FALSCH") {
            println!("{line}");
        }
    }
    for r in results.iter().filter(|r| !r.key_ok || r.bpm_error.abs() > 0.05) {
        println!("{}: {} → {:.3} BPM, {} → {:?}", r.spec.name, r.truth.bpm, r.analysis.bpm, r.truth.key.name_de(), r.analysis.key.map(|k| k.name_de()));
    }
    let s = common::summarize(&results);
    assert_eq!(s.bpm_within, s.tracks);
    assert_eq!(s.octave_errors, 0);
    assert!(s.beat_error_worst_track_ms <= 5.0);
    assert!(s.downbeat_ok as f64 >= 0.85 * s.tracks as f64);
    assert!(s.key_ok as f64 >= 0.9 * s.tracks as f64);
}
