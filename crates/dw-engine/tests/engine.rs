//! Engine-Tests per Offline-Render (Spezifikation 10.2): Sync, zeitgenaue Befehle,
//! Tempo, Crossfader, Kopfhörer-Weg, Wiederholbarkeit.

mod common;

use common::*;
use dw_core::{Band, Command, DeckId, SyncMode, XfAssign};
use dw_engine::{render, Engine};

use DeckId::{A, B};

fn run(engine: &mut Engine, seconds: f64, finders: &mut [&mut ClickFinder]) {
    let mut m = vec![0.0f32; 4096 * 2];
    let mut c = vec![0.0f32; 4096 * 2];
    let mut left = (seconds * SR as f64) as usize;
    while left > 0 {
        let n = left.min(4096);
        engine.process(&mut m[..n * 2], &mut c[..n * 2]);
        for f in finders.iter_mut() {
            f.feed(&m[..n * 2]);
        }
        left -= n;
    }
}

/// DECK-08: Synchronisierte Decks bleiben über 10 Minuten innerhalb von 1 ms deckungsgleich.
#[test]
fn beat_sync_holds_for_ten_minutes() {
    let mut e = Engine::new(SR);
    // A: 128 BPM, nur links. B: 133,37 BPM mit anderem Versatz, nur rechts.
    e.load(A, click_track(128.0, 0.10, 1300, 0));
    e.load(B, click_track(133.37, 0.25, 1300, 1));
    e.apply(Command::Sync { deck: B, mode: SyncMode::Beat });
    e.apply(Command::Play { deck: A, on: true });
    let (mut fl, mut fr) = (ClickFinder::new(0, 0.2), ClickFinder::new(1, 0.2));
    run(&mut e, 3.3, &mut [&mut fl, &mut fr]);
    e.apply(Command::Play { deck: B, on: true });
    run(&mut e, 600.0, &mut [&mut fl, &mut fr]);

    assert!(fl.clicks.len() > 1270 && fr.clicks.len() > 1260, "{} / {} Klicks", fl.clicks.len(), fr.clicks.len());
    // Jeder Klick von B liegt auf einem Klick von A.
    let mut worst = 0i64;
    for &r in &fr.clicks {
        let i = fl.clicks.partition_point(|&l| l < r);
        let d = [i.checked_sub(1), Some(i)]
            .into_iter()
            .flatten()
            .filter_map(|j| fl.clicks.get(j))
            .map(|&l| (l as i64 - r as i64).abs())
            .min()
            .unwrap();
        worst = worst.max(d);
    }
    let worst_ms = worst as f64 / SR as f64 * 1000.0;
    println!("Sync über 10 min: größte Abweichung {worst} Samples = {worst_ms:.4} ms");
    assert!(worst_ms <= 1.0, "Abweichung {worst_ms} ms");

    let s = e.snapshot();
    assert_eq!(s.master, Some(A));
    assert!((s.clock_bpm - 128.0).abs() < 1e-9);
    assert!((s.decks[1].bpm - 128.0).abs() < 1e-3, "B läuft mit {}", s.decks[1].bpm);
    assert!(s.decks[1].phase.abs() < 1e-3, "Phase {}", s.decks[1].phase);
}

/// 7.4: Ein vorgemerkter Befehl wird sample-genau ausgeführt (±1 Sample).
#[test]
fn scheduled_start_is_sample_accurate() {
    // Vergleichslauf: dasselbe Deck startet bei Frame 0. Das liefert die Laufzeit durch EQ und Limiter.
    let track = || click_track(120.0, 0.01, 64, 2);
    let mut reference = Engine::new(SR);
    reference.apply(Command::SetMaster { deck: None });
    reference.load(A, track());
    reference.apply(Command::Play { deck: A, on: true });
    let mut fr = ClickFinder::new(0, 0.2);
    run(&mut reference, 2.0, &mut [&mut fr]);

    let mut e = Engine::new(SR);
    e.apply(Command::SetMaster { deck: None }); // freie Clock, 120 BPM
    e.load(A, track());
    assert!(e.schedule(8.0, Command::Play { deck: A, on: true }));
    let mut f = ClickFinder::new(0, 0.2);
    run(&mut e, 8.0, &mut [&mut f]);
    // Beat 8 der Clock liegt bei 4 s = Frame 192 000.
    let expected = fr.clicks[0] as i64 + 192_000;
    assert!((f.clicks[0] as i64 - expected).abs() <= 1, "erster Klick bei {}, erwartet {expected}", f.clicks[0]);
    for w in f.clicks.windows(2) {
        assert!((w[1] as i64 - w[0] as i64 - 24_000).abs() <= 1);
    }
}

