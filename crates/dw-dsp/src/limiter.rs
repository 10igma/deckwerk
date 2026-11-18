//! Vorausschauender Limiter für die Master-Summe (MIX-08). Stereo gekoppelt.
//! Die Verzögerung beträgt `lookahead()` Samples.

use dw_core::db_to_lin;

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
    pub enabled: bool,
}

impl Limiter {
    pub fn new(sample_rate: f64, ceiling_db: f32) -> Self {
        let look = ((sample_rate * 0.0015).round() as usize).max(8);
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
            let peak = frame[0].abs().max(frame[1].abs());
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
