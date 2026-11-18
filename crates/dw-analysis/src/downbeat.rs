//! Downbeat-Erkennung (Spezifikation 5.2, Schritt 6).
//!
//! Die Neuheitskurve ist hier in einfacher Form selbst berechnet: Unterschied der
//! mittleren Band-Pegel der 8 Beats nach und vor jedem Beat. Die Fassung aus 5.6
//! (Struktur-Analyse) folgt in Stufe 3.

use crate::preprocess::ANALYSIS_RATE;
use crate::spectrum::{BandFeatures, BANDS, WINDOW};

/// Beats je Seite des Vergleichs (2 Takte; Muster mit Taktlänge heben sich damit auf).
const HALF_SPAN: usize = 8;
/// Nur Veränderungen ab diesem Anteil der stärksten zählen.
const MIN_PEAK_SHARE: f64 = 0.2;
/// Kleinste Veränderung, die als Abschnittswechsel zählt: Pegelsprünge aller Bänder zusammen in dB.
const MIN_PEAK_DB: f64 = 3.0;
/// Gewicht des ersten Kick-Anschlags relativ zur Summe aller Veränderungen.
const FIRST_KICK_WEIGHT: f64 = 0.1;

/// Bestimmt, welcher der Beats 0–3 (gezählt ab `anchor`) die erste Eins ist.
///
/// `end` ist das Ende des hörbaren Signals in Sekunden; Stille danach zählt nicht als Veränderung.
pub fn estimate(feat: &BandFeatures, anchor: f64, period: f64, end: f64, first_kick: Option<f64>) -> u8 {
    let fr = BandFeatures::frame_rate();
    let n = feat.frames();
    let seconds = (n as f64 / fr).min(end);
    let beats = (((seconds - anchor) / period).floor().max(0.0)) as usize;
    let kick_vote = first_kick.map(|t| (((t - anchor) / period).round() as i64).rem_euclid(4) as usize);
    let fallback = kick_vote.unwrap_or(0) as u8;
    if beats < 2 * HALF_SPAN + 1 {
        return fallback;
    }

    let guard = (0.5 * WINDOW as f64 / ANALYSIS_RATE as f64).min(0.25 * period);
    // Mittlerer Pegel je Beat und Band in dB, nach unten auf 40 dB unter dem Band-Maximum begrenzt.
    let mut level = vec![[0.0f64; BANDS]; beats];
    for (b, row) in level.iter_mut().enumerate() {
        // Nur Fenster, die ganz im Beat liegen: Sonst färbt der Anschlag des nächsten Beats ab.
        let lo = (((anchor + b as f64 * period + guard) * fr).ceil() as usize).min(n);
        let hi = (((anchor + (b + 1) as f64 * period - guard) * fr).ceil() as usize).min(n).max(lo);
        for band in 0..BANDS {
            let frames = &feat.power[band][lo..hi];
            row[band] = if frames.is_empty() { 0.0 } else { frames.iter().map(|&p| p as f64).sum::<f64>() / frames.len() as f64 };
        }
    }
    for band in 0..BANDS {
        let max = level.iter().map(|r| r[band]).fold(0.0, f64::max);
        let floor = (1e-4 * max).max(1e-12);
        for row in level.iter_mut() {
            row[band] = 10.0 * (row[band] + floor).log10();
        }
    }

    // Neuheit: Summe der Pegelsprünge über die Bänder.
    let mut prefix = vec![[0.0f64; BANDS]; beats + 1];
    for b in 0..beats {
        for band in 0..BANDS {
            prefix[b + 1][band] = prefix[b][band] + level[b][band];
        }
    }
    let mut novelty = vec![0.0f64; beats];
    for b in HALF_SPAN..=beats - HALF_SPAN {
        novelty[b] = (0..BANDS)
            .map(|band| {
                let after = prefix[b + HALF_SPAN][band] - prefix[b][band];
                let before = prefix[b][band] - prefix[b - HALF_SPAN][band];
                (after - before).abs() / HALF_SPAN as f64
            })
            .sum();
    }

    // Stärkste Veränderungen: Maxima im Umkreis von ±4 Beats.
    let max = novelty.iter().cloned().fold(0.0, f64::max);
    if max <= 0.0 {
        return fallback;
    }
    let peaks: Vec<(usize, f64)> = (HALF_SPAN..=beats - HALF_SPAN)
        .filter(|&b| {
            let v = novelty[b];
            v >= (MIN_PEAK_SHARE * max).max(MIN_PEAK_DB)
                && (b.saturating_sub(4)..b).all(|j| novelty[j] < v)
                && (b + 1..=(b + 4).min(beats - 1)).all(|j| novelty[j] <= v)
        })
        .map(|b| (b, novelty[b]))
        .collect();
    if peaks.is_empty() {
        // Keine erkennbare Struktur: Der erste Kick-Anschlag gilt als Eins.
        return fallback;
    }
    let total: f64 = peaks.iter().map(|p| p.1).sum();

    // Bewertung je Lage: Veränderungen auf Taktgrenzen, bevorzugt im 8-Takt-Raster.
    let mut scores = [0.0f64; 4];
    for (c, score) in scores.iter_mut().enumerate() {
        let on_bar: f64 = peaks.iter().filter(|p| p.0 % 4 == c).map(|p| p.1).sum();
        let on_phrase = (0..8)
            .map(|j| peaks.iter().filter(|p| p.0 % 32 == (c + 4 * j) % 32).map(|p| p.1).sum::<f64>())
            .fold(0.0, f64::max);
        *score = 0.5 * on_bar + 0.5 * on_phrase;
        if kick_vote == Some(c) {
            *score += FIRST_KICK_WEIGHT * total;
        }
    }
    (0..4).max_by(|&a, &b| scores[a].total_cmp(&scores[b])).unwrap_or(0) as u8
}
