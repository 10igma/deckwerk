//! Gemeinsame Auswertung für Tests und das Beispiel `kennzahlen`:
//! Analyse des synthetischen Referenz-Sets und Vergleich mit der Grundwahrheit.
#![allow(dead_code)]

use dw_analysis::{analyze, AnalysisConfig};
use dw_core::{Key, TrackAnalysis};
use dw_testgen::{reference_spec, techno_track, GroundTruth, SectionKind, SectionSpec, TrackSpec};
use rayon::prelude::*;
use std::time::Instant;

/// Track für die Geschwindigkeitsmessung (ANA-16): 7 Minuten bei 48 kHz.
pub fn seven_minute_spec() -> TrackSpec {
    let mut spec = reference_spec(0);
    spec.name = "7min".into();
    spec.sample_rate = 48_000;
    spec.bpm = 128.0;
    spec.sections = vec![
        SectionSpec::new(SectionKind::Intro, 32),
        SectionSpec::new(SectionKind::Main, 64),
        SectionSpec::new(SectionKind::Breakdown, 16),
        SectionSpec::new(SectionKind::Buildup, 16),
        SectionSpec::new(SectionKind::Drop, 64),
        SectionSpec::new(SectionKind::Outro, 32),
    ];
    spec
}

/// Vergleich eines Tracks mit seiner Grundwahrheit.
pub struct TrackResult {
    pub spec: TrackSpec,
    pub truth: GroundTruth,
    pub analysis: TrackAnalysis,
    /// Geschätztes minus wahres Tempo.
    pub bpm_error: f64,
    pub octave_error: bool,
    /// Abweichung der Beat-Positionen in ms: Mittel der Beträge, 95. Perzentil, Mittel mit Vorzeichen.
    pub beat_error_mean_ms: f64,
    pub beat_error_p95_ms: f64,
    pub beat_error_signed_ms: f64,
    pub downbeat_ok: bool,
    pub key_ok: bool,
    /// Gewichtete Wertung nach 5.3 (exakt 1,0; Quinte 0,5; parallel 0,3; gleichnamig 0,2).
    pub key_score: f64,
    pub analysis_seconds: f64,
}

/// Gewichtete Tonart-Wertung nach Spezifikation 5.3.
pub fn key_score(truth: Key, estimate: Option<Key>) -> f64 {
    let Some(est) = estimate else { return 0.0 };
    let interval = (est.pitch_class() as i32 - truth.pitch_class() as i32).rem_euclid(12);
    if est == truth {
        1.0
    } else if est.is_minor() == truth.is_minor() && (interval == 7 || interval == 5) {
        0.5
    } else if est.camelot().0 == truth.camelot().0 {
        0.3
    } else if interval == 0 {
        0.2
    } else {
        0.0
    }
}

pub fn evaluate(spec: &TrackSpec) -> TrackResult {
    let (audio, truth) = techno_track(spec);
    let started = Instant::now();
    let analysis = analyze(&audio, &AnalysisConfig::default());
    let analysis_seconds = started.elapsed().as_secs_f64();

    let bpm_error = analysis.bpm - truth.bpm;
    let ratio = analysis.bpm / truth.bpm;
    let octave_error = (ratio / 2.0 - 1.0).abs() < 0.04 || (ratio * 2.0 - 1.0).abs() < 0.04;

    // Abstand jedes wahren Beats zum nächsten Beat des geschätzten Rasters.
    let period = 60.0 / analysis.bpm;
    let mut errors: Vec<f64> = truth
        .beat_times()
        .iter()
        .map(|&t| {
            let beat = analysis.grid.beat_at(t);
            (beat.round() - beat) * period * 1000.0
        })
        .collect();
    let n = errors.len() as f64;
    let beat_error_signed_ms = errors.iter().sum::<f64>() / n;
    let beat_error_mean_ms = errors.iter().map(|e| e.abs()).sum::<f64>() / n;
    errors.iter_mut().for_each(|e| *e = e.abs());
    errors.sort_by(|a, b| a.total_cmp(b));
    let beat_error_p95_ms = errors[(errors.len() - 1) * 95 / 100];

    // Die wahre erste Eins muss im geschätzten Raster auf einer Eins liegen.
    let true_downbeat = truth.grid.sec_at(truth.grid.bar_offset as f64);
    let downbeat_ok = analysis.grid.is_downbeat(analysis.grid.nearest_beat(true_downbeat));

    TrackResult {
        spec: spec.clone(),
        bpm_error,
        octave_error,
        beat_error_mean_ms,
        beat_error_p95_ms,
        beat_error_signed_ms,
        downbeat_ok,
        key_ok: analysis.key == Some(truth.key),
        key_score: key_score(truth.key, analysis.key),
        analysis_seconds,
        truth,
        analysis,
    }
}

/// Erzeugt und analysiert das ganze Referenz-Set (parallel über die Tracks).
pub fn evaluate_reference_set() -> Vec<TrackResult> {
    dw_testgen::reference_set().par_iter().map(evaluate).collect()
}

/// Kennzahlen über das ganze Set.
pub struct Summary {
    pub tracks: usize,
    pub bpm_within: usize,
    pub bpm_max_error: f64,
    pub octave_errors: usize,
    pub beat_error_mean_ms: f64,
    pub beat_error_worst_track_ms: f64,
    pub beat_error_worst_p95_ms: f64,
    pub downbeat_ok: usize,
    pub key_ok: usize,
    pub key_score: f64,
    pub min_bpm_confidence: f32,
    pub min_key_confidence: f32,
    pub max_tuning_error: f32,
}

