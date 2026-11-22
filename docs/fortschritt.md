# Fortschritt

Stand: 21.11.2026. Diese Datei zuerst lesen, dann `docs/spezifikation.md`.

## Wo wir stehen

Stufe 0 (Fundament) ist fertig, Stufe 1 (Kern-Mixing) bis auf die Abnahme am Gerät und die Audio-Einstellungen. Die Desktop-Anwendung läuft unter Linux mit echtem Kern: Musik importieren, Analyse im Hintergrund, zwei Tracks laden, synchron abspielen. Unter macOS ist sie seit dem 19.11.2026 aus den Quellen gebaut und gestartet; dort kommt Ton aus den eingebauten Lautsprechern. Die Abnahme am Gerät ist begonnen (Schritte 1, 2 und 4), die Schritte 5 bis 9 stehen aus. Unter Windows ist sie seit dem 21.11.2026 ebenfalls aus den Quellen gebaut und gestartet; dort ist die Abnahme am 21.11.2026 durchgegangen: Schritte 1, 2, 4, 5, 6, 8 und 9 bestanden, Schritt 3 mit nur einem Referenz-Track (falsch erkannt), Schritt 7 nicht prüfbar.

Die Oberfläche wurde von zwei unabhängigen Prüfern (Art Director, UX) begutachtet; ihre Befunde sind eingearbeitet, die zurückgestellten stehen unten.

## Nächster Schritt

