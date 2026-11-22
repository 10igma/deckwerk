//! Der Weg durch die Anwendung ohne Fenster: Import → Hintergrund-Analyse → Laden → Abspielen.
//! In dieser Umgebung gibt es kein Audiogerät; die Engine läuft dann stumm im Zeittakt.

use deckwerk_app::state::{add_folder, AppState, Settings};
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
    println!("Audiogerät: {:?}", state.audio.lock().unwrap());
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

/// ENG-01: Ein Gerätewechsel behält die Engine. Das geladene Deck läuft danach weiter, die
/// Wahl steht in den Einstellungen; ein unbekanntes Gerät wird abgelehnt, ohne den Stand der
/// Ausgabe oder die gespeicherte Wahl zu ändern. Läuft mit und ohne Audiogerät.
#[test]
fn switching_the_output_device_keeps_the_engine_running() {
    let tmp = tempfile::tempdir().unwrap();
    let music = tmp.path().join("musik");
    std::fs::create_dir_all(&music).unwrap();
    write_track(&music, "Erster.wav", 128.0, Key::from_camelot(8, 'A').unwrap(), 1);
    let data = tmp.path().join("daten");
    let settings_file = data.join("settings.json");

    let state = AppState::start(&data).unwrap();
    state.import(&[music]).unwrap();
    let rows = state.lib.lock().unwrap().tracks(&TrackQuery::default()).unwrap();
    state.load_deck(DeckId::A, rows[0].id).unwrap();
    state.ctl.lock().unwrap().send(Command::Play { deck: DeckId::A, on: true });
    wait_for("Wiedergabe", || state.ctl.lock().unwrap().snapshot().decks[0].pos > 0.2);
    let before = state.audio.lock().unwrap().clone();
    assert_eq!(before.device, None);

    // Unbekanntes Gerät: Fehler mit dem Namen, Ausgabe wie zuvor, nichts gespeichert.
    let error = state.select_device(Some("Gerät, das es nicht gibt".into())).unwrap_err();
    assert!(error.contains("Gerät, das es nicht gibt"), "{error}");
    let after = state.audio.lock().unwrap().clone();
    assert_eq!((after.has_device, after.device.clone(), after.sample_rate), (before.has_device, None, before.sample_rate));
    assert_eq!(Settings::load(&settings_file).audio_device, None);

    // Wechsel auf das Standardgerät: Gelingt er (es gibt ein Gerät), wird die Wahl gespeichert.
    let chosen = state.select_device(None);
    if before.has_device {
        assert_eq!(chosen.unwrap().device, None);
        assert_eq!(Settings::load(&settings_file).audio_device, None);
        assert!(settings_file.exists());
    }

    // Das Deck ist nach zwei Wechseln noch geladen und läuft weiter.
    let pos = state.ctl.lock().unwrap().snapshot().decks[0].pos;
    wait_for("Wiedergabe nach dem Wechsel", || {
        let mut ctl = state.ctl.lock().unwrap();
        let s = ctl.snapshot();
        s.decks[0].loaded && s.decks[0].playing && s.decks[0].pos > pos + 0.3
    });
    assert_eq!(state.engine_rate, before.sample_rate);
}

