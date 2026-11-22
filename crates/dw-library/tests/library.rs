//! Tests der Bibliothek gegen BIB-01, 03, 05, 06, 08, 09, 18 und ANA-15.

use dw_core::{BeatGrid, GridAnchor, Key, TrackAnalysis};
use dw_library::{Library, LibraryError, SortColumn, TrackQuery};
use std::path::{Path, PathBuf};
use std::time::Instant;

fn data() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/data")
}

/// Kopiert die Testdateien in einen eigenen Musikordner mit Unterordner.
fn music_dir(root: &Path) -> PathBuf {
    let music = root.join("musik");
    std::fs::create_dir_all(music.join("unterordner")).unwrap();
    std::fs::copy(data().join("tagged.mp3"), music.join("tagged.mp3")).unwrap();
    std::fs::copy(data().join("tagged.flac"), music.join("unterordner/tagged.flac")).unwrap();
    std::fs::copy(data().join("tagged.m4a"), music.join("unterordner/tagged.m4a")).unwrap();
    std::fs::copy(data().join("untagged.wav"), music.join("Ohne Angaben.wav")).unwrap();
    std::fs::write(music.join("notizen.txt"), "keine Musik").unwrap();
    std::fs::write(music.join("kaputt.mp3"), "kein MP3").unwrap();
    music
}

fn analysis(bpm: f64, key: Key) -> TrackAnalysis {
    TrackAnalysis {
        version: 1,
        duration: 0.3,
        bpm,
        bpm_confidence: 0.9,
        grid: BeatGrid::from_anchors(vec![GridAnchor { pos: 0.12, bpm }, GridAnchor { pos: 60.12, bpm: bpm + 2.0 }], 3),
        key: Some(key),
        key_confidence: 0.7,
        tuning_cents: -8.0,
        lufs: -8.5,
        lra: 4.0,
        true_peak_db: -0.2,
    }
}

#[test]
fn import_reads_metadata_and_skips_duplicates() {
    let tmp = tempfile::tempdir().unwrap();
    let music = music_dir(tmp.path());
    let mut lib = Library::open(&tmp.path().join("lib")).unwrap();

    let report = lib.import_paths(&[music.clone()]).unwrap();
    assert_eq!(report.added, 4, "{report:?}");
    assert_eq!(report.failed.len(), 1, "die kaputte Datei wird gemeldet, bricht aber nichts ab");
    assert!(report.failed[0].0.ends_with("kaputt.mp3"));

    let tracks = lib.tracks(&TrackQuery { sort: SortColumn::Title, ..Default::default() }).unwrap();
    let titles: Vec<&str> = tracks.iter().map(|t| t.title.as_str()).collect();
    assert_eq!(titles, ["Kaltfront", "Nachtzug", "Ohne Angaben", "Unterstrom"]);

    let mp3 = tracks.iter().find(|t| t.title == "Nachtzug").unwrap();
    assert_eq!(mp3.artist, "Lena Öztürk");
    assert_eq!(mp3.album, "Gleis 9");
    assert_eq!(mp3.genre, "Techno");
    assert_eq!(mp3.label, "Stahlwerk");
    assert_eq!(mp3.year, Some(2024));
    assert_eq!(mp3.bpm, Some(134.0));
    assert_eq!(mp3.format, "mp3");
    assert!(mp3.duration > 0.2 && mp3.duration < 0.5);
    assert!(!mp3.analyzed);

    let flac = tracks.iter().find(|t| t.title == "Kaltfront").unwrap();
    assert_eq!((flac.artist.as_str(), flac.label.as_str(), flac.bpm), ("Mira Voss", "Nordwand", Some(140.5)));

    // Zweiter Import: nichts Neues.
    let again = lib.import_paths(&[music.clone()]).unwrap();
    assert_eq!((again.added, again.duplicates), (0, 4));

    // Dieselbe Datei unter anderem Namen ist ein Duplikat.
    std::fs::copy(music.join("tagged.mp3"), music.join("kopie.mp3")).unwrap();
    let copy = lib.import_paths(&[music.join("kopie.mp3")]).unwrap();
    assert_eq!((copy.added, copy.duplicates), (0, 1));
    assert_eq!(lib.track_count().unwrap(), 4);
}

