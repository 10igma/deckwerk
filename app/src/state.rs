//! Gemeinsamer Zustand der Anwendung: Audio-Steuerung, Bibliothek, Cache.

use crate::audio::{self, AudioHandle, AudioInfo};
use dw_audio::Controller;
use dw_automix::{plan_live, LiveDeck, Roles, Transition};
use dw_core::{AudioBuf, Command, DeckId, SyncMode, TrackAnalysis, Waveform, ANALYSIS_VERSION};
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
        let (ctl, audio, audio_handle) = audio::start(settings.audio_device.clone());
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

    /// Plant einen vorbereiteten Übergang vom laufenden Deck `from` zum stehenden Deck `to` ein
    /// (TRANS-05, AUTO-08): Er beginnt an der nächsten Taktgrenze im Raster von `align_bars` Takten,
    /// die Engine fährt die Regler sample-genau. Das neue Deck rückt auf den Taktanfang neben seiner
    /// Position und bekommt Beat-Sync.
    pub fn start_transition(&self, from: DeckId, to: DeckId, transition: &Transition, align_bars: u32) -> Result<TransitionRun, String> {
        let name = |d: DeckId| ["A", "B", "C", "D"][d.index()];
        let snap = self.ctl.lock().unwrap().snapshot().clone();
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
        if inn.playing {
            return Err(format!("Deck {} läuft schon; der Übergang startet es selbst am Einstieg.", name(to)));
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
            LiveDeck { grid: &grid_out, pos: out.pos, duration: out.duration, track_bpm: out.track_bpm },
            LiveDeck { grid: &grid_in, pos: inn.pos, duration: inn.duration, track_bpm: inn.track_bpm },
            align_bars,
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