/// 7.4 mit Beat-Sync: Ein vorgemerkter Start eines synchronisierten Decks liegt exakt auf dem Clock-Beat.
#[test]
fn scheduled_synced_start_lands_on_the_beat() {
    // Laufzeit durch EQ und Limiter aus einem Vergleichslauf.
    let mut reference = Engine::new(SR);
    reference.load(A, click_track(126.0, 0.5, 8, 2));
    reference.apply(Command::Play { deck: A, on: true });
    let mut fr = ClickFinder::new(0, 0.2);
    run(&mut reference, 2.0, &mut [&mut fr]);
    let path_delay = fr.clicks[0] as f64 - 0.5 * SR as f64;

    let mut e = Engine::new(SR);
    e.apply(Command::SetMaster { deck: None });
    e.apply(Command::ClockTempo { bpm: 130.0 });
    // Track mit 126 BPM; das Deck steht auf Track-Beat 4 und startet auf Clock-Beat 16.
    e.load(A, click_track(126.0, 0.5, 64, 2));
    e.apply(Command::Seek { deck: A, pos: 0.5 + 4.0 * 60.0 / 126.0 });
    e.apply(Command::Sync { deck: A, mode: SyncMode::Beat });
    e.schedule(16.0, Command::Play { deck: A, on: true });
    let mut f = ClickFinder::new(0, 0.2);
    run(&mut e, 20.0, &mut [&mut f]);
    let beat_frames = 60.0 / 130.0 * SR as f64;
    // Der Klick am Startpunkt ist angeschnitten; ab dem zweiten liegt jeder Klick auf seinem Clock-Beat.
    println!("Klicks: {:?}", &f.clicks[..4]);
    for (k, &c) in f.clicks.iter().enumerate().skip(1) {
        let expected = (16 + k) as f64 * beat_frames + path_delay;
        assert!((c as f64 - expected).abs() <= 2.0, "Klick {k} bei {c}, erwartet {expected:.1}");
    }
    assert!(f.clicks.len() > 20);
}

/// DECK-06: Der Tempo-Regler ändert das Tempo um genau den eingestellten Faktor.
#[test]
fn tempo_offset_scales_beat_distance() {
    let mut e = Engine::new(SR);
    e.load(A, click_track(120.0, 0.05, 200, 2));
    e.apply(Command::Tempo { deck: A, offset: 0.06 });
    e.apply(Command::Play { deck: A, on: true });
    let mut f = ClickFinder::new(0, 0.2);
    run(&mut e, 60.0, &mut [&mut f]);
    let first = f.clicks[0] as f64;
    let last = *f.clicks.last().unwrap() as f64;
    let spacing = (last - first) / (f.clicks.len() - 1) as f64;
    assert!((spacing - 24_000.0 / 1.06).abs() < 0.05, "Beat-Abstand {spacing}");
    let s = e.snapshot();
    assert!((s.decks[0].bpm - 127.2).abs() < 1e-9);
    assert!((s.clock_bpm - 127.2).abs() < 1e-9);
}

