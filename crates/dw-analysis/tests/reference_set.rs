//! Abnahme auf dem synthetischen Referenz-Set (Spezifikation 10.1, Teil A).
//!
//! Tabelle anzeigen: `cargo test -p dw-analysis --test reference_set -- --nocapture`

mod common;

use common::{evaluate_reference_set, format_report, summarize, TrackResult};
use std::sync::OnceLock;

/// Das Set wird nur einmal erzeugt und analysiert.
fn results() -> &'static [TrackResult] {
    static RESULTS: OnceLock<Vec<TrackResult>> = OnceLock::new();
    RESULTS.get_or_init(|| {
        let results = evaluate_reference_set();
        println!("{}", format_report(&results));
        results
    })
}

#[test]
fn bpm_is_within_tolerance_on_all_tracks() {
    for r in results() {
        assert!(!r.octave_error, "{}: Halb-/Doppeltempo ({} statt {})", r.spec.name, r.analysis.bpm, r.truth.bpm);
        assert!(r.bpm_error.abs() <= 0.05, "{}: {} statt {} BPM", r.spec.name, r.analysis.bpm, r.truth.bpm);
        assert_eq!(r.analysis.grid.anchors().len(), 1);
        assert_eq!(r.analysis.grid.bpm(), r.analysis.bpm);
    }
}

#[test]
fn beat_positions_are_within_tolerance() {
    let summary = summarize(results());
    assert!(summary.beat_error_mean_ms <= 5.0, "mittlere Abweichung {} ms", summary.beat_error_mean_ms);
    // ANA-03 je Track: Mittel ≤ 5 ms, 95 % ≤ 10 ms.
    for r in results() {
        assert!(r.beat_error_mean_ms <= 5.0, "{}: Mittel {} ms", r.spec.name, r.beat_error_mean_ms);
        assert!(r.beat_error_p95_ms <= 10.0, "{}: 95 % {} ms", r.spec.name, r.beat_error_p95_ms);
    }
}

#[test]
fn downbeat_is_right_on_85_percent() {
    let summary = summarize(results());
    assert!(
        summary.downbeat_ok as f64 >= 0.85 * summary.tracks as f64,
        "Downbeat richtig bei {}/{}",
        summary.downbeat_ok,
        summary.tracks
    );
}

#[test]
fn key_is_exact_on_90_percent() {
    let summary = summarize(results());
    assert!(summary.key_ok as f64 >= 0.9 * summary.tracks as f64, "Tonart richtig bei {}/{}", summary.key_ok, summary.tracks);
}

#[test]
fn tempo_is_not_flagged_for_review() {
    // Unter 0,5 würde die Bibliothek „Tempo prüfen" anzeigen.
    for r in results() {
        assert!(r.analysis.bpm_confidence >= 0.5, "{}: Sicherheitswert {}", r.spec.name, r.analysis.bpm_confidence);
    }
}
