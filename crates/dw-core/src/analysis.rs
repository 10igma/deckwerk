//! Ergebnis der Track-Analyse (Spezifikation 5.1–5.5). Wird von `dw-analysis` erzeugt
//! und von Engine, Bibliothek und Oberfläche gelesen.

use crate::{db_json, BeatGrid, Key};
use serde::{Deserialize, Serialize};

/// Wird erhöht, wenn sich ein Analyse-Verfahren so ändert, dass alte Ergebnisse veralten (ANA-17).
pub const ANALYSIS_VERSION: u32 = 2;

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
    /// Integrierte Lautheit in LUFS (−∞ bei Stille).
    #[serde(deserialize_with = "db_json::value")]
    pub lufs: f32,
    /// Lautheitsbereich in LU.
    pub lra: f32,
    /// True Peak in dBTP (−∞ bei Stille).
    #[serde(deserialize_with = "db_json::value")]
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

const WAVEFORM_MAGIC: &[u8; 4] = b"DWWF";
const WAVEFORM_FORMAT: u32 = 1;

impl Waveform {
    /// Kompaktes Binärformat für Cache und Oberfläche (5.5): je Wert ein Byte.
    /// Minimum und Maximum als vorzeichenbehaftetes Byte (±127 = ±1,0), RMS als 0–255.
    ///
    /// Aufbau (Little Endian): „DWWF", Format-Version u32, Abtastrate u32, Frames u64,
    /// Anzahl Stufen u32; je Stufe (Übersicht zuerst): Samples je Punkt u32, Punkte u32, Daten.
    pub fn to_bytes(&self) -> Vec<u8> {
        let levels: Vec<&WaveformLevel> = std::iter::once(&self.overview).chain(self.levels.iter()).collect();
        let size = 24 + levels.iter().map(|l| 8 + l.data.len()).sum::<usize>();
        let mut out = Vec::with_capacity(size);
        out.extend_from_slice(WAVEFORM_MAGIC);
        out.extend_from_slice(&WAVEFORM_FORMAT.to_le_bytes());
        out.extend_from_slice(&self.sample_rate.to_le_bytes());
        out.extend_from_slice(&self.frames.to_le_bytes());
        out.extend_from_slice(&(levels.len() as u32).to_le_bytes());
        for level in levels {
            out.extend_from_slice(&level.samples_per_point.to_le_bytes());
            out.extend_from_slice(&(level.points() as u32).to_le_bytes());
            for (i, &v) in level.data.iter().enumerate() {
                let is_rms = i % 3 == 2;
                out.push(if is_rms {
                    (v.clamp(0.0, 1.0) * 255.0).round() as u8
                } else {
                    ((v.clamp(-1.0, 1.0) * 127.0).round() as i8) as u8
                });
            }
        }
        out
    }

    pub fn from_bytes(bytes: &[u8]) -> Option<Waveform> {
        let u32_at = |i: usize| bytes.get(i..i + 4).map(|b| u32::from_le_bytes(b.try_into().unwrap()));
        if bytes.get(..4)? != WAVEFORM_MAGIC || u32_at(4)? != WAVEFORM_FORMAT {
            return None;
        }
        let sample_rate = u32_at(8)?;
        let frames = u64::from_le_bytes(bytes.get(12..20)?.try_into().unwrap());
        let count = u32_at(20)? as usize;
        let mut pos = 24;
        let mut levels = Vec::with_capacity(count);
        for _ in 0..count {
            let samples_per_point = u32_at(pos)?;
            let len = u32_at(pos + 4)? as usize * WaveformLevel::VALUES_PER_POINT;
            let raw = bytes.get(pos + 8..pos + 8 + len)?;
            let data = raw
                .iter()
                .enumerate()
                .map(|(i, &b)| if i % 3 == 2 { b as f32 / 255.0 } else { (b as i8) as f32 / 127.0 })
                .collect();
            levels.push(WaveformLevel { samples_per_point, data });
            pos += 8 + len;
        }
        if levels.is_empty() {
            return None;
        }
        let overview = levels.remove(0);
        Some(Waveform { sample_rate, frames, overview, levels })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn silence_survives_json() {
        let a = TrackAnalysis {
            version: 1,
            duration: 1.0,
            bpm: 120.0,
            bpm_confidence: 0.0,
            grid: crate::BeatGrid::constant(120.0, 0.0, 0),
            key: None,
            key_confidence: 0.0,
            tuning_cents: 0.0,
            lufs: f32::NEG_INFINITY,
            lra: 0.0,
            true_peak_db: f32::NEG_INFINITY,
        };
        let json = serde_json::to_string(&a).unwrap();
        assert!(json.contains("\"lufs\":null"));
        assert_eq!(serde_json::from_str::<TrackAnalysis>(&json).unwrap(), a);
    }

    #[test]
    fn waveform_binary_roundtrip() {
        let level = |spp: u32, points: usize| WaveformLevel {
            samples_per_point: spp,
            data: (0..points * 9).map(|i| if i % 3 == 2 { (i % 11) as f32 / 10.0 } else { ((i % 21) as f32 - 10.0) / 10.0 }).collect(),
        };
        let w = Waveform { sample_rate: 48_000, frames: 123_456_789, overview: level(61_728, 2000), levels: vec![level(64, 500), level(256, 125)] };
        let bytes = w.to_bytes();
        assert_eq!(bytes.len(), 24 + (8 + 18_000) + (8 + 4500) + (8 + 1125));
        let back = Waveform::from_bytes(&bytes).unwrap();
        assert_eq!((back.sample_rate, back.frames, back.levels.len()), (48_000, 123_456_789, 2));
        assert_eq!(back.levels[1].samples_per_point, 256);
        // Ein Byte je Wert: Abweichung höchstens eine halbe Stufe.
        for (a, b) in w.levels[0].data.iter().zip(&back.levels[0].data) {
            assert!((a - b).abs() <= 0.5 / 127.0 + 1e-6);
        }
        assert!(Waveform::from_bytes(&bytes[..100]).is_none());
        assert!(Waveform::from_bytes(b"kein Wellenform-Format").is_none());
    }
}
