//! Track-Analyse (Spezifikation 5.1–5.5): Tempo, Beatgrid, Downbeat, Tonart, Lautheit
//! und Wellenform-Daten.

pub mod downbeat;
pub mod key;
pub mod loudness;
pub mod preprocess;
pub mod spectrum;
pub mod tempo;
mod waveform;

pub use waveform::waveform;

use dw_core::{AudioBuf, BeatGrid, TrackAnalysis, ANALYSIS_VERSION};

/// Einstellungen der Analyse.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct AnalysisConfig {
    /// Erwarteter Tempobereich für die Oktav-Korrektur (5.2, Schritt 3).
    pub bpm_min: f64,
    pub bpm_max: f64,
}

impl AnalysisConfig {
    /// Voreinstellung „Techno": 110–170 BPM.
    pub const fn techno() -> Self {
        AnalysisConfig { bpm_min: 110.0, bpm_max: 170.0 }
    }

    /// Voreinstellung „frei": 60–200 BPM.
    pub const fn free() -> Self {
        AnalysisConfig { bpm_min: 60.0, bpm_max: 200.0 }
    }
}

impl Default for AnalysisConfig {
    fn default() -> Self {
        Self::techno()
    }
}

/// Tempo, das eingetragen wird, wenn keines erkennbar ist (Sicherheitswert 0).
const FALLBACK_BPM: f64 = 120.0;
/// Der Track „beginnt" und „endet", wo das Signal 2 % seines Spitzenpegels über- bzw. unterschreitet.
const START_THRESHOLD: f32 = 0.02;

/// Ein Raster-Beat, der weniger als 1 ms vor dem ersten Sample läge, wird auf 0 s gelegt.
const ANCHOR_TOLERANCE: f64 = 0.001;

/// Analysiert einen vollständig dekodierten Track.
///
/// Das Beatgrid hat immer genau einen Anker (konstantes Tempo); er liegt auf dem
/// Raster-Beat, der dem Beginn des Signals am nächsten ist. Ohne erkennbares Tempo
/// (Stille, Dauerton, weniger als ca. 5 s) steht `bpm_confidence` auf 0 und das Grid
/// auf 120 BPM ab 0 s.
pub fn analyze(audio: &AudioBuf, config: &AnalysisConfig) -> TrackAnalysis {
    let (loudness, (rhythm, key)) = rayon::join(
        || loudness::measure(audio),
        || {
            let signal = preprocess::analysis_signal(audio);
            rayon::join(|| analyze_rhythm(&signal, config), || key::estimate(&signal))
        },
    );
    let (bpm, bpm_confidence, grid) = rhythm;
    TrackAnalysis {
        version: ANALYSIS_VERSION,
        duration: audio.duration(),
        bpm,
        bpm_confidence,
        grid,
        key: key.key,
        key_confidence: key.confidence,
        tuning_cents: key.tuning_cents,
        lufs: loudness.lufs,
        lra: loudness.lra,
        true_peak_db: loudness.true_peak_db,
    }
}

/// Tempo, Phase und Downbeat; Rückgabe (BPM, Sicherheitswert, Beatgrid).
fn analyze_rhythm(signal: &[f32], config: &AnalysisConfig) -> (f64, f32, BeatGrid) {
    let fallback_bpm = FALLBACK_BPM.clamp(config.bpm_min, config.bpm_max);
    let fallback = (fallback_bpm, 0.0, BeatGrid::constant(fallback_bpm, 0.0, 0));
    if signal.is_empty() {
        return fallback;
    }
    let features = spectrum::band_features(signal);
    let Some(tempo) = tempo::estimate(signal, &features, config) else {
        return fallback;
    };
    let period = 60.0 / tempo.bpm;

    // Anker: Raster-Beat am Beginn des Signals, nicht vor 0 s.
    let peak = signal.iter().fold(0.0f32, |m, s| m.max(s.abs()));
    let audible = |s: &f32| s.abs() > START_THRESHOLD * peak;
    let rate = preprocess::ANALYSIS_RATE as f64;
    let start_sec = signal.iter().position(audible).unwrap_or(0) as f64 / rate;
    let end_sec = signal.iter().rposition(audible).map_or(signal.len(), |i| i + 1) as f64 / rate;
    let mut anchor = tempo.phase + ((start_sec - tempo.phase) / period).round() * period;
    if anchor < 0.0 && anchor > -ANCHOR_TOLERANCE {
        // Beat praktisch auf dem ersten Sample.
        anchor = 0.0;
    }
    while anchor < 0.0 {
        anchor += period;
    }

    let bar_offset = downbeat::estimate(&features, anchor, period, end_sec, tempo.first_kick);
    (tempo.bpm, tempo.confidence, BeatGrid::constant(tempo.bpm, anchor, bar_offset))
}
