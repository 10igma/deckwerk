# Fortschritt

Stand: 18.11.2026. Diese Datei zuerst lesen, dann `docs/spezifikation.md`.

## Wo wir stehen

Stufe 0 (Fundament) ist fertig, Stufe 1 (Kern-Mixing) bis auf die Abnahme am Gerät und die Audio-Einstellungen. Die Desktop-Anwendung läuft unter Linux mit echtem Kern: Musik importieren, Analyse im Hintergrund, zwei Tracks laden, synchron abspielen. Ob dabei Ton aus einem Audiogerät kommt, ist ungeprüft – in der Bauumgebung gibt es keines.

Die Oberfläche wurde von zwei unabhängigen Prüfern (Art Director, UX) begutachtet; ihre Befunde sind eingearbeitet, die zurückgestellten stehen unten.

## Nächster Schritt

1. **Abnahme am Gerät** (braucht dich): Anwendung auf deinem Rechner bauen, Ton, Latenz und Kopfhörer-Weg prüfen. Die Abnahmeliste steht am Ende dieser Datei.
2. **Einstellungen:** Audiogerät, Kanäle, Abtastrate, Puffer, Testton (ENG-01–03, ENG-05); Mixer-Einstellungen (Crossfader-Kurve und -Zuweisung, Split-Cue, Fader-Kurve, Auto-Gain); Clock-Modus (automatisch, fest, frei mit BPM). Im Kern ist das meiste vorhanden, es fehlen die Bedienelemente und für Split-Cue ein Befehl.
3. **Import per Dialog** („Ordner hinzufügen"), zusätzlich zum Ablegen auf dem Fenster.
4. **Schutz und Cue in den Kern verlegen:** Cue-Punkt, CDJ-Cue und der Live-Schutz leben bisher in der Oberfläche; Automation und ein späteres MIDI-Mapping würden sie umgehen.
5. **Pipeline zum ersten Mal laufen lassen** (nach dem ersten Push) und Windows/macOS-Fehler beheben.
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
| Oberfläche | Designsystem, Hülle, Ansicht „Mixen"; VIS-01, 02, 03 (Beat-/Taktlinien), 04, 05, 06, 10, 11; BED-03, 05, 07; Live-Schutz nach 9.3 | `pnpm test` (110 Tests) und `pnpm shots` in `ui/`; das Skript misst Bühnenhöhe und Zielgrößen und bricht bei Unterschreitung ab |
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
- Referenz-Set (40 synthetische Tracks): Tempo 40/40 innerhalb ±0,05 BPM, Beat-Abweichung im Mittel 0,14 ms, Downbeat 40/40, Tonart 40/40.

## Nicht geprüft

- **Analyse an echter Musik.** Generator und Analyse stammen aus derselben Hand; die 100 % auf dem synthetischen Set sagen wenig über echte Tracks. Braucht die Referenzsammlung Teil B/C (Spezifikation 10.1).
- **Ausgabe am Gerät.** `dw-audio::device` ist nur übersetzt. Latenz, Kopfhörer-Weg und Stabilität am Interface sind ungemessen.
- **Windows und macOS.** Gebaut und getestet wurde nur unter Linux. Die Pipeline in `.github/workflows/build.yml` ist geschrieben, aber noch nie gelaufen (kein GitHub-Repository).
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
- `dragDropEnabled` in `app/tauri.conf.json` ist für den Import durch Ablegen nötig, schaltet unter Windows aber laut Tauri das Ziehen innerhalb der Oberfläche ab (Liste → Deck). Unter Windows prüfen; Ausweg ist der Import per Dialog.
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

1. **Push freigeben:** Das Repository `10igma/deckwerk` ist als `origin` eingetragen, hochgeladen ist noch nichts.
2. **Eigene Tracks** mit bekannten BPM- und Tonart-Werten (F9), damit die Analyse an echter Musik gemessen werden kann.
3. **System und Audio-Interface** (F3) für die Abnahme am Gerät.

## Abnahme am Gerät (Stufe 1)

Nur du kannst das prüfen. Bauen und Starten: siehe README.

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

## Befehle

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