/// Die Einstellungen überstehen Schreiben und Lesen; eine fehlende oder kaputte Datei ergibt
/// die Vorgaben statt eines Fehlers.
#[test]
fn settings_roundtrip_and_fallback() {
    let tmp = tempfile::tempdir().unwrap();
    let file = tmp.path().join("settings.json");
    assert_eq!(Settings::load(&file), Settings::default());
    let chosen = Settings { audio_device: Some("Kopfhörer (Beispiel)".into()), music_folders: vec![tmp.path().join("musik")], refresh_on_start: false };
    chosen.save(&file).unwrap();
    assert_eq!(Settings::load(&file), chosen);
    // Eine Datei aus der Zeit vor den neuen Feldern: Die fehlenden bekommen ihre Vorgaben.
    std::fs::write(&file, br#"{ "audio_device": "Alt" }"#).unwrap();
    assert_eq!(Settings::load(&file), Settings { audio_device: Some("Alt".into()), ..Settings::default() });
    assert!(Settings::default().refresh_on_start);
    std::fs::write(&file, b"{ kaputt").unwrap();
    assert_eq!(Settings::load(&file), Settings::default());
}
/// BIB-19: „Bibliothek aktualisieren" findet neue Dateien in den gemerkten Ordnern, lässt bekannte
/// Tracks unberührt, ordnet verschobene neu zu und markiert fehlende.
#[test]
fn refresh_finds_new_files_in_the_remembered_folders() {
    let tmp = tempfile::tempdir().unwrap();
    let music = tmp.path().join("musik");
    std::fs::create_dir_all(music.join("neu")).unwrap();
    write_track(&music, "Erster.wav", 128.0, Key::from_camelot(8, 'A').unwrap(), 1);
    write_track(&music, "Zweiter.wav", 130.0, Key::from_camelot(9, 'A').unwrap(), 2);
    let data = tmp.path().join("daten");

    let state = AppState::start(&data).unwrap();
    assert_eq!(state.import(&[music.clone()]).unwrap().added, 2);
    // Der Ordner ist gemerkt und steht in der Datei.
    assert_eq!(Settings::load(&data.join("settings.json")).music_folders, vec![music.clone()]);

    // Nichts Neues: nichts kommt dazu.
    let r = state.refresh().unwrap();
    assert_eq!((r.folders, r.added, r.relinked, r.missing), (1, 0, 0, 0));

    // Eine neue Datei im Unterordner, eine verschoben, eine gelöscht.
    write_track(&music.join("neu"), "Dritter.wav", 132.0, Key::from_camelot(10, 'A').unwrap(), 3);
    std::fs::rename(music.join("Erster.wav"), music.join("neu").join("Erster.wav")).unwrap();
    std::fs::remove_file(music.join("Zweiter.wav")).unwrap();
    let rev = state.library_rev.load(std::sync::atomic::Ordering::Relaxed);
    let r = state.refresh().unwrap();
    assert_eq!((r.folders, r.unreachable, r.added, r.relinked, r.missing), (1, 0, 1, 1, 1));
    assert!(r.failed.is_empty());
    assert!(state.library_rev.load(std::sync::atomic::Ordering::Relaxed) > rev);
    let rows = state.lib.lock().unwrap().tracks(&TrackQuery::default()).unwrap();
    assert_eq!(rows.len(), 3);
    assert_eq!(rows.iter().filter(|t| t.missing).count(), 1);

    // Ein nicht erreichbarer Musikordner wird gemeldet, statt den Lauf scheitern zu lassen.
    drop(state);
    let state = AppState::start(&data).unwrap();
    std::fs::rename(&music, tmp.path().join("weg")).unwrap();
    let r = state.refresh().unwrap();
    assert_eq!((r.folders, r.unreachable, r.added, r.missing), (0, 1, 0, 3));
}

/// Eine Bibliothek ohne gemerkte Ordner (vor dieser Funktion angelegt) übernimmt beim ersten
/// Aktualisieren die Ordner ihrer Tracks.
#[test]
fn refresh_adopts_the_folders_of_an_older_library() {
    let tmp = tempfile::tempdir().unwrap();
    let music = tmp.path().join("musik");
    std::fs::create_dir_all(&music).unwrap();
    write_track(&music, "Erster.wav", 128.0, Key::from_camelot(8, 'A').unwrap(), 1);
    let state = AppState::start(&tmp.path().join("daten")).unwrap();
    // Einzelne Datei importiert: kein Ordner gemerkt.
    assert_eq!(state.import(&[music.join("Erster.wav")]).unwrap().added, 1);
    assert!(state.settings.lock().unwrap().music_folders.is_empty());
    write_track(&music, "Zweiter.wav", 130.0, Key::from_camelot(9, 'A').unwrap(), 2);
    let r = state.refresh().unwrap();
    assert_eq!((r.folders, r.added), (1, 1));
    assert_eq!(state.settings.lock().unwrap().music_folders, vec![music]);
}

/// Die Liste der Musikordner bleibt frei von Verschachtelung.
#[test]
fn music_folders_do_not_nest() {
    let mut folders = Vec::new();
    let p = std::path::Path::new;
    assert!(add_folder(&mut folders, p("/musik/techno")));
    assert!(!add_folder(&mut folders, p("/musik/techno/2026")));
    assert!(!add_folder(&mut folders, p("/musik/techno")));
    assert!(add_folder(&mut folders, p("/musik")));
    assert_eq!(folders, vec![p("/musik").to_path_buf()]);
}

/// BIB-20: Eine abgebrochene Kopie (hinten fehlt Audio) wird bei der Analyse als unvollständig
/// markiert, eine vollständige Datei nicht; nach dem Ersetzen verschwindet die Markierung.
#[test]
fn incomplete_files_are_marked_in_the_library() {
    let tmp = tempfile::tempdir().unwrap();
    let music = tmp.path().join("musik");
    std::fs::create_dir_all(&music).unwrap();
    write_track(&music, "Ganz.wav", 128.0, Key::from_camelot(8, 'A').unwrap(), 1);
    write_track(&music, "Halb.wav", 130.0, Key::from_camelot(9, 'A').unwrap(), 2);
    let whole = std::fs::read(music.join("Halb.wav")).unwrap();
    std::fs::write(music.join("Halb.wav"), &whole[..whole.len() * 2 / 5]).unwrap();

    let state = AppState::start(&tmp.path().join("daten")).unwrap();
    assert_eq!(state.import(&[music.clone()]).unwrap().added, 2);
    let rows = state.lib.lock().unwrap().tracks(&TrackQuery { sort: SortColumn::Title, ..Default::default() }).unwrap();
    for row in &rows {
        state.analyze_track(row.id).unwrap();
    }
    let rows = state.lib.lock().unwrap().tracks(&TrackQuery { sort: SortColumn::Title, ..Default::default() }).unwrap();
    assert_eq!(rows.iter().map(|r| (r.title.as_str(), r.incomplete)).collect::<Vec<_>>(), vec![("Ganz", false), ("Halb", true)]);

    // Vollständige Datei an denselben Ort: Aktualisieren setzt den Track zurück, die neue Analyse ist sauber.
    std::fs::write(music.join("Halb.wav"), &whole).unwrap();
    let r = state.refresh().unwrap();
    assert_eq!(r.added, 1);
    let row = state.lib.lock().unwrap().track(rows[1].id).unwrap();
    assert!(!row.analyzed && !row.incomplete);
    state.analyze_track(row.id).unwrap();
    let row = state.lib.lock().unwrap().track(row.id).unwrap();
    assert!(row.analyzed && !row.incomplete);
    assert!((row.bpm.unwrap() - 130.0).abs() < 0.05);
}

/// TRANS-05, AUTO-08: Ein vorbereiteter Übergang lässt sich im laufenden Betrieb einplanen. Die
/// Anwendung prüft die Voraussetzungen in Klartext, die Engine startet das neue Deck an der
/// Taktgrenze und fährt die Regler; danach läuft nur noch das neue Deck, offen und als Master.
#[test]
fn a_prepared_transition_plays_live() {
    use dw_automix::{Transition, When};
    let next = When::Next { align_bars: 1 };
    let tmp = tempfile::tempdir().unwrap();
    let music = tmp.path().join("musik");
    std::fs::create_dir_all(&music).unwrap();
    write_track(&music, "Erster.wav", 128.0, Key::from_camelot(8, 'A').unwrap(), 1);
    write_track(&music, "Zweiter.wav", 130.0, Key::from_camelot(9, 'A').unwrap(), 2);
    let state = AppState::start(&tmp.path().join("daten")).unwrap();
    state.import(&[music]).unwrap();
    let rows = state.lib.lock().unwrap().tracks(&TrackQuery { sort: SortColumn::Title, ..Default::default() }).unwrap();
    let fade = Transition::fade(4);

    // Ohne geladene Decks, ohne laufendes Deck, mit laufendem Ziel: jeweils eine klare Absage.
    assert!(state.start_transition(DeckId::A, DeckId::B, &fade, next).unwrap_err().contains("Deck A läuft nicht"));
    state.load_deck(DeckId::A, rows[0].id).unwrap();
    state.ctl.lock().unwrap().send(Command::Play { deck: DeckId::A, on: true });
    wait_for("Wiedergabe", || state.ctl.lock().unwrap().snapshot().decks[0].pos > 0.3);
    assert!(state.start_transition(DeckId::A, DeckId::B, &fade, next).unwrap_err().contains("Auf Deck B liegt kein Track"));
    state.load_deck(DeckId::B, rows[1].id).unwrap();
    wait_for("Deck B geladen", || state.ctl.lock().unwrap().snapshot().decks[1].loaded);
    assert!(state.start_transition(DeckId::A, DeckId::A, &fade, next).is_err());

    // Einplanen: T0 liegt mindestens den Vorlauf und höchstens gut einen Takt voraus.
    let before = state.ctl.lock().unwrap().snapshot().clock_beats;
    let run = state.start_transition(DeckId::A, DeckId::B, &fade, next).unwrap();
    assert_eq!((run.from, run.to, run.bars, run.name.as_str()), (DeckId::A, DeckId::B, 4, "fade"));
    assert!(run.t0_beat - before >= dw_automix::LEAD_BEATS && run.t0_beat - before < 9.0, "T0 in {} Beats", run.t0_beat - before);
    assert!((run.end_beat - (run.t0_beat + 17.0)).abs() < 1e-9);
    assert!(state.start_transition(DeckId::A, DeckId::B, &fade, next).unwrap_err().contains("Es läuft schon ein Übergang"));

    // Kurz nach T0 läuft B im Takt und A noch mit.
    wait_for("Start von B", || state.ctl.lock().unwrap().snapshot().clock_beats > run.t0_beat + 2.0);
    let s = state.ctl.lock().unwrap().snapshot().clone();
    assert!(s.decks[0].playing && s.decks[1].playing);
    assert_eq!(s.decks[1].sync, dw_core::SyncMode::Beat);
    assert!(s.decks[1].phase.abs() < 0.01, "Phase von B {}", s.decks[1].phase);
    assert!(s.decks[1].fader > 0.0 && s.decks[1].fader < 1.0, "Fader von B unterwegs bei {}", s.decks[1].fader);

    // Nach dem Ende: A steht mit Reglern in Ruhestellung, B läuft offen und ist Master.
    wait_for("Ende des Übergangs", || state.ctl.lock().unwrap().snapshot().clock_beats > run.end_beat + 1.0);
    let s = state.ctl.lock().unwrap().snapshot().clone();
    assert!(!s.decks[0].playing && s.decks[1].playing);
    assert_eq!((s.decks[0].fader, s.decks[0].eq_db), (1.0, [0.0; 3]));
    assert_eq!((s.decks[1].fader, s.decks[1].eq_db), (1.0, [0.0; 3]));
    assert_eq!(s.master, Some(DeckId::B));

    // Abbrechen: Die Regler bleiben stehen. Von B zurück nach A, nach dem ersten Stück abgebrochen.
    state.ctl.lock().unwrap().send(Command::Seek { deck: DeckId::A, pos: 10.0 });
    let run = state.start_transition(DeckId::B, DeckId::A, &Transition::bass_swap(8), next).unwrap();
    wait_for("zweiter Übergang", || state.ctl.lock().unwrap().snapshot().clock_beats > run.t0_beat + 6.0);
    state.cancel_transition().unwrap();
    std::thread::sleep(Duration::from_millis(200));
    let at_cancel = state.ctl.lock().unwrap().snapshot().clone();
    assert_eq!(at_cancel.decks[0].eq_db[0], f32::NEG_INFINITY, "Bass von A ist vor dem Tausch noch aus");
    std::thread::sleep(Duration::from_millis(1500));
    let later = state.ctl.lock().unwrap().snapshot().clone();
    assert!(later.decks[0].playing && later.decks[1].playing);
    assert_eq!((later.decks[0].eq_db, later.decks[0].fader), (at_cancel.decks[0].eq_db, at_cancel.decks[0].fader));
    // Beide Decks laufen im Takt (A im Beat-Sync): Jetzt lässt sich ein Übergang mit vorgegebenem
    // Ende einplanen. Liegt das Ende zu nah, kommt die Absage mit den fehlenden Beats.
    let err = state.start_transition(DeckId::B, DeckId::A, &fade, When::EndIn { beats: 8.0 }).unwrap_err();
    assert!(err.contains("Bis zum gewählten Ende"), "{err}");
    let before = state.ctl.lock().unwrap().snapshot().clock_beats;
    let run = state.start_transition(DeckId::B, DeckId::A, &fade, When::EndIn { beats: 30.0 }).unwrap();
    let end = run.t0_beat + 16.0;
    assert!((end - before - 30.0).abs() <= 2.0 + 1e-6, "Ende in {} Beats statt rund 30", end - before);
    wait_for("Ende des dritten Übergangs", || state.ctl.lock().unwrap().snapshot().clock_beats > run.end_beat + 1.0);
    let s = state.ctl.lock().unwrap().snapshot().clone();
    assert!(s.decks[0].playing && !s.decks[1].playing, "A läuft weiter, B ist gestoppt");
    // Fader und Bass von A hat dieser Übergang geöffnet; Mitten und Höhen stehen noch dort, wo der
    // abgebrochene sie gelassen hat – das Überblenden fasst sie nicht an.
    assert_eq!((s.decks[0].fader, s.decks[0].eq_db[0]), (1.0, 0.0));
}
