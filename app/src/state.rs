//! Gemeinsamer Zustand der Anwendung: Audio-Steuerung, Bibliothek, Cache.

use crate::audio::{self, AudioHandle, AudioInfo};
use crate::recorder::{Recorder, RecordingInfo};
use dw_audio::Controller;
use dw_automix::{plan_live, plan_preview, LiveDeck, Roles, Transition, When};
use dw_core::{AudioBuf, Band, Command, DeckId, SyncMode, TrackAnalysis, Waveform, ANALYSIS_VERSION};
use dw_engine::Track;
use dw_library::{ImportReport, Library, TrackId};
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Condvar, Mutex};

/// Referenz-Lautheit für den Auto-Gain (MIX-01).
const REFERENCE_LUFS: f32 = -14.0;

/// Einstellungen, die einen Neustart überdauern (`settings.json` im Datenordner).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Settings {
    /// Gewähltes Ausgabegerät; `None` = Standardgerät des Systems.
    #[serde(default)]
    pub audio_device: Option<String>,
    /// Musikordner: Was als Ordner importiert wurde, durchsucht „Bibliothek aktualisieren" erneut (BIB-19).
    #[serde(default)]
    pub music_folders: Vec<PathBuf>,
    /// Beim Start die Musikordner nach neuen Dateien durchsuchen.
    #[serde(default = "yes")]
    pub refresh_on_start: bool,
}

fn yes() -> bool {
    true
}

impl Default for Settings {
    fn default() -> Self {
        Settings { audio_device: None, music_folders: Vec::new(), refresh_on_start: true }
    }
}

/// Ein eingeplanter oder laufender Übergang (TRANS-05).
#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct TransitionRun {
    pub from: DeckId,
    pub to: DeckId,
    pub name: String,
    pub bars: u32,
    /// Beginn T0 und Ende in Clock-Beats.
    pub t0_beat: f64,
    pub end_beat: f64,
}

/// Ein für ein bestimmtes Paar gespeicherter Übergang (TRANS-07): Art (die Kurven) und Ort.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct PairTransition {
    pub id: u64,
    /// Track, der geht, und Track, der kommt (Kennungen der Bibliothek).
    pub out_track: String,
    pub in_track: String,
    /// Stellen beider Tracks am Beginn T0 des Übergangs, in Sekunden.
    pub out_pos: f64,
    pub in_pos: f64,
    pub transition: Transition,
}

/// Stand eines Decks vor dem Vorhören.
#[derive(Clone, Debug)]
struct SavedDeck {
    deck: DeckId,
    pos: f64,
    fader: f32,
    eq_db: [f32; 3],
    filter: f32,
    sync: SyncMode,
}

#[derive(Clone, Debug)]
struct PreviewSaved {
    decks: [SavedDeck; 2],
    master: Option<DeckId>,
    master_auto: bool,
}

/// Laufendes Vorhören eines Übergangs (TRANS-06) mit dem Stand, der danach wiederhergestellt wird.
#[derive(Clone, Debug)]
pub struct Preview {
    pub run: TransitionRun,
    saved: PreviewSaved,
}

/// Ergebnis von „Bibliothek aktualisieren".
#[derive(Clone, Debug, Default, PartialEq, Serialize)]
pub struct RefreshReport {
    /// Durchsuchte Ordner.
    pub folders: usize,
    /// Musikordner, die gerade nicht erreichbar sind (z. B. Laufwerk nicht angeschlossen).
    pub unreachable: usize,
    pub added: usize,
    pub relinked: usize,
    /// Tracks der Bibliothek, deren Datei fehlt.
    pub missing: usize,
    pub failed: Vec<(String, String)>,
}

/// Nimmt `folder` in die Liste auf. Liegt er in einem schon bekannten Ordner, ändert sich nichts;
/// bekannte Ordner, die in ihm liegen, gehen in ihm auf. Gibt zurück, ob sich die Liste geändert hat.
pub fn add_folder(folders: &mut Vec<PathBuf>, folder: &Path) -> bool {
    if folders.iter().any(|known| folder.starts_with(known)) {
        return false;
    }
    folders.retain(|known| !known.starts_with(folder));
    folders.push(folder.to_path_buf());
    true
}

impl Settings {
    /// Liest die Einstellungen; fehlt die Datei oder ist sie unlesbar, gelten die Vorgaben.
    pub fn load(file: &Path) -> Settings {
        std::fs::read(file).ok().and_then(|bytes| serde_json::from_slice(&bytes).ok()).unwrap_or_default()
    }