/// DECK-09: Übernimmt ein synchronisiertes Deck die Master-Rolle, bleiben Tempo und Phase erhalten.
#[test]
fn master_handover_keeps_tempo_and_phase() {
    let mut e = Engine::new(SR);
    e.load(A, click_track(128.0, 0.0, 400, 0));
    e.load(B, click_track(133.37, 0.2, 400, 1));
    e.apply(Command::Sync { deck: B, mode: SyncMode::Beat });
    e.apply(Command::Play { deck: A, on: true });
    e.apply(Command::Play { deck: B, on: true });
    let mut fr = ClickFinder::new(1, 0.2);
    run(&mut e, 20.0, &mut [&mut fr]);
    e.apply(Command::Play { deck: A, on: false });
    run(&mut e, 20.0, &mut [&mut fr]);
    let s = e.snapshot();
    assert_eq!(s.master, Some(B));
    assert!((s.clock_bpm - 128.0).abs() < 1e-6, "Clock {}", s.clock_bpm);
    // Der erste Klick von B ist am Startpunkt angeschnitten und wird ausgelassen.
    for w in fr.clicks[1..].windows(2) {
        let d = w[1] as i64 - w[0] as i64;
        assert!((d - 22_500).abs() <= 1, "Beat-Abstand {d} nach der Übergabe");
    }
}

/// ENG-07: Zwei Läufe desselben Ablaufs sind bitgenau gleich.
#[test]
fn offline_render_is_deterministic() {
    let scene = || {
        let mut e = Engine::new(SR);
        e.load(A, noise_track(1, 12.0, 0.3));
        e.load(B, click_track(131.0, 0.1, 30, 2));
        e.apply(Command::Sync { deck: B, mode: SyncMode::Beat });
        e.apply(Command::Tempo { deck: A, offset: 0.031 });
        e.apply(Command::Play { deck: A, on: true });
        e.schedule(4.0, Command::Play { deck: B, on: true });
        e.schedule(6.0, Command::Eq { deck: A, band: Band::Low, db: f32::NEG_INFINITY });
        e.schedule(9.5, Command::Filter { deck: A, pos: 0.4 });
        e.schedule(12.0, Command::Seek { deck: A, pos: 1.0 });
        e.schedule(14.0, Command::Crossfader { pos: 0.3 });
        render(&mut e, 10 * SR as usize)
    };
    let (m1, c1) = scene();
    let (m2, c2) = scene();
    assert!(m1.data == m2.data && c1.data == c2.data);
    assert!(m1.peak() > 0.1);
}

/// MIX-06: Kurve „weich" hält die Summenleistung über den ganzen Weg konstant (±0,2 dB).
#[test]
fn crossfader_smooth_curve_is_constant_power() {
    let mut e = Engine::new(SR);
    e.load(A, noise_track(11, 30.0, 0.2));
    e.load(B, noise_track(22, 30.0, 0.2));
    e.apply(Command::XfAssign { deck: A, assign: XfAssign::Left });
    e.apply(Command::XfAssign { deck: B, assign: XfAssign::Right });
    e.apply(Command::Play { deck: A, on: true });
    e.apply(Command::Play { deck: B, on: true });
    let mut levels = Vec::new();
    for pos in [-1.0, -0.6, -0.2, 0.0, 0.3, 0.7, 1.0] {
        e.apply(Command::Crossfader { pos });
        let (m, _) = render(&mut e, SR as usize * 2);
        levels.push(db(rms(&m.data[SR as usize..])));
    }
    let (lo, hi) = levels.iter().fold((f64::MAX, f64::MIN), |(l, h), &v| (l.min(v), h.max(v)));
    println!("Crossfader-Pegel: {levels:.2?}");
    assert!(hi - lo <= 0.4, "Schwankung {:.2} dB", hi - lo);
}

/// MIX-09: Das Cue-Signal erscheint nur auf dem Kopfhörer; MIX-05: Fader unten = stumm.
#[test]
fn cue_goes_to_headphones_only() {
    let mut e = Engine::new(SR);
    e.load(A, sine_track(440.0, 10.0, 0.25));
    e.load(B, sine_track(1000.0, 10.0, 0.25));
    e.apply(Command::Fader { deck: B, value: 0.0 });
    e.apply(Command::Cue { deck: B, on: true });
    e.apply(Command::Play { deck: A, on: true });
    e.apply(Command::Play { deck: B, on: true });
    let (m, c) = render(&mut e, SR as usize);
    let (m, c) = (&m.data[9600..], &c.data[9600..]);
    let expected = 0.25 / 2f64.sqrt();
    // Master: nur A (440 Hz). Kopfhörer bei Cue-Mix 0: nur B (1 kHz).
    assert!((db(rms(m)) - db(expected)).abs() < 0.1, "Master {:.2} dB", db(rms(m)));
    assert!((db(rms(c)) - db(expected)).abs() < 0.1, "Kopfhörer {:.2} dB", db(rms(c)));
    assert!(tone_level(m, 1000.0) < 1e-4, "B ist im Master hörbar");
    assert!(tone_level(c, 440.0) < 1e-4, "A ist im Cue-Signal");

    // Cue-Mix 1: Kopfhörer = Master.
    e.apply(Command::CueMix { value: 1.0 });
    let (m, c) = render(&mut e, SR as usize);
    assert_eq!(m.data[9600..], c.data[9600..]);
}