1. **Reste der Abnahme** (braucht dich): Schritt 3 ist mit Referenzwerten aus dem Netz für 21 Tracks ausgewertet (Abschnitt „Referenzwerte aus dem Netz"), offen bleibt dort dein Urteil zu den vier strittigen Tracks; Schritt 7 an einem Interface mit vier Ausgängen; Schritt 8 mit festgehaltener Dauer und Last; unter macOS die Schritte 5, 6, 8 und 9. Stand und Befunde in den Abschnitten „Abnahme vom 21.11.2026 (Windows)" und „Abnahme vom 19.11.2026 (macOS)".
2. **Einstellungen:** Das Audiogerät ist wählbar (seit 21.11.2026, als Auswahlliste; unter Windows im Fenster bestätigt, unter macOS und Linux nicht). Es fehlen Kanäle, Abtastrate, Puffer, Testton (ENG-01–03, ENG-05); Mixer-Einstellungen (Crossfader-Kurve und -Zuweisung, Split-Cue, Fader-Kurve, Auto-Gain); Clock-Modus (automatisch, fest, frei mit BPM). Im Kern ist das meiste vorhanden, es fehlen die Bedienelemente und für Split-Cue ein Befehl.
3. **Import per Dialog** („Ordner hinzufügen"), zusätzlich zum Ablegen auf dem Fenster.
4. **Schutz und Cue in den Kern verlegen:** Cue-Punkt, CDJ-Cue und der Live-Schutz leben bisher in der Oberfläche; Automation und ein späteres MIDI-Mapping würden sie umgehen.
5. **Installer auf Windows und macOS starten** (braucht dich oder einen Rechner mit diesen Systemen). Sie liegen als Artefakte am Pipeline-Lauf auf GitHub.
6. Danach Stufe 2: Key Lock, Loops, Hotcues, vier Decks in der Oberfläche, Mitschnitt.

## Erledigte Anforderungen

| Bereich | Erledigt | Nachweis |
|---|---|---|
| Bibliothek | BIB-01, 02, 03, 05, 06, 08, 09, 18 | `cargo test -p dw-library`, `-p dw-decode` |
| Analyse | ANA-02, 03, 04, 06, 07, 08, 16 (auf synthetischem Referenz-Set) | `cargo run --release -p dw-analysis --example kennzahlen` |
| Decks | DECK-01, 05 (ohne CDJ-Cue), 06, 07, 08, 09, 20 (im Kern) | `cargo test -p dw-engine` |
| Mixer | MIX-01, 02, 04, 05, 06, 08, 09, 10, 13 | `cargo test -p dw-dsp`, `-p dw-engine`, `-p dw-audio` |
| Engine | ENG-04, 06, 07, 09; ENG-01–03 nur übersetzt | `cargo test -p dw-engine --test realtime`, `-p dw-audio` |
| Übergänge | T1 und T10 aus 5.10; TRANS-02, TRANS-03 für T1 | `cargo test -p dw-automix` |
| Oberfläche | Designsystem, Hülle, Ansicht „Mixen"; VIS-01, 02, 03 (Beat-/Taktlinien), 04, 05, 06, 10, 11, 13 (Vorausblick); BED-03, 05, 07; Live-Schutz nach 9.3 | `pnpm test` (126 Tests) und `pnpm shots` in `ui/`; das Skript misst Bühnenhöhe und Zielgrößen und bricht bei Unterschreitung ab |
| Desktop-Anwendung | Anschluss der Oberfläche an den Kern, Hintergrund-Analyse (ANA-01), Wellenform-Cache, Laden aufs Deck (DECK-04), Wahl des Ausgabegeräts im laufenden Betrieb (ENG-01, nur Gerät) | `cargo test -p deckwerk-app`; Bilder der laufenden Anwendung in `docs/bilder/` |

Gemessene Werte (diese Maschine, 4 Kerne):

- Sync zweier Decks über 10 Minuten: größte Abweichung 1 Sample (0,02 ms). Ziel ≤ 1 ms.
- Offline-Render mit zwei Decks: 61-fache Echtzeit. Ziel ≥ 10-fach.
- Analyse eines 7-Minuten-Tracks: 1,2 s. Ziel ≤ 15 s.
- Suche in 20 000 Tracks: 32 ms. Ziel ≤ 100 ms.
- EQ in Mittelstellung: ±0,1 dB von 20 Hz bis 20 kHz. Filter in Mittelstellung bitgenau.
- Limiter: −0,30 dBTP bei einem Signal, dessen Spitzen zwischen den Samples liegen. Ziel ≤ −0,3 dBTP.
- Automatischer Übergang: Bass im Übergang höchstens 0,1 dB über dem Einzelpegel; die Gegenprobe mit zwei offenen Bässen liegt 5,1 dB darüber. Grenze 3 dB.
- Oberfläche: Bühne 40 % der Höhe bei 1280×720 und 1920×1080; kleinstes Bedienziel 28 px bei 1280×720.
- Linux-Paket: `Deckwerk_0.1.0_amd64.deb`, 5,8 MB, gebaut und gestartet (Import und Analyse von drei Tracks in unter 14 s). Installiert wurde es nicht.
- Referenz-Set (40 synthetische Tracks): Tempo 40/40 innerhalb ±0,05 BPM, Beat-Abweichung im Mittel 0,14 ms, Downbeat 40/40, Tonart 40/40.

## Nicht geprüft

- **Analyse an echter Musik (teilweise).** Seit dem 22.11.2026 gibt es einen Vergleich an 21 echten Tracks gegen Werte aus dem Netz (Abschnitt „Referenzwerte aus dem Netz"): Tempo 18/21, Tonart 15/18. Die Referenz ist nicht von Hand geprüft; Raster, Downbeat und Lautheit sind an echter Musik weiter ungeprüft.
- **Analyse an echter Musik.** Generator und Analyse stammen aus derselben Hand; die 100 % auf dem synthetischen Set sagen wenig über echte Tracks. Braucht die Referenzsammlung Teil B/C (Spezifikation 10.1).
- **Ausgabe am Gerät.** Unter macOS (CoreAudio, eingebaute Lautsprecher, 2 Kanäle, 48 kHz) kommt Ton, sofort und passend zur Wellenform. Latenz, Kopfhörer-Weg (braucht vier Kanäle) und Stabilität über längere Zeit sind ungemessen; ein externes Interface, ASIO und ALSA/JACK sind weiter nur übersetzt. Unter Windows (WASAPI) kam am 21.11.2026 Ton aus dem Standardgerät des Rechners; ein 7.1-Headset ließ sich erst mit cpal 0.18 öffnen (Fehler 4 im Windows-Abschnitt), in der Anwendung ist es seither über die Einstellungen wählbar und wurde so benutzt. cpal 0.18 ist unter macOS und Linux weder gebaut noch gestartet; das zeigt erst die Pipeline.
- **Windows und macOS.** Die Pipeline baut und testet den Kern dort seit dem ersten Lauf erfolgreich und erzeugt Installer für alle drei Systeme. Unter macOS ist die Anwendung aus den Quellen gebaut und gestartet (19.11.2026); den Installer hat dort noch niemand gestartet. Unter Windows ist sie ebenfalls aus den Quellen gebaut und gestartet (21.11.2026), mit der Toolchain `stable-msvc`; den Windows-Installer hat noch niemand gestartet.
- **Klang.** EQ, Filter, Limiter und Übergang sind gemessen, nicht gehört.
- **Bedienung in der echten Anwendung** nur per Tastatur und mit einzelnen Klicks: Zahleneingabe ist dort nicht nachgestellt (im Browser schon). Ziehen von Reglern, Ziehen aus der Liste aufs Deck und das Ablegen eines Ordners auf dem Fenster sind unter Windows am 21.11.2026 von Hand geprüft, unter macOS und Linux nicht.
- **Grafik unter Linux.** In der Bauumgebung kommt WebGL 2 in WebKitGTK nicht direkt auf den Schirm; die Wellenformen werden dort über eine 2D-Fläche kopiert. Wie sich das mit echter Grafikkarte verhält und was es an Bildrate kostet, ist ungemessen.
- **Rot-Grün-Simulation** der Farben (VIS-12).

## Offene Punkte im gebauten Teil

- Wiedergabe mit verändertem Tempo: Interpolation mit 32 Stützstellen, eben bis etwa 17 kHz; Sperrdämpfung 80 dB ist nicht nachgewiesen (5.13).
- Sprünge im laufenden Deck blenden kurz aus und wieder ein (rund 4 ms Lücke) statt überzublenden. Für Loops und Hotcues (Stufe 2) wird eine echte Überblendung gebraucht.
- Tempo-Analyse liefert immer einen Anker; Erkennung von Temposchwankungen fehlt (ANA-05, Stufe 3).
- Tracks außerhalb 110–170 BPM werden mit der Techno-Voreinstellung falsch oktaviert (90 → 180). Die Voreinstellung „frei" existiert, ist aber nirgends wählbar.
- Inhalts-Fingerabdruck der Bibliothek ändert sich, wenn Tags am Dateianfang umgeschrieben werden; die Analyse hängt dann an der alten Kennung.
- AAC-Vorlauf wird aus der Schnittliste der MP4-Datei gelesen; Dateien ohne Schnittliste (nur iTunSMPB-Angabe) sind nicht abgedeckt.
- Stille Tracks: Lautheit −∞ lässt sich nicht als JSON zurücklesen.
- Tempo-Angleichung im Übergang ändert ohne Key Lock die Tonhöhe (Key Lock kommt in Stufe 2).
- Cue-Punkt, CDJ-Cue und Live-Schutz liegen in der Oberfläche statt im Kern. Fader-Kurven zur Auswahl und 4-Band-EQ fehlen.
- `DeckState` trägt keine Track-Kennung; die Oberfläche merkt sich selbst, was geladen ist.
- Geräte, die kein 32-bit-Gleitkomma annehmen, werden nicht geöffnet (keine Umwandlung in 16 bit).
- `dragDropEnabled` in `app/tauri.conf.json` ist für den Import durch Ablegen nötig und schaltet das Drag-and-drop des Browsers innerhalb der Oberfläche ab (unter macOS beobachtet, für Windows von Tauri dokumentiert). Das Ziehen Liste → Deck läuft deshalb über Zeiger-Ereignisse; unter Windows am 21.11.2026 bestätigt, im macOS-Fenster noch nicht.
- Der Import-Bericht wird in der Anwendung verworfen (`let _ = state.import(…)`): Dateien, die sich nicht lesen lassen, fehlen ohne Meldung. Es braucht eine Anzeige in der Oberfläche.
- Die Grundzuweisung am Crossfader setzt die Anwendung, nicht die Engine; Offline-Render und Kommandozeile laufen weiter mit „durch". Wählbar ist die Zuweisung noch nirgends (nächster Schritt 2).
- Tempo-Werte außerhalb 110–170 BPM kommen trotz Techno-Voreinstellung vor (95 bis 110 und 200 BPM an echter Musik); der Punkt zur Oktavierung oben beschreibt das Verhalten also nicht vollständig.
- Zwei Live-Sets von 90 Minuten erzeugen Wellenform-Dateien von je rund 44 MB und bekommen ein einziges Tempo.
- `Cargo.toml`: `rust-version` unter `workspace.package` wird von keinem Crate übernommen, die Mindestversion 1.85 ist damit nirgends wirksam; `dw-cli` steht ungenutzt in den Workspace-Abhängigkeiten. Cargo 1.100 warnt bei jedem Lauf.
- CLAUDE.md nennt pnpm 10; unter macOS installiert `brew install pnpm` inzwischen Version 12.
- Beim Ziehen eines Ordners über das Fenster gibt es keine Rückmeldung (kein Rahmen, kein Hinweis); die Anwendung reagiert nur auf das Loslassen (`app/src/lib.rs`). Der Import selbst geht.
- Geräteauswahl: Die Engine behält die Abtastrate, mit der sie gestartet ist; ein Gerät, das diese Rate nicht annimmt, lässt sich im laufenden Betrieb nicht wählen (unter Windows wandelt das System um). Die Liste der Geräte wird beim Öffnen der Einstellungen gelesen; ein Gerät, das danach angesteckt wird, erscheint erst beim nächsten Öffnen. Fällt das gewählte Gerät im Betrieb weg, wechselt die Anwendung nicht von selbst.
- `pnpm shots` (Maßprüfung der Bedienziele) lief unter Windows nicht: Playwright steht nicht in den Abhängigkeiten der Oberfläche. Die Geräteauswahl in den Einstellungen und die Tasten des Vorausblicks sind damit nicht nachgemessen.
- Vorausblick (VIS-13): `PgDn`/`PgUp` gehören jetzt der Bühne und blättern nicht mehr in der Liste der Bibliothek. Der Vorausblick rechnet mit dem jetzigen Tempo; Tempowechsel im Track (mehrere Anker) bildet er nicht ab. Ein stehendes Deck bleibt an seiner Stelle; Play startet es mit Vorlauf (`play_in`). Am 22.11.2026 unter Windows im Fenster ausprobiert, Rückmeldung „das funktioniert" (ohne Einzelheiten; unter macOS und Linux nicht). Im Fenster nicht geprüft: Anzeige der negativen Zeit und der Wellenform im Vorlauf vor dem Trackanfang, und wie sich der Start mit Vorlauf anfühlt. Ein stehendes Deck im Sync wird mit seinem eigenen Tempo gezeichnet, läuft aber mit dem der Clock; die Stelle in der Bildmitte trifft trotzdem zum richtigen Zeitpunkt ein, nur die Breite der Wellenform ändert sich beim Start. Abschnitte und Cue-Punkte sind in der Detail-Wellenform noch nicht eingezeichnet, der Blick nach vorn zeigt also nur Wellenform, Beat-, Takt- und Phrasenlinien.
- Ab vier Kanälen liegt der Kopfhörer-Weg fest auf den Kanälen 3/4. Bei einem Surround-Gerät (5.1, 7.1) sind das Center und Subwoofer, kein zweiter Stereo-Ausgang; die Zuweisung muss wählbar werden (nächster Schritt 2).
- Bei 1280×720 zeigt die Bibliothek 5 ganze Zeilen (10 mit aufgezogener Liste); die Prüfer wollten 7. Bühne und Zielgröße hatten Vorrang.

## Zurückgestellte Prüfbefunde

| Befund | Kommt mit |
|---|---|
| Einstellungen für Mixer und Clock (UX 9) | nächster Schritt 2 |
| Verträglichkeits-Spalte in der Bibliothek (UX 10) | Stufe 3 |
| Cover und „nächster Abschnitt" am Deck (UX 13) | Stufe 3 |
| „Rückgängig" nach dem Laden in ein stehendes Deck (UX 4) | Stufe 2 |
| Ziehbare Trennlinie für die Bibliothek (UX 10); bisher Taste „Liste" | Stufe 2 |
| Zahleneingabe für Kopfhörer-Mix und -Pegel (UX 12) | Stufe 2 |

## Entscheidungen

| Entscheidung | Grund |
|---|---|
| Kern in Rust, Oberfläche mit Tauri 2, React, WebGL 2 | Spezifikation 7.1 |
| Symphonia 0.5 statt 0.6, rusqlite 0.32, lofty 0.22 | bekannte, stabile Schnittstellen; Anheben als eigene Aufgabe |
| cpal 0.18 statt 0.15 (21.11.2026) | cpal 0.15 öffnet unter WASAPI keine Geräte mit strenger Formatprüfung (Fehler 4 der Windows-Abnahme); 0.18 lässt Windows das Format umwandeln. Folge: Mindestversion von Rust 1.85 statt 1.80 |
| Opus über libopus (mitgebaut) | Symphonia hat keinen eigenen Opus-Decoder |
| Abtastratenwandlung selbst geschrieben statt rubato | wenig Code, vollständig testbar, keine fremde Schnittstelle |
| Beat-Sync als feste Kopplung an die Clock, kein Regelkreis | Abweichung bleibt unter einem Sample; Anschieben verschiebt den Versatz |
| Start ohne Einblenden | der Anschlag am Cue-Punkt bleibt erhalten |
| EQ-Kill mit 24 dB/Oktave | übliche Isolator-Flanke; Abnahmekriterium in MIX-02 angepasst |
| Kanalfader quadratisch | linear in dB wäre bei halbem Weg schon −30 dB; MIX-05 angepasst |
| Bass-Regel wird gegen den Einzelpegel der Tracks gemessen | der Pegel vor dem Übergang ist kein fairer Maßstab, wenn A dort keinen Bass hat; TRANS-03 angepasst |
| Wellenform im Cache mit einem Byte je Wert | 3–4 MB je Track statt 15 MB |
| Kanalfader auf Alt + D/F bzw. Alt + L/; | Strg + F ist die Suche; 8.7 angepasst |
| Crossfader „ganz" auf Alt + Pfeil, Laden auf Shift + Pfeil | 8.7 belegte Shift + Pfeil doppelt |
| Master-Automatik wählt das hörbare Deck | DECK-09; Tempo und Phase bleiben bei der Übergabe erhalten |
| Tonart-Marke: Code in Schriftfarbe mit Farbbalken | gefüllte Marken waren von Deck-Schildern und aktiven Tasten nicht zu unterscheiden |
| Fingerabdruck aus Dateigröße plus erstem und letztem Megabyte | 1 000 Dateien in Sekunden importierbar |
| Vorausblick (VIS-13) verschiebt alle laufenden Decks gemeinsam, in Beats bei Clock-Tempo; ein stehendes Deck bleibt stehen, solange ein anderes läuft | Laufende Decks gemeinsam, weil nur so sichtbar bleibt, was zusammen erklingt. Das stehende Deck bleibt (Rückmeldung vom 22.11.2026), damit sich darin die Stelle suchen lässt, die zu der späteren Stelle des laufenden Tracks passt; die erste Fassung schob es mit. Läuft keines, wandern alle, sonst täte der Griff nichts. Er ist Zustand der Oberfläche wie der Zoom, kein `Command` |
| Play am stehenden Deck im Vorausblick startet mit Vorlauf: eigener Befehl `PlayIn { deck, beats }` im Kern | Gewünscht am 22.11.2026: Die im Vorausblick eingestellte Stelle soll eintreffen. Der Kern rechnet zurück, nicht die Oberfläche, weil nur er das Tempo kennt, mit dem das Deck nach dem Start läuft (Sync), und weil Sprung und Start in einem Schritt geschehen müssen. Positionen vor dem Trackanfang konnte die Engine schon (stumm). Cue-Vorhören bleibt ohne Vorlauf |
| Gerätewechsel ist ein Befehl der Hülle (`audio_select`), kein `Command` der Engine | er betrifft die Ausgabe, nicht den Zustand der Engine; Automation und Offline-Render brauchen ihn nicht |
| Beim Gerätewechsel wandert die Engine von der alten Ausgabe zur neuen, statt neu zu entstehen | geladene Decks und laufende Wiedergabe bleiben erhalten; dafür bleibt die Abtastrate fest |

## Fragen an dich

Stehen in der Spezifikation, Kapitel 12.2 (F1–F9). Für die Arbeit jetzt am wichtigsten:

1. **Abnahme am Gerät:** die neun Schritte im Abschnitt „Abnahme am Gerät".
2. **Eigene Tracks** mit bekannten BPM- und Tonart-Werten (F9), damit die Analyse an echter Musik gemessen werden kann.
3. **System und Audio-Interface** (F3) für die Abnahme am Gerät.

## Abnahme am Gerät (Stufe 1)

Nur du kannst das prüfen. Bauen und Starten: siehe README.

Ergebnisse der ersten Abnahme stehen im Abschnitt „Abnahme vom 19.11.2026 (macOS)" darunter.

| Nr. | Schritt | Erwartet |
|---|---|---|
| 1 | Anwendung starten | Fenster erscheint, kein Hinweis „Kein Audio" |
| 2 | Musikordner auf das Fenster ziehen | Tracks erscheinen sofort, BPM und Tonart folgen innerhalb von Sekunden |
| 3 | 20 eigene Tracks: BPM und Tonart mit deinen bekannten Werten vergleichen | Abweichungen bitte notieren (Track, erwartet, angezeigt) |
| 4 | Track auf Deck A laden, `Q` | Ton ohne Knacken; Wellenform und Ton laufen sichtbar zusammen |
| 5 | Zweiten Track auf B, `O` (Sync), `U` | beide Kicks liegen hörbar übereinander, auch nach fünf Minuten |
| 6 | EQ-Kills, Filter, Kanalfader, Crossfader bewegen | keine Knackser, Kill nimmt das Band vollständig heraus |
| 7 | Mit Kopfhörer an Ausgang 3/4 (Interface mit vier Kanälen): Kopfhörer-Cue an B | B nur im Kopfhörer, nicht auf dem Master |
| 8 | Zehn Minuten laufen lassen, Lastanzeige beobachten | keine Aussetzer, Last stabil |
| 9 | `deckwerk mix` mit zwei eigenen Tracks, Ergebnis anhören | Übergang klingt wie ein sauberer Bass-Tausch; wenn nicht: was stört? |

## Abnahme vom 19.11.2026 (macOS)

Rechner: MacBook Air (Apple Silicon), macOS 27.0.1. Ausgabe über die eingebauten Lautsprecher (2 Kanäle, 48 kHz), kein externes Interface. Musik: 144 eigene MP3-Dateien (2,7 GB), darunter zwei Live-Sets von rund 90 Minuten.

Werkzeuge: Rust 1.100.0, Node 22.22.2, pnpm 12.21.0 (Homebrew liefert 12 statt der geforderten 10; das Lockfile lief mit `--frozen-lockfile` unverändert durch), CMake 4.5.0, Apple clang 21. Oberfläche und Anwendung bauten ohne Eingriff (Release in 1 min 08 s); macOS-spezifisch war nichts zu beheben. `cargo test --workspace`: 93 bestanden, 1 übersprungen, vor den Korrekturen.

| Nr. | Ergebnis | Beobachtet |
|---|---|---|
| 1 | bestanden | Fenster erscheint vollständig, kein Hinweis „Kein Audio", Schriften und Wellenformen sauber. macOS fragt einmal nach Ordnerzugriff, unter dem Namen „Visual Studio Code", weil die Anwendung aus dessen Terminal gestartet wurde. |
| 2 | bestanden nach Korrektur | 142 von 144 Tracks nach rund 1 s in der Liste; Analyse aller 142 in 2 min 26 s, erste Werte nach Sekunden, Oberfläche blieb bedienbar. Zwei Dateien fehlten ohne Meldung (siehe Fehler 2). Nach der Korrektur 144 Tracks, alle analysiert, keine Doppelten. |
| 3 | nicht geprüft | Keine Referenzwerte zur Hand; die Dateien tragen weder BPM noch Tonart in den Tags. Auffällig, aber unbeurteilt: zwei Tracks mit 200,0 BPM, fünf zwischen 95 und 110 BPM, vier mit einer Tempo-Sicherheit unter 30 %. |
| 4 | teilweise bestanden | Ton kommt sofort und läuft passend zur Wellenform. Die Kick ist in der Wellenform schwer zu erkennen. Ziehen aus der Liste aufs Deck ging nicht (Fehler 3); Laden über die Tasten A und B ging. Knacken bei Start und Stopp, Lage der Beat-Linien und Springen in der Übersicht sind nicht ausdrücklich beurteilt. |
| 5–9 | offen | Noch nicht durchgegangen. Schritt 7 lässt sich an diesem Rechner nicht prüfen (nur zwei Ausgangskanäle). |

Gefundene Fehler:

1. **Crossfader ohne Wirkung** (MIX-06). Die Engine startet alle Kanäle mit der Zuweisung „durch", und die Anwendung wies nie eine Seite zu; der Simulator im Browser belegt A/C links und B/D rechts vor, deshalb fiel es dort nicht auf. Behoben: Die Anwendung weist beim Start A/C links und B/D rechts zu (`app/src/audio.rs`). Test: `crossfader_fades_deck_a_out` in `app/tests/state.rs`. Am Gerät bestätigt am 21.11.2026 unter Windows (Schritt 6).
2. **Dateien mit fehlerhaftem Tag fehlen in der Bibliothek** (BIB-01). Ein Datum im Jahres-Feld („08-05-2026" in `TYER`) ließ lofty die ganze Datei ablehnen. Behoben: Tags werden nachsichtig gelesen, zur Not ganz ohne Tags (`crates/dw-library/src/import.rs`). Test: `malformed_tag_does_not_block_import` in `crates/dw-library/tests/library.rs`. Am Gerät bestätigt.
3. **Ziehen aus der Liste aufs Deck ohne Wirkung** (DECK-04), unter macOS. Mit `dragDropEnabled` fängt die Hülle jedes Ziehen über dem Fenster ab (wry ruft das Verhalten des Webviews dann nicht mehr auf), `dragover` und `drop` erreichen die Seite nicht. Für Windows war das als Verdacht vermerkt, es trifft macOS ebenso. Behoben: Das Ziehen läuft über Zeiger-Ereignisse statt über das Drag-and-drop des Browsers (`ui/src/app/dnd.ts`), mit Marke am Zeiger und Abbruch per Esc. Tests: `ui/src/app/dnd.test.ts`; zusätzlich im Browser mit simuliertem Kern nachgestellt (Ziehen auf Deck B lädt, Loslassen daneben nicht, 2 px Weg bleibt ein Klick). Im macOS-Fenster noch nicht bestätigt.

Fehler 1 und 2 sind nicht macOS-spezifisch.

Befund ohne Korrektur: Die Kick hebt sich in der Detail-Wellenform zu wenig ab. Ob es an der Höhe oder an den Farben der Bänder liegt, ist offen.

Wünsche aus der Abnahme (neue Funktionen, keine Befunde):

- **Vorausblick in der Wellenform** (gewünscht am 21.11.2026, gebaut als VIS-13): Ansicht der laufenden Decks gemeinsam nach vorn schieben und zurückspringen, um den zweiten Track auf eine spätere Stelle abzustimmen; das stehende Deck bleibt stehen (nachgeschärft am 22.11.2026).
- **Was passt zum laufenden Track?** Zeichen je Zeile der Bibliothek für Tonart (passt / geht / reibt nach 5.11) und Tempo (Abstand zum laufenden Tempo), dazu Sortieren und Filtern danach. Entspricht dem zurückgestellten Befund UX 10; gewünscht vor Stufe 3.
- **Weitere Darstellungen der Wellenform** zur Auswahl, etwa drei Bänder überlagert oder gestapelt.

## Abnahme vom 21.11.2026 (Windows)

Rechner: Windows 11 Pro (10.0.26300), x64. Ausgabe über WASAPI auf dem Standardgerät des Systems, kein Interface mit vier Kanälen; Schritt 7 bleibt damit ungeprüft. Musik: 142 der 144 MP3-Dateien vom Mac (2,25 GB), ohne die beiden Live-Sets von 90 Minuten.

Werkzeuge: Rust 1.100.0 mit der Toolchain `stable-x86_64-pc-windows-msvc` (Standard auf dem Rechner ist `gnullvm`, gebaut wurde mit `cargo +stable-msvc`), MSVC 14.52 aus Visual Studio Enterprise 2026, Windows SDK 10.0.26100, CMake und Ninja aus Visual Studio (über `vcvars64.bat`), Node 24.18.0 statt 22, pnpm 10.34.9 über `corepack pnpm@10` (nicht installiert). Am Projekt war für den Bau nichts zu ändern; Release-Bau der Anwendung in 72 s. Ein erster Testlauf scheiterte an `LNK1181: dbghelp.lib`, weil der Visual Studio Installer das Windows SDK noch einspielte; kein Fehler des Projekts.

Tests: `cargo test --workspace` 95 bestanden, 1 übersprungen, vor der Korrektur; danach 98 bestanden. Oberfläche: Typprüfung ohne Fehler, 114 Tests.

| Nr. | Ergebnis | Beobachtet |
|---|---|---|
| 1 | bestanden | Fenster erscheint vollständig, Schriften sauber, kein Hinweis „Kein Audio" (am ersten Standardgerät), kein zusätzliches Konsolenfenster, keine Abfrage von Windows. |
| 2 | bestanden | Ordner aufs Fenster gezogen: wird angenommen, 142 Tracks nach rund 0,5 s in der Liste, Analyse läuft sofort an mit rund einem Track je Sekunde, Oberfläche bleibt bedienbar; im Datenordner liegen danach 142 Wellenform-Dateien. Die beiden Antix-Dateien mit dem kaputten Jahres-Feld sind dabei (Gegenprobe zu Fehler 2). Beim Ziehen über das Fenster fehlt eine Rückmeldung (offener Punkt). |
| 3 | ein Track geprüft, falsch | Erste Referenz: `09-ikon-1000_different_stories.mp3` hat 100 BPM und steht in e-Moll (9A); angezeigt werden 200,00 BPM und 12B (E-Dur, als unsicher markiert). Mit `deckwerk analyze` nachgestellt. Tempo: der offene Punkt zur Oktavierung außerhalb 110–170 BPM, jetzt mit einem belegten Fall. Tonart: Grundton richtig, Tongeschlecht falsch (Dur statt Moll). Weitere Referenzwerte fehlen. |
| 4 | bestanden, mit Befund | Ausgabe über „Odyssey G95SC (NVIDIA High Definition Audio)", Stereo, 48 kHz. Ziehen aus der Liste aufs Deck geht (Gegenprobe zu Fehler 3 der macOS-Abnahme), Klick und Doppelklick ebenso. Ton kommt sofort, kein Knacken bei Start und Stopp, Wellenform läuft mit dem Ton, Springen in der Übersicht ohne Knacken. Befund wie am Mac: Die Kick ist in der Detail-Wellenform schwer zu erkennen; am unteren Band und an den Beat-Linien ist sie zu sehen, im oberen Teil gehen die Bänder in ähnlichen Orange- und Gelbtönen ineinander über. Das spricht für die Farben, nicht für die Höhe. |
| 5 | bestanden | Zweiter Track auf B, Sync und Start: Beide Kicks liegen hörbar übereinander, beide Decks zeigen dasselbe Tempo. Wie lange gehört wurde, ist nicht festgehalten; die fünf Minuten sind damit nicht ausdrücklich belegt. |
| 6 | bestanden | Crossfader blendet A und B aus (Gegenprobe zu Fehler 1 der macOS-Abnahme, damit erstmals am Gerät bestätigt), EQ-Kills nehmen das Band heraus, EQ-Regler, Filter und Kanalfader ohne Knackser. Die Regler lassen sich mit der Maus ziehen. |
| 7 | nicht prüfbar | Kein Gerät mit vier nutzbaren Ausgangskanälen. |
| 8 | bestanden laut Rückmeldung | Beide Decks synchron laufen lassen: „passt so". Dauer und Stand der Lastanzeige sind nicht festgehalten; zu Beginn der Abnahme zeigte sie 0 bis 1 %. |
| 9 | bestanden laut Rückmeldung | `deckwerk mix` mit `05-antix-the_hoard_john_monkman_remix.mp3` und `08-antix-cavalier_emok_remix.mp3` (beide 125 BPM, 8A → 7A), 32 Takte: in 1,0 s gerendert, größter Beat-Versatz 0,0000 Beats; angehört, „passt so". Einzelheiten zum Klang des Bass-Tauschs sind nicht festgehalten. |

Gefundene Fehler (Zählung setzt die der macOS-Abnahme fort):

4. **7.1-Headset lässt sich nicht öffnen** (ENG-01, ENG-02), unter Windows. Mit „CORSAIR ST100 Headset Output" als Standardgerät meldet die Anwendung „Kein Audio: Gerät unterstützt die gewünschte Einstellung nicht: 48000 Hz, 4 Kanäle". Zwei Ursachen:
   - Die Voreinstellung begrenzte die Kanalzahl fest auf 2 bis 4; das Gerät meldet 8 Kanäle, und WASAPI nimmt im geteilten Modus neben Stereo nur die Kanalzahl des Geräts an. Behoben: Die Kanalzahl wird in fester Reihenfolge probeweise geöffnet (begrenzte Zahl, Zahl des Geräts, Stereo; `channel_candidates` in `crates/dw-audio/src/device.rs`); ein Gerät mit nur einer Möglichkeit wird wie bisher direkt geöffnet. Tests: drei Tests zu `channel_candidates` ebenda. An einem Gerät, dem das hilft, nicht bestätigt.
   - Auch danach öffnete das Gerät nicht. Direkt an WASAPI gemessen: Es nimmt 8 Kanäle mit Lautsprecher-Zuordnung (Maske 0x63F) und Stereo mit Zuordnung (Maske 0x3) an, lehnt beide ohne Zuordnung (Maske 0) aber ab. cpal 0.15.3 übergibt bei 32-bit-Gleitkomma immer die Maske 0 (`KSAUDIO_SPEAKER_DIRECTOUT`) und wertet die Antwort „ähnliches Format möglich" als Ablehnung; es meldet für das Gerät kein einziges unterstütztes Format. Die drei anderen Ausgänge des Rechners (Stereo) melden je vier Formate und öffnen. Behoben durch das Anheben von cpal auf 0.18.2: Dort öffnet die Ausgabe mit der Umwandlung durch Windows (`AUTOCONVERTPCM`), das Gerät meldet 112 statt 0 Formate. Gemessen über `dw_audio::device::start` mit stummem Signal: 2, 4 und 8 Kanäle bei 48 kHz öffnen. In der Anwendung am 21.11.2026 über die Einstellungen gewählt: Der Wechsel gelang und steht in `settings.json`; Rückmeldung „passt". Klang und Kopfhörer-Weg an diesem Gerät sind nicht eigens beurteilt. `cargo test --workspace` danach: 98 bestanden, 1 übersprungen.

## Referenzwerte aus dem Netz (22.11.2026)

Für Schritt 3 der Abnahme wurden 20 Tracks aus `mp3/` gewählt und Tempo und Tonart im Netz nachgeschlagen (vier Recherchen parallel, ohne Kenntnis der Werte von Deckwerk). Die Werte von Deckwerk stammen aus der Bibliothek dieses Rechners (`library.db`, Stand 22.11.2026).

**Wie belastbar die Referenz ist:** Beatport ließ sich nur für die ersten fünf Tracks als Seite lesen, sonst kamen seine Werte aus Suchtreffern (HTTP 403). Beatport nennt das Tempo nur als ganze Zahl; seine Tonarten sind oft selbst maschinell bestimmt. Mixgraph stimmt überall mit Beatport überein und ist vermutlich keine unabhängige Quelle. SongBPM und Shazam sind eigene Messungen (Shazam liegt mehrfach 1 BPM tiefer). Mit ±0,05 BPM aus ANA-02 lässt sich an ganzzahligen Referenzen nur prüfen, ob Deckwerk dieselbe ganze Zahl trifft. Gehört und von Hand nachgezählt ist nichts davon.

| Datei | Tempo Referenz | Tonart Referenz | Tempo Deckwerk | Tonart Deckwerk | Tempo stimmt | Tonart stimmt | Quellen |
|---|---|---|---|---|---|---|---|
| `01-boris_brejcha-engine` | 125 | As-Dur | 125,00 | As-Dur | ja | ja | Beatport (Seite gelesen), Mixgraph |
| `02-boris_brejcha-lily_at_the_valley_(extended_mix)` | 125 | F-Dur | 125,00 | F-Dur | ja | ja | Beatport (Seite gelesen; dort „Original Mix", 7:50 wie die Datei) |
| `01-boris_brejcha_and_mc_flipside-dirty_beat` | 125 | D-Dur | 125,00 | D-Dur | ja | ja | Beatport (Seite gelesen) |
| `01-boris_brejcha_ft._chemutai_sage-alicante` | 125 | F-Dur | 125,00 | F-Dur | ja | ja | Beatport (Seite gelesen), Mixgraph |
| `02-anyma-sacrifice` | 128 | a-Moll | 128,00 | a-Moll | ja | ja | Beatport (Seite gelesen, Extended Mix), Mixgraph; Shazam 127 |
| `05-anyma-the_light` | 127 | a-Moll | 127,00 | a-Moll | ja | ja | Beatport, SongBPM, Tunebat |
| `09-anyma-pictures_of_you` | 128 | e-Moll | 128,00 | e-Moll | ja | ja | Beatport, SongBPM, Songdata, Musicstax |
| `07-anyma-simulation` | 126 | e-Moll | 126,00 | e-Moll | ja | ja | Beatport, Traxsource, SongBPM, Tunebat |
| `20-anyma-welcome_to_the_opera_(kobosil` | 152 | strittig: F-Dur / f-Moll / Fis-Dur | 152,04 | F-Dur | nein (+0,04) | offen | Beatport führt 76 (halbes Tempo) und F-Dur; SongBPM 152, Fis-Dur; Fandom f-Moll |
| `18-anyma-syren_(amelie_lens_remix)` | 142 | c-Moll | 142,01 | c-Moll | ja | ja | Beatport, Mixgraph; SongBPM nennt As-Dur; Tempo-Sicherheit in Deckwerk 0 % |
| `21-anyma-explore_your_future_(daniel_av` | 105 | strittig: h-Moll / fis-Moll | 105,00 | e-Moll | ja | nein | Beatport h-Moll, SongBPM fis-Moll; Deckwerk trifft keine von beiden |
| `01-anyma-eternity_(massano_remix)` | 127 | strittig: C-Dur / c-Moll | 127,00 | c-Moll | ja | offen | Beatport und Mixgraph C-Dur; SongBPM und Tunebat c-Moll |
| `01-neelix_and_sighter-piano_man` | 140 | as-Moll | 140,00 | as-Moll | ja | ja | Beatport führt 70 (halbes Tempo), as-Moll; Shazam 139 |
| `01-neelix_and_blazy_and_durs-miracle` | 143 | Ges-Dur (= Fis-Dur) | 143,00 | Fis-Dur | ja | ja | Beatport, Setlist.ID |
| `01-liquid_soul-i_believe_in_dragons` | 138 | f-Moll | 138,00 | F-Dur | ja | nein (Dur statt Moll) | nur Volumo (Seite gelesen); Beatport nicht eindeutig zuzuordnen |
| `01-victor_ruiz-the_ritual_phaxe_and_emok_remix` | 140 | Ges-Dur (= Fis-Dur) | 140,00 | Fis-Dur | ja | ja | Beatport, Mixgraph |
| `01-pleasurekraft_and_zimmz-sympoiesis` | 134 | g-Moll | 134,00 | g-Moll | ja | ja | Beatport, Mixgraph, weitere |
| `01-ghost_rider_and_phaxe-falling_vegas_remix` | 140 | D-Dur | 140,00 | D-Dur | ja | ja | Beatport, Mixgraph; Shazam 139 |
| `01-klopfgeister-beneath_the_twilight` | 138 | G-Dur | 138,02 | g-Moll | ja | nein (Moll statt Dur) | Beatport (Liste), Mixgraph; Shazam 137; Tonart die schwächste Angabe dieser Gruppe |
| `01-ikon-close_to_the_sun` | 130 (fraglich) | H-Dur | 137,00 | H-Dur | nein (137 statt 130) | ja | Beatport (Label-Liste), Mixgraph; kein dritter Beleg, Zuordnung der 130 zum Track nicht gesichert |
| `09-ikon-1000_different_stories` | 100 | e-Moll | 200,00 | E-Dur | nein (doppelt) | nein (Dur statt Moll) | Angabe vom 21.11.2026 (siehe Abnahme Windows) |

**Ergebnis (21 Tracks):**

- **Tempo:** 18 von 21 innerhalb ±0,05 BPM der Referenz (86 %; Ziel ANA-02: ≥ 97 %). Ein Doppeltempo-Fehler (1 von 21 = 4,8 %; Ziel ≤ 2 %). Zwei weitere Abweichungen: 152,04 statt 152 (knapp über der Grenze, Tempo-Sicherheit 69 %) und 137 statt 130 bei `close_to_the_sun`, wo die Referenz selbst fraglich ist. Zwei Tracks, die Beatport mit halbem Tempo führt (70 und 76), erkennt Deckwerk im vollen Tempo.
- **Tonart:** Von 18 Tracks mit einer brauchbaren Referenz stimmen 15 exakt (83 %; Ziel ANA-06: ≥ 70 %). Alle drei Fehler haben den richtigen Grundton und das falsche Tongeschlecht (Dur/Moll). Bei drei Tracks widersprechen sich die Quellen; bei einem davon (`explore_your_future`) trifft Deckwerk keine der genannten Tonarten und liegt eine Quinte daneben.
- **Was daraus folgt:** Das Tempo ist im üblichen Bereich zuverlässig; die Schwäche liegt außerhalb davon (100 BPM als 200). Bei der Tonart ist die Verwechslung von Dur und Moll das Muster, nicht der Grundton. Beides ist noch nicht behoben.
- **Nicht gewertet:** Tracks ohne Referenz mit auffälligen Werten (200 BPM, 95–110 BPM, 157,51 BPM, Tempo-Sicherheit nahe 0) – vor allem Noosfære und die kurzen Klipsun-Stücke, für die im Netz keine Werte zu erwarten sind.

## Bauen unter Windows

So lief es am 21.11.2026 (Einzelheiten im Abschnitt „Abnahme vom 21.11.2026 (Windows)"): Visual Studio mit den MSVC-Buildtools und dem Windows SDK, Rust mit der Toolchain `stable-msvc`, Node, pnpm über corepack. CMake und Ninja kommen aus Visual Studio, wenn die Befehle in dessen Umgebung laufen (Eingabeaufforderung):

```bat
"C:\Program Files\Microsoft Visual Studio\18\Enterprise\VC\Auxiliary\Build\vcvars64.bat"
corepack pnpm@10 --dir ui install --frozen-lockfile
corepack pnpm@10 --dir ui build
cargo +stable-msvc test --workspace
cargo +stable-msvc run --release -p deckwerk-app
```

Die Musik (`mp3/`, von Git ausgenommen) liegt je Rechner getrennt; Bibliothek, Analyse und Wellenform-Cache ebenso (unter Windows in `%APPDATA%\app.deckwerk.desktop`).

**Regeln der Zusammenarbeit bei der Abnahme:** je Schritt sagen, was zu tun und worauf zu achten ist, und auf die Rückmeldung warten; nur eindeutig eingrenzbare Fehler beheben, jeweils mit Test, alles andere als offenen Punkt aufschreiben; unter „Nicht geprüft" nur streichen, was wirklich geprüft wurde; erst bei grünen Tests committen und vor dem Push fragen.

## Befehle

Linux-Paket bauen (aus `app/`; für andere Pakettypen `--bundles` weglassen):

```bash
node ../ui/node_modules/@tauri-apps/cli/tauri.js build --bundles deb
```

Anwendung mit Testdaten auf einem virtuellen Bildschirm starten und ein Bild aufnehmen (so entstand `docs/bilder/anwendung-linux.png`):

```bash
xvfb-run -a -s "-screen 0 1600x900x24" bash -c 'DECKWERK_DATA=/tmp/dw-daten DECKWERK_MUSIC=/tmp/dw-musik WEBKIT_DISABLE_DMABUF_RENDERER=1 target/debug/deckwerk-app & APP=$!; sleep 15; import -window root bild.png; kill $APP'
```


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
