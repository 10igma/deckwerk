//! Wellenform-Daten in drei Bändern (Spezifikation 5.5).

use dw_core::{AudioBuf, Waveform, WaveformLevel};
use rayon::prelude::*;

/// Trennfrequenzen der Anzeige-Bänder in Hz.
const CROSSOVER_LOW: f64 = 200.0;
const CROSSOVER_HIGH: f64 = 2500.0;
/// Punkte der festen Übersicht.
const OVERVIEW_POINTS: usize = 2000;
/// Feinste Stufe der Pyramide; jede weitere fasst vier Punkte zusammen.
const BASE_SAMPLES: usize = 64;
const LEVELS: usize = 4;

/// Butterworth-Filter 2. Ordnung (transponierte Direktform II).
#[derive(Clone, Copy)]
struct Biquad {
    b: [f64; 3],
    a: [f64; 2],
    z: [f64; 2],
}

impl Biquad {
    fn new(highpass: bool, freq: f64, sample_rate: f64) -> Self {
        // Über der halben Abtastrate gibt es nichts zu trennen.
        let w = std::f64::consts::TAU * freq.min(0.45 * sample_rate) / sample_rate;
        let (sin, cos) = w.sin_cos();
        let alpha = sin / std::f64::consts::SQRT_2;
        let a0 = 1.0 + alpha;
        let b = if highpass {
            [(1.0 + cos) / 2.0 / a0, -(1.0 + cos) / a0, (1.0 + cos) / 2.0 / a0]
        } else {
            [(1.0 - cos) / 2.0 / a0, (1.0 - cos) / a0, (1.0 - cos) / 2.0 / a0]
        };
        Biquad { b, a: [-2.0 * cos / a0, (1.0 - alpha) / a0], z: [0.0; 2] }
    }

    #[inline]
    fn process(&mut self, x: f64) -> f64 {
        let y = self.b[0] * x + self.z[0];
        self.z[0] = self.b[1] * x - self.a[0] * y + self.z[1];
        self.z[1] = self.b[2] * x - self.a[1] * y;
        y
    }
}

/// Minimum, Maximum und Quadratsumme eines Punkts.
#[derive(Clone, Copy)]
struct Acc {
    min: f32,
    max: f32,
    sum_sq: f64,
    count: u32,
}

impl Acc {
    const EMPTY: Acc = Acc { min: f32::MAX, max: f32::MIN, sum_sq: 0.0, count: 0 };

    #[inline]
    fn push(&mut self, v: f32) {
        self.min = self.min.min(v);
        self.max = self.max.max(v);
        self.sum_sq += (v as f64) * (v as f64);
        self.count += 1;
    }

    fn merge(&mut self, other: &Acc) {
        self.min = self.min.min(other.min);
        self.max = self.max.max(other.max);
        self.sum_sq += other.sum_sq;
        self.count += other.count;
    }

    /// (min, max, rms); ein leerer Punkt ergibt Nullen.
    fn values(&self) -> [f32; 3] {
        if self.count == 0 {
            [0.0; 3]
        } else {
            [self.min, self.max, (self.sum_sq / self.count as f64).sqrt() as f32]
        }
    }
}

/// Berechnet die Wellenform-Daten aus der Mono-Mischung bei der Abtastrate des Tracks.
///
/// Bänder: Linkwitz-Riley-Weichen 4. Ordnung bei 200 Hz und 2,5 kHz. Die Übersicht hat
/// immer 2 000 Punkte; ihr `samples_per_point` ist der abgerundete Mittelwert.
pub fn waveform(audio: &AudioBuf) -> Waveform {
    let mono = audio.to_mono();
    let frames = mono.len();
    let sr = audio.sample_rate.max(1) as f64;
    let base_points = frames.div_ceil(BASE_SAMPLES);

    // Je Band: Filterkette, feinste Stufe und Übersicht in einem Durchlauf.
    let bands: Vec<(Vec<Acc>, Vec<Acc>)> = (0..3)
        .into_par_iter()
        .map(|band| {
            let lp = |f| Biquad::new(false, f, sr);
            let hp = |f| Biquad::new(true, f, sr);
            let mut chain: Vec<Biquad> = match band {
                0 => vec![lp(CROSSOVER_LOW), lp(CROSSOVER_LOW)],
                1 => vec![hp(CROSSOVER_LOW), hp(CROSSOVER_LOW), lp(CROSSOVER_HIGH), lp(CROSSOVER_HIGH)],
                _ => vec![hp(CROSSOVER_HIGH), hp(CROSSOVER_HIGH)],
            };
            let mut base = vec![Acc::EMPTY; base_points];
            let mut overview = vec![Acc::EMPTY; OVERVIEW_POINTS];
            for (n, &x) in mono.iter().enumerate() {
                let mut y = x as f64;
                for stage in chain.iter_mut() {
                    y = stage.process(y);
                }
                let y = y as f32;
                base[n / BASE_SAMPLES].push(y);
                overview[(n as u64 * OVERVIEW_POINTS as u64 / frames as u64) as usize].push(y);
            }
            // Kurze Tracks: Lücken der Übersicht mit dem Punkt davor füllen.
            for i in 1..OVERVIEW_POINTS {
                if overview[i].count == 0 {
                    overview[i] = overview[i - 1];
                }
            }
            (base, overview)
        })
        .collect();

    let interleave = |points: usize, get: &dyn Fn(usize, usize) -> [f32; 3]| {
        let mut data = Vec::with_capacity(points * WaveformLevel::VALUES_PER_POINT);
        for p in 0..points {
            for band in 0..3 {
                data.extend_from_slice(&get(band, p));
            }
        }
        data
    };

    let mut levels = Vec::with_capacity(LEVELS);
    let mut current: Vec<Vec<Acc>> = bands.iter().map(|b| b.0.clone()).collect();
    let mut samples = BASE_SAMPLES;
    for level in 0..LEVELS {
        if level > 0 {
            samples *= 4;
            current = current
                .iter()
                .map(|band| {
                    band.chunks(4)
                        .map(|group| {
                            let mut acc = Acc::EMPTY;
                            group.iter().for_each(|g| acc.merge(g));
                            acc
                        })
                        .collect()
                })
                .collect();
        }
        let points = current[0].len();
        levels.push(WaveformLevel {
            samples_per_point: samples as u32,
            data: interleave(points, &|band, p| current[band][p].values()),
        });
    }

    let overview = WaveformLevel {
        samples_per_point: (frames / OVERVIEW_POINTS).max(1) as u32,
        data: interleave(OVERVIEW_POINTS, &|band, p| bands[band].1[p].values()),
    };
    Waveform { sample_rate: audio.sample_rate, frames: frames as u64, overview, levels }
}

