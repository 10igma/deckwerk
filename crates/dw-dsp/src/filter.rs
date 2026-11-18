//! Filter auf einem Regler (MIX-04, 5.14): Mitte = aus (bitgenau), links Tiefpass,
//! rechts Hochpass. Zustandsvariables Filter 12 dB/Oktave.

use crate::smooth::Smoother;
use std::f64::consts::PI;

const DEAD_ZONE: f32 = 0.02;
const COEFF_INTERVAL: u32 = 16;

#[derive(Clone, Copy, PartialEq)]
enum Mode {
    Low,
    High,
}

/// Grenzfrequenz in Hz für eine Reglerstellung (ohne Ruhezone).
pub fn cutoff_hz(pos: f32) -> f64 {
    let p = pos.abs().min(1.0) as f64;
    if pos < 0.0 {
        20_000.0 * (80.0f64 / 20_000.0).powf(p)
    } else {
        20.0 * (8_000.0f64 / 20.0).powf(p)
    }
}

pub struct FilterKnob {
    sr: f64,
    pos: Smoother,
    mix: Smoother,
    mode: Mode,
    q: f64,
    // Koeffizienten des zustandsvariablen Filters
    a1: f64,
    a2: f64,
    a3: f64,
    k: f64,
    ic1: [f64; 2],
    ic2: [f64; 2],
    counter: u32,
}

impl FilterKnob {
    pub fn new(sample_rate: f64) -> Self {
        let mut f = FilterKnob {
            sr: sample_rate,
            pos: Smoother::new(0.0, sample_rate, 5.0),
            mix: Smoother::new(0.0, sample_rate, 5.0),
            mode: Mode::Low,
            q: 1.2,
            a1: 0.0,
            a2: 0.0,
            a3: 0.0,
            k: 0.0,
            ic1: [0.0; 2],
            ic2: [0.0; 2],
            counter: 0,
        };
        f.update_coeffs(0.5);
        f
    }

    /// Reglerstellung −1 … +1.
    pub fn set_pos(&mut self, pos: f32) {
        self.pos.set(pos.clamp(-1.0, 1.0));
    }

    /// Güte 0,7 … 4.
    pub fn set_resonance(&mut self, q: f64) {
        self.q = q.clamp(0.7, 4.0);
    }

    fn update_coeffs(&mut self, pos_abs: f32) {
        let signed = if self.mode == Mode::Low { -pos_abs } else { pos_abs };
        let fc = cutoff_hz(signed).min(0.45 * self.sr);
        let g = (PI * fc / self.sr).tan();
        self.k = 1.0 / self.q;
        self.a1 = 1.0 / (1.0 + g * (g + self.k));
        self.a2 = g * self.a1;
        self.a3 = g * self.a2;
    }

    /// Verarbeitet verschachtelte Stereo-Samples an Ort und Stelle.
    pub fn process(&mut self, buf: &mut [f32]) {
        // Vollständig aus und eingeschwungen: Signal bleibt unberührt.
        if self.pos.is_settled() && self.pos.value().abs() < DEAD_ZONE && self.mix.is_settled() && self.mix.value() == 0.0 {
            return;
        }
        for frame in buf.chunks_exact_mut(2) {
            let pos = self.pos.next();
            let wanted = if pos < 0.0 { Mode::Low } else { Mode::High };
            let active = pos.abs() >= DEAD_ZONE;
            if active && wanted == self.mode {
                self.mix.set(1.0);
            } else {
                // Ausblenden; ein Wechsel der Filterart geschieht erst bei Mix 0.
                self.mix.set(0.0);
                if self.mix.value() == 0.0 && self.mix.is_settled() && wanted != self.mode {
                    self.mode = wanted;
                    self.ic1 = [0.0; 2];
                    self.ic2 = [0.0; 2];
                    self.counter = 0;
                }
            }
            if self.counter == 0 {
                self.update_coeffs(pos.abs().max(DEAD_ZONE));
            }
            self.counter = (self.counter + 1) % COEFF_INTERVAL;
            let mix = self.mix.next() as f64;
            for (c, s) in frame.iter_mut().enumerate() {
                let x = *s as f64;
                let v3 = x - self.ic2[c];
                let v1 = self.a1 * self.ic1[c] + self.a2 * v3;
                let v2 = self.ic2[c] + self.a2 * self.ic1[c] + self.a3 * v3;
                self.ic1[c] = 2.0 * v1 - self.ic1[c];
                self.ic2[c] = 2.0 * v2 - self.ic2[c];
                let out = match self.mode {
                    Mode::Low => v2,
                    Mode::High => x - self.k * v1 - v2,
                };
                if mix > 0.0 {
                    *s = (x + mix * (out - x)) as f32;
                }
            }
        }
    }
}
