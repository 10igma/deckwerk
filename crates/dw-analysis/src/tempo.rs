//! Tempo und Beat-Phase (Spezifikation 5.2, Schritte 1–4 und 7).

use crate::preprocess::ANALYSIS_RATE;
use crate::spectrum::{BandFeatures, BANDS, HOP};
use crate::AnalysisConfig;
use realfft::RealFftPlanner;

/// Suchbereich der Tempo-Schätzung (Schritt 2), wird bei Bedarf auf den eingestellten Bereich erweitert.
const SCAN_MIN_BPM: f64 = 60.0;
const SCAN_MAX_BPM: f64 = 200.0;
const SCAN_STEP_BPM: f64 = 0.05;
/// Mitte der Glockenkurve; Breite 1 Oktave.
const PRIOR_CENTER_BPM: f64 = 135.0;
/// Oktav-Korrektur: Mindestanteil der Bewertung des Kandidaten im Bereich.
const OCTAVE_MIN_SHARE: f64 = 0.6;
/// Kandidaten, deren Spektrallinie schwächer ist als dieser Anteil der stärksten, werden abgewertet.
const SPECTRAL_VETO_SHARE: f64 = 0.1;
/// Unter dieser Bewertung gilt die Anschlagskurve als nicht periodisch.
const MIN_SCORE: f64 = 0.02;
/// Unter dieser Zahl verwertbarer Kick-Anschläge entfällt die Verfeinerung im Zeitbereich.
const MIN_ONSETS: usize = 8;
/// Typischer Vorlauf der Anschlagskurve vor dem Anschlag (Fensterlänge), nur für den Rückfall.
const FLUX_LEAD_SEC: f64 = 0.012;

/// Ergebnis der Tempo-Schätzung. Das Tempo wird nicht gerundet, damit das Raster
/// über die ganze Track-Länge stimmt.
#[derive(Clone, Debug)]
pub struct TempoEstimate {
    pub bpm: f64,
    /// Zeit des ersten Raster-Beats ≥ 0 in Sekunden.
    pub phase: f64,
    /// 0–1 (Schritt 7).
    pub confidence: f32,
    /// Mittlere Abweichung der Kick-Anschläge vom Raster in Millisekunden.
    pub deviation_ms: f64,
    /// Zeit des ersten Kick-Anschlags, der zum Raster passt.
    pub first_kick: Option<f64>,
}

