//! Abtastratenwandlung für die Wiedergabe mit verändertem Tempo ohne Key Lock (5.13):
//! gefensterter Sinc-Interpolator mit 32 Stützstellen.

use std::f64::consts::PI;
use std::sync::OnceLock;

const TAPS: usize = 32;
const HALF: isize = (TAPS / 2) as isize;
const PHASES: usize = 512;
const CUTOFF: f64 = 0.92;
const KAISER_BETA: f64 = 8.0;

fn bessel_i0(x: f64) -> f64 {
    let mut sum = 1.0;
    let mut term = 1.0;
    let q = x * x / 4.0;
    for k in 1..40 {
        term *= q / (k * k) as f64;
        sum += term;
        if term < 1e-12 * sum {
            break;
        }
    }
    sum
}

/// Tabelle: (PHASES + 1) Zeilen mit je TAPS Koeffizienten.
fn table() -> &'static [f32] {
    static TABLE: OnceLock<Vec<f32>> = OnceLock::new();
    TABLE.get_or_init(|| {
        let mut t = vec![0.0f32; (PHASES + 1) * TAPS];
        let i0_beta = bessel_i0(KAISER_BETA);
        for p in 0..=PHASES {
            let frac = p as f64 / PHASES as f64;
            let mut row = [0.0f64; TAPS];
            let mut sum = 0.0;
            for (k, r) in row.iter_mut().enumerate() {
                // Abstand der Stützstelle k von der gesuchten Position
                let x = (k as isize - (HALF - 1)) as f64 - frac;
                let sinc = if x.abs() < 1e-12 { CUTOFF } else { (PI * CUTOFF * x).sin() / (PI * x) };
                let w = x / HALF as f64;
                let window = if w.abs() >= 1.0 { 0.0 } else { bessel_i0(KAISER_BETA * (1.0 - w * w).sqrt()) / i0_beta };
                *r = sinc * window;
                sum += *r;
            }
            for (k, r) in row.iter().enumerate() {
                // Auf Summe 1 normieren: Gleichanteil bleibt exakt erhalten
                t[p * TAPS + k] = (r / sum) as f32;
            }
        }
        t
    })
}

/// Tabelle vorab erzeugen (außerhalb des Audio-Threads aufrufen).
pub fn init() {
    table();
}

/// Liest ein Stereo-Sample an der gebrochenen Frame-Position `pos` aus einem
/// verschachtelten Puffer. Außerhalb des Puffers ist das Signal null.
#[inline]
pub fn sample_stereo(data: &[f32], pos: f64) -> (f32, f32) {
    let frames = (data.len() / 2) as isize;
    let base = pos.floor();
    let frac = pos - base;
    let i = base as isize;
    if frac == 0.0 {
        return if i >= 0 && i < frames { (data[i as usize * 2], data[i as usize * 2 + 1]) } else { (0.0, 0.0) };
    }
    if i + HALF < 0 || i - HALF >= frames {
        return (0.0, 0.0);
    }
    let t = table();
    let ph = frac * PHASES as f64;
    let p0 = ph as usize;
    let w1 = (ph - p0 as f64) as f32;
    let w0 = 1.0 - w1;
    let row0 = &t[p0 * TAPS..(p0 + 1) * TAPS];
    let row1 = &t[(p0 + 1) * TAPS..(p0 + 2) * TAPS];
    let start = i - (HALF - 1);
    let (mut l, mut r) = (0.0f32, 0.0f32);
    if start >= 0 && start + TAPS as isize <= frames {
        let s = &data[start as usize * 2..(start as usize + TAPS) * 2];
        for k in 0..TAPS {
            let c = row0[k] * w0 + row1[k] * w1;
            l += s[k * 2] * c;
            r += s[k * 2 + 1] * c;
        }
    } else {
        for k in 0..TAPS {
            let idx = start + k as isize;
            if idx >= 0 && idx < frames {
                let c = row0[k] * w0 + row1[k] * w1;
                l += data[idx as usize * 2] * c;
                r += data[idx as usize * 2 + 1] * c;
            }
        }
    }
    (l, r)
}
