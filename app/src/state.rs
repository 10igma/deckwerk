//! Gemeinsamer Zustand der Anwendung: Audio-Steuerung, Bibliothek, Cache.

use crate::audio::{self, AudioInfo};
use dw_audio::Controller;
use dw_core::{AudioBuf, DeckId, TrackAnalysis, Waveform, ANALYSIS_VERSION};
use dw_engine::Track;
use dw_library::{ImportReport, Library, TrackId};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Condvar, Mutex};

/// Referenz-Lautheit für den Auto-Gain (MIX-01).
const REFERENCE_LUFS: f32 = -14.0;

pub struct Inner {
    pub ctl: Mutex<Controller>,
    pub audio: AudioInfo,
    pub lib: Mutex<Library>,
    pub loaded: Mutex<[Option<TrackId>; 4]>,
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
        let (ctl, audio) = audio::start();
        Ok(AppState(Arc::new(Inner {
            ctl: Mutex::new(ctl),
            audio,
            lib: Mutex::new(lib),
            loaded: Mutex::new([None; 4]),
            cache_dir,
            library_rev: AtomicU64::new(1),
            work: (Mutex::new(true), Condvar::new()),
        })))
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
        self.touch_library();
        self.wake_worker();
        Ok(report)
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
        self.lib.lock().unwrap().set_analysis(id, &analysis).map_err(|e| e.to_string())?;
        self.touch_library();
        Ok((decoded.audio, analysis))
    }

    /// Lädt einen Track auf ein Deck: dekodieren, in die Abtastrate der Engine wandeln, übergeben.
    pub fn load_deck(&self, deck: DeckId, id: TrackId) -> Result<(), String> {
        let stored = self.lib.lock().unwrap().analysis(id).map_err(|e| e.to_string())?;
        let (audio, analysis) = match stored {
            Some(a) => {
                let path = self.track_path(id)?;
                (dw_decode::decode_file(&path).map_err(|e| format!("{}: {e}", path.display()))?.audio, a)
            }
            // Noch nicht analysiert: jetzt nachholen, die dekodierten Daten gleich weiterverwenden.
            None => self.analyze_track(id)?,
        };
        let audio = dw_decode::resample(&audio, self.audio.sample_rate);
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