/// Schätzt Tempo und Phase. `None`, wenn das Signal zu kurz ist oder keine Anschläge enthält.
pub fn estimate(signal: &[f32], feat: &BandFeatures, config: &AnalysisConfig) -> Option<TempoEstimate> {
    let fr = BandFeatures::frame_rate();
    let n = feat.frames();
    let scan_min = SCAN_MIN_BPM.min(config.bpm_min).max(20.0);
    let scan_max = SCAN_MAX_BPM.max(config.bpm_max).min(400.0);
    let max_lag = (4.0 * fr * 60.0 / scan_min).ceil() as usize + 2;
    if n < max_lag + 64 {
        return None;
    }

    // Schritt 1: Anschlagskurve, Tief-Band doppelt gewichtet.
    let means: [f64; BANDS] =
        std::array::from_fn(|b| feat.flux[b].iter().map(|&v| v as f64).sum::<f64>() / n as f64);
    let max_mean = means.iter().cloned().fold(0.0, f64::max);
    if max_mean < 1e-7 {
        return None;
    }
    let mut onset = vec![0.0f32; n];
    for b in 0..BANDS {
        // Fast leere Bänder nicht hochziehen.
        let w = (if b == 0 { 2.0 } else { 1.0 } / means[b].max(0.05 * max_mean)) as f32;
        for (o, &f) in onset.iter_mut().zip(&feat.flux[b]) {
            *o += w * f;
        }
    }
    let onset_mean = onset.iter().map(|&v| v as f64).sum::<f64>() / n as f64;
    let centered: Vec<f32> = onset.iter().map(|&v| v - onset_mean as f32).collect();

    // Schritt 2: Autokorrelation und Kamm-Bewertung.
    let (acf, power) = autocorrelation(&centered, max_lag);
    if acf[0] <= 0.0 {
        return None;
    }
    let lag_value = |lag: f64| {
        let i = lag.floor() as usize;
        let frac = lag - i as f64;
        if i + 1 >= acf.len() {
            0.0
        } else {
            (acf[i] * (1.0 - frac) + acf[i + 1] * frac) / acf[0]
        }
    };
    // Stärke der Spektrallinie der Anschlagskurve bei der Beat-Frequenz und ihrer Oktave.
    let fft_size = 2 * (power.len() - 1);
    let line = |hz: f64| {
        let center = hz * fft_size as f64 / fr;
        let reach = (0.015 * center).max(1.0);
        let lo = ((center - reach).floor().max(1.0) as usize).min(power.len() - 1);
        let hi = ((center + reach).ceil() as usize).min(power.len() - 1);
        power[lo..=hi].iter().cloned().fold(0.0f32, f32::max) as f64
    };
    let steps = ((scan_max - scan_min) / SCAN_STEP_BPM).round() as usize + 1;
    let bpm_at = |i: usize| scan_min + i as f64 * SCAN_STEP_BPM;
    let support: Vec<f64> = (0..steps).map(|i| line(bpm_at(i) / 60.0) + line(bpm_at(i) / 30.0)).collect();
    let max_support = support.iter().cloned().fold(0.0, f64::max);
    if max_support <= 0.0 {
        return None;
    }
    let scores: Vec<f64> = (0..steps)
        .map(|i| {
            let bpm = bpm_at(i);
            let period = fr * 60.0 / bpm;
            let comb = (lag_value(period) + lag_value(2.0 * period) + lag_value(4.0 * period)) / 3.0;
            let octaves = (bpm / PRIOR_CENTER_BPM).log2();
            // Perioden wie das 1,5-Fache des Beats passen in den Kamm, haben aber keine
            // Spektrallinie; Oktav-Verwandte bleiben unberührt.
            let veto = (support[i] / (SPECTRAL_VETO_SHARE * max_support)).min(1.0).sqrt();
            comb.max(0.0) * (-0.5 * octaves * octaves).exp() * veto
        })
        .collect();
    let best = (0..steps).max_by(|&a, &b| scores[a].total_cmp(&scores[b]))?;
    if scores[best] < MIN_SCORE {
        return None;
    }

    // Schritt 3: Oktav-Korrektur in den eingestellten Bereich.
    let in_range = |bpm: f64| bpm >= config.bpm_min && bpm <= config.bpm_max;
    let mut chosen = best;
    if !in_range(bpm_at(best)) {
        for factor in [2.0, 0.5] {
            let target = bpm_at(best) * factor;
            if !in_range(target) {
                continue;
            }
            // Stärkster Kandidat in der Nähe des halbierten bzw. verdoppelten Werts.
            let near = (0..steps)
                .filter(|&i| (bpm_at(i) / target - 1.0).abs() <= 0.025 && in_range(bpm_at(i)))
                .max_by(|&a, &b| scores[a].total_cmp(&scores[b]));
            if let Some(i) = near {
                if scores[i] >= OCTAVE_MIN_SHARE * scores[best] {
                    chosen = i;
                    break;
                }
            }
        }
    }
    let coarse_bpm = bpm_at(chosen);
    // Zweitbester Kandidat: stärkstes anderes Maximum, mindestens 4 % entfernt.
    let second = (1..steps.saturating_sub(1))
        .filter(|&i| scores[i] > scores[i - 1] && scores[i] >= scores[i + 1])
        .filter(|&i| (bpm_at(i) / coarse_bpm - 1.0).abs() > 0.04)
        .map(|i| scores[i])
        .fold(0.0, f64::max);
    let ratio = second / scores[chosen];

    // Feinsuche der Beat-Frequenz im Spektrum der Anschlagskurve (Grundton + Oktave).
    // Genau genug, damit sich das Raster über den ganzen Track nicht gegen die Anschläge verschiebt.
    let strength = |f: f64| {
        let w = std::f64::consts::TAU * f / fr;
        dtft_power(&centered, w) + dtft_power(&centered, 2.0 * w)
    };
    // Grob: stärkster Bin des Spektrums im Umkreis von ±2 % um den Kandidaten.
    let bin_hz = fr / fft_size as f64;
    let center = coarse_bpm / 60.0 / bin_hz;
    let k_lo = ((0.98 * center).floor().max(1.0)) as usize;
    let k_hi = (((1.02 * center).ceil()) as usize).min((power.len() - 2) / 2).max(k_lo);
    let k_best = (k_lo..=k_hi)
        .max_by(|&a, &b| {
            let s = |k: usize| power[k] + power[2 * k - 1].max(power[2 * k]).max(power[2 * k + 1]);
            s(a).total_cmp(&s(b))
        })
        .unwrap_or(k_lo);
    // Fein: zwischen den Nachbar-Bins abtasten, dann mit dem Goldenen Schnitt einkreisen.
    let step = bin_hz / 8.0;
    let mut f_best = k_best as f64 * bin_hz;
    let mut s_best = f64::MIN;
    for i in -8..=8 {
        let f = k_best as f64 * bin_hz + i as f64 * step;
        let s = strength(f);
        if s > s_best {
            s_best = s;
            f_best = f;
        }
    }
    let f_fine = golden_max(&strength, f_best - step, f_best + step, 40);
    let mut period = 1.0 / f_fine; // Sekunden

    // Phase: Raster über die Kick-Kurve schieben. Ohne Tiefanteil ersatzweise die Gesamtkurve.
    let kick_curve: &[f32] = if means[0] >= 0.05 * max_mean { &feat.flux[0] } else { &onset };
    let period_frames = period * fr;
    let mut phase_frames = 0.0;
    let mut phase_score = f64::MIN;
    let offsets = (period_frames * 4.0).ceil() as usize;
    for i in 0..offsets {
        let d = i as f64 * 0.25;
        let mut sum = 0.0;
        let mut pos = d;
        while pos < (n - 1) as f64 {
            let j = pos as usize;
            let frac = pos - j as f64;
            sum += kick_curve[j] as f64 * (1.0 - frac) + kick_curve[j + 1] as f64 * frac;
            pos += period_frames;
        }
        if sum > phase_score {
            phase_score = sum;
            phase_frames = d;
        }
    }
    let mut origin = phase_frames / fr; // Zeit von Raster-Beat 0 (Lage der Kurven-Spitzen)

    // Schritt 4, erster Durchgang: Spitzen der Kick-Kurve auf das Raster legen (Fenstergenauigkeit).
    let coarse = kick_peaks(kick_curve, origin, period, 0.15, fr);
    let coarse_points: Vec<(f64, f64)> =
        coarse.iter().map(|&(k, frame)| (k as f64, frame as f64 / fr)).collect();
    let mut deviation_ms = f64::NAN;
    let mut first_kick = None;
    let mut refined_ok = false;
    if let Some(fit) = robust_line_fit(&coarse_points) {
        origin = fit.offset;
        period = fit.slope;
        deviation_ms = fit.mean_abs_residual * 1000.0;

        // Zweiter Durchgang: Anschläge im Zeitbereich auf Bruchteile einer Millisekunde bestimmen.
        let peaks = kick_peaks(kick_curve, origin, period, 0.1, fr);
        let points: Vec<(f64, f64)> = peaks
            .iter()
            .filter_map(|&(k, frame)| {
                refine_onset(signal, frame as f64 * HOP as f64 / ANALYSIS_RATE as f64).map(|t| (k as f64, t))
            })
            .collect();
        match robust_line_fit(&points) {
            Some(fit) if fit.inliers >= MIN_ONSETS => {
                origin = fit.offset;
                period = fit.slope;
                deviation_ms = fit.mean_abs_residual * 1000.0;
                first_kick = Some(fit.first_inlier_time);
                refined_ok = true;
            }
            _ => {
                first_kick = Some(fit.first_inlier_time + FLUX_LEAD_SEC);
                origin += FLUX_LEAD_SEC;
            }
        }
    } else {
        origin += FLUX_LEAD_SEC;
    }
    if !(period.is_finite() && period > 0.0) {
        return None;
    }

    // Schritt 7: Sicherheitswert aus Kandidaten-Verhältnis und Rasterabweichung.
    let c_ratio = ((1.0 - ratio) / 0.2).clamp(0.0, 1.0);
    let c_dev = if deviation_ms.is_finite() { (1.0 - (deviation_ms - 2.0) / 18.0).clamp(0.0, 1.0) } else { 0.3 };
    let c_refined = if refined_ok { 1.0 } else { 0.6 };
    let c_score = (scores[chosen] / 0.3).clamp(0.0, 1.0);

    Some(TempoEstimate {
        bpm: 60.0 / period,
        phase: origin.rem_euclid(period),
        confidence: (c_ratio * c_dev * c_refined * c_score) as f32,
        deviation_ms: if deviation_ms.is_finite() { deviation_ms } else { 0.0 },
        first_kick,
    })
}