#[test]
fn analysis_survives_moving_a_file() {
    let tmp = tempfile::tempdir().unwrap();
    let music = music_dir(tmp.path());
    let mut lib = Library::open(&tmp.path().join("lib")).unwrap();
    lib.import_paths(&[music.clone()]).unwrap();
    let id = lib.tracks(&TrackQuery { text: "nachtzug".into(), ..Default::default() }).unwrap()[0].id;
    let a = analysis(134.02, Key::from_camelot(8, 'A').unwrap());
    assert!(lib.set_analysis(id, &a).unwrap());

    // Datei verschieben und umbenennen, dann den neuen Ort importieren.
    let moved = music.join("unterordner/Neuer Name.mp3");
    std::fs::rename(music.join("tagged.mp3"), &moved).unwrap();
    assert_eq!(lib.mark_missing().unwrap(), 1);
    let report = lib.import_paths(&[music]).unwrap();
    assert_eq!((report.added, report.relinked), (0, 1));

    let row = lib.track(id).unwrap();
    assert_eq!(row.path, moved);
    assert!(!row.missing && row.analyzed);
    assert_eq!(row.bpm, Some(134.02));
    let back = lib.analysis(id).unwrap().unwrap();
    assert_eq!(back.grid, a.grid);
    assert_eq!(back.key, a.key);
    assert!((back.lufs - a.lufs).abs() < 1e-6);
    assert!((back.duration - row.duration).abs() < 1e-9);
}

#[test]
fn locked_analysis_is_not_overwritten() {
    let tmp = tempfile::tempdir().unwrap();
    let music = music_dir(tmp.path());
    let mut lib = Library::open(&tmp.path().join("lib")).unwrap();
    lib.import_paths(&[music]).unwrap();
    let id = lib.tracks(&TrackQuery::default()).unwrap()[0].id;
    assert_eq!(lib.pending_analysis(1).unwrap().len(), 4);
    lib.set_analysis(id, &analysis(130.0, Key::new(0, false))).unwrap();
    assert_eq!(lib.pending_analysis(1).unwrap().len(), 3);
    // Neue Analyse-Version: der Track steht wieder an, solange er nicht gesperrt ist.
    assert_eq!(lib.pending_analysis(2).unwrap().len(), 4);
    lib.set_analysis_locked(id, true).unwrap();
    assert_eq!(lib.pending_analysis(2).unwrap().len(), 3);
    assert!(!lib.set_analysis(id, &analysis(65.0, Key::new(5, true))).unwrap());
    assert_eq!(lib.analysis(id).unwrap().unwrap().bpm, 130.0);
}

#[test]
fn search_filter_and_sort() {
    let tmp = tempfile::tempdir().unwrap();
    let music = music_dir(tmp.path());
    let mut lib = Library::open(&tmp.path().join("lib")).unwrap();
    lib.import_paths(&[music]).unwrap();
    let ids: Vec<i64> = lib.tracks(&TrackQuery { sort: SortColumn::Title, ..Default::default() }).unwrap().iter().map(|t| t.id).collect();
    let k = |n, l| Key::from_camelot(n, l).unwrap();
    lib.set_analysis(ids[0], &analysis(140.5, k(9, 'A'))).unwrap(); // Kaltfront
    lib.set_analysis(ids[1], &analysis(134.0, k(8, 'A'))).unwrap(); // Nachtzug
    lib.set_analysis(ids[3], &analysis(128.0, k(8, 'B'))).unwrap(); // Unterstrom

    let find = |q: TrackQuery| -> Vec<String> { lib.tracks(&q).unwrap().into_iter().map(|t| t.title).collect() };
    // Umlaute und Groß-/Kleinschreibung
    assert_eq!(find(TrackQuery { text: "ÖZTÜRK".into(), ..Default::default() }), ["Nachtzug"]);
    // Mehrere Wörter über mehrere Felder (Künstler + Label)
    assert_eq!(find(TrackQuery { text: "mira nordwand".into(), ..Default::default() }), ["Kaltfront"]);
    // Kommentar-Feld, auch wenn es unter abweichendem Namen in der Datei steht (FLAC und MP3)
    assert_eq!(find(TrackQuery { text: "peak".into(), sort: SortColumn::Title, ..Default::default() }), ["Kaltfront", "Nachtzug"]);
    assert!(find(TrackQuery { text: "gibtsnicht".into(), ..Default::default() }).is_empty());
    // BPM-Bereich und Tonart
    assert_eq!(find(TrackQuery { bpm_range: Some((130.0, 136.0)), ..Default::default() }), ["Nachtzug"]);
    assert_eq!(
        find(TrackQuery { keys: vec![k(8, 'A'), k(8, 'B')], sort: SortColumn::Bpm, ..Default::default() }),
        ["Unterstrom", "Nachtzug"]
    );
    assert_eq!(
        find(TrackQuery { sort: SortColumn::Bpm, descending: true, limit: Some(2), ..Default::default() }),
        ["Kaltfront", "Nachtzug"]
    );
}