    pub fn save(&self, file: &Path) -> Result<(), String> {
        let json = serde_json::to_vec_pretty(self).map_err(|e| e.to_string())?;
        std::fs::write(file, json).map_err(|e| format!("{}: {e}", file.display()))
    }
}

pub struct Inner {
    pub ctl: Mutex<Controller>,
    /// Stand der Ausgabe; ändert sich beim Gerätewechsel.
    pub audio: Mutex<AudioInfo>,
    audio_handle: AudioHandle,
    /// Abtastrate der Engine; bleibt über einen Gerätewechsel hinweg gleich.
    pub engine_rate: u32,
    settings_file: PathBuf,
    pub settings: Mutex<Settings>,
    pub lib: Mutex<Library>,
    pub loaded: Mutex<[Option<TrackId>; 4]>,
    /// Der zuletzt eingeplante Übergang; gilt als laufend, bis die Clock sein Ende erreicht.
    pub transition: Mutex<Option<TransitionRun>>,
    /// Laufendes Vorhören im Editor.
    pub preview: Mutex<Option<Preview>>,
    /// Mitschnitt des Master-Signals (REC-01).
    recorder: Recorder,
    recordings_dir: Mutex<PathBuf>,
    cache_dir: PathBuf,
    /// Zählt bei jeder Änderung der Bibliothek hoch; die Oberfläche lädt die Liste dann neu.
    pub library_rev: AtomicU64,
    /// Weckt die Hintergrund-Analyse, wenn neue Tracks dazugekommen sind.
    pub work: (Mutex<bool>, Condvar),
}

#[derive(Clone)]
pub struct AppState(pub Arc<Inner>);

impl std::ops::Deref for AppState {
    type Target = Inner;
    fn deref(&self) -> &Inner {
        &self.0
    }
}

impl AppState {
    pub fn start(data_dir: &Path) -> Result<AppState, String> {
        let cache_dir = data_dir.join("cache");
        std::fs::create_dir_all(cache_dir.join("wave")).map_err(|e| e.to_string())?;
        let lib = match Library::open(&data_dir.join("library")) {
            Ok(lib) => lib,
            // Beschädigte Bibliothek: neueste Sicherung einspielen (BIB-18).
            Err(dw_library::LibraryError::Corrupt { backups }) => match backups.first() {
                Some(backup) => Library::restore(&data_dir.join("library"), backup).map_err(|e| e.to_string())?,
                None => return Err("Die Bibliothek ist beschädigt und es gibt keine Sicherung.".into()),
            },
            Err(e) => return Err(e.to_string()),
        };
        let settings_file = data_dir.join("settings.json");
        let settings = Settings::load(&settings_file);
        let (mut ctl, audio, audio_handle) = audio::start(settings.audio_device.clone());
        let recorder = Recorder::spawn(ctl.take_master_tap().ok_or("Der Mitschnitt-Abgriff ist schon vergeben.")?);
        Ok(AppState(Arc::new(Inner {
            ctl: Mutex::new(ctl),
            engine_rate: audio.sample_rate,
            audio: Mutex::new(audio),
            audio_handle,
            settings_file,
            settings: Mutex::new(settings),
            lib: Mutex::new(lib),
            loaded: Mutex::new([None; 4]),
            transition: Mutex::new(None),
            preview: Mutex::new(None),
            recorder,
            recordings_dir: Mutex::new(data_dir.join("aufnahmen")),
            cache_dir,
            library_rev: AtomicU64::new(1),
            work: (Mutex::new(true), Condvar::new()),
        })))
    }

    /// Wechselt das Ausgabegerät (ENG-01); `None` = Standardgerät des Systems. Geladene Decks
    /// und laufende Wiedergabe bleiben erhalten. Scheitert der Wechsel, läuft das bisherige
    /// Gerät weiter und die Wahl wird nicht gespeichert.
    pub fn select_device(&self, device: Option<String>) -> Result<AudioInfo, String> {
        let (info, failure) = match self.audio_handle.select(device) {
            Ok(info) => (info, None),
            Err((error, info)) => (info, Some(error)),
        };
        self.ctl.lock().unwrap().set_routing(audio::routing_for(info.channels));
        *self.audio.lock().unwrap() = info.clone();
        match failure {
            Some(error) => Err(error),
            None => {
                let mut settings = self.settings.lock().unwrap();
                settings.audio_device = info.device.clone();
                settings.save(&self.settings_file)?;
                Ok(info)
            }
        }
    }