/// Erwartungstreue Autokorrelation für die Verschiebungen 0 … `max_lag` (über FFT)
/// und das dabei berechnete Leistungsspektrum (mit Nullen aufgefüllt).
fn autocorrelation(x: &[f32], max_lag: usize) -> (Vec<f64>, Vec<f32>) {
    let n = x.len();
    let size = (n + max_lag + 1).next_power_of_two();
    let mut planner = RealFftPlanner::<f32>::new();
    let forward = planner.plan_fft_forward(size);
    let inverse = planner.plan_fft_inverse(size);
    let mut time = vec![0.0f32; size];
    time[..n].copy_from_slice(x);
    let mut spectrum = forward.make_output_vec();
    forward.process(&mut time, &mut spectrum).expect("FFT-Längen passen");
    let power: Vec<f32> = spectrum.iter().map(|c| c.norm_sqr()).collect();
    for (c, &p) in spectrum.iter_mut().zip(&power) {
        *c = realfft::num_complex::Complex::new(p, 0.0);
    }
    inverse.process(&mut spectrum, &mut time).expect("FFT-Längen passen");
    let acf = (0..=max_lag).map(|lag| time[lag] as f64 / size as f64 / (n - lag) as f64).collect();
    (acf, power)
}

/// Leistung der Kurve bei der Kreisfrequenz `omega` (Radiant je Fenster).
fn dtft_power(x: &[f32], omega: f64) -> f64 {
    let (rot_s, rot_c) = omega.sin_cos();
    let (mut s, mut c) = (0.0f64, 1.0f64);
    let (mut re, mut im) = (0.0f64, 0.0f64);
    for &v in x {
        re += v as f64 * c;
        im += v as f64 * s;
        let ns = s * rot_c + c * rot_s;
        c = c * rot_c - s * rot_s;
        s = ns;
    }
    re * re + im * im
}

