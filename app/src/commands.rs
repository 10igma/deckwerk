//! Befehle, die die Oberfläche aufruft. Sie bilden `CoreApi` aus `ui/src/core/api.ts` ab.

use crate::state::AppState;
use dw_core::{Command, DeckId, Snapshot, TrackAnalysis, ANALYSIS_VERSION};
use dw_library::{SortColumn, TrackQuery};
use serde::Serialize;
use std::path::PathBuf;
use std::sync::atomic::Ordering;
use tauri::State;

fn parse_id(id: &str) -> Result<i64, String> {
    id.parse().map_err(|_| format!("ungültige Track-Kennung „{id}“"))
}

#[tauri::command]
pub fn core_send(state: State<AppState>, cmd: Command) -> Result<(), String> {
    if state.ctl.lock().unwrap().send(cmd) {
        Ok(())
    } else {
        Err("Die Audio-Engine nimmt keine Befehle an.".into())
    }
}

#[derive(Serialize)]
pub struct EngineStatus {
    dsp_load: f32,
    xruns: u32,
    sample_rate: u32,
    buffer_size: u32,
    has_device: bool,
    device_error: Option<String>,
    /// Gewähltes Ausgabegerät; `None` = Standardgerät des Systems.
    device: Option<String>,
}

#[derive(Serialize)]
pub struct Poll {
    snapshot: Snapshot,
    status: EngineStatus,
    /// Ändert sich, wenn die Bibliothek neu geladen werden muss.
    library_rev: u64,
    /// Tracks, die noch auf ihre Analyse warten.
    pending_analysis: usize,
}

#[tauri::command]
pub fn core_poll(state: State<AppState>) -> Poll {
    let (snapshot, dsp_load, xruns) = {
        let mut ctl = state.ctl.lock().unwrap();
        (ctl.snapshot().clone(), ctl.take_dsp_load_peak(), ctl.xruns())
    };
    let pending_analysis = state.lib.lock().unwrap().pending_analysis(ANALYSIS_VERSION).map_or(0, |p| p.len());
    let audio = state.audio.lock().unwrap().clone();
    Poll {
        snapshot,
        status: EngineStatus {
            dsp_load,
            xruns,
            sample_rate: audio.sample_rate,
            buffer_size: audio.buffer_size,
            has_device: audio.has_device,
            device_error: audio.error,
            device: audio.device,
        },
        library_rev: state.library_rev.load(Ordering::Relaxed),
        pending_analysis,
    }
}

#[derive(Serialize)]
pub struct TrackInfo {
    id: String,
    title: String,
    artist: String,
    /// 0, solange weder Analyse noch Datei-Angabe vorliegt.
    bpm: f64,
    key: Option<dw_core::Key>,
    duration: f64,
    analyzed: bool,
    missing: bool,
    incomplete: bool,
}

#[tauri::command(async)]
pub fn library_list(state: State<AppState>) -> Result<Vec<TrackInfo>, String> {
    let rows = state
        .lib
        .lock()
        .unwrap()
        .tracks(&TrackQuery { sort: SortColumn::Title, ..Default::default() })
        .map_err(|e| e.to_string())?;
    Ok(rows
        .into_iter()
        .map(|t| TrackInfo {
            id: t.id.to_string(),
            title: t.title,
            artist: t.artist,
            bpm: t.bpm.unwrap_or(0.0),
            key: t.key,
            duration: t.duration,
            analyzed: t.analyzed,
            missing: t.missing,
            incomplete: t.incomplete,
        })
        .collect())
}

/// Eingebaute Vorlagen für Übergänge in der gewünschten Länge (TRANS-01).
#[tauri::command]
pub fn transition_templates(bars: u32) -> Vec<dw_automix::Transition> {
    dw_automix::Transition::templates(bars.clamp(1, 256))
}