    /// Plant einen vorbereiteten Übergang vom laufenden Deck `from` zum Deck `to` ein (TRANS-05,
    /// AUTO-08). `when` legt fest, ob er an der nächsten Taktgrenze beginnt oder an einer vorgegebenen
    /// Stelle endet; die Engine fährt die Regler sample-genau. Steht das neue Deck, wird es gestellt,
    /// bekommt Beat-Sync und startet von selbst; läuft es schon im Takt mit, werden nur die Regler gefahren.
    pub fn start_transition(&self, from: DeckId, to: DeckId, transition: &Transition, when: When) -> Result<TransitionRun, String> {
        let name = |d: DeckId| ["A", "B", "C", "D"][d.index()];
        let snap = self.ctl.lock().unwrap().snapshot().clone();
        if self.preview.lock().unwrap().is_some() {
            return Err("Es läuft gerade ein Vorhören; erst beenden.".into());
        }
        if !when.is_live() {
            return Err("Diese Lage gibt es nur beim Vorhören mit stehenden Decks.".into());
        }
        if let Some(run) = self.transition.lock().unwrap().as_ref() {
            if snap.clock_beats < run.end_beat {
                return Err(format!("Es läuft schon ein Übergang von Deck {} nach Deck {}.", name(run.from), name(run.to)));
            }
        }
        let (out, inn) = (&snap.decks[from.index()], &snap.decks[to.index()]);
        if !out.loaded || !out.playing {
            return Err(format!("Deck {} läuft nicht; der Übergang geht vom laufenden Deck aus.", name(from)));
        }
        if !inn.loaded {
            return Err(format!("Auf Deck {} liegt kein Track.", name(to)));
        }
        // Läuft das neue Deck schon, muss es wie das alte im Gleichschritt mit der Clock sein.
        if inn.playing && !(inn.is_master || inn.sync == SyncMode::Beat) {
            return Err(format!("Deck {} läuft schon, aber nicht im Beat-Sync; so lässt sich der Übergang nicht im Takt fahren.", name(to)));
        }
        // Nur dann laufen die Beats des Decks und die der Clock im Gleichschritt.
        if !(out.is_master || out.sync == SyncMode::Beat) {
            return Err(format!("Deck {} muss Master sein oder im Beat-Sync laufen.", name(from)));
        }
        let grid_of = |deck: DeckId| -> Result<dw_core::BeatGrid, String> {
            let id = self.loaded.lock().unwrap()[deck.index()].ok_or_else(|| format!("Auf Deck {} liegt kein Track.", name(deck)))?;
            let analysis = self.lib.lock().unwrap().analysis(id).map_err(|e| e.to_string())?;
            analysis.map(|a| a.grid).ok_or_else(|| format!("Der Track auf Deck {} ist noch nicht analysiert.", name(deck)))
        };
        let (grid_out, grid_in) = (grid_of(from)?, grid_of(to)?);
        let plan = plan_live(
            transition,
            Roles { out: from, inn: to },
            snap.clock_beats,
            LiveDeck { grid: &grid_out, playing: true, pos: out.pos, duration: out.duration, track_bpm: out.track_bpm },
            LiveDeck { grid: &grid_in, playing: inn.playing, pos: inn.pos, duration: inn.duration, track_bpm: inn.track_bpm },
            when,
        )
        .map_err(|e| e.to_string())?;
        let mut ctl = self.ctl.lock().unwrap();
        let sent = plan.now.iter().all(|cmd| ctl.send(*cmd)) && plan.scheduled.iter().all(|(beat, cmd)| ctl.send_at(*beat, *cmd));
        if !sent {
            // Halb vorgemerkt wäre schlimmer als gar nicht.
            ctl.send(Command::CancelScheduled);
            return Err("Die Audio-Engine nimmt keine Befehle an.".into());
        }
        let run = TransitionRun { from, to, name: transition.name.clone(), bars: transition.bars, t0_beat: plan.t0, end_beat: plan.end };
        *self.transition.lock().unwrap() = Some(run.clone());
        Ok(run)
    }

    /// Bricht den eingeplanten oder laufenden Übergang ab. Die Regler bleiben stehen, wo sie sind.
    pub fn cancel_transition(&self) -> Result<(), String> {
        *self.transition.lock().unwrap() = None;
        if self.ctl.lock().unwrap().send(Command::CancelScheduled) {
            Ok(())
        } else {
            Err("Die Audio-Engine nimmt keine Befehle an.".into())
        }
    }

