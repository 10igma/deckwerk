//! Übergang T1 gegen TRANS-02 (Phrasenraster), TRANS-03 (Bass-Regel), DECK-08 (Sync) und T10 (Tempo).

use dw_analysis::{analyze, AnalysisConfig};
use dw_automix::{plan_blend, render_blend, PlanError};
use dw_core::{AudioBuf, Command, DeckId, Key, SyncMode, TrackAnalysis};
use dw_dsp::{Biquad, Coeffs};
use dw_engine::{Engine, Track};
use dw_testgen::{techno_track, SectionKind, SectionSpec, TrackSpec};
use std::sync::Arc;

const SR: u32 = 48_000;

/// Track, der durchgehend Kick und Bass spielt: der härteste Fall für die Bass-Regel.
fn full_track(bpm: f64, key: Key, bars: u32, first_beat: f64, bar_offset: u8, seed: u64) -> (Arc<Track>, TrackAnalysis) {
    let mut spec = TrackSpec::new(bpm, key, vec![SectionSpec::new(SectionKind::Main, bars)], seed);
    spec.sample_rate = SR;
    spec.first_beat_sec = first_beat;
    spec.bar_offset = bar_offset;
    let (audio, _) = techno_track(&spec);
    let analysis = analyze(&audio, &AnalysisConfig::default());
    let track = Arc::new(Track { audio, grid: analysis.grid.clone(), gain_db: analysis.auto_gain_db(-14.0) });
    (track, analysis)
}

/// Pegel im Band unter 120 Hz, je Abschnitt von `len` Frames, in dB.
fn low_band_levels(audio: &AudioBuf, from: usize, to: usize, len: usize) -> Vec<f64> {
    let c = Coeffs::lowpass(SR as f64, 120.0, std::f64::consts::FRAC_1_SQRT_2);
    let (mut f1, mut f2) = (Biquad::new(c), Biquad::new(c));
    let low: Vec<f64> = audio.data.chunks_exact(2).map(|f| f2.process(f1.process(0.5 * (f[0] + f[1]) as f64))).collect();
    low[from..to]
        .chunks_exact(len)
        .map(|c| 10.0 * (c.iter().map(|v| v * v).sum::<f64>() / len as f64).max(1e-12).log10())
        .collect()
}

fn max(v: &[f64]) -> f64 {
    v.iter().cloned().fold(f64::MIN, f64::max)
}

#[test]
fn blend_follows_the_rules() {
    let (a, an_a) = full_track(132.0, Key::from_camelot(8, 'A').unwrap(), 96, 0.21, 0, 11);
    let (b, an_b) = full_track(132.0, Key::from_camelot(9, 'A').unwrap(), 96, 0.05, 2, 12);
    let plan = plan_blend(&an_a, &an_b, 32, 16, 16).unwrap();

    // TRANS-02: Ausstieg und Einstieg liegen auf Downbeats im 8-Takt-Raster.
    let beat_a = an_a.grid.beat_at(plan.a_out);
    let beat_b = an_b.grid.beat_at(plan.b_in);
    assert!((beat_a - beat_a.round()).abs() < 1e-6 && (beat_b - beat_b.round()).abs() < 1e-6);
    assert!(an_a.grid.is_downbeat(beat_a.round() as i64) && an_b.grid.is_downbeat(beat_b.round() as i64));
    let first_a = an_a.grid.next_downbeat(an_a.grid.beat_at(0.0) - 1e-6);
    assert_eq!((beat_a.round() as i64 - first_a).rem_euclid(32), 0, "Ausstieg nicht auf einer 8-Takt-Grenze");
    assert_eq!((plan.bars, plan.swap_bar(), plan.lead_bars, plan.tail_bars), (32, 16, 16, 16));
    assert_eq!(plan.tempo_ramp, (16.0, 16.0), "bei gleichem Tempo kein Verlauf");

    let (mix, report) = render_blend(a.clone(), b.clone(), &plan);
    let bar = (4.0 * 60.0 / 132.0 * SR as f64).round() as usize;

    // Zeitpunkte: 16 Takte Vorlauf, Tausch nach 16, Ende nach 32 Takten (auf einen Render-Block genau).
    assert!((report.t0_frame as i64 - 16 * bar as i64).abs() < 1024, "T0 bei {}", report.t0_frame);
    assert!((report.swap_frame as i64 - 32 * bar as i64).abs() < 1024);
    assert!((report.end_frame as i64 - 48 * bar as i64).abs() < 1024);
    assert!((mix.frames() as i64 - 64 * bar as i64).abs() < 2048, "Länge {}", mix.frames());

    // DECK-08: Die Beat-Raster beider Decks liegen im Übergang übereinander.
    assert!(report.max_phase_error < 1e-3, "Beat-Versatz {}", report.max_phase_error);

    // TRANS-03: Kein Takt im Übergang ist im Bass mehr als 3 dB lauter als A oder B allein.
    let solo_a = max(&low_band_levels(&mix, 4 * bar, 16 * bar, bar));
    let solo_b = max(&low_band_levels(&mix, 50 * bar, 62 * bar, bar));
    let during = low_band_levels(&mix, 16 * bar, 48 * bar, bar);
    let reference = solo_a.max(solo_b);
    println!("Bass allein: A {solo_a:.1} dB, B {solo_b:.1} dB; im Übergang höchstens {:.1} dB, mindestens {:.1} dB", max(&during), during.iter().cloned().fold(f64::MAX, f64::min));
    assert!(max(&during) <= reference + 3.0, "Bass im Übergang {:.1} dB über dem Einzelpegel", max(&during) - reference);
    // Der Bass fällt im Übergang auch nicht weg: Vor dem Tausch trägt A, danach B.
    assert!(during.iter().all(|&l| l >= reference - 6.0), "Bass-Loch im Übergang: {during:.1?}");

    // Master bleibt unter der Decke des Limiters.
    assert!(mix.peak() <= 10f32.powf(-0.3 / 20.0) + 1e-6);

    // Gegenprobe: Laufen beide Tracks ohne EQ gleichzeitig, schlägt dieselbe Messung an.
    let mut e = Engine::new(SR);
    e.apply(Command::SetMaster { deck: None });
    e.apply(Command::ClockTempo { bpm: 132.0 });
    e.apply(Command::Limiter { on: false });
    e.load(DeckId::A, a);
    e.load(DeckId::B, b);
    e.apply(Command::Seek { deck: DeckId::A, pos: plan.a_out });
    e.apply(Command::Seek { deck: DeckId::B, pos: plan.b_in });
    for deck in [DeckId::A, DeckId::B] {
        e.apply(Command::Sync { deck, mode: SyncMode::Beat });
        e.schedule(4.0, Command::Play { deck, on: true });
    }
    let (both, _) = dw_engine::render(&mut e, 12 * bar);
    let doubled = max(&low_band_levels(&both, 4 * bar, 12 * bar, bar));
    println!("Gegenprobe mit zwei offenen Bässen: {:.1} dB über dem Einzelpegel", doubled - reference);
    assert!(doubled > reference + 3.0, "die Messung erkennt doppelten Bass nicht");
}

