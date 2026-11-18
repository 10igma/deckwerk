// Unter Windows kein Konsolenfenster neben der Anwendung.
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

fn main() {
    deckwerk_app::run()
}