    /// Spielt einen Übergang zum Vorhören (TRANS-06): Beide Decks halten an, werden kurz vor die
    /// Stelle gestellt, an der der Übergang mit `when` läge, und spielen ihn ab dem Takt von
    /// `from_beat` (Beats ab T0). Am Ende – oder mit [`AppState::stop_preview`] – stehen Decks und
    /// Regler wieder so wie vorher; nur gestartet wird nichts wieder.
    pub fn preview_transition(&self, from: DeckId, to: DeckId, transition: &Transition, when: When, from_beat: f64) -> Result<TransitionRun, String> {
        let name = |d: DeckId| ["A", "B", "C", "D"][d.index()];
        // Läuft schon ein Vorhören, gilt dessen gemerkter Stand als „vorher": Die Engine hat das
        // Zurückstellen noch nicht ausgeführt, wenn hier gleich weiter geplant wird.
        let previous = self.preview.lock().unwrap().as_ref().map(|p| p.saved.clone());
        self.stop_preview()?;
        let snap = self.ctl.lock().unwrap().snapshot().clone();
        if let Some(run) = self.transition.lock().unwrap().as_ref() {
            if snap.clock_beats < run.end_beat {
                return Err(format!("Es läuft schon ein Übergang von Deck {} nach Deck {}.", name(run.from), name(run.to)));
            }
        }
        let (out, inn) = (&snap.decks[from.index()], &snap.decks[to.index()]);
        let before = |d: DeckId| -> SavedDeck {
            let s = &snap.decks[d.index()];
            previous
                .as_ref()
                .and_then(|p| p.decks.iter().find(|x| x.deck == d).cloned())
                .unwrap_or(SavedDeck { deck: d, pos: s.pos, fader: s.fader, eq_db: s.eq_db, filter: s.filter, sync: s.sync })
        };
        for deck in [from, to] {
            if !snap.decks[deck.index()].loaded {
                return Err(format!("Auf Deck {} liegt kein Track.", name(deck)));
            }
        }
        let grid_of = |deck: DeckId| -> Result<dw_core::BeatGrid, String> {
            let id = self.loaded.lock().unwrap()[deck.index()].ok_or_else(|| format!("Auf Deck {} liegt kein Track.", name(deck)))?;
            let analysis = self.lib.lock().unwrap().analysis(id).map_err(|e| e.to_string())?;
            analysis.map(|a| a.grid).ok_or_else(|| format!("Der Track auf Deck {} ist noch nicht analysiert.", name(deck)))
        };
        let (grid_out, grid_in) = (grid_of(from)?, grid_of(to)?);
        // Das neue Deck gilt nur dann als „läuft schon mit", wenn es das im Takt tut.
        let in_runs = previous.is_none() && out.playing && inn.playing && (inn.is_master || inn.sync == SyncMode::Beat);
        let plan = plan_preview(
            transition,
            Roles { out: from, inn: to },
            LiveDeck { grid: &grid_out, playing: true, pos: before(from).pos, duration: out.duration, track_bpm: out.track_bpm },
            LiveDeck { grid: &grid_in, playing: in_runs, pos: before(to).pos, duration: inn.duration, track_bpm: inn.track_bpm },
            when,
            from_beat,
        )
        .map_err(|e| e.to_string())?;
        let saved = PreviewSaved {
            decks: [from, to].map(before),
            master: previous.as_ref().map_or(snap.master, |p| p.master),
            master_auto: previous.as_ref().map_or(snap.master_auto, |p| p.master_auto),
        };
        // Die Clock läuft frei, solange kein Deck spielt. Das alte Deck startet an einem ganzen
        // Clock-Beat kurz voraus; von dort zählen alle Zeiten des Übergangs.
        let begin = (snap.clock_beats + 1.5).ceil();
        let t0 = begin - plan.start;
        let mut ctl = self.ctl.lock().unwrap();
        let prepare = [
            Command::CancelScheduled,
            Command::Play { deck: from, on: false },
            Command::Play { deck: to, on: false },
            Command::SetMaster { deck: Some(from) },
            Command::Seek { deck: from, pos: plan.out_pos },
            Command::Seek { deck: to, pos: plan.in_pos },
            Command::Sync { deck: to, mode: SyncMode::Beat },
        ];
        let sent = prepare.iter().chain(plan.now.iter()).all(|cmd| ctl.send(*cmd)) && plan.scheduled.iter().all(|(beat, cmd)| ctl.send_at(t0 + *beat, *cmd));
        drop(ctl);
        let run = TransitionRun { from, to, name: transition.name.clone(), bars: transition.bars, t0_beat: t0, end_beat: t0 + plan.stop };
        // Auch wenn nicht alles ankam: gemerkt wird, damit das Zurückstellen aufräumt.
        *self.preview.lock().unwrap() = Some(Preview { run: run.clone(), saved });
        if !sent {
            let _ = self.stop_preview();
            return Err("Die Audio-Engine nimmt keine Befehle an.".into());
        }
        Ok(run)
    }

