//! Der Weg durch die Anwendung ohne Fenster: Import → Hintergrund-Analyse → Laden → Abspielen.
//! In dieser Umgebung gibt es kein Audiogerät; die Engine läuft dann stumm im Zeittakt.

use deckwerk_app::state::AppState;
use deckwerk_app::worker;
use dw_core::{Command, DeckId, Key, Waveform, XfAssign, ANALYSIS_VERSION};
use dw_library::{SortColumn, TrackQuery};
use dw_testgen::{techno_track, SectionKind, SectionSpec, TrackSpec};
use std::time::{Duration, Instant};

fn write_track(dir: &std::path::Path, name: &str, bpm: f64, key: Key, seed: u64) {
    let mut spec = TrackSpec::new(bpm, key, vec![SectionSpec::new(SectionKind::Intro, 8), SectionSpec::new(SectionKind::Main, 24)], seed);
    spec.sample_rate = 44_100;
    let (audio, _) = techno_track(&spec);
    dw_decode::write_wav(&dir.join(name), &audio, 16).unwrap();
}

fn wait_for(what: &str, mut done: impl FnMut() -> bool) {
    let start = Instant::now();
    while !done() {
        assert!(start.elapsed() < Duration::from_secs(60), "Zeit abgelaufen: {what}");
        std::thread::sleep(Duration::from_millis(50));
    }
}

#[test]
fn import_analyse_load_and_play() {
    let tmp = tempfile::tempdir().unwrap();
    let music = tmp.path().join("musik");
    std::fs::create_dir_all(&music).unwrap();
    write_track(&music, "Erster.wav", 128.0, Key::from_camelot(8, 'A').unwrap(), 1);
    write_track(&music, "Zweiter.wav", 133.5, Key::from_camelot(9, 'A').unwrap(), 2);

    let state = AppState::start(&tmp.path().join("daten")).unwrap();
    worker::spawn(state.clone());
    let rev = state.library_rev.load(std::sync::atomic::Ordering::Relaxed);
    let report = state.import(&[music]).unwrap();
    assert_eq!(report.added, 2);
    assert!(state.library_rev.load(std::sync::atomic::Ordering::Relaxed) > rev);

    // Die Hintergrund-Analyse arbeitet beide Tracks ab.
    wait_for("Analyse", || state.lib.lock().unwrap().pending_analysis(ANALYSIS_VERSION).unwrap().is_empty());
    let rows = state.lib.lock().unwrap().tracks(&TrackQuery { sort: SortColumn::Title, ..Default::default() }).unwrap();
    assert_eq!(rows.len(), 2);
    assert!((rows[0].bpm.unwrap() - 128.0).abs() < 0.05 && (rows[1].bpm.unwrap() - 133.5).abs() < 0.05);
    assert_eq!(rows[0].key, Key::from_camelot(8, 'A'));

    // Wellenform liegt im Cache und ist lesbar.
    let wave = Waveform::from_bytes(&state.waveform_bytes(rows[0].id).unwrap()).unwrap();
    assert_eq!(wave.sample_rate, 44_100);
    assert_eq!(wave.overview.points(), 2000);
    assert_eq!(wave.levels.len(), 4);

    // Beide Tracks laden (44,1 kHz → Abtastrate der Engine), synchron starten.
    state.load_deck(DeckId::A, rows[0].id).unwrap();
    state.load_deck(DeckId::B, rows[1].id).unwrap();
    assert_eq!(*state.loaded.lock().unwrap(), [Some(rows[0].id), Some(rows[1].id), None, None]);
    {
        let mut ctl = state.ctl.lock().unwrap();
        ctl.send(Command::Sync { deck: DeckId::B, mode: dw_core::SyncMode::Beat });
        ctl.send(Command::Play { deck: DeckId::A, on: true });
        ctl.send(Command::Play { deck: DeckId::B, on: true });
    }
    wait_for("Wiedergabe", || {
        let mut ctl = state.ctl.lock().unwrap();
        let s = ctl.snapshot();
        s.decks[0].playing && s.decks[1].playing && s.decks[0].pos > 0.5
    });
    let snap = state.ctl.lock().unwrap().snapshot().clone();
    assert_eq!(snap.master, Some(DeckId::A));
    assert!((snap.clock_bpm - 128.0).abs() < 0.05, "Clock {}", snap.clock_bpm);
    assert!((snap.decks[1].bpm - 128.0).abs() < 0.05, "B läuft mit {}", snap.decks[1].bpm);
    assert!(snap.decks[1].phase.abs() < 0.01, "Phase {}", snap.decks[1].phase);
    assert!((snap.decks[0].duration - rows[0].duration).abs() < 0.01);
    println!("Audiogerät: {:?}", state.audio);
}

