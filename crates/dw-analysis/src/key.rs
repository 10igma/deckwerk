//! Tonart-Erkennung (Spezifikation 5.3).

use crate::preprocess::ANALYSIS_RATE;
use dw_core::Key;
use rayon::prelude::*;
use realfft::RealFftPlanner;

/// Langes Fenster für feine Frequenzauflösung (2,7 Hz je Bin).
const WINDOW: usize = 16_384;
const HOP: usize = 8192;
/// Ausgewertete Halbtöne: A1 (55 Hz) bis H6 (1 976 Hz), je ± ½ Halbton.
const MIDI_MIN: f64 = 33.0;
const MIDI_MAX: f64 = 95.0;
/// Eine Spitze zählt nur mit dem Anteil, der das 1,5-Fache ihrer spektralen Umgebung
/// übersteigt. Glatte Spektren (Kick-Sweep, Rauschen) liefern so fast nichts.
const SALIENCE_FACTOR: f32 = 1.5;

/// Profile nach Krumhansl-Kessler (Startwerte laut Spezifikation), Index = Halbtöne über dem Grundton.
const PROFILE_MAJOR: [f64; 12] = [6.35, 2.23, 3.48, 2.33, 4.38, 4.09, 2.52, 5.19, 2.39, 3.66, 2.29, 2.88];
const PROFILE_MINOR: [f64; 12] = [6.33, 2.68, 3.52, 5.38, 2.60, 3.53, 2.54, 4.75, 3.98, 2.69, 3.34, 3.17];

/// Ergebnis der Tonart-Erkennung.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct KeyEstimate {
    /// `None`, wenn das Signal keine auswertbaren Töne enthält.
    pub key: Option<Key>,
    /// 0–1; unter 0,3 gilt der Track als perkussiv/atonal.
    pub confidence: f32,
    /// Abweichung vom Kammerton in Cent (−50 … +50).
    pub tuning_cents: f32,
    /// Chroma-Profil (C … H), auf Summe 1 normiert.
    pub chroma: [f32; 12],
}

struct FramePeaks {
    /// (Tonhöhe in Halbtönen relativ zu a′, Gewicht)
    peaks: Vec<(f32, f32)>,
    /// Spektrale Flachheit im ausgewerteten Bereich, 0–1.
    flatness: f32,
}

/// Schätzt Tonart, Sicherheitswert und Stimmung aus dem Analyse-Signal.
pub fn estimate(signal: &[f32]) -> KeyEstimate {
    let none = KeyEstimate { key: None, confidence: 0.0, tuning_cents: 0.0, chroma: [0.0; 12] };
    if signal.is_empty() {
        return none;
    }
    let frames = analyze_frames(signal);

    // Schritt 2: Stimmung aus der Lage der Spitzen innerhalb des Halbtons (zirkuläres Mittel).
    let (mut re, mut im) = (0.0f64, 0.0f64);
    for frame in &frames {
        let w = frame_weight(frame);
        for &(pitch, amp) in &frame.peaks {
            let angle = std::f64::consts::TAU * pitch as f64;
            re += w * amp as f64 * angle.cos();
            im += w * amp as f64 * angle.sin();
        }
    }
    if re == 0.0 && im == 0.0 {
        return none;
    }
    let tuning = im.atan2(re) / std::f64::consts::TAU; // Halbtöne, −0,5 … 0,5

    // Schritt 3: Chroma-Profil, geräuschhafte Fenster geringer gewichtet.
    let mut chroma = [0.0f64; 12];
    for frame in &frames {
        let w = frame_weight(frame);
        for &(pitch, amp) in &frame.peaks {
            let shifted = pitch as f64 - tuning;
            let nearest = shifted.round();
            let off = shifted - nearest;
            // a′ hat Tonklasse 9.
            let class = (nearest as i64 + 9).rem_euclid(12) as usize;
            let cos = (std::f64::consts::PI * off).cos();
            chroma[class] += w * amp as f64 * cos * cos;
        }
    }
    let sum: f64 = chroma.iter().sum();
    if sum <= 0.0 {
        return none;
    }

    // Schritt 4: Korrelation mit den 24 Tonart-Profilen.
    let mut scores: Vec<(f64, u8)> = (0..24u8)
        .map(|index| {
            let tonic = (index % 12) as usize;
            let profile = if index < 12 { &PROFILE_MAJOR } else { &PROFILE_MINOR };
            let rotated: [f64; 12] = std::array::from_fn(|class| profile[(class + 12 - tonic) % 12]);
            (correlation(&chroma, &rotated), index)
        })
        .collect();
    scores.sort_by(|a, b| b.0.total_cmp(&a.0));
    let (best, second) = (scores[0].0, scores[1].0);

    // Schritt 5: Abstand zwischen bester und zweitbester Korrelation.
    let confidence = (((best - second) / 0.15).clamp(0.0, 1.0) * (best / 0.5).clamp(0.0, 1.0)) as f32;

    KeyEstimate {
        key: Key::from_index(scores[0].1),
        confidence,
        tuning_cents: (tuning * 100.0) as f32,
        chroma: std::array::from_fn(|i| (chroma[i] / sum) as f32),
    }
}