    /// Das laufende Vorhören, falls eines läuft.
    pub fn preview_run(&self) -> Option<TransitionRun> {
        self.preview.lock().unwrap().as_ref().map(|p| p.run.clone())
    }

    /// Beendet das Vorhören: Decks anhalten, Regler, Stellen, Sync und Master wie vorher. Ohne
    /// laufendes Vorhören geschieht nichts.
    pub fn stop_preview(&self) -> Result<(), String> {
        let Some(preview) = self.preview.lock().unwrap().take() else { return Ok(()) };
        let mut cmds = vec![Command::CancelScheduled];
        for d in &preview.saved.decks {
            cmds.push(Command::Play { deck: d.deck, on: false });
        }
        for d in &preview.saved.decks {
            cmds.push(Command::Fader { deck: d.deck, value: d.fader });
            for (band, db) in [Band::Low, Band::Mid, Band::High].into_iter().zip(d.eq_db) {
                cmds.push(Command::Eq { deck: d.deck, band, db });
            }
            cmds.push(Command::Filter { deck: d.deck, pos: d.filter });
            cmds.push(Command::Sync { deck: d.deck, mode: d.sync });
            cmds.push(Command::Seek { deck: d.deck, pos: d.pos });
        }
        cmds.push(if preview.saved.master_auto { Command::MasterAuto } else { Command::SetMaster { deck: preview.saved.master } });
        let mut ctl = self.ctl.lock().unwrap();
        if cmds.iter().all(|cmd| ctl.send(*cmd)) {
            Ok(())
        } else {
            Err("Die Audio-Engine nimmt keine Befehle an.".into())
        }
    }

    /// Räumt auf, sobald das Vorhören zu Ende gespielt ist; wird mit jeder Abfrage des Zustands
    /// aufgerufen. Liefert, ob aufgeräumt wurde.
    pub fn finish_preview_if_over(&self, clock_beats: f64) -> bool {
        let over = self.preview.lock().unwrap().as_ref().is_some_and(|p| clock_beats > p.run.end_beat + 0.25);
        over && self.stop_preview().is_ok()
    }
    fn pairs_file(&self) -> PathBuf {
        self.settings_file.with_file_name("pair_transitions.json")
    }

    fn all_pair_transitions(&self) -> Vec<PairTransition> {
        std::fs::read(self.pairs_file()).ok().and_then(|bytes| serde_json::from_slice(&bytes).ok()).unwrap_or_default()
    }

    fn write_pair_transitions(&self, all: &[PairTransition]) -> Result<(), String> {
        let file = self.pairs_file();
        let json = serde_json::to_vec_pretty(all).map_err(|e| e.to_string())?;
        std::fs::write(&file, json).map_err(|e| format!("{}: {e}", file.display()))
    }

    fn track_on(&self, deck: DeckId) -> Result<String, String> {
        self.loaded.lock().unwrap()[deck.index()].map(|t| t.to_string()).ok_or_else(|| format!("Auf Deck {} liegt kein Track.", ["A", "B", "C", "D"][deck.index()]))
    }

    /// Für das Paar der Tracks auf den Decks `a` und `b` gespeicherte Übergänge (TRANS-07), in
    /// beiden Richtungen, in der Reihenfolge des Anlegens. Fehlt ein Track, ist die Liste leer.
    pub fn pair_transitions(&self, a: DeckId, b: DeckId) -> Vec<PairTransition> {
        let (Ok(ta), Ok(tb)) = (self.track_on(a), self.track_on(b)) else { return Vec::new() };
        self.all_pair_transitions().into_iter().filter(|p| (p.out_track == ta && p.in_track == tb) || (p.out_track == tb && p.in_track == ta)).collect()
    }