#[cfg(test)]
mod tests {
    use super::*;
    use dw_testgen::sine;

    /// RMS-Werte (Tief, Mitte, Hoch) eines Punkts.
    fn rms(level: &WaveformLevel, point: usize) -> [f32; 3] {
        let p = &level.data[point * WaveformLevel::VALUES_PER_POINT..];
        [p[2], p[5], p[8]]
    }

    #[test]
    fn pyramid_has_the_specified_resolutions() {
        let audio = sine(44_100, 440.0, 3.0, 0.5);
        let w = waveform(&audio);
        assert_eq!(w.sample_rate, 44_100);
        assert_eq!(w.frames, 132_300);
        let per_point: Vec<u32> = w.levels.iter().map(|l| l.samples_per_point).collect();
        assert_eq!(per_point, vec![64, 256, 1024, 4096]);
        for level in &w.levels {
            assert_eq!(level.points(), 132_300usize.div_ceil(level.samples_per_point as usize));
            assert_eq!(level.data.len(), level.points() * 9);
        }
        assert_eq!(w.overview.points(), 2000);
        assert_eq!(w.overview.samples_per_point, 66);
    }

    #[test]
    fn bands_separate_low_mid_and_high() {
        for (freq, band) in [(60.0, 0), (800.0, 1), (9000.0, 2)] {
            let w = waveform(&sine(48_000, freq, 2.0, 0.8));
            let level = &w.levels[2];
            let values = rms(level, level.points() / 2);
            for other in 0..3 {
                if other == band {
                    // Sinus mit Amplitude 0,8: Effektivwert 0,566.
                    assert!((values[other] - 0.566).abs() < 0.02, "{freq} Hz: Band {other} = {}", values[other]);
                } else {
                    assert!(values[other] < 0.03, "{freq} Hz: Band {other} = {}", values[other]);
                }
            }
            let p = &level.data[(level.points() / 2) * 9 + band * 3..];
            assert!(p[0] < -0.75 && p[0] >= -0.82 && p[1] > 0.75 && p[1] <= 0.82, "min {} max {}", p[0], p[1]);
        }
    }

    #[test]
    fn coarser_levels_summarize_finer_ones() {
        let (audio, _) = dw_testgen::techno_track(&dw_testgen::TrackSpec::new(
            130.0,
            dw_core::Key::new(0, true),
            vec![dw_testgen::SectionSpec::new(dw_testgen::SectionKind::Drop, 4)],
            1,
        ));
        let w = waveform(&audio);
        let (fine, coarse) = (&w.levels[0], &w.levels[1]);
        for point in (0..coarse.points() - 1).step_by(17) {
            for band in 0..3 {
                let c = &coarse.data[point * 9 + band * 3..point * 9 + band * 3 + 3];
                let group: Vec<&[f32]> =
                    (0..4).map(|i| &fine.data[(point * 4 + i) * 9 + band * 3..(point * 4 + i) * 9 + band * 3 + 3]).collect();
                let min = group.iter().map(|g| g[0]).fold(f32::MAX, f32::min);
                let max = group.iter().map(|g| g[1]).fold(f32::MIN, f32::max);
                let rms = (group.iter().map(|g| g[2] * g[2]).sum::<f32>() / 4.0).sqrt();
                assert_eq!(c[0], min);
                assert_eq!(c[1], max);
                assert!((c[2] - rms).abs() < 1e-5);
                assert!(c[0] <= c[1] && c[2] >= 0.0 && c[2] <= c[0].abs().max(c[1].abs()) + 1e-6);
            }
        }
        // Die Kick dominiert das Tief-Band, die Hi-Hats nur das Hoch-Band.
        let overall = |band: usize| {
            let l = &w.levels[3];
            (0..l.points()).map(|p| l.data[p * 9 + band * 3 + 2]).fold(0.0f32, f32::max)
        };
        assert!(overall(0) > 3.0 * overall(2));
        assert!(overall(2) > 0.005);
    }
}
