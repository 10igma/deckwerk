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

/// Eine abgebrochene Kopie (Kopfdaten nennen die volle Länge, hinten fehlt Audio) wird erkannt;
/// vollständige Dateien aller Formate gelten nicht als unvollständig.
#[test]
fn truncated_files_are_reported_as_incomplete() {
    for name in ["ref.wav", "ref.flac", "ref.mp3", "ref.ogg", "ref.opus", "ref.aiff", "ref_aac.m4a", "ref_alac.m4a"] {
        let d = decode_file(&data(name)).unwrap_or_else(|e| panic!("{name}: {e}"));
        assert!(!d.is_incomplete(), "{name}: {:?} angekündigt, {} dekodiert", d.announced_frames, d.audio.frames());
    }
    let dir = std::env::temp_dir().join(format!("dw-decode-incomplete-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    // MP3: hintere Hälfte mit Nullen überschrieben (so sieht eine vorbelegte, nicht fertig kopierte Datei aus).
    let mut mp3 = std::fs::read(data("ref.mp3")).unwrap();
    let half = mp3.len() / 2;
    mp3[half..].fill(0);
    std::fs::write(dir.join("halb.mp3"), &mp3).unwrap();
    // WAV: hinten abgeschnitten.
    let wav = std::fs::read(data("ref.wav")).unwrap();
    std::fs::write(dir.join("halb.wav"), &wav[..wav.len() / 2]).unwrap();
    for name in ["halb.mp3", "halb.wav"] {
        let d = decode_file(&dir.join(name)).unwrap_or_else(|e| panic!("{name}: {e}"));
        let announced = d.announced_frames.unwrap_or_else(|| panic!("{name}: keine angekündigte Länge"));
        let share = d.audio.frames() as f64 / announced as f64;
        println!("{name}: {} von {announced} Frames ({:.0} %)", d.audio.frames(), share * 100.0);
        assert!(d.is_incomplete(), "{name}");
        assert!((0.3..0.7).contains(&share), "{name}: Anteil {share}");
    }
    let _ = std::fs::remove_dir_all(&dir);
}

/// REC-01: Eine laufend geschriebene WAV-Datei ist nach jedem Abgleich vollständig lesbar – auch
/// ohne ordentlichen Abschluss (Absturz) – und enthält bitgenau, was hineingeschrieben wurde.
#[test]
fn streamed_wav_is_readable_after_every_sync() {
    use dw_decode::WavWriter;
    let dir = std::env::temp_dir().join(format!("dw-decode-stream-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    for bits in [16u16, 24] {
        let path = dir.join(format!("mitschnitt-{bits}.wav"));
        let scale = (1i64 << (bits - 1)) as f32;
        // Werte auf dem Raster der Bittiefe, damit der Vergleich bitgenau sein kann.
        let sample = |i: usize| (((i * 7919) % 20001) as f32 - 10000.0) / 10000.0 * 0.9;
        let quantised = |v: f32| (v * scale).round() / scale;
        let all: Vec<f32> = (0..48_000 * 2 * 3).map(|i| quantised(sample(i))).collect();
        let mut w = WavWriter::create(&path, 48_000, bits).unwrap();
        // Direkt nach dem Anlegen: gültige, leere Datei (kein Ton dekodierbar, aber kein kaputter Kopf).
        assert_eq!(std::fs::metadata(&path).unwrap().len(), 44);
        let mut written = 0;
        for piece in all.chunks(48_000 * 2 / 3 + 10) {
            w.write(piece).unwrap();
            written += piece.len();
            w.sync().unwrap();
            assert_eq!(w.frames() as usize, written / 2);
            // „Absturz": Die Datei wird gelesen, während der Schreiber noch offen ist.
            let d = decode_file(&path).unwrap_or_else(|e| panic!("{bits} bit nach {written} Werten: {e}"));
            assert_eq!(d.audio.sample_rate, 48_000);
            assert_eq!(d.audio.data.len(), written, "{bits} bit");
            assert!(d.audio.data == all[..written], "{bits} bit: Inhalt weicht ab");
            assert!(!d.is_incomplete());
        }
        // Was nach dem letzten Abgleich geschrieben wurde, steht erst nach dem Abschluss in der Länge.
        w.write(&all[..960]).unwrap();
        w.finish().unwrap();
        assert_eq!(decode_file(&path).unwrap().audio.data.len(), all.len() + 960);
        // Begrenzung statt Überlauf.
        let mut w = WavWriter::create(&path, 44_100, bits).unwrap();
        w.write(&[2.0, -2.0, 1.0, -1.0]).unwrap();
        w.finish().unwrap();
        let d = decode_file(&path).unwrap();
        assert_eq!(d.audio.sample_rate, 44_100);
        assert!(d.audio.data[0] > 0.999 && d.audio.data[1] == -1.0 && d.audio.data[3] == -1.0);
    }
    let _ = std::fs::remove_dir_all(&dir);
}