    /// Speichert `transition` für das Paar der Tracks auf `from` und `to`, mit dem Ort: Die Stellen,
    /// an denen beide Decks stehen, sind Beginn (`When::StartHere`) oder Ende (`When::EndHere`) –
    /// wie beim Vorhören. Läuft gerade ein Vorhören, gelten die Stellen von davor. Gleicher Name für
    /// dasselbe Paar in derselben Richtung ersetzt; sonst kommt ein Eintrag dazu. Liefert die Liste
    /// des Paars.
    pub fn save_pair_transition(&self, from: DeckId, to: DeckId, transition: &Transition, when: When) -> Result<Vec<PairTransition>, String> {
        let name = transition.name.trim();
        if name.is_empty() || name.chars().count() > 60 {
            return Err("Der Übergang braucht einen Namen mit höchstens 60 Zeichen.".into());
        }
        if !matches!(when, When::StartHere | When::EndHere) {
            return Err("Der Ort ergibt sich aus den Stellen der stehenden Decks: als Beginn oder als Ende.".into());
        }
        let (out_track, in_track) = (self.track_on(from)?, self.track_on(to)?);
        let snap = self.ctl.lock().unwrap().snapshot().clone();
        let previewed = self.preview.lock().unwrap().as_ref().map(|p| p.saved.clone());
        let pos_of = |d: DeckId| -> Result<f64, String> {
            if let Some(saved) = previewed.as_ref().and_then(|p| p.decks.iter().find(|x| x.deck == d)) {
                return Ok(saved.pos);
            }
            let s = &snap.decks[d.index()];
            if s.playing {
                return Err("Zum Speichern für das Paar müssen beide Decks stehen; ihre Stellen legen den Ort fest.".into());
            }
            Ok(s.pos)
        };
        let (out_now, in_now) = (pos_of(from)?, pos_of(to)?);
        let (grid_out, grid_in) = (self.grid_on(from)?, self.grid_on(to)?);
        let (out, inn) = (&snap.decks[from.index()], &snap.decks[to.index()]);
        let plan = plan_live(
            transition,
            Roles { out: from, inn: to },
            0.0,
            LiveDeck { grid: &grid_out, playing: true, pos: out_now, duration: out.duration, track_bpm: out.track_bpm },
            LiveDeck { grid: &grid_in, playing: false, pos: in_now, duration: inn.duration, track_bpm: inn.track_bpm },
            when,
        )
        .map_err(|e| e.to_string())?;
        let out_pos = grid_out.sec_at(grid_out.beat_at(out_now) + plan.t0);
        let mut all = self.all_pair_transitions();
        let clean = Transition { name: name.to_string(), ..transition.clone() };
        match all.iter_mut().find(|p| p.out_track == out_track && p.in_track == in_track && p.transition.name == name) {
            Some(slot) => {
                slot.out_pos = out_pos;
                slot.in_pos = plan.in_pos;
                slot.transition = clean;
            }
            None => {
                let id = all.iter().map(|p| p.id).max().unwrap_or(0) + 1;
                all.push(PairTransition { id, out_track, in_track, out_pos, in_pos: plan.in_pos, transition: clean });
            }
        }
        self.write_pair_transitions(&all)?;
        Ok(self.pair_transitions(from, to))
    }

    pub fn delete_pair_transition(&self, id: u64) -> Result<(), String> {
        let mut all = self.all_pair_transitions();
        all.retain(|p| p.id != id);
        self.write_pair_transitions(&all)
    }

    fn grid_on(&self, deck: DeckId) -> Result<dw_core::BeatGrid, String> {
        let name = ["A", "B", "C", "D"][deck.index()];
        let id = self.loaded.lock().unwrap()[deck.index()].ok_or_else(|| format!("Auf Deck {name} liegt kein Track."))?;
        let analysis = self.lib.lock().unwrap().analysis(id).map_err(|e| e.to_string())?;
        analysis.map(|a| a.grid).ok_or_else(|| format!("Der Track auf Deck {name} ist noch nicht analysiert."))
    }
    /// Ordner, in dem Mitschnitte landen.
    pub fn set_recordings_dir(&self, dir: PathBuf) {
        *self.recordings_dir.lock().unwrap() = dir;
    }

    /// Beginnt einen Mitschnitt des Master-Signals (REC-01). `stamp` ist der Zeitstempel für den
    /// Dateinamen (die Oberfläche kennt die Ortszeit); fehlt er, zählt die Unix-Zeit.
    pub fn start_recording(&self, stamp: Option<&str>) -> Result<RecordingInfo, String> {
        let dir = self.recordings_dir.lock().unwrap().clone();
        std::fs::create_dir_all(&dir).map_err(|e| format!("{}: {e}", dir.display()))?;
        // Nur Ziffern, Leerzeichen und Bindestriche: Der Name kommt von außen.
        let clean: String = stamp.unwrap_or("").chars().filter(|c| c.is_ascii_digit() || *c == ' ' || *c == '-').take(32).collect();
        let stamp = if clean.trim().is_empty() {
            std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map_or(0, |d| d.as_secs()).to_string()
        } else {
            clean.trim().to_string()
        };
        // Vorhandene Dateien nie überschreiben.
        let mut path = dir.join(format!("Deckwerk {stamp}.wav"));
        let mut n = 2;
        while path.exists() {
            path = dir.join(format!("Deckwerk {stamp} ({n}).wav"));
            n += 1;
        }
        self.recorder.start(&path)
    }

    /// Beendet den Mitschnitt und schließt die Datei ab.
    pub fn stop_recording(&self) -> Result<RecordingInfo, String> {
        self.recorder.stop()
    }

