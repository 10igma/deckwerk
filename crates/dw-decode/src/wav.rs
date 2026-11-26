//! Schreiben von WAV-Dateien (PCM 16 oder 24 bit, stereo).

use dw_core::AudioBuf;
use std::io::{BufWriter, Seek, SeekFrom, Write};
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

/// Schreibt eine WAV-Datei Stück für Stück (Mitschnitt, REC-01). Nach jedem [`WavWriter::sync`] stehen
/// die Längen im Dateikopf und die Daten auf der Platte: Die Datei ist dann lesbar, auch wenn das
/// Programm danach abstürzt.
pub struct WavWriter {
    file: BufWriter<std::fs::File>,
    bits: u16,
    /// Bisher geschriebene Bytes im Datenteil.
    data_len: u64,
}

impl WavWriter {
    /// Legt die Datei an (stereo). `bits` ist 16 oder 24.
    pub fn create(path: &Path, sample_rate: u32, bits: u16) -> std::io::Result<WavWriter> {
        assert!(bits == 16 || bits == 24, "nur 16 oder 24 bit");
        let bytes_per_sample = (bits / 8) as u32;
        let mut file = BufWriter::new(std::fs::File::create(path)?);
        file.write_all(b"RIFF")?;
        file.write_all(&36u32.to_le_bytes())?;
        file.write_all(b"WAVEfmt ")?;
        file.write_all(&16u32.to_le_bytes())?;
        file.write_all(&1u16.to_le_bytes())?; // PCM
        file.write_all(&2u16.to_le_bytes())?; // Kanäle
        file.write_all(&sample_rate.to_le_bytes())?;
        file.write_all(&(sample_rate * 2 * bytes_per_sample).to_le_bytes())?;
        file.write_all(&((2 * bytes_per_sample) as u16).to_le_bytes())?;
        file.write_all(&bits.to_le_bytes())?;
        file.write_all(b"data")?;
        file.write_all(&0u32.to_le_bytes())?;
        let mut writer = WavWriter { file, bits, data_len: 0 };
        writer.sync()?;
        Ok(writer)
    }

    /// Hängt verschachtelte Stereo-Werte an; Werte außerhalb ±1 werden begrenzt.
    pub fn write(&mut self, samples: &[f32]) -> std::io::Result<()> {
        let bytes = (self.bits / 8) as usize;
        let scale = (1i64 << (self.bits - 1)) as f64;
        for &s in samples {
            let v = ((s as f64 * scale).round() as i64).clamp(-(scale as i64), scale as i64 - 1) as i32;
            self.file.write_all(&v.to_le_bytes()[..bytes])?;
        }
        self.data_len += (samples.len() * bytes) as u64;
        Ok(())
    }

    /// Bisher geschriebene Frames.
    pub fn frames(&self) -> u64 {
        self.data_len / (2 * (self.bits / 8) as u64)
    }

    /// Trägt die Längen in den Dateikopf ein und bringt alles auf die Platte.
    pub fn sync(&mut self) -> std::io::Result<()> {
        // WAV fasst höchstens 4 GiB; darüber bleibt die Längenangabe am Anschlag stehen.
        let len = self.data_len.min(u32::MAX as u64 - 36) as u32;
        self.file.seek(SeekFrom::Start(4))?;
        self.file.write_all(&(36 + len).to_le_bytes())?;
        self.file.seek(SeekFrom::Start(40))?;
        self.file.write_all(&len.to_le_bytes())?;
        self.file.seek(SeekFrom::End(0))?;
        self.file.flush()?;
        self.file.get_ref().sync_data()
    }

    /// Schließt die Datei ab.
    pub fn finish(mut self) -> std::io::Result<()> {
        self.sync()
    }
}
