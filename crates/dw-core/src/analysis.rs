//! Ergebnis der Track-Analyse (Spezifikation 5.1–5.5). Wird von `dw-analysis` erzeugt
//! und von Engine, Bibliothek und Oberfläche gelesen.

use crate::{BeatGrid, Key};
use serde::{Deserialize, Serialize};

/// Wird erhöht, wenn sich ein Analyse-Verfahren so ändert, dass alte Ergebnisse veralten (ANA-17).
pub const ANALYSIS_VERSION: u32 = 1;

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct TrackAnalysis {
    pub version: u32,
    /// Dauer in Sekunden.
    pub duration: f64,
    pub bpm: f64,
    /// 0–1; unter 0,5 gilt „Tempo prüfen".
    pub bpm_confidence: f32,
    pub grid: BeatGrid,
    pub key: Option<Key>,
    /// 0–1; unter 0,3 gilt der Track als perkussiv/atonal.
    pub key_confidence: f32,
    /// Abweichung vom Kammerton in Cent (−50 … +50).
    pub tuning_cents: f32,
    /// Integrierte Lautheit in LUFS.
    pub lufs: f32,
    /// Lautheitsbereich in LU.
    pub lra: f32,
    /// True Peak in dBTP.
    pub true_peak_db: f32,
}

impl TrackAnalysis {
    /// Auto-Gain in dB für die Referenz-Lautheit, begrenzt auf ±12 dB (5.4).
    pub fn auto_gain_db(&self, reference_lufs: f32) -> f32 {
        if self.lufs.is_finite() {
            (reference_lufs - self.lufs).clamp(-12.0, 12.0)
        } else {
            0.0
        }
    }
}

/// Eine Auflösungsstufe der Wellenform: je Punkt und Band Minimum, Maximum und RMS.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct WaveformLevel {
    /// Audio-Samples (bei der Abtastrate des Tracks) je Punkt.
    pub samples_per_point: u32,
    /// Je Punkt 9 Werte: Tief (min, max, rms), Mitte (min, max, rms), Hoch (min, max, rms).
    pub data: Vec<f32>,
}

impl WaveformLevel {
    pub const VALUES_PER_POINT: usize = 9;

    pub fn points(&self) -> usize {
        self.data.len() / Self::VALUES_PER_POINT
    }
}

/// Wellenform-Daten in drei Bändern (Tief < 200 Hz, Mitte 200 Hz–2,5 kHz, Hoch > 2,5 kHz).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Waveform {
    pub sample_rate: u32,
    pub frames: u64,
    /// Feste Übersicht mit 2 000 Punkten über den ganzen Track.
    pub overview: WaveformLevel,
    /// Pyramide mit 64, 256, 1 024 und 4 096 Samples je Punkt.
    pub levels: Vec<WaveformLevel>,
}