#[test]
fn playlists_keep_their_order_across_restarts() {
    let tmp = tempfile::tempdir().unwrap();
    let music = music_dir(tmp.path());
    let lib_dir = tmp.path().join("lib");
    let (folder, list, ids) = {
        let mut lib = Library::open(&lib_dir).unwrap();
        lib.import_paths(&[music]).unwrap();
        let ids: Vec<i64> = lib.tracks(&TrackQuery { sort: SortColumn::Title, ..Default::default() }).unwrap().iter().map(|t| t.id).collect();
        let folder = lib.create_playlist("Sets", None, true).unwrap();
        let list = lib.create_playlist("Freitag", Some(folder), false).unwrap();
        lib.add_to_playlist(list, &[ids[0], ids[1], ids[2]], None).unwrap();
        lib.add_to_playlist(list, &[ids[3]], Some(1)).unwrap(); // 0 3 1 2
        lib.move_in_playlist(list, 3, 0).unwrap(); // 2 0 3 1
        lib.remove_from_playlist(list, 1).unwrap(); // 2 3 1
        lib.rename_playlist(list, "Freitag Nacht").unwrap();
        (folder, list, ids)
    };
    let mut lib = Library::open(&lib_dir).unwrap();
    let pls = lib.playlists().unwrap();
    assert_eq!(pls.len(), 2);
    assert!(pls.iter().any(|p| p.id == list && p.name == "Freitag Nacht" && p.parent == Some(folder) && !p.is_folder));
    let order: Vec<i64> = lib.tracks(&TrackQuery { playlist: Some(list), ..Default::default() }).unwrap().iter().map(|t| t.id).collect();
    assert_eq!(order, [ids[2], ids[3], ids[1]]);
    // Ordner löschen entfernt die Playlist darin, nicht die Tracks.
    lib.delete_playlist(folder).unwrap();
    assert!(lib.playlists().unwrap().is_empty());
    assert_eq!(lib.track_count().unwrap(), 4);
}

#[test]
fn backup_on_start_and_recovery_from_corruption() {
    let tmp = tempfile::tempdir().unwrap();
    let music = music_dir(tmp.path());
    let lib_dir = tmp.path().join("lib");
    {
        let mut lib = Library::open(&lib_dir).unwrap();
        lib.import_paths(&[music]).unwrap();
    }
    assert!(Library::backups(&lib_dir).is_empty(), "beim ersten Start gibt es noch nichts zu sichern");
    for _ in 0..13 {
        drop(Library::open(&lib_dir).unwrap());
    }
    assert_eq!(Library::backups(&lib_dir).len(), 10, "es bleiben die letzten 10 Sicherungen");

    // Datenbank zerstören: Der Start meldet den Schaden und nennt die Sicherungen.
    for f in ["library.db-wal", "library.db-shm"] {
        let _ = std::fs::remove_file(lib_dir.join(f));
    }
    std::fs::write(lib_dir.join("library.db"), vec![0x42u8; 8192]).unwrap();
    let backups = match Library::open(&lib_dir) {
        Err(LibraryError::Corrupt { backups }) => backups,
        Err(e) => panic!("anderer Fehler: {e}"),
        Ok(_) => panic!("beschädigte Datenbank wurde nicht erkannt"),
    };
    assert_eq!(backups.len(), 10);
    let lib = Library::restore(&lib_dir, &backups[0]).unwrap();
    assert_eq!(lib.track_count().unwrap(), 4);
}

