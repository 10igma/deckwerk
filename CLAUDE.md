# Deckwerk

Desktop-DJ-Software für Techno. Kern in Rust (`crates/`), Desktop-Hülle mit Tauri 2 (`app/`), Oberfläche in TypeScript/React/WebGL 2 (`ui/`).

## Zuerst lesen

1. `docs/fortschritt.md` – Stand, nächster Schritt, offene Punkte, Abnahmeliste. Nach jeder Arbeit aktualisieren.
2. `docs/spezifikation.md` – verbindlich. Anforderungen haben Kennungen (z. B. `MIX-02`); in Tests und Commits darauf verweisen. Bei Lücken oder Widersprüchen begründet entscheiden und die Entscheidung in `docs/fortschritt.md` festhalten; wenn sich die Umsetzung als richtig und die Spezifikation als falsch erweist, die Spezifikation anpassen.
3. Nach Bedarf: `docs/handbuch.md` (Bedienung), `docs/designsystem.md` (Gestaltung), `docs/kern-schnittstelle.md` (Befehle zwischen `ui/` und `app/`).

## Voraussetzungen

Rust (stabil, ab 1.85), Node 22, pnpm 10, C-Compiler, CMake (für den mitgebauten Opus-Decoder).

- macOS: `xcode-select --install`, `brew install cmake pnpm`
- Linux: `sudo apt-get install -y libwebkit2gtk-4.1-dev librsvg2-dev libasound2-dev cmake`

## Befehle

Aus dem Wurzelordner:

| Zweck | Befehl |
|---|---|
| Oberfläche bauen (muss vor dem Bau von `app/` geschehen, sie wird eingebettet) | `pnpm --dir ui install && pnpm --dir ui build` |
| Anwendung starten | `cargo run --release -p deckwerk-app` |
| Alle Kern-Tests | `cargo test --workspace` |
| Tests eines Crates | `cargo test -p dw-engine` |
| Oberfläche prüfen | `pnpm --dir ui typecheck && pnpm --dir ui test` |
| Oberfläche im Browser mit simuliertem Kern | `pnpm --dir ui dev` (Zustände per `?scene=empty\|loaded\|synced\|transition`) |
| Screenshots und Maßprüfung der Oberfläche | `pnpm --dir ui build && pnpm --dir ui shots` |
| Kommandozeile | `cargo run --release -p dw-cli -- analyze <Datei>` bzw. `mix <A> <B> -o mix.wav` bzw. `check docs/referenz-tracks.csv <Musikordner>` (Analyse gegen Referenzwerte); `raster <Datei>` misst, ob das Beat-Raster auf den Anschlägen sitzt; `transition <A> <B> -o mix.wav …` rendert einen Übergang wie live |
| Installer (aus `app/`) | `node ../ui/node_modules/@tauri-apps/cli/tauri.js build` |

Umgebungsvariablen der Anwendung: `DECKWERK_DATA` (Ordner für Bibliothek und Cache), `DECKWERK_MUSIC` (Ordner, der beim Start importiert wird), `DECKWERK_RECORDINGS` (Ordner für Mitschnitte).

## Aufbau

| Ort | Inhalt |
|---|---|
| `crates/dw-core` | Grundtypen, Befehle (`Command`), Zustand (`Snapshot`) – die Schnittstelle aller Teile |
| `crates/dw-dsp` | EQ, Filter, Limiter, Interpolation, Abtastratenwandlung |
| `crates/dw-engine` | Decks, Clock, Sync, Mixer; läuft in Echtzeit und offline identisch |
| `crates/dw-audio` | Echtzeit-Anbindung (Ringpuffer) und Geräte-Ausgabe (cpal) |
| `crates/dw-analysis`, `dw-decode`, `dw-library`, `dw-automix` | Analyse, Dateien lesen, Bibliothek (SQLite), Übergänge |
| `crates/dw-testgen` | synthetische Tracks mit bekannter Grundwahrheit für Tests |
| `crates/dw-cli` | Kommandozeile `deckwerk` |
| `app/` | Tauri-Hülle: Audio-Start, Zustand, Hintergrund-Analyse, Befehle |
| `ui/src/core/` | `CoreApi` mit zwei Implementierungen: `TauriCore` (echt) und `MockCore` (Browser) |
| `ui/src/design/` | Tokens und Komponenten des Designsystems |

## Regeln

- **Audio-Thread:** In `Engine::process` und `Processor::process` kein Speicher anfordern oder freigeben, keine Sperren, keine Datei- oder Systemaufrufe. Tests mit zählendem Allocator sichern das ab (`crates/dw-engine/tests/realtime.rs`, `crates/dw-audio/tests/host.rs`).
- **Befehlsschicht:** Jede Bedienhandlung ist ein `Command` in `dw-core`. Neue Funktionen bekommen dort einen Befehl; die Oberfläche spricht den Kern nur über `CoreApi` an. Ändert sich `Command` oder `Snapshot`, müssen `ui/src/core/types.ts`, der `MockCore` und `docs/kern-schnittstelle.md` mitziehen.
- **Messbar prüfen:** Signalverarbeitung, Analyse und Übergänge werden gegen Zahlen aus der Spezifikation getestet (Frequenzgang, Abweichung in ms, Trefferquote), über den Offline-Render. Kein Test, der nur „läuft durch" prüft.
- **Zeit-Tests:** Mehrfach messen und den schnellsten Lauf werten; die Build-Maschinen sind unterschiedlich ausgelastet.
- **Lizenzen:** Keine Abhängigkeiten unter GPL. Erlaubt sind MIT, Apache-2.0, BSD, MPL-2.0.
- **Abhängigkeiten:** Symphonia 0.5, cpal 0.18, rusqlite 0.32, lofty 0.22, Tauri 2 sind bewusst festgelegt; ein Versionssprung ist eine eigene Aufgabe.
- **Sprache:** Bezeichner englisch; Kommentare, Dokumentation, Commit-Nachrichten und Texte der Oberfläche deutsch. Texte der Oberfläche stehen in `ui/src/i18n/` (Deutsch und Englisch).
- **Oberfläche:** Farben, Abstände und Schriften nur über die Tokens in `ui/src/design/tokens.css`. Animierte Flächen (Wellenformen, Pegel) zeichnen in einer eigenen Bildschleife, nicht über React. Bedienziele mindestens 28 px hoch bei 1280×720; `pnpm shots` misst das.
- **Vor jedem Commit:** `cargo test --workspace` und, wenn `ui/` berührt wurde, `pnpm --dir ui typecheck && pnpm --dir ui test` müssen grün sein. Kein Commit mit fehlschlagenden Tests.
- **Ehrlich berichten:** Was nur übersetzt oder nur gemessen, aber nicht am Gerät oder mit echter Musik geprüft ist, steht in `docs/fortschritt.md` unter „Nicht geprüft". Dort nichts streichen, bevor es wirklich geprüft wurde.

## Was sich hier nicht prüfen lässt

Klang, Latenz und Stabilität am Audio-Interface, Kopfhörer-Weg, Analyse an echter Musik. Dafür gibt es die Abnahmeliste in `docs/fortschritt.md`; die Ergebnisse trägt dort ein, wer sie am Gerät durchgeht.
