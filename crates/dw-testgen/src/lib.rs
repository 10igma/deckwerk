//! Generator für synthetische Referenz-Tracks und Messsignale (Spezifikation 10.1, Teil A).
//!
//! Alles ist deterministisch: gleiche Parameter und gleicher Seed ergeben bitgleiche Samples.
//! Es werden keine Audiodateien gelesen oder geschrieben.

mod reference;
mod rng;
mod track;

pub use reference::{reference_set, reference_spec, REFERENCE_SET_SIZE};
pub use track::{
    techno_track, Elements, Event, EventKind, GroundTruth, HatPattern, Section, SectionKind,
    SectionSpec, TrackSpec,
};

use dw_core::AudioBuf;
use rng::Rng;

/// Spitzenpegel eines Klicks in [`click_track`].
pub const CLICK_PEAK: f32 = 0.9;

/// Klick-Spur: kurze breitbandige Impulse exakt auf den Beats, sonst Stille.
///
/// Beat `k` (0 … `beats − 1`) liegt bei `first_beat_sec + k · 60 / bpm`; der Klick beginnt
/// auf dem nächstgelegenen Frame (Fehler ≤ ½ Frame, kein Aufsummieren). Jeder Klick ist
/// 1 ms lang, sein erstes Sample hat den Wert [`CLICK_PEAK`], danach klingt Rauschen
/// schnell ab; alle Klicks sind identisch. Die Spur endet eine Beat-Länge nach dem
/// letzten Klick, lässt sich also nahtlos wiederholen.
pub fn click_track(sample_rate: u32, bpm: f64, first_beat_sec: f64, beats: usize) -> AudioBuf {
    assert!(sample_rate > 0 && bpm > 0.0 && first_beat_sec >= 0.0);
    let sr = sample_rate as f64;
    let period = 60.0 / bpm;
    let frames = ((first_beat_sec + beats as f64 * period) * sr).ceil() as usize;
    let mut buf = AudioBuf::silence(sample_rate, frames);

    // Klick-Form: Impuls, dann abklingendes Rauschen (Zeitkonstante 0,2 ms).
    let len = ((sr * 0.001).round() as usize).max(1);
    let tau = sr * 0.0002;
    let mut rng = Rng::new(0xC11C);
    let click: Vec<f32> = (0..len)
        .map(|i| {
            if i == 0 {
                CLICK_PEAK
            } else {
                CLICK_PEAK * 0.8 * rng.bipolar() * (-(i as f64) / tau).exp() as f32
            }
        })
        .collect();

    for k in 0..beats {
        let start = ((first_beat_sec + k as f64 * period) * sr).round() as usize;
        for (i, &s) in click.iter().enumerate() {
            let f = start + i;
            if f >= frames {
                break;
            }
            buf.data[2 * f] = s;
            buf.data[2 * f + 1] = s;
        }
    }
    buf
}

/// Sinuston (beide Kanäle gleich), Startphase 0. `amplitude` ist der lineare Spitzenwert.
pub fn sine(sample_rate: u32, freq_hz: f64, seconds: f64, amplitude: f32) -> AudioBuf {
    assert!(sample_rate > 0 && seconds >= 0.0);
    let sr = sample_rate as f64;
    let frames = (seconds * sr).round() as usize;
    let w = std::f64::consts::TAU * freq_hz / sr;
    let mut data = Vec::with_capacity(frames * 2);
    for n in 0..frames {
        let s = (amplitude as f64 * (w * n as f64).sin()) as f32;
        data.push(s);
        data.push(s);
    }
    AudioBuf::new(sample_rate, data)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn click_track_has_clicks_exactly_on_beats() {
        let sr = 48_000;
        let bpm = 133.37;
        let first = 0.25;
        let buf = click_track(sr, bpm, first, 16);
        let period = 60.0 / bpm;
        assert_eq!(buf.frames(), ((first + 16.0 * period) * sr as f64).ceil() as usize);
        for k in 0..16 {
            let f = ((first + k as f64 * period) * sr as f64).round() as usize;
            assert_eq!(buf.data[2 * f], CLICK_PEAK);
            assert_eq!(buf.data[2 * f + 1], CLICK_PEAK);
            // Direkt vor dem Klick ist Stille.
            assert_eq!(buf.data[2 * (f - 1)], 0.0);
        }
        // Außerhalb der Klicks (1 ms) ist alles still.
        let click_len = (sr as f64 * 0.001).round() as usize;
        let nonzero = buf.data.chunks_exact(2).filter(|f| f[0] != 0.0).count();
        assert!(nonzero <= 16 * click_len);
        assert_eq!(buf.peak(), CLICK_PEAK);
    }

    #[test]
    fn click_track_is_deterministic() {
        assert_eq!(click_track(44_100, 128.0, 0.0, 8), click_track(44_100, 128.0, 0.0, 8));
    }

    #[test]
    fn sine_has_requested_level_and_length() {
        let buf = sine(48_000, 1000.0, 2.0, 0.5);
        assert_eq!(buf.frames(), 96_000);
        assert!((buf.peak() - 0.5).abs() < 1e-6);
        let rms = (buf.data.iter().map(|s| (*s as f64).powi(2)).sum::<f64>() / buf.data.len() as f64).sqrt();
        assert!((rms - 0.5 / 2f64.sqrt()).abs() < 1e-4);
        assert_eq!(buf.data[0], 0.0);
    }
}