/// Maximum einer unimodalen Funktion in `[lo, hi]` (Goldener Schnitt).
fn golden_max(f: &dyn Fn(f64) -> f64, mut lo: f64, mut hi: f64, iterations: usize) -> f64 {
    const INV_PHI: f64 = 0.618_033_988_749_894_9;
    let mut a = hi - INV_PHI * (hi - lo);
    let mut b = lo + INV_PHI * (hi - lo);
    let (mut fa, mut fb) = (f(a), f(b));
    for _ in 0..iterations {
        if fa < fb {
            lo = a;
            a = b;
            fa = fb;
            b = lo + INV_PHI * (hi - lo);
            fb = f(b);
        } else {
            hi = b;
            b = a;
            fb = fa;
            a = hi - INV_PHI * (hi - lo);
            fa = f(a);
        }
    }
    0.5 * (lo + hi)
}

/// Sucht um jeden Raster-Beat die Spitze der Kick-Kurve und behält die deutlichen.
/// Rückgabe: (Beat-Nummer, Fenster-Index der Spitze).
fn kick_peaks(curve: &[f32], origin: f64, period: f64, tolerance: f64, fr: f64) -> Vec<(i64, usize)> {
    let n = curve.len();
    let radius = (tolerance * period * fr).max(1.0);
    // Auch ein Beat kurz vor 0 s zählt: Die Kurven-Spitze eines Anschlags bei 0 s liegt im ersten Fenster.
    let first = ((-radius / fr - origin) / period).ceil() as i64;
    let mut found: Vec<(i64, usize, f32)> = Vec::new();
    let mut k = first;
    loop {
        let center = (origin + k as f64 * period) * fr;
        if center >= n as f64 {
            break;
        }
        let lo = (center - radius).ceil().max(0.0) as usize;
        let hi = ((center + radius).floor().max(0.0) as usize).min(n - 1);
        if lo <= hi && center + radius >= 0.0 {
            let (idx, val) = (lo..=hi).map(|i| (i, curve[i])).fold((lo, f32::MIN), |m, c| if c.1 > m.1 { c } else { m });
            // Nur echte Spitzen, keine Flanken am Fensterrand.
            let is_peak = (idx == 0 || curve[idx - 1] <= val) && (idx + 1 >= n || curve[idx + 1] <= val);
            if is_peak && val > 0.0 {
                found.push((k, idx, val));
            }
        }
        k += 1;
    }
    if found.is_empty() {
        return Vec::new();
    }
    // Schwelle: 30 % des 90. Perzentils der Spitzenwerte.
    let mut values: Vec<f32> = found.iter().map(|f| f.2).collect();
    values.sort_by(|a, b| a.total_cmp(b));
    let threshold = 0.3 * values[(values.len() - 1) * 9 / 10];
    found.into_iter().filter(|f| f.2 >= threshold).map(|f| (f.0, f.1)).collect()
}

