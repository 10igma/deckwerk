# Fortschritt

Stand: 21.11.2026. Diese Datei zuerst lesen, dann `docs/spezifikation.md`.

## Wo wir stehen

Stufe 0 (Fundament) ist fertig, Stufe 1 (Kern-Mixing) bis auf die Abnahme am Gerät und die Audio-Einstellungen. Die Desktop-Anwendung läuft unter Linux mit echtem Kern: Musik importieren, Analyse im Hintergrund, zwei Tracks laden, synchron abspielen. Unter macOS ist sie seit dem 19.11.2026 aus den Quellen gebaut und gestartet; dort kommt Ton aus den eingebauten Lautsprechern. Die Abnahme am Gerät ist begonnen (Schritte 1, 2 und 4), die Schritte 5 bis 9 stehen aus.

Die Oberfläche wurde von zwei unabhängigen Prüfern (Art Director, UX) begutachtet; ihre Befunde sind eingearbeitet, die zurückgestellten stehen unten.

## Nächster Schritt

Die Abnahme geht auf einem Windows-Rechner weiter; siehe „Übergabe: Weiter unter Windows" weiter unten.

1. **Abnahme am Gerät fortsetzen** (braucht dich): Ziehen aus der Liste aufs Deck und Crossfader bestätigen, dann Schritte 5 bis 9. Stand und Befunde im Abschnitt „Abnahme vom 19.11.2026 (macOS)".
2. **Einstellungen:** Audiogerät, Kanäle, Abtastrate, Puffer, Testton (ENG-01–03, ENG-05); Mixer-Einstellungen (Crossfader-Kurve und -Zuweisung, Split-Cue, Fader-Kurve, Auto-Gain); Clock-Modus (automatisch, fest, frei mit BPM). Im Kern ist das meiste vorhanden, es fehlen die Bedienelemente und für Split-Cue ein Befehl.
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
| Oberfläche | Designsystem, Hülle, Ansicht „Mixen"; VIS-01, 02, 03 (Beat-/Taktlinien), 04, 05, 06, 10, 11; BED-03, 05, 07; Live-Schutz nach 9.3 | `pnpm test` (114 Tests) und `pnpm shots` in `ui/`; das Skript misst Bühnenhöhe und Zielgrößen und bricht bei Unterschreitung ab |
| Desktop-Anwendung | Anschluss der Oberfläche an den Kern, Hintergrund-Analyse (ANA-01), Wellenform-Cache, Laden aufs Deck (DECK-04) | `cargo test -p deckwerk-app`; Bilder der laufenden Anwendung in `docs/bilder/` |

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

- **Analyse an echter Musik.** Generator und Analyse stammen aus derselben Hand; die 100 % auf dem synthetischen Set sagen wenig über echte Tracks. Braucht die Referenzsammlung Teil B/C (Spezifikation 10.1).
- **Ausgabe am Gerät.** Unter macOS (CoreAudio, eingebaute Lautsprecher, 2 Kanäle, 48 kHz) kommt Ton, sofort und passend zur Wellenform. Latenz, Kopfhörer-Weg (braucht vier Kanäle) und Stabilität über längere Zeit sind ungemessen; ein externes Interface, WASAPI/ASIO und ALSA/JACK sind weiter nur übersetzt.
- **Windows und macOS.** Die Pipeline baut und testet den Kern dort seit dem ersten Lauf erfolgreich und erzeugt Installer für alle drei Systeme. Unter macOS ist die Anwendung aus den Quellen gebaut und gestartet (19.11.2026); den Installer hat dort noch niemand gestartet. Unter Windows ist nichts davon geschehen.
- **Klang.** EQ, Filter, Limiter und Übergang sind gemessen, nicht gehört.
- **Bedienung in der echten Anwendung** nur per Tastatur und mit einzelnen Klicks: Ziehen von Reglern, Zahleneingabe, Ziehen aus der Liste aufs Deck und das Ablegen von Dateien auf dem Fenster sind dort nicht nachgestellt (im Browser schon).
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
- `dragDropEnabled` in `app/tauri.conf.json` ist für den Import durch Ablegen nötig und schaltet das Drag-and-drop des Browsers innerhalb der Oberfläche ab (unter macOS beobachtet, für Windows von Tauri dokumentiert). Das Ziehen Liste → Deck läuft deshalb über Zeiger-Ereignisse; unter Windows und im macOS-Fenster noch zu bestätigen.
- Der Import-Bericht wird in der Anwendung verworfen (`let _ = state.import(…)`): Dateien, die sich nicht lesen lassen, fehlen ohne Meldung. Es braucht eine Anzeige in der Oberfläche.
- Die Grundzuweisung am Crossfader setzt die Anwendung, nicht die Engine; Offline-Render und Kommandozeile laufen weiter mit „durch". Wählbar ist die Zuweisung noch nirgends (nächster Schritt 2).
- Tempo-Werte außerhalb 110–170 BPM kommen trotz Techno-Voreinstellung vor (95 bis 110 und 200 BPM an echter Musik); der Punkt zur Oktavierung oben beschreibt das Verhalten also nicht vollständig.
- Zwei Live-Sets von 90 Minuten erzeugen Wellenform-Dateien von je rund 44 MB und bekommen ein einziges Tempo.
- `Cargo.toml`: `rust-version` unter `workspace.package` wird von keinem Crate übernommen, die Mindestversion 1.80 ist damit nirgends wirksam; `dw-cli` steht ungenutzt in den Workspace-Abhängigkeiten. Cargo 1.100 warnt bei jedem Lauf.
- CLAUDE.md nennt pnpm 10; unter macOS installiert `brew install pnpm` inzwischen Version 12.
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
| Symphonia 0.5 statt 0.6, cpal 0.15, rusqlite 0.32, lofty 0.22 | bekannte, stabile Schnittstellen; Anheben als eigene Aufgabe |
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