/// BIB-08, BIB-09: Suche und Sortierung bei 20 000 Tracks in ≤ 100 ms.
#[test]
fn search_and_sort_are_fast_with_20000_tracks() {
    let tmp = tempfile::tempdir().unwrap();
    let lib_dir = tmp.path().join("lib");
    drop(Library::open(&lib_dir).unwrap());
    // Einträge direkt einfügen; es geht um die Abfragezeit, nicht um den Import.
    let conn = rusqlite::Connection::open(lib_dir.join("library.db")).unwrap();
    conn.execute_batch("BEGIN").unwrap();
    {
        let mut ins = conn
            .prepare("INSERT INTO track (path, hash, size, mtime, title, artist, label, duration, tag_bpm, search) VALUES (?1, ?2, 1, 1, ?3, ?4, ?5, ?6, ?7, ?8)")
            .unwrap();
        let words = ["Nacht", "Stahl", "Echo", "Raster", "Flut", "Kern", "Signal", "Dunst", "Achse", "Puls"];
        for i in 0..20_000usize {
            let title = format!("{} {} {}", words[i % 10], words[(i / 10) % 10], i);
            let artist = format!("Künstler {}", i % 700);
            let label = format!("Label {}", i % 90);
            let search = format!("{title} {artist} {label}").to_lowercase();
            ins.execute(rusqlite::params![format!("/m/{i}.flac"), format!("{i:032}"), title, artist, label, 300.0 + (i % 200) as f64, 120.0 + (i % 400) as f64 / 10.0, search])
                .unwrap();
        }
    }
    conn.execute_batch("COMMIT").unwrap();
    drop(conn);

    let lib = Library::open(&lib_dir).unwrap();
    let mut worst = 0.0f64;
    // Jede Abfrage läuft fünfmal, gewertet wird der schnellste Lauf: Störungen durch andere
    // Tests oder eine ausgelastete Maschine verlängern eine Messung nur, sie verkürzen sie nie.
    let mut timed = |q: TrackQuery, expect_min: usize| {
        let mut best = f64::MAX;
        for _ in 0..5 {
            let t = Instant::now();
            let rows = lib.tracks(&q).unwrap();
            best = best.min(t.elapsed().as_secs_f64() * 1000.0);
            assert!(rows.len() >= expect_min, "{} Treffer", rows.len());
        }
        worst = worst.max(best);
    };
    timed(TrackQuery { text: "stahl echo".into(), ..Default::default() }, 100);
    timed(TrackQuery { text: "künstler 42".into(), sort: SortColumn::Bpm, ..Default::default() }, 20);
    timed(TrackQuery { sort: SortColumn::Title, ..Default::default() }, 20_000);
    timed(TrackQuery { sort: SortColumn::Bpm, descending: true, ..Default::default() }, 20_000);
    timed(TrackQuery { bpm_range: Some((130.0, 135.0)), sort: SortColumn::Artist, ..Default::default() }, 1000);
    println!("langsamste Abfrage bei 20 000 Tracks: {worst:.1} ms");
    assert!(worst <= 100.0, "{worst:.1} ms");
}