/// Bestimmt den Beginn eines Anschlags im Zeitbereich.
///
/// `coarse` ist die Zeit der Spitze der Anschlagskurve; der Anschlag liegt wegen der
/// Fensterlänge bis zu 23 ms dahinter. Gesucht wird die Stelle, an der die Hüllkurve
/// (gleitender Effektivwert über 1 ms) erstmals 30 % des Anstiegs vom Pegel davor zur
/// Spitze erreicht.
fn refine_onset(signal: &[f32], coarse: f64) -> Option<f64> {
    let sr = ANALYSIS_RATE as f64;
    let half = (0.0005 * sr) as i64; // halbe Breite der Hüllkurve
    let center = (coarse * sr).round() as i64;
    let base_lo = center - (0.022 * sr) as i64;
    let search_lo = center - (0.012 * sr) as i64;
    let search_hi = center + (0.030 * sr) as i64;
    if search_lo >= signal.len() as i64 || search_hi < 0 {
        return None;
    }
    // Präfixsummen der Leistung ab `base_lo − half`; außerhalb des Signals liegt Stille.
    let start = base_lo - half;
    let end = search_hi + half + 1;
    let mut prefix = Vec::with_capacity((end - start + 1) as usize);
    let mut acc = 0.0f64;
    prefix.push(0.0);
    for i in start..end {
        let s = if i >= 0 && i < signal.len() as i64 { signal[i as usize] as f64 } else { 0.0 };
        acc += s * s;
        prefix.push(acc);
    }
    let envelope = |i: i64| {
        let a = (i - half - start) as usize;
        let b = (i + half + 1 - start) as usize;
        ((prefix[b] - prefix[a]) / (2 * half + 1) as f64).sqrt()
    };
    let base = {
        let a = (base_lo - start) as usize;
        let b = (search_lo - start) as usize;
        ((prefix[b] - prefix[a]) / (b - a) as f64).sqrt()
    };
    let peak = (search_lo..=search_hi).map(envelope).fold(0.0, f64::max);
    if peak < 2.0 * base + 1e-5 {
        return None;
    }
    let threshold = base + 0.3 * (peak - base);
    let mut prev = envelope(search_lo);
    if prev >= threshold {
        return None;
    }
    for i in search_lo + 1..=search_hi {
        let cur = envelope(i);
        if cur >= threshold {
            let frac = (threshold - prev) / (cur - prev);
            return Some(((i - 1) as f64 + frac) / sr);
        }
        prev = cur;
    }
    None
}

