//! Hilfen für die Engine-Tests: Testsignale und Klick-Erkennung.
#![allow(dead_code)]

use dw_core::{AudioBuf, BeatGrid};
use dw_engine::Track;
use std::sync::Arc;

pub const SR: u32 = 48_000;

/// Klick-Track: kurzer Impuls (9 Samples, Maximum in der Mitte) auf jedem Beat.
/// `channel`: 0 = nur links, 1 = nur rechts, 2 = beide.
pub fn click_track(bpm: f64, first_beat: f64, beats: usize, channel: usize) -> Arc<Track> {
    let frames = ((first_beat + (beats as f64 + 2.0) * 60.0 / bpm) * SR as f64) as usize;
    let mut data = vec![0.0f32; frames * 2];
    for b in 0..beats {
        let centre = ((first_beat + b as f64 * 60.0 / bpm) * SR as f64).round() as isize;
        for k in -4isize..=4 {
            let i = centre + k;
            if i >= 0 && (i as usize) < frames {
                let v = 0.8 * (0.5 + 0.5 * (std::f32::consts::PI * k as f32 / 5.0).cos());
                if channel != 1 {
                    data[i as usize * 2] = v;
                }
                if channel != 0 {
                    data[i as usize * 2 + 1] = v;
                }
            }
        }
    }
    Arc::new(Track { audio: AudioBuf::new(SR, data), grid: BeatGrid::constant(bpm, first_beat, 0), gain_db: 0.0 })
}

pub fn sine_track(hz: f64, seconds: f64, amp: f32) -> Arc<Track> {
    let frames = (seconds * SR as f64) as usize;
    let data: Vec<f32> = (0..frames)
        .flat_map(|i| {
            let s = amp * (2.0 * std::f64::consts::PI * hz * i as f64 / SR as f64).sin() as f32;
            [s, s]
        })
        .collect();
    Arc::new(Track { audio: AudioBuf::new(SR, data), grid: BeatGrid::constant(120.0, 0.0, 0), gain_db: 0.0 })
}

/// Gleichverteiltes Rauschen aus einem einfachen Zufallsgenerator mit festem Startwert.
pub fn noise_track(seed: u64, seconds: f64, amp: f32) -> Arc<Track> {
    let frames = (seconds * SR as f64) as usize;
    let mut state = seed.wrapping_mul(0x9E37_79B9_7F4A_7C15) | 1;
    let mut next = move || {
        state ^= state << 13;
        state ^= state >> 7;
        state ^= state << 17;
        (state >> 40) as f32 / (1u64 << 24) as f32 * 2.0 - 1.0
    };
    let data: Vec<f32> = (0..frames * 2).map(|_| amp * next()).collect();
    Arc::new(Track { audio: AudioBuf::new(SR, data), grid: BeatGrid::constant(120.0, 0.0, 0), gain_db: 0.0 })
}

/// Findet Klicks in einem Kanal, auch über Blockgrenzen hinweg. Liefert Frame-Nummern der Maxima.
pub struct ClickFinder {
    channel: usize,
    threshold: f32,
    best: Option<(u64, f32)>,
    quiet: u32,
    frame: u64,
    pub clicks: Vec<u64>,
}

impl ClickFinder {
    pub fn new(channel: usize, threshold: f32) -> Self {
        ClickFinder { channel, threshold, best: None, quiet: 0, frame: 0, clicks: Vec::new() }
    }

    pub fn feed(&mut self, interleaved: &[f32]) {
        for f in interleaved.chunks_exact(2) {
            let v = f[self.channel].abs();
            if v > self.threshold {
                self.quiet = 0;
                if self.best.map_or(true, |(_, b)| v > b) {
                    self.best = Some((self.frame, v));
                }
            } else if let Some((at, _)) = self.best {
                self.quiet += 1;
                if self.quiet > 100 {
                    self.clicks.push(at);
                    self.best = None;
                }
            }
            self.frame += 1;
        }
    }
}

pub fn rms(x: &[f32]) -> f64 {
    (x.iter().map(|&v| v as f64 * v as f64).sum::<f64>() / x.len() as f64).sqrt()
}

pub fn db(x: f64) -> f64 {
    20.0 * x.max(1e-12).log10()
}