/// Baut eine MP3-Datei mit ID3v2.3-Angaben, deren Jahres-Feld ein ganzes Datum in falscher
/// Form enthält („08-05-2026"). So kam sie in der Abnahme am Gerät vor.
fn mp3_with_bad_year(target: &Path) {
    let original = std::fs::read(data().join("tagged.mp3")).unwrap();
    // Vorhandene ID3v2-Angaben abtrennen: Kopf 10 Bytes, Länge in 4 × 7 Bit.
    let audio = if original.starts_with(b"ID3") {
        let len = original[6..10].iter().fold(0usize, |n, &b| (n << 7) | (b & 0x7f) as usize);
        &original[10 + len..]
    } else {
        &original[..]
    };
    let frame = |id: &[u8; 4], text: &str| {
        let mut f = id.to_vec();
        f.extend_from_slice(&(text.len() as u32 + 1).to_be_bytes());
        f.extend_from_slice(&[0, 0, 0]); // Flags, Kodierung Latin-1
        f.extend_from_slice(text.as_bytes());
        f
    };
    let frames = [frame(b"TIT2", "Schiefes Datum"), frame(b"TPE1", "Antje Kovac"), frame(b"TYER", "08-05-2026")].concat();
    let n = frames.len();
    let mut file = b"ID3\x03\x00\x00".to_vec();
    file.extend_from_slice(&[(n >> 21) as u8 & 0x7f, (n >> 14) as u8 & 0x7f, (n >> 7) as u8 & 0x7f, n as u8 & 0x7f]);
    file.extend_from_slice(&frames);
    file.extend_from_slice(audio);
    std::fs::write(target, file).unwrap();
}

/// BIB-01: Eine fehlerhafte Angabe in den Tags darf den Track nicht aus der Bibliothek halten.
/// Die lesbaren Angaben bleiben erhalten.
#[test]
fn malformed_tag_does_not_block_import() {
    let tmp = tempfile::tempdir().unwrap();
    let file = tmp.path().join("schief.mp3");
    mp3_with_bad_year(&file);

    let mut lib = Library::open(&tmp.path().join("bib")).unwrap();
    let report = lib.import_paths(&[file]).unwrap();
    assert_eq!((report.added, report.failed.len()), (1, 0), "{:?}", report.failed);

    let tracks = lib.tracks(&TrackQuery::default()).unwrap();
    assert_eq!(tracks[0].title, "Schiefes Datum");
    assert_eq!(tracks[0].artist, "Antje Kovac");
    assert!(tracks[0].duration > 0.2 && tracks[0].duration < 0.5, "Länge {}", tracks[0].duration);
}

/// BIB-20: Wird eine Datei am selben Ort ersetzt (z. B. eine abgebrochene Kopie durch die
/// vollständige), verwirft der nächste Import die alte Analyse und die Markierung „unvollständig".
#[test]
fn replaced_file_starts_over() {
    let tmp = tempfile::tempdir().unwrap();
    let music = music_dir(tmp.path());
    let mut lib = Library::open(&tmp.path().join("lib")).unwrap();
    lib.import_paths(&[music.clone()]).unwrap();
    let id = lib.tracks(&TrackQuery { text: "nachtzug".into(), ..Default::default() }).unwrap()[0].id;
    assert!(lib.set_analysis(id, &analysis(134.02, Key::from_camelot(8, 'A').unwrap())).unwrap());
    lib.set_incomplete(id, true).unwrap();
    assert!(lib.track(id).unwrap().incomplete);
    let hash = lib.fingerprint_of(id).unwrap();

    // Unverändert: gilt als vorhanden, nichts geht verloren.
    let report = lib.import_paths(&[music.clone()]).unwrap();
    assert_eq!((report.added, report.updated), (0, 0));
    assert!(lib.track(id).unwrap().analyzed);

    // Inhalt ersetzt: gleiche Kennung und gleicher Ort, aber Analyse und Markierung sind weg.
    let file = music.join("tagged.mp3");
    let mut bytes = std::fs::read(&file).unwrap();
    bytes.extend_from_slice(&[0x55; 4096]);
    std::fs::write(&file, bytes).unwrap();
    let report = lib.import_paths(&[music]).unwrap();
    assert_eq!((report.added, report.updated), (0, 1));
    let row = lib.track(id).unwrap();
    assert_eq!(row.path, file);
    assert!(!row.analyzed && !row.incomplete && !row.missing);
    assert!(lib.analysis(id).unwrap().is_none());
    assert_ne!(lib.fingerprint_of(id).unwrap(), hash);
}