struct LineFit {
    /// Zeit bei Beat-Nummer 0.
    offset: f64,
    /// Sekunden je Beat.
    slope: f64,
    mean_abs_residual: f64,
    inliers: usize,
    first_inlier_time: f64,
}

/// Ausgleichsgerade `t = offset + slope · k` mit Verwerfen von Ausreißern
/// (mehr als das Dreifache der mittleren Streuung, mindestens 2 ms).
fn robust_line_fit(points: &[(f64, f64)]) -> Option<LineFit> {
    if points.len() < 2 {
        return None;
    }
    let mut keep = vec![true; points.len()];
    let mut fit = (0.0, 0.0);
    for _ in 0..5 {
        let used: Vec<(f64, f64)> = points.iter().zip(&keep).filter(|p| *p.1).map(|p| *p.0).collect();
        if used.len() < 2 {
            return None;
        }
        let n = used.len() as f64;
        let mean_k = used.iter().map(|p| p.0).sum::<f64>() / n;
        let mean_t = used.iter().map(|p| p.1).sum::<f64>() / n;
        let var_k = used.iter().map(|p| (p.0 - mean_k).powi(2)).sum::<f64>();
        if var_k <= 0.0 {
            return None;
        }
        let slope = used.iter().map(|p| (p.0 - mean_k) * (p.1 - mean_t)).sum::<f64>() / var_k;
        fit = (mean_t - slope * mean_k, slope);

        let mut abs: Vec<f64> = points.iter().map(|p| (p.1 - fit.0 - fit.1 * p.0).abs()).collect();
        let residuals = abs.clone();
        abs.sort_by(|a, b| a.total_cmp(b));
        let median = abs[abs.len() / 2];
        let tolerance = (3.0 * 1.4826 * median).max(0.002);
        let next: Vec<bool> = residuals.iter().map(|&r| r <= tolerance).collect();
        if next == keep {
            break;
        }
        keep = next;
    }
    let inlier_points: Vec<&(f64, f64)> = points.iter().zip(&keep).filter(|p| *p.1).map(|p| p.0).collect();
    if inlier_points.len() < 2 {
        return None;
    }
    let mean_abs_residual =
        inlier_points.iter().map(|p| (p.1 - fit.0 - fit.1 * p.0).abs()).sum::<f64>() / inlier_points.len() as f64;
    Some(LineFit {
        offset: fit.0,
        slope: fit.1,
        mean_abs_residual,
        inliers: inlier_points.len(),
        first_inlier_time: inlier_points.iter().map(|p| p.1).fold(f64::MAX, f64::min),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn line_fit_ignores_outliers() {
        let mut points: Vec<(f64, f64)> = (0..200).map(|k| (k as f64, 0.3 + 0.4615 * k as f64)).collect();
        points[17].1 += 0.05;
        points[120].1 -= 0.08;
        let fit = robust_line_fit(&points).unwrap();
        assert!((fit.slope - 0.4615).abs() < 1e-9);
        assert!((fit.offset - 0.3).abs() < 1e-9);
        assert_eq!(fit.inliers, 198);
    }

    #[test]
    fn autocorrelation_finds_the_period() {
        let x: Vec<f32> = (0..4000).map(|i| if i % 50 == 0 { 1.0 } else { -0.02 }).collect();
        let (acf, power) = autocorrelation(&x, 200);
        // Spektrallinie bei 1/50 je Sample.
        let bin = power.len() * 2 / 50;
        assert!(power[bin - 2..bin + 3].iter().cloned().fold(0.0, f32::max) > 100.0 * power[bin / 2]);
        assert!(acf[50] > 0.9 * acf[0] && acf[100] > 0.9 * acf[0]);
        assert!(acf[25] < 0.1 * acf[0]);
    }
}
