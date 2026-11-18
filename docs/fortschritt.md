# Fortschritt

Stand: 18.11.2026. Diese Datei zuerst lesen, dann `docs/spezifikation.md`.

## Wo wir stehen

Stufe 0 (Fundament) und der Kern von Stufe 1 (Kern-Mixing) sind gebaut und getestet. Die Oberfläche läuft im Browser gegen einen simulierten Kern. **Noch nicht verbunden:** Oberfläche und echter Kern (Tauri-Hülle fehlt). Bis dahin ist das Programm über die Kommandozeile nutzbar.

## Nächster Schritt

1. Tauri-Hülle unter `app/` anlegen: `CoreApi` der Oberfläche (`ui/src/core/`) auf den echten Kern legen (`dw-audio::Controller`, `dw-library`, `dw-analysis`, `dw-decode`). Die Systembibliotheken für Linux sind in dieser Umgebung installierbar (`libwebkit2gtk-4.1-dev`, `libasound2-dev`).
2. Hintergrund-Analyse mit Warteschlange (ANA-01) und Wellenform-Cache auf der Platte (`Waveform::to_bytes`).
3. Einstellungen für Audiogerät, Kanäle, Puffer (ENG-01–03, ENG-05) und Testton.
4. Installer in der Build-Pipeline ergänzen.
5. Danach Stufe 2 (Key Lock, Loops, Hotcues, vier Decks in der Oberfläche, Mitschnitt).

## Erledigte Anforderungen

| Bereich | Erledigt | Nachweis |
|---|---|---|
| Bibliothek | BIB-01, 02, 03, 05, 06, 08, 09, 18 | `cargo test -p dw-library`, `-p dw-decode` |
| Analyse | ANA-02, 03, 04, 06, 07, 08, 16 (auf synthetischem Referenz-Set) | `cargo run --release -p dw-analysis --example kennzahlen` |
| Decks | DECK-01, 05 (ohne CDJ-Cue), 06, 07, 08, 09, 20 (im Kern) | `cargo test -p dw-engine` |
| Mixer | MIX-01, 02, 04, 05, 06, 08, 09, 10, 13 | `cargo test -p dw-dsp`, `-p dw-engine`, `-p dw-audio` |
| Engine | ENG-04, 06, 07, 09; ENG-01–03 nur übersetzt | `cargo test -p dw-engine --test realtime`, `-p dw-audio` |
| Übergänge | T1 und T10 aus 5.10; TRANS-02, TRANS-03 für T1 | `cargo test -p dw-automix` |
| Oberfläche | Designsystem, Hülle, Ansicht „Mixen" gegen simulierten Kern; VIS-01, 02, 04, 05, 06, 10 | `pnpm shots` in `ui/` |

Gemessene Werte (diese Maschine, 4 Kerne):

- Sync zweier Decks über 10 Minuten: größte Abweichung 1 Sample (0,02 ms). Ziel ≤ 1 ms.
- Offline-Render mit zwei Decks: 61-fache Echtzeit. Ziel ≥ 10-fach.
- Analyse eines 7-Minuten-Tracks: 1,2 s. Ziel ≤ 15 s.
- Suche in 20 000 Tracks: 32 ms. Ziel ≤ 100 ms.
- EQ in Mittelstellung: ±0,1 dB von 20 Hz bis 20 kHz. Filter in Mittelstellung bitgenau.
- Referenz-Set (40 synthetische Tracks): Tempo 40/40 innerhalb ±0,05 BPM, Beat-Abweichung im Mittel 0,14 ms, Downbeat 40/40, Tonart 40/40.

## Nicht geprüft

- **Analyse an echter Musik.** Generator und Analyse stammen aus derselben Hand; die 100 % auf dem synthetischen Set sagen wenig über echte Tracks. Braucht die Referenzsammlung Teil B/C (Spezifikation 10.1).
- **Ausgabe am Gerät.** `dw-audio::device` ist nur übersetzt. Latenz, Kopfhörer-Weg und Stabilität am Interface sind ungemessen.
- **Windows und macOS.** Gebaut und getestet wurde nur unter Linux. Die Pipeline in `.github/workflows/build.yml` ist geschrieben, aber noch nie gelaufen (kein GitHub-Repository).
- **Klang.** EQ, Filter, Limiter und Übergang sind gemessen, nicht gehört.

