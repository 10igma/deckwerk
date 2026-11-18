//! Vorverarbeitung (Spezifikation 5.1): Analyse-Signal mono mit 44,1 kHz.

use dw_core::AudioBuf;
use rayon::prelude::*;

/// Abtastrate des Analyse-Signals.
pub const ANALYSIS_RATE: u32 = 44_100;

/// Nulldurchgänge des Sinc-Kerns je Seite.
const ZERO_CROSSINGS: f64 = 16.0;
/// Phasenzahl der Tabelle, wenn das Ratenverhältnis nicht als kleiner Bruch darstellbar ist.
const FALLBACK_PHASES: usize = 1024;

/// Mono-Mischung, bei Bedarf auf 44,1 kHz umgerechnet. Zeitachse bleibt erhalten:
/// Sample `n` liegt bei `n / 44 100` Sekunden.
pub fn analysis_signal(audio: &AudioBuf) -> Vec<f32> {
    let mono = audio.to_mono();
    if audio.sample_rate == ANALYSIS_RATE {
        mono
    } else {
        resample(&mono, audio.sample_rate, ANALYSIS_RATE)
    }
}

fn gcd(a: u64, b: u64) -> u64 {
    if b == 0 {
        a
    } else {
        gcd(b, a % b)
    }
}

/// Abtastratenwandlung mit gefenstertem Sinc (Blackman), phasenlinear und ohne Zeitversatz.
pub fn resample(input: &[f32], from: u32, to: u32) -> Vec<f32> {
    assert!(from > 0 && to > 0);
    if from == to || input.is_empty() {
        return input.to_vec();
    }
    let g = gcd(from as u64, to as u64);
    let (l, m) = (to as u64 / g, from as u64 / g);
    let exact = l <= 2048;
    let phases = if exact { l as usize } else { FALLBACK_PHASES };

    // Grenzfrequenz relativ zur Eingangs-Nyquist-Frequenz; beim Herunterrechnen unter der neuen.
    let cutoff = (to as f64 / from as f64).min(1.0) * 0.94;
    let half = (ZERO_CROSSINGS / cutoff).ceil() as usize;
    let taps = 2 * half;
    let mut table = vec![0.0f32; phases * taps];
    for p in 0..phases {
        let frac = p as f64 / phases as f64;
        let row = &mut table[p * taps..(p + 1) * taps];
        let mut sum = 0.0f64;
        let coeffs: Vec<f64> = (0..taps)
            .map(|j| {
                // Abstand des Eingangs-Samples von der Ausgabeposition.
                let t = j as f64 - half as f64 + 1.0 - frac;
                let x = std::f64::consts::PI * cutoff * t;
                let sinc = if x.abs() < 1e-12 { 1.0 } else { x.sin() / x };
                let u = (t / half as f64).clamp(-1.0, 1.0) * std::f64::consts::PI;
                let window = 0.42 + 0.5 * u.cos() + 0.08 * (2.0 * u).cos();
                let c = sinc * window;
                sum += c;
                c
            })
            .collect();
        // Gleichanteil je Phase auf 1 normieren.
        for (r, c) in row.iter_mut().zip(coeffs) {
            *r = (c / sum) as f32;
        }
    }

    let out_len = (input.len() as u64 * to as u64 / from as u64) as usize;
    let mut out = vec![0.0f32; out_len];
    let ratio = from as f64 / to as f64;
    const CHUNK: usize = 16_384;
    out.par_chunks_mut(CHUNK).enumerate().for_each(|(ci, chunk)| {
        for (k, o) in chunk.iter_mut().enumerate() {
            let n = (ci * CHUNK + k) as u64;
            let (i0, p) = if exact {
                let num = n * m;
                ((num / l) as i64, (num % l) as usize)
            } else {
                let x = n as f64 * ratio;
                let i0 = x.floor();
                let p = ((x - i0) * phases as f64).round() as usize;
                if p == phases {
                    (i0 as i64 + 1, 0)
                } else {
                    (i0 as i64, p)
                }
            };
            let row = &table[p * taps..(p + 1) * taps];
            let first = i0 - half as i64 + 1;
            let mut acc = 0.0f32;
            if first >= 0 && (first as usize + taps) <= input.len() {
                let src = &input[first as usize..first as usize + taps];
                for (c, s) in row.iter().zip(src) {
                    acc += c * s;
                }
            } else {
                // Ränder: außerhalb liegt Stille.
                for (j, c) in row.iter().enumerate() {
                    let idx = first + j as i64;
                    if idx >= 0 && (idx as usize) < input.len() {
                        acc += c * input[idx as usize];
                    }
                }
            }
            *o = acc;
        }
    });
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn resampling_keeps_level_and_time_axis() {
        // 1-kHz-Sinus und ein Impuls bei 0,5 s, 48 kHz → 44,1 kHz.
        let from = 48_000u32;
        let mut input: Vec<f32> = (0..from as usize)
            .map(|n| 0.5 * (std::f64::consts::TAU * 1000.0 * n as f64 / from as f64).sin() as f32)
            .collect();
        let out = resample(&input, from, ANALYSIS_RATE);
        assert_eq!(out.len(), 44_100);
        for n in (2000..42_000).step_by(37) {
            let expected = 0.5 * (std::f64::consts::TAU * 1000.0 * n as f64 / 44_100.0).sin();
            assert!((out[n] as f64 - expected).abs() < 1e-3, "Sample {n}: {} statt {expected}", out[n]);
        }

        input.iter_mut().for_each(|s| *s = 0.0);
        input[24_000] = 1.0;
        let out = resample(&input, from, ANALYSIS_RATE);
        let peak = out.iter().enumerate().max_by(|a, b| a.1.abs().total_cmp(&b.1.abs())).unwrap().0;
        assert_eq!(peak, 22_050);
    }

    #[test]
    fn odd_rates_use_the_fallback_table() {
        let from = 47_999u32;
        let input: Vec<f32> = (0..from as usize)
            .map(|n| 0.5 * (std::f64::consts::TAU * 440.0 * n as f64 / from as f64).sin() as f32)
            .collect();
        let out = resample(&input, from, ANALYSIS_RATE);
        for n in (2000..42_000).step_by(41) {
            let expected = 0.5 * (std::f64::consts::TAU * 440.0 * n as f64 / 44_100.0).sin();
            assert!((out[n] as f64 - expected).abs() < 2e-3);
        }
    }
}