/// Amplitude eines Sinus bei `hz` im linken Kanal.
fn tone_level(interleaved: &[f32], hz: f64) -> f64 {
    let n = interleaved.len() / 2;
    let (mut a, mut b) = (0.0, 0.0);
    for i in 0..n {
        let ph = 2.0 * std::f64::consts::PI * hz * i as f64 / SR as f64;
        a += interleaved[i * 2] as f64 * ph.sin();
        b += interleaved[i * 2] as f64 * ph.cos();
    }
    2.0 * (a * a + b * b).sqrt() / n as f64
}

/// Der Weg vom Deck zum Master ist bei neutralen Reglern pegelneutral; Auto-Gain wirkt.
#[test]
fn signal_path_is_level_neutral() {
    let mut e = Engine::new(SR);
    e.load(A, sine_track(1000.0, 5.0, 0.25));
    e.apply(Command::Play { deck: A, on: true });
    let (m, _) = render(&mut e, SR as usize);
    let level = db(rms(&m.data[9600..]));
    assert!((level - db(0.25 / 2f64.sqrt())).abs() < 0.02, "Pegel {level:.3} dB");

    e.apply(Command::Gain { deck: A, db: -6.0 });
    let (m, _) = render(&mut e, SR as usize);
    let quieter = db(rms(&m.data[9600..]));
    assert!((level - quieter - 6.0).abs() < 0.02);
}

/// Stopp blendet kurz aus und hält die Position; ein Sprung im laufenden Deck knackt nicht.
#[test]
fn stop_holds_position_and_seek_is_click_free() {
    let mut e = Engine::new(SR);
    e.load(A, sine_track(100.0, 20.0, 0.5));
    e.apply(Command::Play { deck: A, on: true });
    render(&mut e, 48_000);
    e.apply(Command::Play { deck: A, on: false });
    let pos = e.snapshot().decks[0].pos;
    render(&mut e, 4_800);
    let s = e.snapshot();
    assert!(!s.decks[0].playing);
    assert!((s.decks[0].pos - pos).abs() < 1e-9, "Position nach Stopp verschoben");

    e.apply(Command::Play { deck: A, on: true });
    render(&mut e, 4_800);
    // Sprung auf eine Stelle mit anderer Phasenlage des Sinus
    e.apply(Command::Seek { deck: A, pos: 7.0037 });
    let (m, _) = render(&mut e, 4_800);
    // Größter Sprung zwischen zwei Samples: Ein 100-Hz-Sinus mit Amplitude 0,5 ändert sich um höchstens 0,0066 je Sample.
    let max_step = m.data.chunks_exact(2).map(|f| f[0]).collect::<Vec<_>>().windows(2).map(|w| (w[1] - w[0]).abs()).fold(0.0f32, f32::max);
    assert!(max_step < 0.02, "Sprungstelle {max_step}");
    assert!((e.snapshot().decks[0].pos - 7.0037).abs() < 0.2);
}

/// Am Trackende hält das Deck an.
#[test]
fn deck_stops_at_track_end() {
    let mut e = Engine::new(SR);
    e.load(A, sine_track(200.0, 1.0, 0.2));
    e.apply(Command::Play { deck: A, on: true });
    render(&mut e, 2 * SR as usize);
    let s = e.snapshot();
    assert!(!s.decks[0].playing);
    assert!((s.decks[0].pos - 1.0).abs() < 1e-6);
    assert_eq!(s.master, None);
}