fn frame_weight(frame: &FramePeaks) -> f64 {
    let tonal = 1.0 - frame.flatness as f64;
    tonal * tonal
}

fn correlation(a: &[f64; 12], b: &[f64; 12]) -> f64 {
    let mean_a = a.iter().sum::<f64>() / 12.0;
    let mean_b = b.iter().sum::<f64>() / 12.0;
    let (mut ab, mut aa, mut bb) = (0.0, 0.0, 0.0);
    for i in 0..12 {
        let (x, y) = (a[i] - mean_a, b[i] - mean_b);
        ab += x * y;
        aa += x * x;
        bb += y * y;
    }
    if aa <= 0.0 || bb <= 0.0 {
        0.0
    } else {
        ab / (aa * bb).sqrt()
    }
}

/// Schritt 1: Spektral-Spitzen je Fenster.
fn analyze_frames(signal: &[f32]) -> Vec<FramePeaks> {
    let fft = RealFftPlanner::<f32>::new().plan_fft_forward(WINDOW);
    let window: Vec<f32> = (0..WINDOW)
        .map(|i| 0.5 - 0.5 * (std::f32::consts::TAU * i as f32 / WINDOW as f32).cos())
        .collect();
    let norm = 2.0 / window.iter().sum::<f32>();
    let bin_hz = ANALYSIS_RATE as f64 / WINDOW as f64;
    let hz = |midi: f64| 440.0 * 2f64.powf((midi - 69.0) / 12.0);
    let (f_lo, f_hi) = (hz(MIDI_MIN - 0.5), hz(MIDI_MAX + 0.5));
    let k_lo = (f_lo / bin_hz).ceil() as usize;
    let k_hi = (f_hi / bin_hz).floor() as usize;
    // Umgebung für die Bewertung einer Spitze: ±12 % der Frequenz, mindestens ±8 Bins.
    let reach = |k: usize| ((0.12 * k as f64).round() as usize).max(8);
    let top = k_hi + reach(k_hi) + 2;

    let n_frames = if signal.len() <= WINDOW { 1 } else { (signal.len() - WINDOW) / HOP + 1 };
    (0..n_frames)
        .into_par_iter()
        .map_init(
            || (fft.make_input_vec(), fft.make_output_vec(), fft.make_scratch_vec(), vec![0.0f32; top + 1], vec![0.0f64; top + 2]),
            |(input, spectrum, scratch, mag, prefix), frame| {
                let start = frame * HOP;
                for (i, x) in input.iter_mut().enumerate() {
                    *x = signal.get(start + i).copied().unwrap_or(0.0) * window[i];
                }
                fft.process_with_scratch(input, spectrum, scratch).expect("FFT-Längen passen");
                prefix[0] = 0.0;
                for k in 0..=top {
                    mag[k] = spectrum[k].norm() * norm;
                    prefix[k + 1] = prefix[k] + mag[k] as f64;
                }

                // Flachheit der Leistung im ausgewerteten Bereich.
                let (mut log_sum, mut lin_sum) = (0.0f64, 0.0f64);
                for &m in &mag[k_lo..=k_hi] {
                    let p = (m as f64) * (m as f64) + 1e-20;
                    log_sum += p.ln();
                    lin_sum += p;
                }
                let count = (k_hi - k_lo + 1) as f64;
                let mean = lin_sum / count;
                if mean < 1e-14 {
                    // Stille.
                    return FramePeaks { peaks: Vec::new(), flatness: 1.0 };
                }
                let flatness = ((log_sum / count).exp() / mean).clamp(0.0, 1.0) as f32;

                let mut peaks = Vec::new();
                for k in k_lo.max(3)..=k_hi {
                    let m = mag[k];
                    if !(m > mag[k - 1] && m >= mag[k + 1]) || m < 1e-5 {
                        continue;
                    }
                    // Mittel der Umgebung ohne die Spitze selbst (Hauptkeule ±2 Bins).
                    let r = reach(k);
                    let lo = k.saturating_sub(r);
                    let hi = k + r;
                    let around = (prefix[hi + 1] - prefix[lo]) - (prefix[k + 3] - prefix[k - 2]);
                    let env = (around / (hi - lo + 1 - 5) as f64) as f32;
                    let salient = m - SALIENCE_FACTOR * env;
                    if salient <= 0.0 {
                        continue;
                    }
                    // Parabel durch die logarithmierten Nachbarwerte: genaue Frequenz.
                    let (a, b, c) = (mag[k - 1].max(1e-12).ln(), m.ln(), mag[k + 1].max(1e-12).ln());
                    let denom = a - 2.0 * b + c;
                    let delta = if denom < 0.0 { (0.5 * (a - c) / denom).clamp(-0.5, 0.5) } else { 0.0 };
                    let freq = (k as f64 + delta as f64) * bin_hz;
                    if freq < f_lo || freq > f_hi {
                        continue;
                    }
                    let pitch = 12.0 * (freq / 440.0).log2();
                    peaks.push((pitch as f32, salient));
                }
                FramePeaks { peaks, flatness }
            },
        )
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn tone_mix(freqs: &[(f64, f32)], seconds: f64) -> Vec<f32> {
        let n = (seconds * ANALYSIS_RATE as f64) as usize;
        (0..n)
            .map(|i| {
                freqs
                    .iter()
                    .map(|&(f, a)| a * (std::f64::consts::TAU * f * i as f64 / ANALYSIS_RATE as f64).sin() as f32)
                    .sum()
            })
            .collect()
    }

    #[test]
    fn c_major_scale_tones_give_c_major() {
        // C-Dur-Dreiklang betont, übrige Leitertöne leiser.
        let hz = |midi: i32| 440.0 * 2f64.powf((midi - 69) as f64 / 12.0);
        let tones: Vec<(f64, f32)> = [(60, 0.3), (64, 0.2), (67, 0.25), (62, 0.08), (65, 0.1), (69, 0.08), (71, 0.06)]
            .iter()
            .map(|&(m, a)| (hz(m), a))
            .collect();
        let est = estimate(&tone_mix(&tones, 6.0));
        assert_eq!(est.key, Some(Key::new(0, false)));
        assert!(est.tuning_cents.abs() < 3.0, "Stimmung {}", est.tuning_cents);
        assert!(est.confidence > 0.3);
    }

    #[test]
    fn detuned_a_minor_is_found_with_tuning() {
        let hz = |midi: i32| 440.0 * 2f64.powf((midi - 69) as f64 / 12.0 + 30.0 / 1200.0);
        let tones: Vec<(f64, f32)> = [(57, 0.3), (60, 0.2), (64, 0.25), (59, 0.08), (62, 0.1), (65, 0.08), (67, 0.06)]
            .iter()
            .map(|&(m, a)| (hz(m), a))
            .collect();
        let est = estimate(&tone_mix(&tones, 6.0));
        assert_eq!(est.key, Some(Key::new(9, true)));
        assert!((est.tuning_cents - 30.0).abs() < 3.0, "Stimmung {}", est.tuning_cents);
    }

    #[test]
    fn silence_has_no_key() {
        let est = estimate(&vec![0.0; 100_000]);
        assert_eq!(est.key, None);
        assert_eq!(est.confidence, 0.0);
    }
}
