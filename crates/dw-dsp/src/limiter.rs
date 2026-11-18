//! Vorausschauender Limiter für die Master-Summe (MIX-08). Stereo gekoppelt.
//! Die Verzögerung beträgt `lookahead()` Samples.
//!
//! Gemessen wird der True Peak: Das Signal wird für die Pegelerkennung vierfach
//! überabgetastet, damit auch Spitzen zwischen zwei Samples erfasst werden.

use dw_core::db_to_lin;
use std::f64::consts::PI;

/// Stützstellen je Phase des Interpolationsfilters der Pegelerkennung.
const TP_TAPS: usize = 12;
const TP_PHASES: usize = 4;

fn bessel_i0(x: f64) -> f64 {
    let mut sum = 1.0;
    let mut term = 1.0;
    let q = x * x / 4.0;
    for k in 1..40 {
        term *= q / (k * k) as f64;
        sum += term;
    }
    sum
}

/// Koeffizienten für die Zwischenwerte bei 1/4, 2/4 und 3/4 zwischen zwei Samples.
fn true_peak_filter() -> [[f32; TP_TAPS]; TP_PHASES - 1] {
    let mut out = [[0.0f32; TP_TAPS]; TP_PHASES - 1];
    let beta = 8.0;
    let half = TP_TAPS as f64 / 2.0;
    for (p, row) in out.iter_mut().enumerate() {
        let frac = (p + 1) as f64 / TP_PHASES as f64;
        let mut tmp = [0.0f64; TP_TAPS];
        let mut sum = 0.0;
        for (k, t) in tmp.iter_mut().enumerate() {
            let x = k as f64 - (half - 1.0) - frac;
            let sinc = if x.abs() < 1e-12 { 1.0 } else { (PI * x).sin() / (PI * x) };
            let w = x / half;
            let window = if w.abs() >= 1.0 { 0.0 } else { bessel_i0(beta * (1.0 - w * w).sqrt()) / bessel_i0(beta) };
            *t = sinc * window;
            sum += *t;
        }
        for (r, t) in row.iter_mut().zip(tmp) {
            *r = (t / sum) as f32;
        }
    }
    out
}

pub struct Limiter {
    ceiling: f32,
    look: usize,
    /// Verzögertes Signal (verschachtelt), Ringpuffer mit `look` Frames.
    delay: Vec<f32>,
    /// Benötigter Gain der letzten `look + 1` Frames.
    need: Vec<f32>,
    /// Geglätteter Gain der letzten `look` Frames für den gleitenden Mittelwert.
    hist: Vec<f32>,
    hist_sum: f64,
    idx: usize,
    need_idx: usize,
    release: f32,
    release_coef: f32,
    /// Kleinster Gain seit dem letzten Abruf.
    min_gain: f32,
    /// Letzte Samples je Kanal für die True-Peak-Erkennung (doppelt abgelegt, damit das Fenster nie umbricht).
    tp_hist: [[f32; TP_TAPS * 2]; 2],
    tp_pos: usize,
    tp_filter: [[f32; TP_TAPS]; TP_PHASES - 1],
    pub enabled: bool,
}

impl Limiter {
    pub fn new(sample_rate: f64, ceiling_db: f32) -> Self {
        // Die Vorschau muss länger sein als die Laufzeit des Erkennungsfilters.
        let look = ((sample_rate * 0.0015).round() as usize).max(TP_TAPS * 2);
        Limiter {
            ceiling: db_to_lin(ceiling_db),
            look,
            delay: vec![0.0; look * 2],
            need: vec![1.0; look + 1],
            hist: vec![1.0; look],
            hist_sum: look as f64,
            idx: 0,
            need_idx: 0,
            release: 1.0,
            // Rückkehr auf 63 % in 80 ms
            release_coef: 1.0 - (-1.0 / (0.080 * sample_rate as f32)).exp(),
            min_gain: 1.0,
            tp_hist: [[0.0; TP_TAPS * 2]; 2],
            tp_pos: 0,
            tp_filter: true_peak_filter(),
            enabled: true,
        }
    }

    pub fn lookahead(&self) -> usize {
        self.look
    }

    /// Kleinster angewandter Gain seit dem letzten Aufruf (1 = keine Reduktion).
    pub fn take_min_gain(&mut self) -> f32 {
        std::mem::replace(&mut self.min_gain, 1.0)
    }

    /// Verarbeitet verschachtelte Stereo-Samples an Ort und Stelle.
    pub fn process(&mut self, buf: &mut [f32]) {
        for frame in buf.chunks_exact_mut(2) {
            // True Peak: das Sample selbst und drei Zwischenwerte je Kanal.
            let mut peak = frame[0].abs().max(frame[1].abs());
            for c in 0..2 {
                self.tp_hist[c][self.tp_pos] = frame[c];
                self.tp_hist[c][self.tp_pos + TP_TAPS] = frame[c];
            }
            self.tp_pos = (self.tp_pos + 1) % TP_TAPS;
            for c in 0..2 {
                let window = &self.tp_hist[c][self.tp_pos..self.tp_pos + TP_TAPS];
                for row in &self.tp_filter {
                    let mut v = 0.0f32;
                    for k in 0..TP_TAPS {
                        v += window[k] * row[k];
                    }
                    peak = peak.max(v.abs());
                }
            }
            let need = if self.enabled && peak > self.ceiling { self.ceiling / peak } else { 1.0 };
            self.need[self.need_idx] = need;
            self.need_idx = (self.need_idx + 1) % self.need.len();
            // Minimum über das Vorschau-Fenster
            let mut m = 1.0f32;
            for &n in &self.need {
                m = m.min(n);
            }
            // Langsame Rückkehr nach oben, sofort nach unten
            self.release = m.min(self.release + (1.0 - self.release) * self.release_coef);
            // Gleitender Mittelwert über `look` Frames: lineare Rampe, die rechtzeitig unten ankommt
            let h = self.idx % self.look;
            self.hist_sum += self.release as f64 - self.hist[h] as f64;
            self.hist[h] = self.release;
            let gain = (self.hist_sum / self.look as f64) as f32;

            let d = (self.idx % self.look) * 2;
            let out = [self.delay[d] * gain, self.delay[d + 1] * gain];
            self.delay[d] = frame[0];
            self.delay[d + 1] = frame[1];
            self.idx += 1;
            if self.idx % (self.look * 4096) == 0 {
                // Summe gelegentlich neu bilden, damit sich Rundungsfehler nicht aufbauen
                self.hist_sum = self.hist.iter().map(|&v| v as f64).sum();
            }
            self.min_gain = self.min_gain.min(gain);
            if self.enabled {
                frame[0] = out[0].clamp(-self.ceiling, self.ceiling);
                frame[1] = out[1].clamp(-self.ceiling, self.ceiling);
            } else {
                frame[0] = out[0];
                frame[1] = out[1];
            }
        }
    }
}