/// 9.1: Offline-Render mindestens mit 10-facher Echtzeit (zwei Decks, eines umgerechnet).
#[test]
fn offline_render_speed() {
    let mut e = Engine::new(SR);
    e.load(A, noise_track(5, 70.0, 0.2));
    e.load(B, noise_track(6, 70.0, 0.2));
    e.apply(Command::Sync { deck: B, mode: SyncMode::Tempo });
    e.apply(Command::Tempo { deck: A, offset: 0.03 });
    e.apply(Command::Filter { deck: A, pos: 0.3 });
    e.apply(Command::Play { deck: A, on: true });
    e.apply(Command::Play { deck: B, on: true });
    let t = std::time::Instant::now();
    let (m, _) = render(&mut e, 60 * SR as usize);
    let factor = 60.0 / t.elapsed().as_secs_f64();
    println!("Offline-Render: {factor:.0}-fache Echtzeit");
    assert!(m.peak() > 0.05);
    assert!(factor >= 10.0, "nur {factor:.1}-fache Echtzeit");
}

/// DECK-09: Die Automatik wählt das hörbare Deck. Wird der Master ausgeblendet, übernimmt das
/// andere laufende Deck, ohne dass sich Tempo oder Phase ändern.
#[test]
fn auto_master_follows_the_audible_deck() {
    let mut e = Engine::new(SR);
    e.load(A, click_track(128.0, 0.0, 400, 0));
    e.load(B, click_track(133.37, 0.2, 400, 1));
    e.apply(Command::Sync { deck: B, mode: SyncMode::Beat });
    e.apply(Command::Play { deck: A, on: true });
    e.apply(Command::Play { deck: B, on: true });
    let mut fr = ClickFinder::new(1, 0.2);
    run(&mut e, 10.0, &mut [&mut fr]);
    assert_eq!(e.snapshot().master, Some(A));

    // A ausblenden: B ist jetzt das einzige hörbare Deck und wird Master.
    e.apply(Command::Fader { deck: A, value: 0.0 });
    run(&mut e, 10.0, &mut [&mut fr]);
    let s = e.snapshot();
    assert_eq!(s.master, Some(B));
    assert!(s.decks[0].playing, "A läuft unhörbar weiter");
    assert!((s.clock_bpm - 128.0).abs() < 1e-6, "Clock {}", s.clock_bpm);
    for w in fr.clicks[1..].windows(2) {
        let d = w[1] as i64 - w[0] as i64;
        assert!((d - 22_500).abs() <= 1, "Beat-Abstand {d} nach der Übergabe");
    }

    // Beide unhörbar: Der Master bleibt, solange er läuft.
    e.apply(Command::Fader { deck: B, value: 0.0 });
    run(&mut e, 1.0, &mut [&mut fr]);
    assert_eq!(e.snapshot().master, Some(B));

    // Crossfader ganz von B weg, A wieder offen: A ist hörbar und übernimmt.
    e.apply(Command::Fader { deck: A, value: 1.0 });
    e.apply(Command::Fader { deck: B, value: 1.0 });
    e.apply(Command::XfAssign { deck: A, assign: XfAssign::Left });
    e.apply(Command::XfAssign { deck: B, assign: XfAssign::Right });
    e.apply(Command::Crossfader { pos: -1.0 });
    run(&mut e, 1.0, &mut [&mut fr]);
    assert_eq!(e.snapshot().master, Some(A));
}

