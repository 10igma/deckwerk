//! Abtastratenwandlung beim Laden: gefensterter Sinc-Filter mit 128 Stützstellen
//! (bei Abwärtswandlung entsprechend mehr), auf mehrere Threads verteilt.

use std::f64::consts::PI;

const HALF_TAPS: usize = 64;
const PHASES: usize = 1024;
const CUTOFF: f64 = 0.95;
const KAISER_BETA: f64 = 10.0;

fn bessel_i0(x: f64) -> f64 {
    let mut sum = 1.0;
    let mut term = 1.0;
    let q = x * x / 4.0;
    for k in 1..60 {
        term *= q / (k * k) as f64;
        sum += term;
        if term < 1e-14 * sum {
            break;
        }
    }
    sum
}

/// Wandelt verschachtelte Stereo-Daten von `src_rate` nach `dst_rate`.
/// Die Zeitlage bleibt erhalten: Frame 0 der Ausgabe entspricht Frame 0 der Eingabe.
pub fn resample_stereo(data: &[f32], src_rate: u32, dst_rate: u32) -> Vec<f32> {
    if src_rate == dst_rate {
        return data.to_vec();
    }
    let frames_in = data.len() / 2;
    let ratio = dst_rate as f64 / src_rate as f64;
    let frames_out = (frames_in as f64 * ratio).round() as usize;
    // Beim Abwärtswandeln liegt die Grenzfrequenz an der Ziel-Nyquistfrequenz; der Kern wird entsprechend breiter.
    let scale = ratio.min(1.0);
    let half = (HALF_TAPS as f64 / scale).ceil() as usize;
    let taps = half * 2;
    let fc = CUTOFF * scale;

    let i0_beta = bessel_i0(KAISER_BETA);
    let mut table = vec![0.0f32; (PHASES + 1) * taps];
    for p in 0..=PHASES {
        let frac = p as f64 / PHASES as f64;
        let row = &mut table[p * taps..(p + 1) * taps];
        let mut sum = 0.0f64;
        let mut tmp = vec![0.0f64; taps];
        for (k, t) in tmp.iter_mut().enumerate() {
            let x = k as f64 - (half as f64 - 1.0) - frac;
            let sinc = if x.abs() < 1e-12 { fc } else { (PI * fc * x).sin() / (PI * x) };
            let w = x / half as f64;
            let window = if w.abs() >= 1.0 { 0.0 } else { bessel_i0(KAISER_BETA * (1.0 - w * w).sqrt()) / i0_beta };
            *t = sinc * window;
            sum += *t;
        }
        for (r, t) in row.iter_mut().zip(&tmp) {
            *r = (t / sum) as f32;
        }
    }

    let mut out = vec![0.0f32; frames_out * 2];
    let step = 1.0 / ratio;
    let threads = std::thread::available_parallelism().map_or(1, |n| n.get()).min(8);
    let chunk_frames = frames_out.div_ceil(threads).max(1);
    let table = &table;
    std::thread::scope(|scope| {
        for (c, chunk) in out.chunks_mut(chunk_frames * 2).enumerate() {
            scope.spawn(move || {
                let first = c * chunk_frames;
                for (j, frame) in chunk.chunks_exact_mut(2).enumerate() {
                    let pos = (first + j) as f64 * step;
                    let base = pos.floor();
                    let ph = (pos - base) * PHASES as f64;
                    let p0 = ph as usize;
                    let w1 = (ph - p0 as f64) as f32;
                    let w0 = 1.0 - w1;
                    let row0 = &table[p0 * taps..(p0 + 1) * taps];
                    let row1 = &table[(p0 + 1) * taps..(p0 + 2) * taps];
                    let start = base as isize - (half as isize - 1);
                    let (mut l, mut r) = (0.0f32, 0.0f32);
                    if start >= 0 && start as usize + taps <= frames_in {
                        let s = &data[start as usize * 2..(start as usize + taps) * 2];
                        for k in 0..taps {
                            let coeff = row0[k] * w0 + row1[k] * w1;
                            l += s[k * 2] * coeff;
                            r += s[k * 2 + 1] * coeff;
                        }
                    } else {
                        for k in 0..taps {
                            let idx = start + k as isize;
                            if idx >= 0 && (idx as usize) < frames_in {
                                let coeff = row0[k] * w0 + row1[k] * w1;
                                l += data[idx as usize * 2] * coeff;
                                r += data[idx as usize * 2 + 1] * coeff;
                            }
                        }
                    }
                    frame[0] = l;
                    frame[1] = r;
                }
            });
        }
    });
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sine(sr: u32, hz: f64, frames: usize) -> Vec<f32> {
        (0..frames)
            .flat_map(|i| {
                let s = 0.5 * (2.0 * PI * hz * i as f64 / sr as f64).sin() as f32;
                [s, s]
            })
            .collect()
    }

    /// Restpegel gegenüber einem idealen Sinus in dB.
    fn residual_db(x: &[f32], sr: u32, hz: f64) -> f64 {
        let v: Vec<f64> = x.chunks_exact(2).map(|f| f[0] as f64).collect();
        let v = &v[2000..v.len() - 2000];
        // Kleinste-Quadrate-Anpassung von a·sin + b·cos (2×2-Normalgleichungen).
        let (mut ss, mut cc, mut sc, mut ys, mut yc) = (0.0, 0.0, 0.0, 0.0, 0.0);
        for (i, &s) in v.iter().enumerate() {
            let ph = 2.0 * PI * hz * i as f64 / sr as f64;
            let (sn, cs) = ph.sin_cos();
            ss += sn * sn;
            cc += cs * cs;
            sc += sn * cs;
            ys += s * sn;
            yc += s * cs;
        }
        let det = ss * cc - sc * sc;
        let (a, b) = ((ys * cc - yc * sc) / det, (yc * ss - ys * sc) / det);
        let mut err = 0.0;
        let mut sig = 0.0;
        for (i, &s) in v.iter().enumerate() {
            let ph = 2.0 * PI * hz * i as f64 / sr as f64;
            let fit = a * ph.sin() + b * ph.cos();
            err += (s - fit) * (s - fit);
            sig += fit * fit;
        }
        // Pegel muss erhalten bleiben
        assert!(((a * a + b * b).sqrt() - 0.5).abs() < 0.003, "Pegel {}", (a * a + b * b).sqrt());
        10.0 * (err / sig).log10()
    }

    #[test]
    fn up_and_down_conversion_is_clean() {
        for (src, dst) in [(44_100, 48_000), (48_000, 44_100), (96_000, 48_000), (22_050, 48_000)] {
            for hz in [100.0, 1000.0, 9000.0] {
                let out = resample_stereo(&sine(src, hz, src as usize), src, dst);
                assert_eq!(out.len() / 2, dst as usize);
                let res = residual_db(&out, dst, hz);
                assert!(res < -80.0, "{src} → {dst}, {hz} Hz: Rest {res:.1} dB");
            }
        }
    }

    #[test]
    fn downsampling_removes_content_above_new_nyquist() {
        // 30 kHz bei 96 kHz würde bei 48 kHz als 18 kHz erscheinen.
        let out = resample_stereo(&sine(96_000, 30_000.0, 96_000), 96_000, 48_000);
        let peak = out[4000..out.len() - 4000].iter().fold(0.0f32, |m, s| m.max(s.abs()));
        assert!(peak < 0.5 * 10f32.powf(-80.0 / 20.0), "Alias-Pegel {peak}");
    }

    #[test]
    fn timing_is_preserved() {
        // Impuls bei 1,0 s bleibt bei 1,0 s.
        let mut src = vec![0.0f32; 44_100 * 4];
        src[44_100 * 2] = 1.0;
        src[44_100 * 2 + 1] = 1.0;
        let out = resample_stereo(&src, 44_100, 48_000);
        let (idx, _) = out.chunks_exact(2).enumerate().fold((0, 0.0f32), |m, (i, f)| if f[0].abs() > m.1 { (i, f[0].abs()) } else { m });
        assert_eq!(idx, 48_000);
    }
}
