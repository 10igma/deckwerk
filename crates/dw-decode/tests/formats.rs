//! BIB-02: Je Format wird eine Testdatei dekodiert. Verlustfreie Formate müssen
//! bitgenau der Referenz entsprechen, verlustbehaftete in Zeitlage, Länge und Pegel.

use dw_decode::{decode_file, load_for_engine, DecodeError};
use std::path::PathBuf;

fn data(name: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/data").join(name)
}

fn reference() -> Vec<f32> {
    let d = decode_file(&data("ref.wav")).unwrap();
    assert_eq!(d.audio.sample_rate, 44_100);
    assert_eq!(d.audio.frames(), 44_100);
    assert_eq!(d.source_channels, 2);
    d.audio.data
}

#[test]
fn wav_matches_the_generated_signal() {
    let r = reference();
    // Klick links bei 0,5 s; rechts 1-kHz-Sinus mit Amplitude 0,3.
    assert!((r[22_050 * 2] - 0.9).abs() < 1e-4);
    let peak_r = r.chunks_exact(2).map(|f| f[1].abs()).fold(0.0f32, f32::max);
    assert!((peak_r - 0.3).abs() < 1e-3);
}

#[test]
fn lossless_formats_are_bit_exact() {
    let r = reference();
    for name in ["ref.flac", "ref.aiff", "ref_alac.m4a"] {
        let d = decode_file(&data(name)).unwrap_or_else(|e| panic!("{name}: {e}"));
        assert_eq!(d.audio.sample_rate, 44_100, "{name}");
        assert_eq!(d.audio.data.len(), r.len(), "{name}: Länge");
        assert!(d.audio.data == r, "{name}: weicht von der Referenz ab");
    }
}

/// Position des Klicks (größter Betrag links) und Effektivwert rechts.
fn click_and_level(samples: &[f32]) -> (usize, f64) {
    let (idx, _) = samples.chunks_exact(2).enumerate().fold((0, 0.0f32), |m, (i, f)| if f[0].abs() > m.1 { (i, f[0].abs()) } else { m });
    let n = samples.len() / 2;
    let rms = (samples.chunks_exact(2).map(|f| f[1] as f64 * f[1] as f64).sum::<f64>() / n as f64).sqrt();
    (idx, rms)
}

#[test]
fn lossy_formats_keep_timing_length_and_level() {
    let (_, ref_rms) = click_and_level(&reference());
    for (name, rate) in [("ref.mp3", 44_100), ("ref_aac.m4a", 44_100), ("ref.ogg", 44_100), ("ref.opus", 48_000)] {
        let d = decode_file(&data(name)).unwrap_or_else(|e| panic!("{name}: {e}"));
        assert_eq!(d.audio.sample_rate, rate, "{name}");
        assert_eq!(d.skipped_packets, 0, "{name}");
        let frames = d.audio.frames() as f64;
        let (click, rms) = click_and_level(&d.audio.data);
        let click_sec = click as f64 / rate as f64;
        println!("{name}: {frames} Frames, Klick bei {click_sec:.5} s, Pegel rechts {:.2} dB", 20.0 * (rms / ref_rms).log10());
        // Encoder-Vorlauf ist abgeschnitten: Der Klick liegt bei 0,5 s (±1 ms), die Länge bei 1 s (±30 ms).
        assert!((click_sec - 0.5).abs() < 0.001, "{name}: Klick bei {click_sec} s");
        assert!((frames / rate as f64 - 1.0).abs() < 0.03, "{name}: Länge {} s", frames / rate as f64);
        assert!((20.0 * (rms / ref_rms).log10()).abs() < 0.5, "{name}: Pegel");
    }
}

#[test]
fn mono_and_high_resolution_files() {
    let mono = decode_file(&data("ref_mono.wav")).unwrap();
    assert_eq!(mono.source_channels, 1);
    assert_eq!(mono.audio.frames(), 44_100);
    assert!(mono.audio.data.chunks_exact(2).all(|f| f[0] == f[1]));

    let hi = decode_file(&data("ref_96k_24bit.wav")).unwrap();
    assert_eq!(hi.audio.sample_rate, 96_000);
    assert_eq!(hi.bits_per_sample, Some(24));
    assert_eq!(hi.audio.frames(), 96_000);
}

#[test]
fn loading_for_the_engine_converts_the_sample_rate() {
    let a = load_for_engine(&data("ref.flac"), 48_000).unwrap();
    assert_eq!(a.sample_rate, 48_000);
    assert_eq!(a.frames(), 48_000);
    let (click, _) = click_and_level(&a.data);
    assert_eq!(click, 24_000);
}

#[test]
fn broken_and_missing_files_give_errors_instead_of_panics() {
    assert!(matches!(decode_file(&data("fehlt.mp3")), Err(DecodeError::Io(_))));
    let junk = std::env::temp_dir().join("deckwerk_junk.mp3");
    std::fs::write(&junk, b"das ist keine Audiodatei, nur Text. ".repeat(200)).unwrap();
    assert!(decode_file(&junk).is_err());
    let _ = std::fs::remove_file(junk);
}

#[test]
fn written_wav_reads_back_identically() {
    let r = dw_decode::decode_file(&data("ref.wav")).unwrap();
    for bits in [16u16, 24] {
        let out = std::env::temp_dir().join(format!("deckwerk_roundtrip_{bits}.wav"));
        dw_decode::write_wav(&out, &r.audio, bits).unwrap();
        let back = decode_file(&out).unwrap();
        assert_eq!(back.bits_per_sample, Some(bits as u32));
        assert_eq!(back.audio.data.len(), r.audio.data.len());
        // Die Referenz ist 16 bit; beide Tiefen geben sie bitgenau wieder.
        assert!(back.audio.data == r.audio.data, "{bits} bit: weicht ab");
        let _ = std::fs::remove_file(out);
    }
}