/// VIS-13: Start mit Vorlauf. Das stehende Deck springt so weit zurück, dass es seine jetzige
/// Stelle nach der genannten Zahl Clock-Beats erreicht – auf 1 ms genau.
#[test]
fn play_in_reaches_the_cued_position_after_the_lead() {
    let mut e = Engine::new(SR);
    e.load(A, click_track(120.0, 0.0, 400, 0));
    e.load(B, click_track(126.0, 0.0, 400, 1));
    // B läuft mit eigenem Tempo (+2 %), ohne Sync.
    e.apply(Command::Tempo { deck: B, offset: 0.02 });
    e.apply(Command::Seek { deck: B, pos: 30.0 });
    e.apply(Command::Play { deck: A, on: true });
    run(&mut e, 2.0, &mut []);
    assert!((e.snapshot().decks[1].pos - 30.0).abs() < 1e-9);

    // 16 Beats bei 120 BPM = 8 s Echtzeit; B legt darin 8 s · 1,02 zurück.
    e.apply(Command::PlayIn { deck: B, beats: 16.0 });
    let s = e.snapshot();
    assert!(s.decks[1].playing);
    assert!((s.decks[1].pos - (30.0 - 8.0 * 1.02)).abs() < 1e-9, "Start bei {}", s.decks[1].pos);
    assert_eq!(s.master, Some(A), "das laufende Deck bleibt Master");
    run(&mut e, 8.0, &mut []);
    let pos = e.snapshot().decks[1].pos;
    assert!((pos - 30.0).abs() <= 0.001, "nach dem Vorlauf bei {pos} s statt 30 s");
}

/// VIS-13: Liegt der Start vor dem Anfang des Tracks, läuft das Deck bis dorthin stumm.
#[test]
fn play_in_before_the_track_start_plays_silence_first() {
    let mut e = Engine::new(SR);
    e.load(A, click_track(120.0, 0.0, 400, 0));
    e.load(B, click_track(120.0, 0.0, 400, 1));
    e.apply(Command::Seek { deck: B, pos: 2.0 });
    e.apply(Command::Play { deck: A, on: true });
    run(&mut e, 1.0, &mut []);

    // 8 s Vorlauf ab Sekunde 2: Der Track beginnt erst nach 6 s.
    e.apply(Command::PlayIn { deck: B, beats: 16.0 });
    let s = e.snapshot();
    assert!(s.decks[1].playing);
    assert!((s.decks[1].pos + 6.0).abs() < 1e-9, "Start bei {}", s.decks[1].pos);
    let mut fr = ClickFinder::new(1, 0.2);
    run(&mut e, 8.0, &mut [&mut fr]);
    // Erster Klick von B (Beat 0 bei Sekunde 0) nach 6 s plus Laufzeit der Engine, auf 1 ms genau.
    let expected = 6.0 * SR as f64 + e.latency() as f64;
    let first = *fr.clicks.first().expect("B bleibt stumm") as f64;
    assert!((first - expected).abs() <= 0.001 * SR as f64, "erster Klick bei Frame {first}, erwartet {expected}");
    assert!((e.snapshot().decks[1].pos - 2.0).abs() <= 0.001);
}

/// VIS-13 mit DECK-08: Ein Deck im Beat-Sync rechnet den Vorlauf im Tempo der Clock und rastet
/// beim Start ein – es erreicht seine Stelle auf höchstens einen halben Beat genau und im Takt.
#[test]
fn play_in_with_beat_sync_stays_in_phase() {
    let mut e = Engine::new(SR);
    e.load(A, click_track(120.0, 0.0, 400, 0));
    e.load(B, click_track(126.0, 0.0, 400, 1));
    e.apply(Command::Sync { deck: B, mode: SyncMode::Beat });
    let cued = 64.0 * 60.0 / 126.0; // Beat 64 von B
    e.apply(Command::Seek { deck: B, pos: cued });
    e.apply(Command::Play { deck: A, on: true });
    run(&mut e, 2.1, &mut []);

    e.apply(Command::PlayIn { deck: B, beats: 16.0 });
    run(&mut e, 8.0, &mut []);
    let s = e.snapshot();
    let half_beat = 0.5 * 60.0 / 126.0;
    assert!((s.decks[1].pos - cued).abs() <= half_beat + 0.001, "B bei {} s statt {cued} s", s.decks[1].pos);
    assert!(s.decks[1].phase.abs() < 1e-3, "Phase {}", s.decks[1].phase);
    assert!((s.decks[1].bpm - 120.0).abs() < 1e-3);
}

