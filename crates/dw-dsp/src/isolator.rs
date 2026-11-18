//! 3-Band-Isolator-EQ (MIX-02, 5.14): Frequenzweiche aus Linkwitz-Riley-Filtern
//! 4. Ordnung bei 200 Hz und 2,5 kHz. In Mittelstellung ist die Summe ein Allpass,
//! der Betragsfrequenzgang also eben.

use crate::biquad::{Biquad, Coeffs};
use crate::smooth::Smoother;

pub const XOVER_LOW_HZ: f64 = 200.0;
pub const XOVER_HIGH_HZ: f64 = 2500.0;
const BUTTERWORTH_Q: f64 = std::f64::consts::FRAC_1_SQRT_2;

/// Linkwitz-Riley 4. Ordnung = zwei Butterworth-Filter 2. Ordnung in Reihe.
#[derive(Clone, Copy)]
struct Lr4([Biquad; 2]);

impl Lr4 {
    fn lowpass(sr: f64, f: f64) -> Self {
        Lr4([Biquad::new(Coeffs::lowpass(sr, f, BUTTERWORTH_Q)); 2])
    }
    fn highpass(sr: f64, f: f64) -> Self {
        Lr4([Biquad::new(Coeffs::highpass(sr, f, BUTTERWORTH_Q)); 2])
    }
    #[inline]
    fn process(&mut self, x: f64) -> f64 {
        let y = self.0[0].process(x);
        self.0[1].process(y)
    }
}

#[derive(Clone, Copy)]
struct Channel {
    lp_low: Lr4,
    hp_low: Lr4,
    // Allpass auf dem Tief-Zweig (Summe aus Tief- und Hochpass der oberen Weiche),
    // damit alle drei Zweige dieselbe Phase haben.
    ap_lp: Lr4,
    ap_hp: Lr4,
    lp_high: Lr4,
    hp_high: Lr4,
}

impl Channel {
    fn new(sr: f64) -> Self {
        Channel {
            lp_low: Lr4::lowpass(sr, XOVER_LOW_HZ),
            hp_low: Lr4::highpass(sr, XOVER_LOW_HZ),
            ap_lp: Lr4::lowpass(sr, XOVER_HIGH_HZ),
            ap_hp: Lr4::highpass(sr, XOVER_HIGH_HZ),
            lp_high: Lr4::lowpass(sr, XOVER_HIGH_HZ),
            hp_high: Lr4::highpass(sr, XOVER_HIGH_HZ),
        }
    }

    #[inline]
    fn split(&mut self, x: f64) -> [f64; 3] {
        let low_raw = self.lp_low.process(x);
        let rest = self.hp_low.process(x);
        let low = self.ap_lp.process(low_raw) + self.ap_hp.process(low_raw);
        let mid = self.lp_high.process(rest);
        let high = self.hp_high.process(rest);
        [low, mid, high]
    }
}

/// Stereo-Isolator mit geglätteten Band-Gains (linear; 0 = Kill).
pub struct Isolator {
    ch: [Channel; 2],
    gains: [Smoother; 3],
}

impl Isolator {
    pub fn new(sample_rate: f64) -> Self {
        Isolator {
            ch: [Channel::new(sample_rate); 2],
            gains: [
                Smoother::new(1.0, sample_rate, 5.0),
                Smoother::new(1.0, sample_rate, 5.0),
                Smoother::new(1.0, sample_rate, 5.0),
            ],
        }
    }

    /// `band`: 0 = Tief, 1 = Mitte, 2 = Hoch. `gain` linear, 0 … 2 (+6 dB).
    pub fn set_gain(&mut self, band: usize, gain: f32) {
        self.gains[band].set(gain.clamp(0.0, 2.0));
    }

    /// Verarbeitet verschachtelte Stereo-Samples an Ort und Stelle.
    pub fn process(&mut self, buf: &mut [f32]) {
        for frame in buf.chunks_exact_mut(2) {
            let g = [
                self.gains[0].next() as f64,
                self.gains[1].next() as f64,
                self.gains[2].next() as f64,
            ];
            for (c, s) in frame.iter_mut().enumerate() {
                let b = self.ch[c].split(*s as f64);
                *s = (b[0] * g[0] + b[1] * g[1] + b[2] * g[2]) as f32;
            }
        }
    }
}