1. **Crossfader ohne Wirkung** (MIX-06). Die Engine startet alle Kanäle mit der Zuweisung „durch", und die Anwendung wies nie eine Seite zu; der Simulator im Browser belegt A/C links und B/D rechts vor, deshalb fiel es dort nicht auf. Behoben: Die Anwendung weist beim Start A/C links und B/D rechts zu (`app/src/audio.rs`). Test: `crossfader_fades_deck_a_out` in `app/tests/state.rs`. Am Gerät noch nicht ausdrücklich bestätigt; das geschieht in Schritt 6.
2. **Dateien mit fehlerhaftem Tag fehlen in der Bibliothek** (BIB-01). Ein Datum im Jahres-Feld („08-05-2026" in `TYER`) ließ lofty die ganze Datei ablehnen. Behoben: Tags werden nachsichtig gelesen, zur Not ganz ohne Tags (`crates/dw-library/src/import.rs`). Test: `malformed_tag_does_not_block_import` in `crates/dw-library/tests/library.rs`. Am Gerät bestätigt.
3. **Ziehen aus der Liste aufs Deck ohne Wirkung** (DECK-04), unter macOS. Mit `dragDropEnabled` fängt die Hülle jedes Ziehen über dem Fenster ab (wry ruft das Verhalten des Webviews dann nicht mehr auf), `dragover` und `drop` erreichen die Seite nicht. Für Windows war das als Verdacht vermerkt, es trifft macOS ebenso. Behoben: Das Ziehen läuft über Zeiger-Ereignisse statt über das Drag-and-drop des Browsers (`ui/src/app/dnd.ts`), mit Marke am Zeiger und Abbruch per Esc. Tests: `ui/src/app/dnd.test.ts`; zusätzlich im Browser mit simuliertem Kern nachgestellt (Ziehen auf Deck B lädt, Loslassen daneben nicht, 2 px Weg bleibt ein Klick). Im macOS-Fenster noch nicht bestätigt.

Fehler 1 und 2 sind nicht macOS-spezifisch.

Befund ohne Korrektur: Die Kick hebt sich in der Detail-Wellenform zu wenig ab. Ob es an der Höhe oder an den Farben der Bänder liegt, ist offen.

Wünsche aus der Abnahme (neue Funktionen, keine Befunde):

- **Was passt zum laufenden Track?** Zeichen je Zeile der Bibliothek für Tonart (passt / geht / reibt nach 5.11) und Tempo (Abstand zum laufenden Tempo), dazu Sortieren und Filtern danach. Entspricht dem zurückgestellten Befund UX 10; gewünscht vor Stufe 3.
- **Weitere Darstellungen der Wellenform** zur Auswahl, etwa drei Bänder überlagert oder gestapelt.

## Übergabe: Weiter unter Windows

Die Abnahme wurde am 19.11.2026 unter macOS begonnen und wird auf einem Windows-Rechner fortgesetzt. Dieser Abschnitt enthält alles, was die nächste Sitzung dafür braucht; nach Abschluss der Abnahme kann er entfallen.

**Stand im Repository.** Die Arbeit liegt auf dem Branch `abnahme-macos` (ein Commit über `main`). Dort weiterarbeiten oder ihn zuerst nach `main` übernehmen.

**Musik.** Der Ordner `mp3/` (144 Dateien, 2,7 GB) ist von Git ausgenommen und liegt nur auf dem Mac. Auf den Windows-Rechner von Hand kopieren, am besten wieder nach `mp3/` im Wurzelordner. Bibliothek, Analyse und Wellenform-Cache sind je Rechner getrennt; unter Windows wird neu importiert und analysiert.

**Voraussetzungen unter Windows.** Rust (stabil, ab 1.80, MSVC-Toolchain), Visual Studio Build Tools mit „Desktopentwicklung mit C++", Node 22, pnpm, CMake, WebView2-Laufzeit (bei Windows 11 vorhanden). Unter macOS liefen pnpm 12 und CMake 4.5 ohne Probleme, obwohl CLAUDE.md pnpm 10 nennt. Erst prüfen, was fehlt, und nur nach Zustimmung installieren.

**Ablauf.**

1. `pnpm --dir ui install && pnpm --dir ui build`, dann `cargo run --release -p deckwerk-app`. Schlägt der Bau fehl: Ursache beheben und festhalten, was Windows-spezifisch war.
2. `cargo test --workspace` sowie `pnpm --dir ui typecheck && pnpm --dir ui test`. Stand vom Mac: 95 bestanden, 1 übersprungen; Oberfläche 114 Tests.
3. Die Abnahmeliste von Schritt 1 an neu durchgehen. Es ist ein anderes System mit anderer Audio-Schnittstelle (WASAPI), die Ergebnisse vom Mac gelten hier nicht. Ergebnisse in einen eigenen Abschnitt „Abnahme vom … (Windows)" eintragen, nach dem Muster des macOS-Abschnitts.

**Worauf unter Windows besonders zu achten ist.**

- **Ziehen aus der Liste aufs Deck.** Unter macOS ging es nicht (Fehler 3); die Korrektur über Zeiger-Ereignisse ist bisher nur im Browser nachgestellt, in keinem echten Fenster bestätigt. Prüfen: Marke am Zeiger, Markierung des Zieldecks, Laden beim Loslassen; Klick und Doppelklick auf die Zeile müssen weiter gehen.
- **Musikordner aufs Fenster ziehen** (Import). Hängt an `dragDropEnabled`; unter Windows nie ausprobiert.
- **Crossfader.** Korrektur (Fehler 1) ist am Gerät nicht ausdrücklich bestätigt: Track auf A, Crossfader ganz nach rechts, A muss verstummen.
- **Import.** Es müssen 144 Tracks erscheinen. Die beiden Antix-Dateien mit dem kaputten Jahres-Feld (`03-antix-human_nature_vosne_and_clutch_remix.mp3`, `10-antix-oma_rapeti_red_project_remix.mp3`) sind die Gegenprobe für Fehler 2.
- **Audiogerät.** Geräte, die kein 32-bit-Gleitkomma annehmen, werden nicht geöffnet (offener Punkt); erscheint „Kein Audio", ist das der erste Verdacht. ASIO ist nur übersetzt.
- **Schritt 7 (Kopfhörer auf 3/4)** braucht ein Interface mit vier Ausgangskanälen; am Mac war er nicht prüfbar.
- **Schritt 3 (BPM und Tonart)** braucht Referenzwerte für 20 Tracks; die Dateien tragen keine in den Tags. Ohne Referenz bleibt er ungeprüft. Auffällige Werte stehen in der Tabelle des macOS-Abschnitts.
- **Installer.** Den Windows-Installer aus der Pipeline hat noch niemand gestartet (nächster Schritt 5).

**Am Mac noch offen** (dort oder unter Windows nachholen): Schritte 5, 6, 8 und 9; in Schritt 4 das Knacken bei Start und Stopp, die Lage der Beat-Linien auf den Kicks und das Springen in der Übersicht; die Frage, ob die Kick in der Wellenform wegen der Höhe oder wegen der Farben schwer zu erkennen ist.

**Regeln der Zusammenarbeit**, wie in der ersten Sitzung vereinbart: je Schritt sagen, was zu tun und worauf zu achten ist, und auf die Rückmeldung warten; nur eindeutig eingrenzbare Fehler beheben, jeweils mit Test, alles andere als offenen Punkt aufschreiben; unter „Nicht geprüft" nur streichen, was wirklich geprüft wurde; erst bei grünen Tests committen und vor dem Push fragen.

**Einstieg für die neue Sitzung** (so eingeben):

> Lies CLAUDE.md und docs/fortschritt.md, dort vor allem „Übergabe: Weiter unter Windows". Ich setze die Abnahme am Gerät auf diesem Windows-Rechner fort. Prüfe die Voraussetzungen, baue und starte die Anwendung, lass die Tests laufen und geh dann die Abnahmeliste Schritt für Schritt mit mir durch. Mein Audio-Interface: … Meine Musik liegt in: …

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