    pub fn recording_status(&self) -> Option<RecordingInfo> {
        self.recorder.status()
    }

    fn transitions_file(&self) -> PathBuf {
        self.settings_file.with_file_name("transitions.json")
    }

    /// Selbst gebaute Übergänge (TRANS-04), in der Reihenfolge des Anlegens. Fehlt die Datei oder
    /// ist sie unlesbar, ist die Liste leer.
    pub fn saved_transitions(&self) -> Vec<Transition> {
        std::fs::read(self.transitions_file()).ok().and_then(|bytes| serde_json::from_slice(&bytes).ok()).unwrap_or_default()
    }

    /// Speichert einen Übergang unter seinem Namen; ein vorhandener gleichen Namens wird ersetzt.
    pub fn save_transition(&self, transition: &Transition) -> Result<Vec<Transition>, String> {
        let name = transition.name.trim();
        if name.is_empty() || name.chars().count() > 60 {
            return Err("Der Übergang braucht einen Namen mit höchstens 60 Zeichen.".into());
        }
        if Transition::templates(transition.bars.max(1)).iter().any(|t| t.name == name) {
            return Err(format!("„{name}“ ist der Name einer eingebauten Vorlage."));
        }
        transition.validate()?;
        let mut all = self.saved_transitions();
        let clean = Transition { name: name.to_string(), ..transition.clone() };
        match all.iter_mut().find(|t| t.name == name) {
            Some(slot) => *slot = clean,
            None => all.push(clean),
        }
        self.write_transitions(&all)?;
        Ok(all)
    }

    pub fn delete_transition(&self, name: &str) -> Result<Vec<Transition>, String> {
        let mut all = self.saved_transitions();
        all.retain(|t| t.name != name);
        self.write_transitions(&all)?;
        Ok(all)
    }

    fn write_transitions(&self, all: &[Transition]) -> Result<(), String> {
        let file = self.transitions_file();
        let json = serde_json::to_vec_pretty(all).map_err(|e| e.to_string())?;
        std::fs::write(&file, json).map_err(|e| format!("{}: {e}", file.display()))
    }

    pub fn touch_library(&self) {
        self.library_rev.fetch_add(1, Ordering::Relaxed);
    }

    pub fn wake_worker(&self) {
        *self.work.0.lock().unwrap() = true;
        self.work.1.notify_all();
    }

    pub fn import(&self, paths: &[PathBuf]) -> Result<ImportReport, String> {
        let report = self.lib.lock().unwrap().import_paths(paths).map_err(|e| e.to_string())?;
        // Importierte Ordner merken: „Bibliothek aktualisieren" durchsucht sie später erneut (BIB-19).
        {
            let mut settings = self.settings.lock().unwrap();
            let mut changed = false;
            for dir in paths.iter().filter(|p| p.is_dir()) {
                changed |= add_folder(&mut settings.music_folders, dir);
            }
            if changed {
                settings.save(&self.settings_file)?;
            }
        }
        self.touch_library();
        self.wake_worker();
        Ok(report)
    }

    /// Bibliothek aktualisieren (BIB-19): durchsucht die Musikordner nach neuen Dateien, nimmt sie
    /// auf, ordnet verschobene Dateien neu zu und markiert fehlende. Bekannte Tracks bleiben unberührt.
    pub fn refresh(&self) -> Result<RefreshReport, String> {
        let folders = {
            let mut settings = self.settings.lock().unwrap();
            if settings.music_folders.is_empty() {
                // Bibliothek aus der Zeit, bevor Ordner gemerkt wurden: einmalig die Ordner übernehmen,
                // in denen ihre Tracks liegen.
                let known = self.lib.lock().unwrap().track_folders().map_err(|e| e.to_string())?;
                for dir in &known {
                    add_folder(&mut settings.music_folders, dir);
                }
                if !settings.music_folders.is_empty() {
                    settings.save(&self.settings_file)?;
                }
            }
            settings.music_folders.clone()
        };
        let (reachable, gone): (Vec<PathBuf>, Vec<PathBuf>) = folders.into_iter().partition(|dir| dir.is_dir());
        let (import, missing) = {
            let mut lib = self.lib.lock().unwrap();
            let import = lib.import_paths(&reachable).map_err(|e| e.to_string())?;
            (import, lib.mark_missing().map_err(|e| e.to_string())?)
        };
        self.touch_library();
        self.wake_worker();
        Ok(RefreshReport {
            folders: reachable.len(),
            unreachable: gone.len(),
            // Ersetzte Dateien zählen wie neue: Sie werden neu analysiert.
            added: import.added + import.updated,
            relinked: import.relinked,
            missing,
            failed: import.failed.into_iter().map(|(p, why)| (p.to_string_lossy().to_string(), why)).collect(),
        })
    }