/// Vorbereiteten Übergang einplanen (TRANS-05).
#[tauri::command]
pub fn transition_start(
    state: State<AppState>,
    from: DeckId,
    to: DeckId,
    transition: dw_automix::Transition,
    when: dw_automix::When,
) -> Result<crate::state::TransitionRun, String> {
    state.start_transition(from, to, &transition, when)
}

#[tauri::command]
pub fn transition_cancel(state: State<AppState>) -> Result<(), String> {
    state.cancel_transition()
}

/// Musikordner neu durchsuchen (BIB-19).
#[tauri::command(async)]
pub fn library_refresh(state: State<AppState>) -> Result<crate::state::RefreshReport, String> {
    state.refresh()
}

#[derive(Serialize)]
pub struct AppSettings {
    refresh_on_start: bool,
    music_folders: Vec<String>,
}

#[tauri::command]
pub fn settings_get(state: State<AppState>) -> AppSettings {
    let s = state.settings.lock().unwrap();
    AppSettings { refresh_on_start: s.refresh_on_start, music_folders: s.music_folders.iter().map(|p| p.to_string_lossy().to_string()).collect() }
}

#[tauri::command]
pub fn settings_refresh_on_start(state: State<AppState>, on: bool) -> Result<(), String> {
    state.set_refresh_on_start(on)
}

#[derive(Serialize)]
pub struct ImportResult {
    added: usize,
    duplicates: usize,
    relinked: usize,
    failed: Vec<(String, String)>,
}

#[tauri::command(async)]
pub fn library_import(state: State<AppState>, paths: Vec<String>) -> Result<ImportResult, String> {
    let paths: Vec<PathBuf> = paths.into_iter().map(PathBuf::from).collect();
    let r = state.import(&paths)?;
    Ok(ImportResult {
        added: r.added,
        duplicates: r.duplicates,
        relinked: r.relinked,
        failed: r.failed.into_iter().map(|(p, why)| (p.to_string_lossy().to_string(), why)).collect(),
    })
}

/// Lädt einen Track auf ein Deck: dekodieren, in die Abtastrate der Engine wandeln, übergeben (DECK-04).
#[tauri::command(async)]
pub fn deck_load(state: State<AppState>, deck: DeckId, track_id: String) -> Result<(), String> {
    state.load_deck(deck, parse_id(&track_id)?)
}

#[tauri::command]
pub fn deck_loaded(state: State<AppState>) -> serde_json::Value {
    let loaded = state.loaded.lock().unwrap();
    let id = |i: usize| loaded[i].map(|t| t.to_string());
    serde_json::json!({ "a": id(0), "b": id(1), "c": id(2), "d": id(3) })
}

/// Wellenform im Binärformat „DWWF" (siehe `Waveform::to_bytes` in dw-core).
#[tauri::command(async)]
pub fn track_waveform(state: State<AppState>, track_id: String) -> Result<tauri::ipc::Response, String> {
    Ok(tauri::ipc::Response::new(state.waveform_bytes(parse_id(&track_id)?)?))
}

#[tauri::command(async)]
pub fn track_analysis(state: State<AppState>, track_id: String) -> Result<TrackAnalysis, String> {
    state.analysis(parse_id(&track_id)?)
}

#[derive(Serialize)]
pub struct DeviceEntry {
    host: String,
    name: String,
    is_default: bool,
    max_channels: u16,
    sample_rates: Vec<u32>,
}

#[tauri::command(async)]
pub fn audio_devices() -> Vec<DeviceEntry> {
    dw_audio::device::list_devices()
        .into_iter()
        .map(|d| DeviceEntry { host: d.host, name: d.name, is_default: d.is_default, max_channels: d.max_channels, sample_rates: d.sample_rates })
        .collect()
}

/// Wechselt das Ausgabegerät; `device: null` = Standardgerät des Systems (ENG-01).
#[tauri::command(async)]
pub fn audio_select(state: State<AppState>, device: Option<String>) -> Result<(), String> {
    state.select_device(device).map(|_| ())
}