/// DECK-20: Ein Sprung im Takt erhält die Phase, auch wenn das Ziel aus einer veralteten Position
/// berechnet wurde (die Oberfläche kennt die Position nur mit einigen Millisekunden Verzug).
/// Geprüft am Master-Deck und an einem Deck im Beat-Sync; danach liegen die Klicks beider Decks
/// wieder innerhalb von 1 ms übereinander.
#[test]
fn snapped_seek_keeps_the_phase_whatever_the_delay() {
    let mut e = Engine::new(SR);
    e.load(A, click_track(120.0, 0.0, 600, 0));
    e.load(B, click_track(126.0, 0.0, 600, 1));
    e.apply(Command::Sync { deck: B, mode: SyncMode::Beat });
    e.apply(Command::Play { deck: A, on: true });
    e.apply(Command::Play { deck: B, on: true });
    run(&mut e, 4.0, &mut []);

    // Ziel wie in der Oberfläche: ganze Beats ab der bekannten Position – die aber 20 ms alt ist.
    let stale = e.snapshot().decks[0].pos;
    run(&mut e, 0.020, &mut []);

    // Zum Vergleich der einfache Sprung: Der Verzug bleibt als Versatz des Masters zu seiner Clock stehen.
    let mut plain = Engine::new(SR);
    plain.load(A, click_track(120.0, 0.0, 600, 0));
    plain.apply(Command::Play { deck: A, on: true });
    run(&mut plain, 4.02, &mut []);
    plain.apply(Command::Seek { deck: A, pos: stale + 32.0 * 0.5 });
    run(&mut plain, 1.0, &mut []);
    let off = plain.snapshot().decks[0].phase;
    println!("einfacher Sprung mit 20 ms Verzug: Phase des Masters {off:.4} Beats");
    assert!(off.abs() > 0.03, "Vergleichsfall zeigt den Fehler nicht: {off}");

    // Sprung im Takt am Master (A).
    e.apply(Command::SeekSnap { deck: A, pos: stale + 32.0 * 0.5 });
    run(&mut e, 1.0, &mut []);
    let s = e.snapshot();
    assert_eq!(s.master, Some(A));
    assert!(s.decks[0].phase.abs() < 1e-3, "Master nach dem Sprung: Phase {}", s.decks[0].phase);
    assert!(s.decks[1].phase.abs() < 1e-3, "B nach dem Sprung von A: Phase {}", s.decks[1].phase);
    // Gelandet wird höchstens einen halben Beat neben dem Ziel (hier: 20 ms Verzug + 1 s Laufzeit danach).
    assert!((s.decks[0].pos - (stale + 16.0 + 1.02)).abs() < 0.26, "A bei {}", s.decks[0].pos);

    // Sprung im Takt am Deck im Beat-Sync (B), Ziel mitten zwischen zwei Beats.
    e.apply(Command::SeekSnap { deck: B, pos: 100.123 });
    run(&mut e, 1.0, &mut []);
    let s = e.snapshot();
    assert!(s.decks[1].phase.abs() < 1e-3, "B nach eigenem Sprung: Phase {}", s.decks[1].phase);
    assert!((s.decks[1].pos - (100.123 + 120.0 / 126.0)).abs() < 0.26, "B bei {}", s.decks[1].pos);

    // Hörbar im Takt: Jeder Klick von B liegt auf einem Klick von A.
    let (mut fl, mut fr) = (ClickFinder::new(0, 0.2), ClickFinder::new(1, 0.2));
    run(&mut e, 20.0, &mut [&mut fl, &mut fr]);
    assert!(fl.clicks.len() >= 38 && fr.clicks.len() >= 38, "{} / {} Klicks", fl.clicks.len(), fr.clicks.len());
    let worst = fr.clicks.iter().map(|&r| fl.clicks.iter().map(|&l| (l as i64 - r as i64).abs()).min().unwrap()).max().unwrap();
    let worst_ms = worst as f64 / SR as f64 * 1000.0;
    println!("nach zwei Sprüngen im Takt: größte Abweichung {worst_ms:.4} ms");
    assert!(worst_ms <= 1.0, "Abweichung {worst_ms} ms");

    // Ein stehendes Deck springt genau auf das Ziel.
    e.apply(Command::Play { deck: B, on: false });
    run(&mut e, 0.1, &mut []);
    e.apply(Command::SeekSnap { deck: B, pos: 50.123 });
    run(&mut e, 0.1, &mut []);
    assert!((e.snapshot().decks[1].pos - 50.123).abs() < 1e-9);
}
