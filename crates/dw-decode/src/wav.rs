//! Schreiben von WAV-Dateien (PCM 16 oder 24 bit, stereo).

use dw_core::AudioBuf;
use std::io::{BufWriter, Write};
use std::path::Path;

/// Schreibt `audio` als WAV. `bits` ist 16 oder 24; Werte außerhalb ±1 werden begrenzt.
pub fn write_wav(path: &Path, audio: &AudioBuf, bits: u16) -> std::io::Result<()> {
    assert!(bits == 16 || bits == 24, "nur 16 oder 24 bit");
    let bytes_per_sample = (bits / 8) as u32;
    let data_len = audio.data.len() as u32 * bytes_per_sample;
    let mut w = BufWriter::new(std::fs::File::create(path)?);
    w.write_all(b"RIFF")?;
    w.write_all(&(36 + data_len).to_le_bytes())?;
    w.write_all(b"WAVEfmt ")?;
    w.write_all(&16u32.to_le_bytes())?;
    w.write_all(&1u16.to_le_bytes())?; // PCM
    w.write_all(&2u16.to_le_bytes())?; // Kanäle
    w.write_all(&audio.sample_rate.to_le_bytes())?;
    w.write_all(&(audio.sample_rate * 2 * bytes_per_sample).to_le_bytes())?;
    w.write_all(&((2 * bytes_per_sample) as u16).to_le_bytes())?;
    w.write_all(&bits.to_le_bytes())?;
    w.write_all(b"data")?;
    w.write_all(&data_len.to_le_bytes())?;
    // Maßstab 2^(bits−1) wie beim Lesen: Eine gelesene Datei wird bitgenau wieder geschrieben.
    let scale = (1i64 << (bits - 1)) as f64;
    for &s in &audio.data {
        let v = ((s as f64 * scale).round() as i64).clamp(-(scale as i64), scale as i64 - 1) as i32;
        w.write_all(&v.to_le_bytes()[..bytes_per_sample as usize])?;
    }
    w.flush()
}