/// Spitzenpegel von Deck A (vor dem Fader) und Master über ein Zeitfenster.
fn peaks_over(state: &AppState, window: Duration) -> (f32, f32) {
    // Erst einschwingen lassen und Altes verwerfen, dann messen.
    std::thread::sleep(Duration::from_millis(300));
    state.ctl.lock().unwrap().snapshot();
    std::thread::sleep(window);
    let mut ctl = state.ctl.lock().unwrap();
    let s = ctl.snapshot();
    (s.decks[0].peak[0].max(s.decks[0].peak[1]), s.mixer.master_peak[0].max(s.mixer.master_peak[1]))
}

/// MIX-06: Die Anwendung startet mit A/C links und B/D rechts am Crossfader. Ohne diese
/// Zuweisung bleibt der Regler wirkungslos (Befund der Abnahme am Gerät unter macOS).
#[test]
fn crossfader_fades_deck_a_out() {
    let tmp = tempfile::tempdir().unwrap();
    let music = tmp.path().join("musik");
    std::fs::create_dir_all(&music).unwrap();
    write_track(&music, "Erster.wav", 128.0, Key::from_camelot(8, 'A').unwrap(), 1);

    let state = AppState::start(&tmp.path().join("daten")).unwrap();
    state.import(&[music]).unwrap();
    let rows = state.lib.lock().unwrap().tracks(&TrackQuery::default()).unwrap();
    state.load_deck(DeckId::A, rows[0].id).unwrap();

    wait_for("Zuweisung", || state.ctl.lock().unwrap().snapshot().decks[0].loaded);
    let snap = state.ctl.lock().unwrap().snapshot().clone();
    let assign: Vec<_> = snap.decks.iter().map(|d| d.xf_assign).collect();
    assert_eq!(assign, [XfAssign::Left, XfAssign::Right, XfAssign::Left, XfAssign::Right]);

    // Im Hauptteil des Tracks starten, damit durchgehend Pegel anliegt.
    {
        let mut ctl = state.ctl.lock().unwrap();
        ctl.send(Command::Seek { deck: DeckId::A, pos: 20.0 });
        ctl.send(Command::Play { deck: DeckId::A, on: true });
        ctl.send(Command::Crossfader { pos: -1.0 });
    }
    wait_for("Wiedergabe", || state.ctl.lock().unwrap().snapshot().decks[0].playing);
    let (deck, master) = peaks_over(&state, Duration::from_millis(600));
    assert!(deck > 0.05 && master > 0.05, "Crossfader bei A: Deck {deck}, Master {master}");

    // Ganz zur anderen Seite: A liegt weiter am Kanal an, der Master ist still (unter −80 dB).
    state.ctl.lock().unwrap().send(Command::Crossfader { pos: 1.0 });
    let (deck, master) = peaks_over(&state, Duration::from_millis(600));
    assert!(deck > 0.05, "Deck A läuft nicht mehr: {deck}");
    assert!(master < 1e-4, "Crossfader bei B, A noch auf dem Master: {master}");
}