    pub fn set_refresh_on_start(&self, on: bool) -> Result<(), String> {
        let mut settings = self.settings.lock().unwrap();
        settings.refresh_on_start = on;
        settings.save(&self.settings_file)
    }

    fn waveform_path(&self, id: TrackId) -> Result<PathBuf, String> {
        let hash = self.lib.lock().unwrap().fingerprint_of(id).map_err(|e| e.to_string())?;
        Ok(self.cache_dir.join("wave").join(format!("{hash}.dwwf")))
    }

    pub fn track_path(&self, id: TrackId) -> Result<PathBuf, String> {
        Ok(self.lib.lock().unwrap().track(id).map_err(|e| e.to_string())?.path)
    }

    /// Analysiert einen Track vollständig und legt Ergebnis und Wellenform ab.
    /// Gibt die dekodierten Audiodaten zurück, damit der Aufrufer sie weiterverwenden kann.
    pub fn analyze_track(&self, id: TrackId) -> Result<(AudioBuf, TrackAnalysis), String> {
        let path = self.track_path(id)?;
        let decoded = dw_decode::decode_file(&path).map_err(|e| format!("{}: {e}", path.display()))?;
        let analysis = dw_analysis::analyze(&decoded.audio, &dw_analysis::AnalysisConfig::default());
        let wave = dw_analysis::waveform(&decoded.audio);
        std::fs::write(self.waveform_path(id)?, wave.to_bytes()).map_err(|e| e.to_string())?;
        {
            let mut lib = self.lib.lock().unwrap();
            lib.set_analysis(id, &analysis).map_err(|e| e.to_string())?;
            // Weniger Audio als angekündigt: in der Bibliothek kenntlich machen (BIB-20).
            lib.set_incomplete(id, decoded.is_incomplete()).map_err(|e| e.to_string())?;
        }
        self.touch_library();
        Ok((decoded.audio, analysis))
    }

    /// Lädt einen Track auf ein Deck: dekodieren, in die Abtastrate der Engine wandeln, übergeben.
    pub fn load_deck(&self, deck: DeckId, id: TrackId) -> Result<(), String> {
        let stored = self.lib.lock().unwrap().analysis(id).map_err(|e| e.to_string())?;
        let (audio, analysis) = match stored {
            Some(a) => {
                let path = self.track_path(id)?;
                let decoded = dw_decode::decode_file(&path).map_err(|e| format!("{}: {e}", path.display()))?;
                // Auch beim Laden prüfen: Tracks, die vor dieser Prüfung analysiert wurden, fallen so auf.
                if decoded.is_incomplete() {
                    self.lib.lock().unwrap().set_incomplete(id, true).map_err(|e| e.to_string())?;
                    self.touch_library();
                }
                (decoded.audio, a)
            }
            // Noch nicht analysiert: jetzt nachholen, die dekodierten Daten gleich weiterverwenden.
            None => self.analyze_track(id)?,
        };
        let audio = dw_decode::resample(&audio, self.engine_rate);
        let track = Arc::new(Track { audio, grid: analysis.grid.clone(), gain_db: analysis.auto_gain_db(REFERENCE_LUFS) });
        if !self.ctl.lock().unwrap().load(deck, track) {
            return Err("Die Audio-Engine nimmt keine Befehle an.".into());
        }
        self.loaded.lock().unwrap()[deck.index()] = Some(id);
        Ok(())
    }

    /// Analyse aus der Bibliothek; fehlt sie oder ist sie veraltet, wird jetzt analysiert.
    pub fn analysis(&self, id: TrackId) -> Result<TrackAnalysis, String> {
        let stored = self.lib.lock().unwrap().analysis(id).map_err(|e| e.to_string())?;
        match stored {
            Some(a) if a.version >= ANALYSIS_VERSION => Ok(a),
            Some(a) if self.waveform_path(id).is_ok_and(|p| p.exists()) => Ok(a),
            _ => Ok(self.analyze_track(id)?.1),
        }
    }

    /// Wellenform im Binärformat aus dem Cache; fehlt sie, wird sie berechnet.
    pub fn waveform_bytes(&self, id: TrackId) -> Result<Vec<u8>, String> {
        let file = self.waveform_path(id)?;
        if let Ok(bytes) = std::fs::read(&file) {
            if Waveform::from_bytes(&bytes).is_some() {
                return Ok(bytes);
            }
        }
        self.analyze_track(id)?;
        std::fs::read(&file).map_err(|e| e.to_string())
    }
}
