//! Desktop-Anwendung: verbindet die Oberfläche (ui/) mit dem Kern.

pub mod audio;
pub mod commands;
pub mod state;
pub mod worker;

use state::AppState;
use tauri::Manager;

pub fn run() {
    tauri::Builder::default()
        .setup(|app| {
            let data_dir = match std::env::var_os("DECKWERK_DATA") {
                Some(dir) => std::path::PathBuf::from(dir),
                None => app.path().app_data_dir()?,
            };
            let state = AppState::start(&data_dir).map_err(|e| Box::<dyn std::error::Error>::from(e))?;
            // Musikordner aus der Umgebung beim Start importieren (für Tests und den ersten Start).
            if let Some(dir) = std::env::var_os("DECKWERK_MUSIC") {
                let _ = state.import(&[std::path::PathBuf::from(dir)]);
            }
            // Musikordner nach neuen Dateien durchsuchen, ohne den Start aufzuhalten (abschaltbar).
            if state.settings.lock().unwrap().refresh_on_start {
                let state = state.clone();
                std::thread::spawn(move || {
                    let _ = state.refresh();
                });
            }
            worker::spawn(state.clone());
            app.manage(state);
            Ok(())
        })
        .on_window_event(|window, event| {
            // Dateien und Ordner, die auf das Fenster gezogen werden, wandern in die Bibliothek (BIB-01).
            if let tauri::WindowEvent::DragDrop(tauri::DragDropEvent::Drop { paths, .. }) = event {
                let state = window.state::<AppState>().inner().clone();
                let paths = paths.clone();
                std::thread::spawn(move || {
                    let _ = state.import(&paths);
                });
            }
        })
        .invoke_handler(tauri::generate_handler![
            commands::core_send,
            commands::core_poll,
            commands::library_list,
            commands::library_import,
            commands::library_refresh,
            commands::settings_get,
            commands::settings_refresh_on_start,
            commands::deck_load,
            commands::deck_loaded,
            commands::track_waveform,
            commands::track_analysis,
            commands::audio_devices,
            commands::audio_select,
        ])
        .run(tauri::generate_context!())
        .expect("Deckwerk konnte nicht gestartet werden");
}
