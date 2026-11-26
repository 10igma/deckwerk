# Deckwerk

[![Bauen und Testen](https://github.com/10igma/deckwerk/actions/workflows/build.yml/badge.svg)](https://github.com/10igma/deckwerk/actions/workflows/build.yml)
[![Neueste Version](https://img.shields.io/github/v/release/10igma/deckwerk)](https://github.com/10igma/deckwerk/releases/latest)
[![Lizenz: GPL-3.0](https://img.shields.io/badge/Lizenz-GPL--3.0-blue)](LICENSE)

> **English:** Deckwerk is a free, open-source desktop DJ application for techno. It analyses your tracks (tempo, beat grid, key, loudness), plays them beat-synced on virtual decks, and lets you draw transitions as automation curves, preview them and play them back live. Rust audio core, Tauri shell, runs on Windows, macOS and Linux. Early version – see [status](#stand). The interface is available in German and English; the documentation is in German. [Download](https://github.com/10igma/deckwerk/releases/latest)

Deckwerk ist eine freie Desktop-DJ-Software für Techno. Sie analysiert Tracks (Tempo, Beat-Raster, Tonart, Lautheit), spielt sie synchron an virtuellen Decks und macht Übergänge planbar: Reglerverläufe beider Decks werden wie in einem Sequenzer gezeichnet, vorgehört und live abgespielt. Der Kern ist in Rust geschrieben, die Oberfläche läuft in einer nativen Hülle (Tauri) unter Windows, macOS und Linux.

![Ansicht „Mixen" während eines Übergangs: Deck A läuft aus, bei Deck B ist der Bass noch herausgenommen](docs/bilder/mixen-uebergang.png)

## Herunterladen

Installer für Windows (`.msi`, `.exe`), macOS (`.dmg`, Apple Silicon) und Linux (`.deb`, `.rpm`, `.AppImage`) hängen an der [neuesten Version](https://github.com/10igma/deckwerk/releases/latest).

Die Installer sind nicht signiert. Windows zeigt deshalb beim Start eine SmartScreen-Warnung („Weitere Informationen" → „Trotzdem ausführen"), macOS verlangt das Öffnen über das Kontextmenü („Öffnen") oder die Freigabe in den Systemeinstellungen unter „Datenschutz & Sicherheit". Die Installer entstehen in der Build-Pipeline; von Hand installiert und gestartet hat sie noch niemand – geprüft ist die Anwendung bisher aus den Quellen gebaut. Rückmeldungen sind als [Issue](https://github.com/10igma/deckwerk/issues) willkommen.

Nach dem Start: Musikordner auf das Fenster ziehen, die Analyse läuft im Hintergrund. Die Bedienung beschreibt das [Handbuch](docs/handbuch.md).

## Was Deckwerk kann

- **Dateien lesen:** MP3, WAV, AIFF, FLAC, AAC/M4A, ALAC, OGG Vorbis, Opus. Die Musikdateien werden nur gelesen, nie verändert.
- **Analyse:** Tempo, Beat-Raster, Downbeat, Tonart (Camelot), Lautheit und Wellenform, im Hintergrund; ein Track von sieben Minuten braucht ein bis zwei Sekunden.
- **Mixen:** zwei Decks in der Oberfläche (der Kern kann vier), Beat-Sync mit unter 1 ms Abweichung, Mixer mit 3-Band-EQ und Kills, Filter, Kanalfader, Crossfader und Limiter.
- **Wellenformen:** beide Decks an einer gemeinsamen Beat-Schiene, Zoom bis 128 Beats, Vorausblick auf spätere Stellen beider Tracks.
- **Übergänge:** Vorlagen (Bass-Tausch, Blende) per Taste live abspielen; im Editor eigene Kurven je Regler zeichnen, vorhören und speichern – allgemein oder für ein bestimmtes Paar von Tracks mit Ort.
- **Bibliothek:** Import durch Ablegen, Suche, Filter „Passend" (Tempo und Tonart zum laufenden Deck), Aktualisieren der Musikordner, Erkennen unvollständiger Dateien.
- **Mitschnitt:** das Master-Signal als WAV mit 24 bit.
- **Kommandozeile:** analysieren, Übergänge offline rendern, Analyse gegen Referenzwerte prüfen.

## Stand

Version 0.1.0 ist eine frühe Fassung: Fundament und Kern-Mixing sind gebaut (Stufe 0 und 1 von 8 im [Umsetzungsplan](docs/spezifikation.md#11-umsetzungsplan)), dazu vorgezogen Übergänge und Mitschnitt. Vieles ist gemessen, aber noch wenig am Gerät und mit echter Musik geprüft. Der Name ist ein Arbeitstitel.

| Bereich | Stand |
|---|---|
| Dateien lesen | fertig, getestet |
| Analyse | auf 40 synthetischen Tracks alle Werte getroffen; an 52 echten Tracks gegen Werte aus dem Netz: Tempo 28 von 28 bei doppelt belegter Referenz, 37 von 48 mit den einfach belegten, Tonart 29 von 34. Schwächen bei Material unter 110 BPM (halbes oder doppeltes Tempo) und bei der Wahl zwischen Dur und Moll |
| Engine und Mixer | fertig, gegen die Zahlen der Spezifikation getestet; Klang gemessen, nicht systematisch gehört |
| Oberfläche | Ansicht „Mixen" mit zwei Decks; die Ansichten „Bibliothek", „Vorbereiten" und „Auto-Mix" fehlen |
| Übergänge | Vorlagen, Editor, Vorhören und Speichern je Paar sind gebaut und in den Rechenregeln getestet; in der laufenden Anwendung erst teilweise ausprobiert |
| Ton-Ausgabe | unter Windows (WASAPI) und macOS (CoreAudio) am eingebauten Ausgabegerät gehört; Latenz, externe Interfaces, Kopfhörer-Weg, ASIO und Linux (ALSA/JACK) sind ungeprüft |
| Systeme | unter Windows, macOS und Linux aus den Quellen gebaut und gestartet; die Installer baut die Pipeline, gestartet wurden sie noch nicht |
| Noch nicht begonnen | Key Lock, Loops, Hotcues, Effekte, Auto-Mix für ganze Sets, Stems, Remix-Decks, Import aus Traktor und Rekordbox. Controller (MIDI/HID) sind nicht vorgesehen |

Den genauen Stand mit Messwerten, offenen Punkten und dem nächsten Schritt führt [docs/fortschritt.md](docs/fortschritt.md).

## Bilder

| | |
|---|---|
| ![Zwei geladene Decks](docs/bilder/mixen-geladen.png) Zwei Tracks mit unterschiedlichem Tempo. Die Wellenformen wachsen von einer gemeinsamen Beat-Schiene weg; der Versatz der Beats ist auf der Schiene gelb markiert. | ![Echte Anwendung unter Linux](docs/bilder/anwendung-linux.png) Die Desktop-Anwendung mit echtem Kern unter Linux: drei importierte und analysierte Tracks, zwei davon laufen synchron. Ein Audiogerät gab es auf der Testmaschine nicht, daher der Hinweis „Kein Audio". |
| ![Leere Ansicht](docs/bilder/mixen-leer.png) Nach dem ersten Start: Musik wird auf das Fenster gezogen. | ![Bibliothek während der Analyse](docs/bilder/bibliothek-analyse.png) Importierte Tracks werden im Hintergrund analysiert. |
| ![Schutz vor Fehlgriffen](docs/bilder/schutz-stopp.png) Ein hörbares Deck stoppt erst beim zweiten Griff. | |

Bis auf das Bild der echten Anwendung stammen die Bilder aus der Oberfläche mit simuliertem Kern; die Tracknamen darin sind erfunden. Sie zeigen einen älteren Stand der Oberfläche: Editor für Übergänge, Vorausblick und Mitschnitt sind darauf noch nicht zu sehen. Sie entstehen automatisch (`pnpm shots` in `ui/`) und werden mit `ui/scripts/doku-bilder.sh` in die Dokumentation übernommen.

## Selbst bauen

Voraussetzungen: Rust (stabil, ab 1.85), Node 22 mit pnpm 10, ein C-Compiler und CMake. Unter Linux zusätzlich:

```bash
sudo apt-get install -y libwebkit2gtk-4.1-dev librsvg2-dev libasound2-dev
```

Unter Windows mit den MSVC-Buildtools; die Einzelheiten stehen in [docs/fortschritt.md](docs/fortschritt.md#bauen-unter-windows).

Oberfläche bauen (sie wird in die Anwendung eingebettet), dann starten:

```bash
pnpm --dir ui install
```

```bash
pnpm --dir ui build
```

```bash
cargo run --release -p deckwerk-app
```

Ohne Oberfläche, über die Kommandozeile:

```bash
cargo run --release -p dw-cli -- analyze track1.mp3 track2.flac
```

```bash
cargo run --release -p dw-cli -- mix track1.mp3 track2.flac -o mix.wav --bars 32
```

Wer keine passenden Tracks zur Hand hat, erzeugt Testmaterial mit bekanntem Tempo und bekannter Tonart:

```bash
cargo run --release -p dw-cli -- testtrack a.wav --bpm 132 --key 8A
```

Alle Tests:

```bash
cargo test --workspace
```

```bash
pnpm --dir ui test
```

Eine neue Version entsteht über ein Tag der Form `v0.1.0`: Die Pipeline baut dann die Installer für alle drei Systeme und veröffentlicht sie ([release.yml](.github/workflows/release.yml)). Die Versionsnummer steht in `Cargo.toml`, `app/tauri.conf.json` und `ui/package.json`.

## Aufbau

```
crates/
  dw-core       Grundtypen: Tonart, Beatgrid, Befehle, Zustands-Schnappschuss
  dw-dsp        EQ, Filter, Limiter, Interpolation, Abtastratenwandlung
  dw-decode     Audiodateien lesen, WAV schreiben
  dw-analysis   Tempo, Beatgrid, Tonart, Lautheit, Wellenform
  dw-engine     Decks, Master-Clock, Sync, Mixer, Offline-Render
  dw-audio      Echtzeit-Anbindung und Ausgabe am Audiogerät
  dw-library    Bibliothek (SQLite): Import, Suche, Playlists, Sicherung
  dw-automix    Übergänge: Automationsspuren, Vorlagen, Planung für live und offline
  dw-testgen    synthetische Referenz-Tracks für die Tests
  dw-cli        Kommandozeile „deckwerk"
app/            Desktop-Hülle (Tauri 2)
ui/             Oberfläche (TypeScript, React, WebGL 2)
docs/           Spezifikation, Handbuch, Fortschritt, Designsystem
```

Der Ton entsteht vollständig im Rust-Kern in einem eigenen Echtzeit-Thread. Die Oberfläche zeigt nur an und sendet Befehle; ruckelt sie, bleibt der Ton unberührt.

## Dokumentation

| Dokument | Inhalt |
|---|---|
| [Handbuch](docs/handbuch.md) | Bedienung: Musik importieren, mixen, Tastenbelegung, Kommandozeile |
| [Spezifikation](docs/spezifikation.md) | Anforderungen, Verfahren, Architektur, Umsetzungsplan |
| [Fortschritt](docs/fortschritt.md) | was fertig ist, was gemessen wurde, was offen ist, nächster Schritt |
| [Designsystem](docs/designsystem.md) | Leitidee, Farben, Schriften, Komponenten |
| [Schnittstelle Oberfläche–Kern](docs/kern-schnittstelle.md) | Befehle und Datenformate zwischen `ui/` und `app/` |
| [CLAUDE.md](CLAUDE.md) | Einstieg für Claude Code: Befehle, Aufbau, Regeln |
| [Prompts](docs/prompts/) | die beiden Aufträge, aus denen Spezifikation und Bau hervorgegangen sind |

## Lizenz

Deckwerk ist freie Software unter der [GNU General Public License, Version 3](LICENSE): Jeder darf sie nutzen, untersuchen, verändern und weitergeben; wer eine veränderte Fassung weitergibt, muss deren Quelltext unter derselben Lizenz offenlegen. Es gibt keine Gewährleistung.

Die verwendeten Bausteine stehen unter MIT, Apache-2.0, BSD oder MPL-2.0.