## Offene Punkte im gebauten Teil

- Limiter hält die Decke als Sample-Spitze (−0,3 dBFS), nicht als True Peak (MIX-08 verlangt dBTP).
- Wiedergabe mit verändertem Tempo: Interpolation mit 32 Stützstellen, eben bis etwa 17 kHz; Sperrdämpfung 80 dB ist nicht nachgewiesen (5.13).
- Sprünge im laufenden Deck blenden kurz aus und wieder ein (rund 4 ms Lücke) statt überzublenden. Für Loops und Hotcues (Stufe 2) wird eine echte Überblendung gebraucht.
- Tempo-Analyse liefert immer einen Anker; Erkennung von Temposchwankungen fehlt (ANA-05, Stufe 3).
- Tracks außerhalb 110–170 BPM werden mit der Techno-Voreinstellung falsch oktaviert (90 → 180). Die Voreinstellung „frei" existiert, ist aber nirgends wählbar.
- Inhalts-Fingerabdruck der Bibliothek ändert sich, wenn Tags am Dateianfang umgeschrieben werden; die Analyse hängt dann an der alten Kennung.
- AAC-Vorlauf wird aus der Schnittliste der MP4-Datei gelesen; Dateien ohne Schnittliste (nur iTunSMPB-Angabe) sind nicht abgedeckt.
- Stille Tracks: Lautheit −∞ lässt sich nicht als JSON zurücklesen.
- Tempo-Angleichung im Übergang ändert ohne Key Lock die Tonhöhe (Key Lock kommt in Stufe 2).
- CDJ-Verhalten der Cue-Taste, Auto-Master-Regel „hörbar" (nur „läuft" ist umgesetzt), Fader-Kurven zur Auswahl und 4-Band-EQ fehlen.

## Entscheidungen

| Entscheidung | Grund |
|---|---|
| Kern in Rust, Oberfläche mit Tauri 2, React, WebGL 2 | Spezifikation 7.1 |
| Symphonia 0.5 statt 0.6, cpal 0.15, rusqlite 0.32, lofty 0.22 | bekannte, stabile Schnittstellen; Anheben als eigene Aufgabe |
| Opus über libopus (mitgebaut) | Symphonia hat keinen eigenen Opus-Decoder |
| Abtastratenwandlung selbst geschrieben statt rubato | wenig Code, vollständig testbar, keine fremde Schnittstelle |
| Beat-Sync als feste Kopplung an die Clock, kein Regelkreis | Abweichung bleibt unter einem Sample; Anschieben verschiebt den Versatz |
| Start ohne Einblenden | der Anschlag am Cue-Punkt bleibt erhalten |
| EQ-Kill mit 24 dB/Oktave | übliche Isolator-Flanke; Abnahmekriterium in MIX-02 angepasst |
| Kanalfader quadratisch | linear in dB wäre bei halbem Weg schon −30 dB; MIX-05 angepasst |
| Bass-Regel wird gegen den Einzelpegel der Tracks gemessen | der Pegel vor dem Übergang ist kein fairer Maßstab, wenn A dort keinen Bass hat; TRANS-03 angepasst |
| Wellenform im Cache mit einem Byte je Wert | 3–4 MB je Track statt 15 MB |
| Fingerabdruck aus Dateigröße plus erstem und letztem Megabyte | 1 000 Dateien in Sekunden importierbar |

## Fragen an dich

Stehen in der Spezifikation, Kapitel 12.2 (F1–F9). Für die Arbeit jetzt am wichtigsten:

1. **GitHub-Repository:** Bitte ein leeres privates Repository anlegen und mir den Namen nennen. Ohne das liegt alles nur in dieser Sitzung.
2. **Eigene Tracks** mit bekannten BPM- und Tonart-Werten (F9), damit die Analyse an echter Musik gemessen werden kann.
3. **System und Audio-Interface** (F3) für die Abnahme am Gerät.

## Befehle

```bash
cargo test --workspace
```

```bash
cargo run --release -p dw-cli -- analyze <Datei>...
```

```bash
cargo run --release -p dw-cli -- mix <A> <B> -o mix.wav --bars 32
```

```bash
cd ui && pnpm install && pnpm dev
```