#[test]
fn tempo_is_matched_across_the_transition() {
    let (a, an_a) = full_track(132.0, Key::from_camelot(8, 'A').unwrap(), 96, 0.1, 0, 21);
    let (b, an_b) = full_track(135.3, Key::from_camelot(8, 'A').unwrap(), 96, 0.1, 0, 22);
    let plan = plan_blend(&an_a, &an_b, 32, 16, 24).unwrap();
    // 2,5 % Abstand: Verlauf über den ganzen Übergang, höchstens 0,25 % je Takt.
    assert_eq!(plan.tempo_ramp, (0.0, 32.0));
    assert!(plan.tempo_gap_percent() / 32.0 <= 0.25);

    let (mix, report) = render_blend(a, b, &plan);
    assert!(report.max_phase_error < 1e-3, "Beat-Versatz {} trotz Tempo-Verlauf", report.max_phase_error);
    // Vor dem Übergang läuft der Mix im Tempo von A, danach im Tempo von B.
    let part = |from: usize, to: usize| AudioBuf::new(SR, mix.data[from * 2..to * 2].to_vec());
    let before = analyze(&part(0, report.t0_frame), &AnalysisConfig::default());
    let after = analyze(&part(report.end_frame, mix.frames()), &AnalysisConfig::default());
    println!("Tempo vor dem Übergang {:.2}, danach {:.2} BPM", before.bpm, after.bpm);
    assert!((before.bpm - 132.0).abs() < 0.05, "vorher {}", before.bpm);
    assert!((after.bpm - 135.3).abs() < 0.05, "nachher {}", after.bpm);
}

#[test]
fn planning_rejects_what_does_not_fit() {
    let (_, slow) = full_track(126.0, Key::new(0, false), 64, 0.1, 0, 31);
    let (_, fast) = full_track(140.0, Key::new(0, false), 64, 0.1, 0, 32);
    let (_, short) = full_track(126.0, Key::new(0, false), 24, 0.1, 0, 33);
    assert!(matches!(plan_blend(&slow, &fast, 32, 16, 16), Err(PlanError::TempoGap(..))));
    assert_eq!(plan_blend(&slow, &slow, 20, 16, 16), Err(PlanError::BadLength));
    assert_eq!(plan_blend(&short, &slow, 32, 16, 16), Err(PlanError::TooShort('A', 32)));
    assert_eq!(plan_blend(&slow, &short, 32, 16, 16), Err(PlanError::TooShort('B', 32)));
    // Ein großer Abstand braucht Vor- und Nachlauf für den Tempo-Verlauf.
    let (_, faster) = full_track(133.0, Key::new(0, false), 96, 0.1, 0, 34);
    let plan = plan_blend(&slow, &faster, 16, 16, 16).unwrap();
    assert!(plan.tempo_ramp.0 < 0.0 && plan.tempo_ramp.1 > 16.0, "{:?}", plan.tempo_ramp);
}
