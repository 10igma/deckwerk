//! Lautheit nach ITU-R BS.1770-4 / EBU R 128 (Spezifikation 5.4), gemessen mit dem Crate `ebur128`.

use dw_core::AudioBuf;
use ebur128::{EbuR128, Mode};

/// Ergebnis der Lautheitsmessung.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Loudness {
    /// Integrierte Lautheit in LUFS; `-inf`, wenn nichts über der Gate-Schwelle liegt.
    pub lufs: f32,
    /// Lautheitsbereich in LU.
    pub lra: f32,
    /// True Peak in dBTP (vierfache Überabtastung bis 96 kHz); `-inf` bei Stille.
    pub true_peak_db: f32,
}

/// Misst integrierte Lautheit, Lautheitsbereich und True Peak bei der Abtastrate des Tracks.
pub fn measure(audio: &AudioBuf) -> Loudness {
    let silent = Loudness { lufs: f32::NEG_INFINITY, lra: 0.0, true_peak_db: f32::NEG_INFINITY };
    if audio.frames() == 0 {
        return silent;
    }
    let Ok(mut meter) = EbuR128::new(2, audio.sample_rate, Mode::I | Mode::LRA | Mode::TRUE_PEAK) else {
        return silent;
    };
    if meter.add_frames_f32(&audio.data).is_err() {
        return silent;
    }
    let lufs = meter.loudness_global().unwrap_or(f64::NEG_INFINITY);
    let lra = meter.loudness_range().unwrap_or(0.0);
    let peak = (0..2).filter_map(|ch| meter.true_peak(ch).ok()).fold(0.0, f64::max);
    Loudness {
        lufs: if lufs.is_finite() { lufs as f32 } else { f32::NEG_INFINITY },
        lra: if lra.is_finite() { lra as f32 } else { 0.0 },
        true_peak_db: if peak > 0.0 { (20.0 * peak.log10()) as f32 } else { f32::NEG_INFINITY },
    }
}
