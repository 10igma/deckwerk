//! Spektrogramm und Band-Merkmale (Spezifikation 5.1, 5.2 Schritt 1).
//!
//! Das Spektrogramm selbst wird nicht gespeichert; je Fenster bleiben pro Band die
//! Anschlagsstärke (positive Spektraldifferenz, logarithmisch komprimiert) und die Leistung.

use crate::preprocess::ANALYSIS_RATE;
use rayon::prelude::*;
use realfft::RealFftPlanner;

/// Fensterlänge der Kurzzeit-Fourier-Transformation.
pub const WINDOW: usize = 2048;
/// Schrittweite (5,8 ms bei 44,1 kHz).
pub const HOP: usize = 256;
/// Bänder: Tief, Tiefmitten, Mitten, Höhen.
pub const BANDS: usize = 4;
/// Bandgrenzen in Hz.
const BAND_EDGES: [f64; BANDS + 1] = [20.0, 150.0, 500.0, 5000.0, 16_000.0];
/// Stärke der logarithmischen Kompression: `ln(1 + GAMMA · |X|)`, |X| = 1 bei Vollaussteuerung.
const GAMMA: f32 = 100.0;

/// Band-Merkmale je Fenster. Fenster `i` ist um die Zeit `i · HOP / 44 100` zentriert.
pub struct BandFeatures {
    /// Positive Spektraldifferenz zum vorigen Fenster, gemittelt über die Bins des Bands.
    pub flux: [Vec<f32>; BANDS],
    /// Leistung im Band.
    pub power: [Vec<f32>; BANDS],
}

impl BandFeatures {
    pub fn frames(&self) -> usize {
        self.flux[0].len()
    }

    /// Fenster je Sekunde.
    pub fn frame_rate() -> f64 {
        ANALYSIS_RATE as f64 / HOP as f64
    }
}

/// Bin-Bereich `[lo, hi)` jedes Bands.
fn band_bins() -> [(usize, usize); BANDS] {
    let bin_hz = ANALYSIS_RATE as f64 / WINDOW as f64;
    let edge = |f: f64| (f / bin_hz).ceil() as usize;
    std::array::from_fn(|b| (edge(BAND_EDGES[b]), edge(BAND_EDGES[b + 1])))
}

/// Berechnet die Band-Merkmale des Analyse-Signals.
pub fn band_features(signal: &[f32]) -> BandFeatures {
    let n_frames = signal.len() / HOP + 1;
    let bins = band_bins();
    let top = bins[BANDS - 1].1;
    let fft = RealFftPlanner::<f32>::new().plan_fft_forward(WINDOW);
    let window: Vec<f32> = (0..WINDOW)
        .map(|i| 0.5 - 0.5 * (std::f32::consts::TAU * i as f32 / WINDOW as f32).cos())
        .collect();
    // Normierung: Sinus mit Amplitude 1 ergibt |X| = 1.
    let norm = 2.0 / window.iter().sum::<f32>();

    const CHUNK: usize = 512;
    let chunks: Vec<Vec<[f32; 2 * BANDS]>> = (0..n_frames.div_ceil(CHUNK))
        .into_par_iter()
        .map(|c| {
            let start = c * CHUNK;
            let end = (start + CHUNK).min(n_frames);
            let mut input = fft.make_input_vec();
            let mut spectrum = fft.make_output_vec();
            let mut scratch = fft.make_scratch_vec();
            let mut prev = vec![0.0f32; top];
            let mut cur = vec![0.0f32; top];
            let mut out = Vec::with_capacity(end - start);
            // Ein Fenster Vorlauf, damit die Differenz am Stückanfang stimmt.
            let first = start.saturating_sub(1);
            for frame in first..end {
                let center = frame * HOP;
                for (i, x) in input.iter_mut().enumerate() {
                    let idx = (center + i) as isize - (WINDOW / 2) as isize;
                    *x = if idx >= 0 && (idx as usize) < signal.len() {
                        signal[idx as usize] * window[i]
                    } else {
                        0.0
                    };
                }
                fft.process_with_scratch(&mut input, &mut spectrum, &mut scratch).expect("FFT-Längen passen");
                let mut feat = [0.0f32; 2 * BANDS];
                for (b, &(lo, hi)) in bins.iter().enumerate() {
                    let (mut flux, mut power) = (0.0f32, 0.0f32);
                    for k in lo..hi {
                        let mag = spectrum[k].norm() * norm;
                        power += mag * mag;
                        let compressed = (GAMMA * mag).ln_1p();
                        flux += (compressed - prev[k]).max(0.0);
                        cur[k] = compressed;
                    }
                    feat[b] = flux / (hi - lo) as f32;
                    feat[BANDS + b] = power;
                }
                std::mem::swap(&mut prev, &mut cur);
                // Vor dem ersten Fenster liegt Stille; ein Klang ab 0 s zählt als Anschlag.
                if frame >= start {
                    out.push(feat);
                }
            }
            out
        })
        .collect();

    let mut flux: [Vec<f32>; BANDS] = std::array::from_fn(|_| Vec::with_capacity(n_frames));
    let mut power: [Vec<f32>; BANDS] = std::array::from_fn(|_| Vec::with_capacity(n_frames));
    for feat in chunks.iter().flatten() {
        for b in 0..BANDS {
            flux[b].push(feat[b]);
            power[b].push(feat[BANDS + b]);
        }
    }
    BandFeatures { flux, power }
}
