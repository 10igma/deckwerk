//! Messsignale und Sonderfälle: Lautheit, Klick-Spur, perkussives Material,
//! Stille, Geschwindigkeit.

mod common;

use dw_analysis::{analyze, waveform, AnalysisConfig};
use dw_core::{AudioBuf, ANALYSIS_VERSION};
use dw_testgen::{click_track, reference_spec, sine, techno_track, SectionKind, SectionSpec};
use std::time::Instant;

fn config() -> AnalysisConfig {
    AnalysisConfig::default()
}

#[test]
fn default_config_is_techno_range() {
    assert_eq!(config(), AnalysisConfig { bpm_min: 110.0, bpm_max: 170.0 });
    assert_eq!(AnalysisConfig::free(), AnalysisConfig { bpm_min: 60.0, bpm_max: 200.0 });
}

/// ANA-07: 1-kHz-Sinus, stereo, −23 dBFS ergibt −23,0 LUFS ±0,1.
#[test]
fn sine_at_minus_23_dbfs_measures_minus_23_lufs() {
    for sample_rate in [48_000, 44_100] {
        let tone = sine(sample_rate, 1000.0, 20.0, 10f32.powf(-23.0 / 20.0));
        let a = analyze(&tone, &config());
        assert!((a.lufs + 23.0).abs() <= 0.1, "{sample_rate} Hz: {} LUFS", a.lufs);
        assert!((a.true_peak_db + 23.0).abs() <= 0.1, "{sample_rate} Hz: {} dBTP", a.true_peak_db);
        assert!(a.lra.abs() <= 0.1, "{sample_rate} Hz: LRA {}", a.lra);
        assert_eq!(a.version, ANALYSIS_VERSION);
        assert!((a.duration - 20.0).abs() < 1e-9);
        // Ein Dauerton hat kein Tempo.
        assert_eq!(a.bpm_confidence, 0.0);
        // 1 kHz liegt 21 Cent über h″.
        assert!((a.tuning_cents - 21.3).abs() < 2.0, "Stimmung {}", a.tuning_cents);
    }
}

#[test]
fn louder_signal_measures_proportionally_louder() {
    let quiet = analyze(&sine(48_000, 1000.0, 10.0, 0.05), &config());
    let loud = analyze(&sine(48_000, 1000.0, 10.0, 0.5), &config());
    assert!((loud.lufs - quiet.lufs - 20.0).abs() < 0.05);
    assert!((loud.auto_gain_db(-14.0) - (-14.0 - loud.lufs)).abs() < 1e-4);
}

#[test]
fn click_track_gives_exact_tempo_and_beats() {
    for (sample_rate, bpm, first, beats) in
        [(44_100, 128.0, 0.0, 64), (48_000, 133.37, 0.25, 128), (44_100, 150.5, 1.3, 200)]
    {
        let audio = click_track(sample_rate, bpm, first, beats);
        let a = analyze(&audio, &config());
        assert!((a.bpm - bpm).abs() <= 0.05, "{bpm} BPM: erkannt {}", a.bpm);
        assert!(a.bpm_confidence >= 0.5, "{bpm} BPM: Sicherheitswert {}", a.bpm_confidence);
        let period = 60.0 / a.bpm;
        let mean: f64 = (0..beats)
            .map(|k| {
                let beat = a.grid.beat_at(first + k as f64 * 60.0 / bpm);
                ((beat.round() - beat) * period).abs() * 1000.0
            })
            .sum::<f64>()
            / beats as f64;
        assert!(mean <= 1.0, "{bpm} BPM: mittlere Abweichung {mean} ms");
        // Der Anker liegt auf dem ersten Klick, der als Eins gilt (keine Struktur erkennbar).
        assert!((a.grid.anchors()[0].pos - first).abs() <= 0.002, "Anker {}", a.grid.anchors()[0].pos);
        assert_eq!(a.grid.bar_offset, 0);
        // Klicks haben keine Tonart.
        assert!(a.key_confidence < 0.3);
    }
}

#[test]
fn percussive_track_has_low_key_confidence() {
    let mut spec = reference_spec(8);
    spec.sections = vec![SectionSpec::new(SectionKind::Intro, 48)];
    let (audio, truth) = techno_track(&spec);
    let a = analyze(&audio, &config());
    assert!((a.bpm - truth.bpm).abs() <= 0.05);
    assert!(a.key_confidence < 0.3, "Sicherheitswert {}", a.key_confidence);
}

#[test]
fn analysis_is_deterministic() {
    let mut spec = reference_spec(13);
    spec.sections = vec![
        SectionSpec::new(SectionKind::Intro, 8),
        SectionSpec::new(SectionKind::Main, 16),
        SectionSpec::new(SectionKind::Breakdown, 8),
        SectionSpec::new(SectionKind::Drop, 16),
    ];
    let (audio, _) = techno_track(&spec);
    assert_eq!(analyze(&audio, &config()), analyze(&audio, &config()));
    assert_eq!(waveform(&audio), waveform(&audio));
}

#[test]
fn degenerate_inputs_do_not_panic() {
    let inputs = [
        AudioBuf::silence(44_100, 0),
        AudioBuf::silence(44_100, 1),
        AudioBuf::silence(48_000, 480_000),
        click_track(44_100, 128.0, 0.0, 3),
        sine(8000, 440.0, 0.3, 0.5),
        sine(96_000, 440.0, 2.0, 0.5),
    ];
    for audio in &inputs {
        let a = analyze(audio, &config());
        assert_eq!(a.bpm_confidence, 0.0);
        assert!(a.bpm >= 110.0 && a.bpm <= 170.0);
        assert!(a.key_confidence >= 0.0 && a.key_confidence <= 1.0);
        let w = waveform(audio);
        assert_eq!(w.overview.points(), 2000);
        assert_eq!(w.levels.len(), 4);
    }
    let silent = analyze(&inputs[2], &config());
    assert_eq!(silent.lufs, f32::NEG_INFINITY);
    assert_eq!(silent.key, None);
    assert_eq!(silent.auto_gain_db(-14.0), 0.0);
}

/// ANA-16: 7-Minuten-Track (48 kHz) in ≤ 15 s, hier einschließlich Wellenform-Daten.
/// Die maßgebliche Messung im Release-Modus macht das Beispiel `kennzahlen`.
#[test]
fn seven_minute_track_is_analyzed_within_15_seconds() {
    let (audio, truth) = techno_track(&common::seven_minute_spec());
    assert!(audio.duration() >= 420.0);
    let started = Instant::now();
    let a = analyze(&audio, &config());
    let w = waveform(&audio);
    let seconds = started.elapsed().as_secs_f64();
    println!("7-Minuten-Track: Analyse und Wellenform in {seconds:.2} s");
    assert!(seconds <= 15.0, "{seconds} s");
    assert!((a.bpm - truth.bpm).abs() <= 0.05);
    assert_eq!(w.frames, audio.frames() as u64);
}