pub fn summarize(results: &[TrackResult]) -> Summary {
    let n = results.len();
    // Mittel über alle Beats aller Tracks.
    let beats: f64 = results.iter().map(|r| r.truth.beats as f64).sum();
    let weighted: f64 = results.iter().map(|r| r.beat_error_mean_ms * r.truth.beats as f64).sum();
    Summary {
        tracks: n,
        bpm_within: results.iter().filter(|r| r.bpm_error.abs() <= 0.05).count(),
        bpm_max_error: results.iter().map(|r| r.bpm_error.abs()).fold(0.0, f64::max),
        octave_errors: results.iter().filter(|r| r.octave_error).count(),
        beat_error_mean_ms: weighted / beats,
        beat_error_worst_track_ms: results.iter().map(|r| r.beat_error_mean_ms).fold(0.0, f64::max),
        beat_error_worst_p95_ms: results.iter().map(|r| r.beat_error_p95_ms).fold(0.0, f64::max),
        downbeat_ok: results.iter().filter(|r| r.downbeat_ok).count(),
        key_ok: results.iter().filter(|r| r.key_ok).count(),
        key_score: results.iter().map(|r| r.key_score).sum::<f64>() / n as f64,
        min_bpm_confidence: results.iter().map(|r| r.analysis.bpm_confidence).fold(1.0, f32::min),
        min_key_confidence: results.iter().map(|r| r.analysis.key_confidence).fold(1.0, f32::min),
        max_tuning_error: results
            .iter()
            .map(|r| (r.analysis.tuning_cents - r.truth.tuning_cents).abs())
            .fold(0.0, f32::max),
    }
}

/// Tabelle je Track und Zusammenfassung mit den Zielwerten.
pub fn format_report(results: &[TrackResult]) -> String {
    use std::fmt::Write;
    let mut out = String::new();
    let pct = |count: usize, of: usize| 100.0 * count as f64 / of as f64;
    writeln!(
        out,
        "{:<7} {:>5} {:>7} {:>8} {:>7} {:>5} | {:>6} {:>6} {:>6} | {:>3} {:>4} | {:<9} {:<9} {:>4} {:>5} | {:>6} {:>6}",
        "Track", "kHz", "BPM", "erkannt", "Δ", "Sich.", "Beat∅", "Beat95", "Versch", "Off", "Eins", "Tonart", "erkannt", "Sich", "Cent", "LUFS", "Zeit s"
    )
    .unwrap();
    for r in results {
        writeln!(
            out,
            "{:<7} {:>5.1} {:>7.2} {:>8.3} {:>+7.3} {:>5.2} | {:>6.2} {:>6.2} {:>+6.2} | {:>3} {:>4} | {:<9} {:<9} {:>4.2} {:>+5.0} | {:>6.1} {:>6.2}",
            r.spec.name,
            r.spec.sample_rate as f64 / 1000.0,
            r.truth.bpm,
            r.analysis.bpm,
            r.bpm_error,
            r.analysis.bpm_confidence,
            r.beat_error_mean_ms,
            r.beat_error_p95_ms,
            r.beat_error_signed_ms,
            r.truth.grid.bar_offset,
            if r.downbeat_ok { "ok" } else { "FALSCH" },
            r.truth.key.name_de(),
            r.analysis.key.map_or("–", |k| k.name_de()),
            r.analysis.key_confidence,
            r.analysis.tuning_cents,
            r.analysis.lufs,
            r.analysis_seconds,
        )
        .unwrap();
    }
    let s = summarize(results);
    let rows: [(&str, String, &str); 10] = [
        (
            "BPM innerhalb ±0,05",
            format!("{:.1} % (größter Fehler {:.4})", pct(s.bpm_within, s.tracks), s.bpm_max_error),
            "100 %",
        ),
        ("Halb-/Doppeltempo-Fehler", format!("{}", s.octave_errors), "0"),
        ("Beat-Abweichung, Mittel", format!("{:.3} ms", s.beat_error_mean_ms), "≤ 5 ms"),
        (
            "  schlechtester Track (Mittel / 95 %)",
            format!("{:.3} / {:.3} ms", s.beat_error_worst_track_ms, s.beat_error_worst_p95_ms),
            "(≤ 5 / ≤ 10 ms)",
        ),
        (
            "Downbeat richtig",
            format!("{:.1} % ({}/{})", pct(s.downbeat_ok, s.tracks), s.downbeat_ok, s.tracks),
            "≥ 85 %",
        ),
        ("Tonart exakt richtig", format!("{:.1} % ({}/{})", pct(s.key_ok, s.tracks), s.key_ok, s.tracks), "≥ 90 %"),
        ("Tonart, gewichtete Wertung (5.3)", format!("{:.3}", s.key_score), "(≥ 0,80)"),
        ("Stimmung, größter Fehler", format!("{:.1} Cent", s.max_tuning_error), "–"),
        ("kleinster Tempo-Sicherheitswert", format!("{:.2}", s.min_bpm_confidence), "(≥ 0,5)"),
        ("kleinster Tonart-Sicherheitswert", format!("{:.2}", s.min_key_confidence), "(≥ 0,3)"),
    ];
    writeln!(out).unwrap();
    writeln!(out, "{:<37} | {:<32} | Ziel", "Kennzahl", "gemessen").unwrap();
    writeln!(out, "{:-<38}+{:-<34}+{:-<17}", "", "", "").unwrap();
    for (name, measured, target) in rows {
        writeln!(out, "{name:<37} | {measured:<32} | {target}").unwrap();
    }
    out
}
