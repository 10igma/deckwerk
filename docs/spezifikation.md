# Deckwerk – Spezifikation

Version 0.1 · Stand 18.11.2026 · Arbeitstitel „Deckwerk" (Annahme, frei änderbar)

Stellen, an denen ich ohne deine Vorgabe entschieden habe, sind mit **Annahme** markiert. Die offenen Fragen dazu stehen gesammelt in Kapitel 12.

---

## 1. Überblick, Ziele und Abgrenzung

### 1.1 Produkt

Deckwerk ist eine Desktop-Anwendung für Windows, macOS und Linux zum Mixen von Techno. Sie arbeitet lokal und offline mit Musikdateien auf der Festplatte und hat zwei Betriebsarten auf einer gemeinsamen Audio-Engine:

- **Live-Modus:** vier virtuelle Decks, Mixer, Effekte, Loops und Cue-Punkte, bedient mit Maus und Tastatur, mit getrenntem Kopfhörer-Ausgang zum Vorhören.
- **Auto-Mix-Modus:** Aus einer Trackauswahl plant die Software Reihenfolge, Ein- und Ausstiegspunkte und Übergänge, zeigt den Plan als bearbeitbare Zeitleiste und rendert ihn als Audiodatei.

Beide Betriebsarten nutzen dieselben Analysedaten (Tempo, Beatgrid, Tonart, Struktur, Ereignisse) und dieselbe Signalverarbeitung. Ein Übergang klingt im gerenderten Mix deshalb genauso wie live gespielt.

### 1.2 Ziele

| Nr. | Ziel | Messgröße |
|---|---|---|
| Z1 | Ein Techno-Track ist ohne Handarbeit mixbereit | BPM, Beatgrid, Tonart und Cue-Vorschläge liegen nach dem Import vor; Genauigkeit laut 5.1–5.8 |
| Z2 | Live-Mixen fühlt sich direkt an | Ausgabelatenz ≤ 10 ms, Bildrate 60 fps, keine Aussetzer in 4 h Dauerbetrieb |
| Z3 | Automatische Übergänge klingen wie von einem DJ gespielt | jeder Übergang liegt auf einer Phrasengrenze, nie zwei volle Bässe gleichzeitig, Tonart-Regeln eingehalten |
| Z4 | Ein fertiger Mix entsteht aus einer Playlist in Minuten | 60-Minuten-Set geplant in ≤ 10 s, gerendert in ≤ 6 min |
| Z5 | Eigenständiges Erscheinungsbild | eigenes Designsystem, kein an bestehende DJ-Software angelehntes Layout |

### 1.3 Abgrenzung der ersten Version

Nicht enthalten:

- MIDI- und HID-Controller. Die Architektur führt alle Bedienhandlungen über eine zentrale Befehlsschicht (7.4), an die später ein MIDI-Mapping angeschlossen werden kann.
- Timecode-Vinyl (DVS).
- Streaming-Dienste und kopiergeschützte Dateien.
- Video, Licht-Steuerung, Netzwerk-Synchronisation zwischen Rechnern (Ableton Link ist „Kann").
- Mikrofon- und Line-Eingänge (Annahme; „Kann" in einer späteren Stufe).
- Mobile Plattformen.

### 1.4 Referenzrechner

Alle Leistungswerte in diesem Dokument beziehen sich auf: 4 CPU-Kerne (Baujahr 2018 oder neuer), 8 GB RAM, SSD, integrierte Grafik mit WebGL 2 (**Annahme**).

---

## 2. Glossar

| Begriff | Bedeutung |
|---|---|
| Beat | Ein Schlag (Viertelnote). Im Techno fällt auf jeden Beat eine Kick. |
| Takt (Bar) | 4 Beats. |
| Phrase | Musikalischer Abschnitt aus 8, 16 oder 32 Takten. Veränderungen im Track passieren fast immer an Phrasengrenzen. |
| Downbeat, „die Eins" | Erster Beat eines Takts. |
| BPM | Beats pro Minute, das Tempo. |
| Beatgrid | Raster aller Beat-Positionen eines Tracks, Grundlage für Sync, Loops und Quantisierung. |
| Camelot-Code | Schreibweise für Tonarten als Zahl 1–12 plus A (Moll) oder B (Dur). Benachbarte Zahlen passen harmonisch zusammen. |
| Open Key | Gleiche Idee wie Camelot mit anderer Nummerierung: Zahl 1–12 plus m (Moll) oder d (Dur). |
| Cue-Punkt | Gespeicherte Position im Track. |
| Hotcue | Cue-Punkt auf einer Taste, der sofort angesprungen wird. |
| Loop | Abschnitt, der wiederholt wird. |
| Beatjump | Sprung um eine feste Anzahl Beats, ohne aus dem Takt zu fallen. |
| Flux / Slip | Der Track läuft unhörbar weiter, während ein Loop, Hotcue oder Rückwärtslauf zu hören ist. Beim Loslassen geht es dort weiter, wo der Track ohne Eingriff wäre. |
| Key Lock | Die Tonhöhe bleibt gleich, wenn das Tempo geändert wird. |
| Sync | Automatisches Angleichen von Tempo und Beat-Position an die Master-Clock. |
| Master-Clock | Taktgeber, an dem sich alle synchronisierten Decks und Effekte ausrichten. |
| Phase | Versatz der Beats zweier Decks zueinander. Phase 0 heißt: Die Kicks liegen exakt übereinander. |
| Quantisierung | Aktionen (Start, Hotcue, Loop) rasten auf den nächsten Beat ein. |
| Cue (Vorhören) | Signal eines Kanals nur auf dem Kopfhörer, bevor es das Publikum hört. |
| Gain | Vorverstärkung eines Kanals. |
| EQ-Kill | Vollständiges Auslöschen eines Frequenzbands. |
| Crossfader | Querregler, der zwischen zwei Seiten des Mixers überblendet. |
| Intro / Outro | Anfang und Ende eines Tracks, meist nur Drums, zum Mixen gedacht. |
| Breakdown | Abschnitt ohne Kick, in dem Spannung aufgebaut wird. |
| Build-up (Aufbau) | Steigerung am Ende des Breakdowns mit Risern und Rolls. |
| Drop | Moment, in dem Kick und Bass nach dem Breakdown wieder einsetzen. |
| Riser | Aufsteigender Klang, der einen Drop ankündigt. |
| Roll | Schnelle Wiederholung von Kick oder Snare (1/8, 1/16, 1/32) vor einem Drop. |
| Blend | Langer Übergang, bei dem beide Tracks gleichzeitig laufen. |
| Bass-Swap | Der Bass des alten Tracks wird am Phrasenwechsel gegen den des neuen getauscht. |
| Double Drop | Die Drops zweier Tracks fallen auf denselben Moment. |
| Stem | Einzelspur eines Tracks (Drums, Bass, Melodie, Vocals). |
| One-Shot | Sample, das einmal abgespielt wird. |
| LUFS | Maß für die empfundene Lautheit (EBU R 128). |
| True Peak (dBTP) | Tatsächlicher Spitzenpegel nach der Wandlung, auch zwischen zwei Abtastwerten. |
| Xrun | Aussetzer, weil die Audio-Berechnung nicht rechtzeitig fertig wurde. |

---

## 3. Nutzungsszenarien

### 3.1 Bibliothek aufbauen

1. Ich ziehe einen Ordner mit 300 Tracks ins Fenster.
2. Die Tracks erscheinen sofort in der Liste, zunächst nur mit den Angaben aus den Dateien. Die Analyse läuft im Hintergrund, ein Zähler zeigt den Fortschritt.
3. Nach der Analyse hat jeder Track BPM, Tonart, Energie, Lautheit, Wellenform, Abschnitte und Cue-Vorschläge.
4. Ich öffne einen Track in der Ansicht „Vorbereiten", sehe Beatgrid und erkannte Abschnitte, verschiebe den Drop-Marker um einen Takt und sperre die Analyse, damit sie nicht überschrieben wird.
5. Ich importiere meine Traktor-Sammlung. Vorhandene Cue-Punkte und Beatgrids werden übernommen und haben Vorrang vor der eigenen Analyse.

### 3.2 Live mixen

1. Ich lade einen Track auf Deck A und starte ihn. Er läuft auf dem Master-Ausgang.
2. In der Bibliothek sind passende Tracks farbig markiert (Tonart und Tempo verträglich). Ich lade einen auf Deck B.
3. Deck B springt auf den vorgeschlagenen Einstiegspunkt. Ich schalte Cue für Kanal B ein und höre ihn auf dem Kopfhörer.
4. Sync gleicht Tempo und Phase an. In den parallelen Wellenformen liegen die Beats übereinander.
5. Ich nehme bei B den Bass heraus, starte B am Phrasenwechsel von A, ziehe den Fader hoch, tausche acht Takte später die Bässe und blende A aus.
6. Ein Loop mit halbierender Länge und ein Echo-Out beenden A.
7. Der Mitschnitt läuft seit dem Start; am Ende exportiere ich ihn mit Trackliste.

### 3.3 Set automatisch erstellen

1. Ich wähle eine Playlist mit 14 Tracks, setze die Zieldauer auf 60 Minuten, zeichne einen Energieverlauf („ruhig beginnen, Höhepunkt bei 45 Minuten") und fixiere den ersten und den letzten Track.
2. Die Software schlägt eine Reihenfolge vor und zeigt die Zeitleiste: Tracks als Blöcke, Übergänge als Überlappungen, darunter die Reglerverläufe.
3. Jeder Übergang trägt die gewählte Technik und eine Bewertung mit Begründung („8A → 9A, 134,0 → 135,0 BPM, Bass-Swap über 32 Takte").
4. Ich höre Übergang 5 vor, wechsle ihn von „Blend" auf „Echo-Out" und verschiebe den Einstiegspunkt um eine Phrase.
5. Ich rendere den Mix als FLAC mit Ziel-Lautheit −14 LUFS und erhalte Audiodatei, Trackliste und Cue-Sheet.
6. Alternativ starte ich denselben Plan im Live-Modus als Autopilot und übernehme mittendrin selbst.

---

## 4. Funktionale Anforderungen

Priorität: **M** = Muss, **S** = Soll, **K** = Kann. Die Spalte „Stufe" verweist auf den Umsetzungsplan in Kapitel 11.

### 4.1 Musikbibliothek (BIB)

| ID | Anforderung | Prio | Stufe | Abnahmekriterium |
|---|---|---|---|---|
| BIB-01 | Import von Dateien und Ordnern per Drag-and-drop und Dialog, rekursiv | M | 1 | 1 000 Dateien erscheinen in ≤ 10 s in der Liste, Duplikate (gleicher Pfad oder gleicher Inhalts-Hash) werden nicht doppelt angelegt |
| BIB-02 | Formate: MP3, WAV, AIFF, FLAC, AAC/M4A, ALAC, OGG Vorbis, Opus; 8–32 bit, 22,05–192 kHz, mono und stereo | M | 1 | je Format eine Testdatei wird geladen, analysiert und abgespielt; Dekodierung weicht bei verlustfreien Formaten bitgenau nicht von der Referenz ab |
| BIB-03 | Metadaten lesen: Titel, Künstler, Album, Label, Genre, Jahr, Kommentar, Cover, vorhandene BPM- und Tonart-Angaben | M | 1 | ID3v2, Vorbis-Comment, MP4-Atome und RIFF-INFO werden korrekt gelesen (Testdateien) |
| BIB-04 | Metadaten bearbeiten und auf Wunsch in die Datei zurückschreiben | S | 3 | geänderter Titel steht nach Neustart in der Bibliothek und in der Datei |
| BIB-05 | Analyseergebnisse dauerhaft speichern, zugeordnet über Inhalts-Hash | M | 1 | nach Verschieben oder Umbenennen einer Datei bleibt die Analyse erhalten |
| BIB-06 | Playlists und Ordner für Playlists, Reihenfolge frei sortierbar | M | 1 | Anlegen, Umbenennen, Löschen, Umsortieren per Drag-and-drop; Zustand übersteht Neustart |
| BIB-07 | Intelligente Playlists aus Regeln (z. B. BPM 130–136 und Tonart 8A/9A/8B) | S | 3 | Liste aktualisiert sich bei neuen Tracks ohne Zutun |
| BIB-08 | Suche über Titel, Künstler, Label, Kommentar; Filter nach BPM-Bereich, Tonart, Energie, Genre, Bewertung | M | 1 | Ergebnis bei 20 000 Tracks in ≤ 100 ms nach Tastendruck |
| BIB-09 | Sortieren nach jeder Spalte, Spalten ein- und ausblendbar | M | 1 | Sortierung in ≤ 100 ms bei 20 000 Tracks |
| BIB-10 | Verträglichkeitsanzeige zum laufenden Master-Track: Tonart-Farbe, BPM-Abstand in %, Gesamtwert | M | 3 | Werte stimmen mit der Bewertungslogik aus 5.11 überein |
| BIB-11 | Filter „passt zum laufenden Track" | S | 3 | zeigt nur Tracks mit Paar-Bewertung ≥ einstellbarem Schwellwert |
| BIB-12 | Bewertung 0–5 Sterne, Farbmarkierung, Kommentar, „gespielt"-Zähler und Verlauf | S | 2 | gespielte Tracks einer Sitzung sind markiert |
| BIB-13 | Fehlende Dateien erkennen und neu zuordnen (einzeln oder per Ordner-Suche über Hash) | S | 8 | 100 verschobene Dateien werden per Ordner-Suche wiedergefunden |
| BIB-14 | Import Traktor `collection.nml`: Tracks, Playlists, Cue-Punkte, Loops, Beatgrid, BPM, Tonart | M | 8 | Testsammlung: Anzahl Tracks, Playlists und Cue-Punkte stimmt, Cue-Positionen weichen ≤ 5 ms ab |
| BIB-15 | Import Rekordbox-XML: Tracks, Playlists, Hotcues, Memory-Cues, Loops, Tempo-Abschnitte | M | 8 | wie BIB-14 |
| BIB-16 | Export der Bibliothek als Rekordbox-XML | K | 8 | exportierte Datei lässt sich von BIB-15 verlustfrei wieder einlesen |
| BIB-17 | Vorhören eines Tracks direkt aus der Liste auf dem Kopfhörer-Ausgang | S | 2 | Start in ≤ 300 ms, Springen per Klick in die Mini-Wellenform |
| BIB-18 | Automatische Sicherung der Datenbank bei jedem Programmstart, letzte 10 Stände | M | 1 | beschädigte Datenbank wird erkannt und die letzte Sicherung angeboten |

### 4.2 Track-Analyse (ANA)

| ID | Anforderung | Prio | Stufe | Abnahmekriterium |
|---|---|---|---|---|
| ANA-01 | Automatische Analyse beim Import im Hintergrund, mit Warteschlange, Pause und Fortschritt | M | 1 | Wiedergabe bleibt während der Analyse aussetzerfrei |
| ANA-02 | Tempo auf 0,01 BPM | M | 1 | Referenzsammlung: ≥ 97 % innerhalb ±0,05 BPM, Halb-/Doppeltempo-Fehler ≤ 2 % |
| ANA-03 | Beatgrid mit Position jedes Beats | M | 1 | mittlere Abweichung der Beat-Positionen ≤ 5 ms, 95 % ≤ 10 ms |
| ANA-04 | Downbeat-Erkennung | M | 1 | ≥ 85 % korrekt auf der Referenzsammlung |
| ANA-05 | Erkennung von Temposchwankungen; dann Beatgrid mit mehreren Tempo-Ankern | S | 3 | Testtrack mit Tempowechsel von 130 auf 134 BPM erhält zwei Anker, Abweichung ≤ 10 ms |
| ANA-06 | Tonart mit Anzeige musikalisch, Camelot und Open Key sowie Sicherheitswert 0–1 | M | 1 | exakt richtig ≥ 70 %, gewichtete Wertung ≥ 0,80 (5.3) |
| ANA-07 | Lautheit (integriert, LUFS) und True Peak | M | 1 | Abweichung ≤ 0,1 LU gegenüber den EBU-Testsignalen |
| ANA-08 | Wellenform-Daten in drei Frequenzbändern, mehrere Auflösungen | M | 1 | Zoom von Gesamtansicht bis 1 Beat ohne Nachladen sichtbarer Verzögerung |
| ANA-09 | Abschnitte: Intro, Aufbau, Breakdown, Drop, Hauptteil, Outro | M | 3 | Abschnittsgrenzen: ≥ 80 % innerhalb ±1 Takt der Referenz |
| ANA-10 | Phrasengrenzen im 8/16/32-Takt-Raster | M | 3 | ≥ 85 % der markierten Phrasengrenzen stimmen mit der Referenz überein |
| ANA-11 | Energieverlauf (pro Takt, 0–1) und Energiewert pro Track (1–10) | M | 3 | Rangfolge von 20 Referenztracks korreliert mit der Handbewertung (Spearman ≥ 0,7) |
| ANA-12 | Ereignisse: Kick/Bass setzt ein oder fällt weg; Höhen setzen ein oder fallen weg; Kick-Verdopplung; Roll; Riser; Stille | M | 3 | Trefferquote und Genauigkeit je Ereignistyp ≥ 80 % bei Toleranz ±1 Beat |
| ANA-13 | Automatische Cue-Vorschläge: Laden, Mix-In, Mix-Out, Drop, Breakdown; jeweils mit Eignungswert 0–1 | M | 3 | alle Vorschläge liegen auf Downbeats; Mix-In/-Out liegen auf Phrasengrenzen; bei ≥ 80 % der Tracks liegt der beste Mix-In-Vorschlag ≤ 1 Phrase neben der Referenz |
| ANA-14 | Manuelle Korrektur: BPM eintippen, halbieren/verdoppeln, Tap-Tempo, Grid verschieben (±1 ms, ±1 Beat), Downbeat setzen, Tonart wählen, Abschnitte und Ereignisse verschieben, anlegen, löschen | M | 3 | jede Korrektur wirkt sofort auf Sync, Loops und Cue-Vorschläge |
| ANA-15 | Analyse sperren (Schloss), damit eine erneute Analyse Handkorrekturen nicht überschreibt | M | 3 | gesperrter Track bleibt bei „alles neu analysieren" unverändert |
| ANA-16 | Analysedauer | M | 1 | 7-Minuten-Track ohne Stems in ≤ 15 s auf dem Referenzrechner |
| ANA-17 | Angabe der Analyse-Version je Track; gezielte Neuanalyse nach einem Update | S | 3 | Filter „mit älterer Version analysiert" vorhanden |

### 4.3 Decks (DECK)

| ID | Anforderung | Prio | Stufe | Abnahmekriterium |
|---|---|---|---|---|
| DECK-01 | Zwei Decks (A, B) | M | 1 | laden, starten, stoppen |
| DECK-02 | Vier Decks (A–D), Ansicht umschaltbar zwischen 2 und 4 | M | 2 | alle vier laufen gleichzeitig innerhalb der Lastgrenze aus 9.1 |
| DECK-03 | Deck-Typ je Deck wählbar: Track, Stem, Remix | M | 6/7 | Umschalten ohne Neustart der Engine |
| DECK-04 | Laden per Drag-and-drop, Tastatur und Doppelklick; Schutz vor Laden in ein laufendes Deck (abschaltbar) | M | 1 | Track ist in ≤ 2 s spielbereit |
| DECK-05 | Play/Pause, Cue (CDJ-Verhalten: halten = spielen, loslassen = zurück), Sprung zum Anfang | M | 1 | Start erfolgt ≤ 1 Audiopuffer nach dem Befehl bzw. quantisiert auf dem nächsten Beat |
| DECK-06 | Tempo-Regler mit Bereich ±2/4/6/8/10/16/25/50/100 %, Auflösung 0,01 %, Doppelklick = Reset | M | 1 | angezeigtes BPM = Original-BPM × (1 + Regler) |
| DECK-07 | Anschieben/Abbremsen (Pitch Bend) per Taste, Stärke einstellbar (Standard ±4 %), und per Ziehen in der Wellenform | M | 1 | Phase verschiebt sich stufenlos, kein Knacken |
| DECK-08 | Tempo-Sync und Phasen-Sync zur Master-Clock; Modi „Tempo" und „Beat" | M | 1 | synchronisierte Decks: Beat-Abweichung ≤ 1 ms über 10 min (Offline-Messung) |
| DECK-09 | Master-Clock: automatisch (das am längsten laufende hörbare Deck) oder fest gewählt oder freie Clock mit eigenem BPM | M | 1 | Wechsel des Masters ohne Temposprung |
| DECK-10 | Key Lock (Tonhöhe bleibt bei Tempoänderung) | M | 2 | bei +6 % Tempo weicht die Grundfrequenz eines Testtons ≤ 3 Cent ab |
| DECK-11 | Transponieren ±12 Halbtöne in Halbtonschritten, „Tonart an Master angleichen" | M | 2 | Anzeige der resultierenden Tonart; Testton um +1 Halbton = Faktor 1,0595 ±3 Cent |
| DECK-12 | 8 Hotcues je Track mit Typ (Cue, Laden, Mix-In, Mix-Out, Drop, Breakdown, Grid, Loop), Name und Farbe | M | 2 | Setzen, Springen, Löschen; quantisiert bei aktivem Snap |
| DECK-13 | Manuelle Loops (In/Out) und Auto-Loops mit 1/32 bis 32 Beats; Halbieren, Verdoppeln, Verschieben, Aktivieren/Verlassen | M | 2 | Loop-Länge exakt auf dem Beatgrid; nahtloser Rücksprung ohne Knacken (Überblendung 2 ms) |
| DECK-14 | Beatjump vor/zurück um 1/32 bis 64 Beats | M | 2 | Phase zum Master bleibt nach dem Sprung erhalten |
| DECK-15 | Flux-/Slip-Modus für Loops, Hotcues, Rückwärtslauf und Scratch in der Wellenform | M | 2 | nach dem Loslassen ist die Position gleich der einer ungestörten Wiedergabe (±1 Sample) |
| DECK-16 | Rückwärtslauf | S | 2 | läuft synchron zum Beatgrid |
| DECK-17 | Quantisierung (Q) und Snap (S) global und je Deck schaltbar; Quantisierungsweite 1/4, 1, 4, 16 Beats | M | 2 | Aktionen starten exakt auf dem Rasterpunkt |
| DECK-18 | Anzeige: Titel, Künstler, Cover, BPM (Original und aktuell), Tonart (aktuell), Tempo-Abweichung in %, gespielte und verbleibende Zeit, Takt-/Beat-Zähler, Beats bis zum nächsten Cue/Abschnitt | M | 1 | Werte aktualisieren sich mit ≥ 30 Hz |
| DECK-19 | Warnung am Trackende (einstellbar, Standard 30 s) | S | 2 | Deck blinkt, sobald die Restzeit unter dem Wert liegt |
| DECK-20 | Wiedergabe-Position per Klick in die Übersichts-Wellenform; im laufenden Deck nur mit aktivem Snap auf Beat-Raster (phasenerhaltend) | M | 1 | Sprung ohne Knacken |
| DECK-21 | Auto-Cue: nach dem Laden auf „Laden"-Cue oder ersten Beat springen | M | 2 | einstellbar |
| DECK-22 | Deck duplizieren (Track samt Position und Tempo auf ein anderes Deck kopieren) | S | 2 | beide Decks laufen phasengleich |

### 4.4 Mixer (MIX)

| ID | Anforderung | Prio | Stufe | Abnahmekriterium |
|---|---|---|---|---|
| MIX-01 | Je Kanal Gain −∞ bis +12 dB mit Auto-Gain auf Referenz-Lautheit (Standard −14 LUFS) | M | 1 | zwei Tracks mit −7 und −12 LUFS liegen nach Auto-Gain ≤ 0,5 LU auseinander |
| MIX-02 | 3-Band-Equalizer als Isolator: Tief < 200 Hz, Mitte 200 Hz–2,5 kHz, Hoch > 2,5 kHz; je Band −∞ (Kill) bis +6 dB; Summe aller Bänder in Mittelstellung ist klangneutral | M | 1 | Mittelstellung: Betragsfrequenzgang ±0,1 dB; Kill: Dämpfung ≥ 24 dB eine Oktave und ≥ 48 dB zwei Oktaven innerhalb des Bands (24 dB/Oktave) |
| MIX-03 | Umschaltbar auf 4 Bänder (zusätzlich Tiefmitten 200–800 Hz) und auf klassischen EQ mit −26/+6 dB | S | 4 | Umschalten ohne Knacken |
| MIX-04 | Filter auf einem Regler: Mitte = aus; links Tiefpass 20 kHz → 80 Hz; rechts Hochpass 20 Hz → 8 kHz; 12 dB/Oktave, Resonanz einstellbar | M | 1 | Mittelstellung ist bitgenau neutral (Bypass); Grenzfrequenz folgt der Kennlinie in 5.14 |
| MIX-05 | Kanalfader mit wählbarer Kurve (Standard quadratisch: Faktor = Stellung²; alternativ linear oder linear in dB bis −60 dB) | M | 1 | Fader oben = 0 dB, unten = stumm |
| MIX-06 | Crossfader mit Kurven „weich" (gleiche Leistung), „scharf" (Cut) und „aus"; Zuweisung je Kanal: links, rechts, durch | M | 1 | Kurve „weich": Summenleistung konstant ±0,2 dB über den ganzen Weg |
| MIX-07 | Pegelanzeigen je Kanal (vor dem Fader) und Master (Stereo), Spitze und RMS, Übersteuerungsanzeige mit Halten | M | 1 | Anzeige stimmt ±0,5 dB mit Messsignal überein |
| MIX-08 | Master: Lautstärke, Limiter (Decke −0,3 dBTP, schaltbar), Anzeige der Pegelreduktion | M | 1 | bei +12 dB Übersteuerung überschreitet der Ausgang −0,3 dBTP nicht |
| MIX-09 | Kopfhörer: Cue-Taste je Kanal, Cue/Master-Mischregler, Lautstärke | M | 1 | Cue-Signal erscheint ausschließlich auf dem Kopfhörer-Ausgang |
| MIX-10 | Split-Cue für Geräte mit einem Stereo-Ausgang: links Master (mono), rechts Cue (mono) | M | 1 | Messung zeigt getrennte Kanäle |
| MIX-11 | Zuweisung der Effekteinheiten je Kanal | M | 4 | siehe FX-02 |
| MIX-12 | Kanal-Stummschaltung und Solo-Vorhören von EQ-Bändern | K | 4 | – |
| MIX-13 | Alle Mixer-Parameter werden intern über 5 ms geglättet | M | 1 | Sprung eines Reglers erzeugt kein Knacken (kein Breitbandimpuls im Spektrum) |

### 4.5 Effekte (FX)

| ID | Anforderung | Prio | Stufe | Abnahmekriterium |
|---|---|---|---|---|
| FX-01 | Vier Effekteinheiten; je Einheit Modus „Gruppe" (drei Effekte in Reihe, je ein Regler, gemeinsamer Dry/Wet) oder „Einzel" (ein Effekt mit drei Parametern und Dry/Wet) | M | 4 | Umschalten ohne Knacken |
| FX-02 | Routing je Einheit: Insert in beliebige Kanäle, Send (nach Fader) oder Master | M | 4 | Signalfluss messbar korrekt |
| FX-03 | Temposynchrone Parameter in Notenwerten (1/32 … 4 Takte, punktiert und triolisch) zur Master-Clock | M | 4 | Delay-Zeit = Notenwert × 60/BPM, Abweichung ≤ 1 Sample |
| FX-04 | Effekte laut Katalog 5.15: Reverb, Delay, Filter, Flanger, Phaser, Gater, Beat-Repeat, Bitcrusher, Echo-Out; dazu Tape-Stop, Ringmodulator, Verzerrung als „Soll" | M | 4 | je Effekt Referenzmessung (Impulsantwort bzw. Testsignal) im Test hinterlegt |
| FX-05 | Effekt-Ausklang läuft weiter, wenn der Effekt abgeschaltet oder der Kanal geschlossen wird (bei Reverb, Delay, Echo-Out) | M | 4 | Ausklang hörbar und messbar nach Abschalten |
| FX-06 | Speichern und Laden von Effekt-Einstellungen als Presets | S | 4 | Preset übersteht Neustart |
| FX-07 | Mixer-Effekt je Kanal auf dem Filter-Regler wählbar (Filter, Reverb, Delay, Noise, Gater) | S | 4 | Mittelstellung neutral |

### 4.6 Stems (STEM)

| ID | Anforderung | Prio | Stufe | Abnahmekriterium |
|---|---|---|---|---|
| STEM-01 | Zerlegung eines Tracks in Drums, Bass, Melodie/Rest und Vocals, offline, im Hintergrund, mit Warteschlange | M | 6 | Summe der vier Stems weicht vom Original ≤ −40 dB ab (Restsignal wird dem Stem „Melodie/Rest" zugeschlagen) |
| STEM-02 | Speicherung der Stems im Cache, verlustfrei komprimiert; Speicherbedarf wird angezeigt, Cache-Grenze einstellbar | M | 6 | erneutes Laden ohne Neuberechnung |
| STEM-03 | Rechenzeit | M | 6 | 7-Minuten-Track in ≤ 7 min auf dem Referenzrechner (CPU), ≤ 1 min mit unterstützter GPU |
| STEM-04 | Stem-Deck: je Stem Lautstärke, Stumm, Filter, Effekt-Send; eigene Wellenform je Stem | M | 6 | Stummschalten wirkt in ≤ 1 Puffer, geglättet |
| STEM-05 | Stems stehen der Übergangslogik und dem Auto-Mix zur Verfügung | M | 6 | Übergang „Stem-Tausch" (5.10) rendert korrekt |
| STEM-06 | Import fertiger Stem-Dateien im offenen Format `.stem.mp4` | S | 6 | Testdatei wird als Stem-Deck geladen |
| STEM-07 | Analyse nutzt Stems, wenn vorhanden (Tonart aus Bass + Melodie, Ereignisse aus Drums/Bass) | K | 6 | Tonart-Trefferquote sinkt dadurch nicht |

### 4.7 Remix- und Sample-Decks (RMX)

| ID | Anforderung | Prio | Stufe | Abnahmekriterium |
|---|---|---|---|---|
| RMX-01 | Remix-Deck mit 4 Spuren × 4 Zellen; je Spur spielt höchstens eine Zelle | M | 7 | 16 Samples ladbar, je Spur eine aktiv |
| RMX-02 | Zelle ist Loop (temposynchron, mit Key Lock) oder One-Shot; Auslösen quantisiert auf 1/4, 1, 4 oder 16 Beats | M | 7 | Start exakt auf dem Rasterpunkt, Loop bleibt 10 min phasengleich zum Master |
| RMX-03 | Je Spur Lautstärke, Stumm, Filter, Effekt-Send | M | 7 | wie STEM-04 |
| RMX-04 | Aufnahme eines Samples aus einem laufenden Deck (aktueller Loop oder die letzten 1–16 Beats) direkt in eine Zelle | M | 7 | Sample ist beat-genau geschnitten und sofort spielbar |
| RMX-05 | Sample-Bibliothek mit eigener Kategorie, BPM- und Tonart-Analyse | S | 7 | Samples erscheinen in der Bibliothek mit Typ-Kennzeichnung |
| RMX-06 | Remix-Sets speichern und laden | M | 7 | Set übersteht Neustart |
| RMX-07 | Loop-Recorder auf dem Master oder einem Kanal mit Overdub | K | 7 | – |

### 4.8 Übergänge (TRANS)

| ID | Anforderung | Prio | Stufe | Abnahmekriterium |
|---|---|---|---|---|
| TRANS-01 | Katalog der zehn Techniken aus 5.10, jede als parametrisierte Vorlage mit Reglerverläufen | M | 5 | jede Technik rendert für ein passendes Testpaar; Verläufe entsprechen der Vorlage (±1 Sample am Rasterpunkt) |
| TRANS-02 | Jeder Übergang ist an Phrasengrenzen ausgerichtet | M | 5 | Start, Tauschpunkt und Ende liegen auf Downbeats im Phrasenraster beider Tracks |
| TRANS-03 | Bass-Regel: Es spielen nie zwei ungedämpfte Bässe länger als 1 Takt gleichzeitig | M | 5 | Messung je Takt: Der Pegel im Band < 120 Hz liegt im Übergang nie mehr als 3 dB über dem lautesten Takt, den A oder B allein in diesem Mix erreichen |
| TRANS-04 | Eigene Vorlagen anlegen, ändern, speichern | S | 5 | Vorlage ist im Auto-Mix wählbar |
| TRANS-05 | Übergänge sind im Live-Modus einzeln auslösbar („nächsten Übergang jetzt spielen") | S | 5 | Auslösen startet quantisiert an der nächsten Phrasengrenze |

### 4.9 Auto-Mix (AUTO)

| ID | Anforderung | Prio | Stufe | Abnahmekriterium |
|---|---|---|---|---|
| AUTO-01 | Reihenfolge aus Trackauswahl nach Bewertungslogik 5.11 | M | 5 | Ergebnis ist bei gleicher Eingabe reproduzierbar; Gesamtbewertung ≥ der einer zufälligen Reihenfolge in 100 % von 50 Testläufen |
| AUTO-02 | Vorgaben: Zieldauer, Energieverlauf als Kurve, feste Positionen, Tracks ausschließen, „alle Tracks verwenden" oder „beste Auswahl" | M | 5 | Vorgaben werden eingehalten; Zieldauer ±2 % |
| AUTO-03 | Übergangsplanung: Ausstiegs- und Einstiegspunkt, Technik, Dauer, Tempo-Verlauf, ggf. Transposition | M | 5 | TRANS-02 und TRANS-03 erfüllt für jeden Übergang |
| AUTO-04 | Offenlegung: Je Übergang werden Teilbewertungen und Begründung angezeigt | M | 5 | sichtbar in der Zeitleiste |
| AUTO-05 | Zeitleisten-Editor: Tracks verschieben, tauschen, entfernen, einfügen; Übergangspunkt und -länge im Phrasenraster verschieben; Technik wechseln; Reglerkurven bearbeiten | M | 5 | jede Änderung ist sofort vorhörbar; Rückgängig/Wiederholen über mindestens 100 Schritte |
| AUTO-06 | Vorhören eines Übergangs oder ab beliebiger Position | M | 5 | Start in ≤ 500 ms |
| AUTO-07 | Offline-Rendern | M | 5 | 60-Minuten-Mix in ≤ 6 min; Ergebnis bitgenau reproduzierbar |
| AUTO-08 | Autopilot im Live-Modus: spielt den Plan auf den Decks, alle Regler bewegen sich sichtbar; Eingriff an einem Regler übergibt diesen Regler an mich; „Übernehmen" beendet den Autopiloten nach dem laufenden Übergang oder sofort | M | 5 | Autopilot-Ausgabe entspricht dem Offline-Render (Differenz ≤ −60 dB) |
| AUTO-09 | Mix-Pläne speichern, laden, duplizieren | M | 5 | Datei `.dwmix` (6.3) |
| AUTO-10 | Planungsdauer | M | 5 | 30 Tracks in ≤ 10 s |
| AUTO-11 | Neu planen ab einem Punkt („ab Track 6 neu vorschlagen"), Handänderungen davor bleiben | S | 5 | – |

### 4.10 Aufnahme und Export (REC)

| ID | Anforderung | Prio | Stufe | Abnahmekriterium |
|---|---|---|---|---|
| REC-01 | Mitschnitt des Master-Signals (nach Limiter) als WAV oder FLAC, 16/24 bit | M | 2 | 4-h-Aufnahme ohne Aussetzer, Datei auch nach Absturz lesbar (laufendes Schreiben) |
| REC-02 | Export als WAV, FLAC, MP3 (192–320 kbit/s) | M | 5 | Dateien werden von gängigen Playern abgespielt |
| REC-03 | Ziel-Lautheit wählbar (aus, −14, −11, −9 LUFS, frei) mit True-Peak-Decke −1 dBTP | M | 5 | gemessene Lautheit ±0,3 LU |
| REC-04 | Trackliste mit Zeitstempeln als Text; Cue-Sheet (`.cue`); Kapitelmarken in den Metadaten | M | 5 | Zeitstempel = Zeitpunkt, an dem der neue Track lauter ist als der alte (±1 Takt) |
| REC-05 | Im Live-Mitschnitt wird die Trackliste aus dem Verlauf erzeugt | S | 5 | wie REC-04 |

### 4.11 Visuelle Darstellung (VIS)

| ID | Anforderung | Prio | Stufe | Abnahmekriterium |
|---|---|---|---|---|
| VIS-01 | Übersichts-Wellenform je Deck (ganzer Track) mit Position, Cues, Loops, Abschnitten | M | 1 | Klick springt zur Position |
| VIS-02 | Scrollende Detail-Wellenform je Deck, nach drei Frequenzbändern eingefärbt, Zoom 1–32 Beats sichtbar | M | 1 | 60 fps mit vier Decks auf dem Referenzrechner; Position sample-genau zur Audioausgabe (Latenz kompensiert) |
| VIS-03 | Beatgrid-Linien, Takt-Linien betont, Phrasenmarker, Abschnittsbänder, Ereignis-Marker in der Wellenform | M | 1/3 | Linien liegen deckungsgleich auf den Kick-Anschlägen |
| VIS-04 | Parallele Wellenformen aller laufenden Decks übereinander mit gemeinsamer Zeitachse relativ zur Abspielposition | M | 1 | bei Phase 0 liegen die Beat-Linien exakt untereinander |
| VIS-05 | Phasenanzeige je Deck (Versatz zum Master in Beats, ±0,5) | M | 1 | Anzeige 0 bei gemessener Abweichung < 1 ms |
| VIS-06 | Tonart-Anzeige mit Farbe nach Camelot-Position; Verträglichkeit zwischen Decks als Symbol (passt / geht / reibt) | M | 1/3 | Farben und Symbole folgen 5.11 |
| VIS-07 | Pegelanzeigen (MIX-07) und Master-Spektrum (30 Bänder, 1/3 Oktave) | M/S | 1/4 | – |
| VIS-08 | Stem-Wellenformen gestapelt im Stem-Deck | M | 6 | – |
| VIS-09 | Zeitleisten-Editor für den Auto-Mix mit Automationskurven | M | 5 | siehe 8.5 |
| VIS-10 | Layout skaliert von 1280×720 bis 4K, HiDPI-fähig | M | 0 | keine abgeschnittenen Elemente in den Prüfauflösungen 1280×720, 1920×1080, 2560×1440, 3840×2160 |
| VIS-11 | Dunkles Design als Standard; zusätzlich kontrastreiches Design | M/S | 0 | Text-Kontrast ≥ 4,5:1, Bedienelemente ≥ 3:1 |
| VIS-12 | Farben für Frequenzbänder und Tonarten sind auch bei Rot-Grün-Sehschwäche unterscheidbar (zusätzlich Helligkeit/Form) | S | 0 | Prüfung mit Simulationsfilter |
| VIS-13 | Vorausblick: Die Detail-Wellenformen der laufenden Decks lassen sich gemeinsam vor- und zurückschieben, ohne die Wiedergabe zu verändern; ein stehendes Deck bleibt dabei an seiner Stelle, solange ein anderes läuft; ein Griff führt zurück zur Abspielstelle. Wird ein stehendes Deck im Vorausblick gestartet, beginnt es um den Vorausblick früher (Start mit Vorlauf), notfalls vor dem Anfang des Tracks und bis dorthin stumm | M | 1 | Bei einem Vorausblick von n Beats zeigt die Bildmitte je laufendem Deck die Stelle, die es in n Beats (Clock-Tempo) erreicht; Beat-Linien synchroner Decks bleiben untereinander; ein stehendes Deck zeigt in der Bildmitte weiter seine Abspielstelle; nach einem Start mit Vorlauf von n Beats erreicht das Deck diese Stelle nach n Beats der Clock auf ≤ 1 ms (im Beat-Sync nach dem Einrasten auf ≤ einen halben Beat); die verschobene Ansicht ist als solche gekennzeichnet |

### 4.12 Audio-Engine (ENG)

| ID | Anforderung | Prio | Stufe | Abnahmekriterium |
|---|---|---|---|---|
| ENG-01 | Ausgabe über WASAPI und ASIO (Windows), CoreAudio (macOS), ALSA, JACK und PipeWire (Linux) | M | 1 | Geräteliste erscheint, Ausgabe funktioniert je Schnittstelle |
| ENG-02 | Freie Zuordnung: Master und Kopfhörer auf beliebige Kanalpaare eines Geräts | M | 1 | – |
| ENG-03 | Abtastrate 44,1/48/88,2/96 kHz, Puffergröße 64–2048 Samples wählbar | M | 1 | – |
| ENG-04 | Interne Verarbeitung 32-bit-Gleitkomma, stereo | M | 1 | – |
| ENG-05 | Ausgabelatenz | M | 1 | ≤ 10 ms bei 256 Samples/48 kHz mit ASIO, CoreAudio oder JACK; Anzeige der tatsächlichen Latenz |
| ENG-06 | Keine Speicheranforderung, keine Sperren, kein Dateizugriff im Audio-Thread | M | 1 | automatischer Test mit Allocator-Überwachung im Audio-Pfad |
| ENG-07 | Offline-Betrieb derselben Engine (ohne Audiogerät, schneller als Echtzeit, deterministisch) | M | 1 | zwei Render-Läufe sind bitgenau gleich |
| ENG-08 | Geräteausfall wird erkannt; Wiedergabe wird nach Wiederanschluss fortgesetzt oder auf ein Ersatzgerät gelegt | M | 2 | Abziehen des Interfaces führt nicht zum Absturz |
| ENG-09 | Anzeige von DSP-Last und Xrun-Zähler | M | 1 | – |
| ENG-10 | Time-Stretching und Pitch-Shifting in Echtzeit, Qualität laut 5.13 | M | 2 | – |
| ENG-11 | Ableton Link | K | – | – |

### 4.13 Bedienung (BED)

| ID | Anforderung | Prio | Stufe | Abnahmekriterium |
|---|---|---|---|---|
| BED-01 | Jede Funktion ist per Maus erreichbar | M | laufend | – |
| BED-02 | Jede Live-Funktion ist per Tastatur erreichbar; Standardbelegung laut 8.7 | M | 2 | – |
| BED-03 | Belegung frei änderbar, an die physische Tastenposition gebunden (unabhängig vom Tastaturlayout), mehrere Profile, Import/Export | M | 2 | Belegung wirkt identisch auf QWERTZ und QWERTY |
| BED-04 | Tasten-Modi: Auslösen, Halten, Umschalten; Umschalt-Ebenen | M | 2 | – |
| BED-05 | Regler mit der Maus: Ziehen, Mausrad, Feineinstellung mit Shift, Doppelklick = Standardwert, Zahleneingabe | M | 1 | – |
| BED-06 | Rückgängig/Wiederholen für Bibliotheks- und Vorbereitungs-Aktionen sowie im Auto-Mix-Editor | M | 3/5 | – |
| BED-07 | Sprache der Oberfläche: Deutsch und Englisch | M/S | 0 | alle Texte liegen in Sprachdateien |
| BED-08 | Einstellungen, Fensterzustand und Deck-Zustand überstehen den Neustart | M | 1 | – |
| BED-09 | Absturzsicherung: Nach einem Absturz werden geladene Tracks und Positionen angeboten | S | 2 | – |


---

## 5. Verfahren im Detail

### 5.1 Vorverarbeitung

Jede Datei wird einmal vollständig dekodiert. Für die Analyse entstehen daraus:

- **Analyse-Signal:** mono, 44,1 kHz, 32-bit-Gleitkomma.
- **Spektrogramm:** Kurzzeit-Fourier-Transformation, Fenster 2048 Samples (Hann), Schrittweite 256 Samples (5,8 ms).
- **Bänder:** Tief 20–150 Hz, Tiefmitten 150–500 Hz, Mitten 500 Hz–5 kHz, Höhen 5–16 kHz.

Positionen werden überall als Sekunden ab dem ersten dekodierten Sample gespeichert (64-bit-Gleitkomma). Bei MP3 und AAC wird die Encoder-Verzögerung aus den Datei-Angaben (LAME-/iTunSMPB-Tag) abgezogen, damit Positionen unabhängig vom Decoder stimmen.

### 5.2 Tempo und Beatgrid

**Eingabe:** Spektrogramm. **Ausgabe:** BPM, Zeitpunkt des ersten Beats, Downbeat-Versatz (0–3), Liste von Tempo-Ankern, Sicherheitswert.

1. **Anschlagskurve:** positive Spektraldifferenz je Band, logarithmisch komprimiert. Das Tief-Band (Kick) wird doppelt gewichtet.
2. **Tempo-Schätzung:** Autokorrelation der Anschlagskurve für Perioden von 60 bis 200 BPM. Jede Kandidaten-Periode wird mit einem Kamm aus 1-, 2- und 4-facher Periode bewertet und mit einer Glockenkurve um 135 BPM gewichtet (Breite ±1 Oktave).
3. **Oktav-Korrektur:** Liegt der beste Kandidat außerhalb von 110–170 BPM und sein halbierter oder verdoppelter Wert innerhalb, wird der Wert im Bereich gewählt, sofern dessen Bewertung mindestens 60 % des besten beträgt. Der Bereich ist einstellbar (Voreinstellungen: Techno 110–170, frei 60–200).
4. **Verfeinerung:** Die Zeitpunkte der Kick-Anschläge werden auf ein Raster mit konstantem Abstand gelegt (kleinste Quadrate über alle Anschläge, Ausreißer verworfen). Ergebnis: BPM auf 0,01 genau und Phase.
5. **Prüfung auf Schwankung:** Die Rasterabweichung wird in Fenstern von 32 Takten geprüft. Bleibt sie überall unter 10 ms, gilt das Tempo als konstant (ein Anker). Andernfalls wird der Track an den Stellen größter Abweichung geteilt und je Abschnitt neu angepasst (mehrere Anker).
6. **Downbeat:** Für die vier möglichen Lagen der Eins wird bewertet, wie gut die stärksten Veränderungen im Track (Neuheitskurve aus 5.6) auf Vielfache von 8 Takten fallen. Zusätzlich zählt der Abstand des ersten Kick-Anschlags zum Raster.
7. **Sicherheitswert:** Verhältnis der Bewertung des besten zum zweitbesten Tempo-Kandidaten, verrechnet mit der mittleren Rasterabweichung, auf 0–1 abgebildet. Unter 0,5 wird der Track in der Bibliothek als „Tempo prüfen" markiert.

**Qualitätsmaße:** ANA-02, ANA-03, ANA-04.

**Korrektur von Hand:** BPM eintippen, ×2, ÷2, Tap-Tempo, Raster in Schritten von 1 ms oder 1 Beat verschieben, Downbeat an der aktuellen Position setzen, Anker hinzufügen oder löschen.

### 5.3 Tonart

**Eingabe:** Analyse-Signal. **Ausgabe:** Tonart (24 Klassen), Sicherheitswert, Stimmungs-Abweichung in Cent.

1. **Tonhöhen-Spektrum:** Fourier-Transformation mit langem Fenster (16 384 Samples bei 44,1 kHz), Auswertung der Spitzen zwischen 55 Hz und 2 kHz.
2. **Stimmung:** Schätzung der Abweichung vom Kammerton (±50 Cent) aus der Verteilung der Spitzen; die Zuordnung zu Halbtönen wird entsprechend verschoben.
3. **Chroma-Profil:** Energie je Halbtonklasse (12 Werte) pro Fenster. Fenster mit überwiegend geräuschhaftem Inhalt (hohe spektrale Flachheit, z. B. reine Drum-Passagen) werden geringer gewichtet. Die Fundamentale der Kick (meist 40–60 Hz) liegt unter der Auswertungsgrenze und verfälscht das Profil nicht.
4. **Vergleich:** Das über den Track gemittelte Profil wird mit 24 Tonart-Profilen korreliert. Startwerte sind die Profile nach Krumhansl-Kessler; sie werden auf der Referenzsammlung für elektronische Musik nachjustiert.
5. **Sicherheitswert:** Abstand zwischen bester und zweitbester Korrelation, auf 0–1 abgebildet. Unter 0,3 gilt der Track als „atonal/perkussiv"; die Tonart wird dann in der Bewertung nur schwach berücksichtigt.

**Qualitätsmaß (ANA-06):** Anteil exakt richtiger Tonarten sowie gewichtete Wertung: exakt 1,0; Quinte daneben 0,5; Paralleltonart (gleiche Vorzeichen) 0,3; gleichnamige Tonart (Dur/Moll) 0,2. Zur Einordnung: Auch die besten bekannten Verfahren erreichen bei elektronischer Musik nur rund 70–75 % exakte Treffer. Die Handkorrektur ist deshalb fester Teil des Ablaufs.

**Schreibweisen:**

| Camelot | Open Key | Moll (A / m) | Camelot | Open Key | Dur (B / d) |
|---|---|---|---|---|---|
| 1A | 6m | as-Moll | 1B | 6d | H-Dur |
| 2A | 7m | es-Moll | 2B | 7d | Fis-Dur |
| 3A | 8m | b-Moll | 3B | 8d | Des-Dur |
| 4A | 9m | f-Moll | 4B | 9d | As-Dur |
| 5A | 10m | c-Moll | 5B | 10d | Es-Dur |
| 6A | 11m | g-Moll | 6B | 11d | B-Dur |
| 7A | 12m | d-Moll | 7B | 12d | F-Dur |
| 8A | 1m | a-Moll | 8B | 1d | C-Dur |
| 9A | 2m | e-Moll | 9B | 2d | G-Dur |
| 10A | 3m | h-Moll | 10B | 3d | D-Dur |
| 11A | 4m | fis-Moll | 11B | 4d | A-Dur |
| 12A | 5m | cis-Moll | 12B | 5d | E-Dur |

Transponieren um +1 Halbton verschiebt die Camelot-Zahl um +7 (modulo 12). Ohne Key Lock verschiebt eine Tempoänderung um x % die Tonhöhe um 12 · log₂(1 + x/100) Halbtöne; die angezeigte Tonart berücksichtigt das.

### 5.4 Lautheit

Messung nach ITU-R BS.1770-4 / EBU R 128: integrierte Lautheit in LUFS, Lautheitsbereich (LRA), True Peak mit vierfacher Überabtastung. Auto-Gain = Referenz-Lautheit − gemessene Lautheit, begrenzt auf ±12 dB.

### 5.5 Wellenform-Daten

Je Track werden für die drei Anzeige-Bänder (Tief < 200 Hz, Mitte 200 Hz–2,5 kHz, Hoch > 2,5 kHz) Minimum, Maximum und RMS gespeichert, in einer Pyramide mit den Auflösungen 64, 256, 1 024 und 4 096 Samples je Punkt sowie einer festen Übersicht mit 2 000 Punkten. Speicherung als Binärdatei im Cache (ca. 3–4 MB je Track).

### 5.6 Struktur und Phrasen

**Eingabe:** Beatgrid, Spektrogramm. **Ausgabe:** Liste von Abschnitten (Typ, Start-Takt, Länge) und Phrasengrenzen.

1. **Merkmale pro Takt:** Pegel je Band, Kick-Stärke (Regelmäßigkeit der Tief-Anschläge auf den Beats), Bass-Pegel zwischen den Kicks, Höhen-Pegel, Anschlagsdichte, spektraler Schwerpunkt, Tonalität.
2. **Neuheitskurve:** Abstand der Merkmale der vier Takte vor und nach jeder Taktgrenze.
3. **Phrasenraster:** Es wird der Versatz gesucht, bei dem die meisten Neuheits-Spitzen auf Vielfache von 8 Takten fallen. Spitzen auf diesem Raster sind Phrasengrenzen; ihr Abstand (8, 16, 32 Takte) wird je Abschnitt festgehalten. Tracks mit einem Auftakt oder einem zusätzlichen Takt erhalten einen Rasterversatz ab dieser Stelle.
4. **Einteilung** nach Regeln auf den Merkmalen:

| Abschnitt | Regel |
|---|---|
| Intro | vom Anfang bis zur ersten Phrasengrenze, an der Kick und Bass gemeinsam auf vollem Pegel sind |
| Hauptteil | Kick und Bass vorhanden, Pegel ≥ 80 % des Track-Maximums |
| Breakdown | Kick fehlt ≥ 4 Takte, davor gab es einen Hauptteil, und es folgt wieder Kick |
| Aufbau | die letzten Takte eines Breakdowns mit steigender Anschlagsdichte, steigendem Höhenanteil oder Riser |
| Drop | erste Phrase nach einem Breakdown, in der Kick und Bass wieder einsetzen |
| Outro | von der letzten Phrasengrenze, ab der Bass oder mehrere Elemente dauerhaft fehlen, bis zum Ende |

**Qualitätsmaße:** ANA-09, ANA-10.

### 5.7 Energie

- **Verlauf:** je Takt gewichtete Summe aus Lautheit (40 %), Tief-Pegel (25 %), Anschlagsdichte (20 %) und Höhen-Pegel (15 %), jeweils auf das Track-Maximum normiert.
- **Track-Wert 1–10:** aus der mittleren Energie der Hauptteile, absolut skaliert über Lautheit, Anschlagsdichte, spektralen Schwerpunkt und BPM. Die Skalierung wird an der Referenzsammlung geeicht (ANA-11).

### 5.8 Ereignisse

Alle Ereignisse werden auf den nächsten Beat gerastet und tragen eine Stärke 0–1.

| Ereignis | Erkennung |
|---|---|
| Kick setzt ein / fällt weg | Kick-Stärke überschreitet bzw. unterschreitet für ≥ 2 Takte die Schwelle |
| Bass setzt ein / fällt weg | Tief-Pegel zwischen den Kicks ändert sich um ≥ 6 dB für ≥ 2 Takte |
| Höhen setzen ein / fallen weg | Höhen-Pegel ändert sich um ≥ 6 dB für ≥ 2 Takte (typisch: Hi-Hat kommt dazu) |
| Kick-Verdopplung | Tief-Anschläge im Abstand eines halben Beats über ≥ 2 Beats |
| Roll | Anschlagsdichte im Mitten-Band verdoppelt sich stufenweise (1/8 → 1/16 → 1/32) innerhalb von ≤ 4 Takten |
| Riser | spektraler Schwerpunkt oder Rausch-Pegel steigt über ≥ 4 Takte stetig an |
| Stille / Break | Gesamtpegel < −40 dB unter dem Track-Mittel für ≥ 1 Beat |

Die ersten beiden Zeilen bilden die Veränderungen im Tiefenbereich ab, die dritte die im Höhenbereich, die vierte und fünfte die Verdopplungen vor einem Drop.

**Qualitätsmaß:** ANA-12.

### 5.9 Cue-Vorschläge

Kandidaten sind alle Phrasengrenzen. Jeder Kandidat erhält Eignungswerte:

- **Laden:** erster Downbeat des Tracks.
- **Mix-In:** hoch bewertet, wenn ab hier ≥ 16 Takte Schlagzeug ohne Bass und ohne Melodie folgen (Intro) und danach ein Ereignis „Bass setzt ein" auf einer Phrasengrenze liegt. Zusätzliche Kandidaten: Beginn des Aufbaus vor dem ersten Drop (für Breakdown-Einstieg) und der Drop selbst (für Schnitte).
- **Mix-Out:** hoch bewertet, wenn ab hier ≥ 16 Takte mit Kick folgen, die Melodie fehlt oder ausdünnt und der Track noch mindestens so lang läuft wie der längste gewünschte Übergang. Zusätzliche Kandidaten: Beginn jedes Breakdowns, Beginn des Outros, letzte Phrasengrenze vor „Bass fällt weg".
- **Drop, Breakdown:** direkt aus 5.6.

Die besten Vorschläge (höchstens zwei je Typ) werden als Cue-Punkte vom Typ „automatisch" angelegt. Importierte oder von Hand gesetzte Cues desselben Typs haben Vorrang.

### 5.10 Übergangskatalog

**Grundregeln für alle Techniken**

- A ist der auslaufende, B der einlaufende Track. Beide laufen synchron zur Master-Clock. Alle Zeiten sind in Takten ab Übergangsbeginn T0 angegeben; L ist die Länge, S der Tauschpunkt.
- T0, S und T0 + L liegen in beiden Tracks auf Phrasengrenzen (TRANS-02).
- Reglerverläufe sind stückweise linear zwischen den genannten Punkten, in Master-Beats gerechnet. EQ-Werte in dB, „Kill" = −∞. Fader 0–1. Filter-Stellung −1 (Tiefpass ganz zu) … 0 (aus) … +1 (Hochpass ganz zu), Kennlinie in 5.14.
- Bass-Regel (TRANS-03): Der Tausch der Bässe geschieht innerhalb des letzten Beats vor S.
- K = harmonische Verträglichkeit (5.11), Δ = Tempo-Abstand in % vor dem Angleichen.

**T1 – Langer Blend mit Bass-Tausch** · L = 16, 32 oder 64 · S = L/2

| Regler | Verlauf |
|---|---|
| B Fader | 0 → 1 von Takt 0 bis L/8 |
| B Tief | Kill bis S − 1 Beat, dann → 0 dB bei S |
| B Mitte | −6 dB bei 0 → 0 dB bei S |
| B Hoch | −3 dB bei 0 → 0 dB bei S/2 |
| A Tief | 0 dB bis S − 1 Beat, dann → Kill bei S |
| A Mitte | 0 dB bis S → −9 dB bei L − L/8 |
| A Hoch | 0 dB bis S → −12 dB bei L − L/8 |
| A Fader | 1 bis L − L/4 → 0 bei L |

Passt, wenn: Δ ≤ 6 %; K ≥ 0,6 oder mindestens einer der überlappenden Abschnitte perkussiv ist; A hat ab T0 mindestens S Takte mit Kick; B hat ab Einstieg ein Intro von mindestens S Takten. Ideal: In B liegt „Bass setzt ein" genau auf S. Das ist der Standard-Übergang im Techno.

**T2 – Filter-Übergang** · L = 8, 16 oder 32 · S = L/2

| Regler | Verlauf |
|---|---|
| A Filter | 0 bei 0 → +0,38 bei S (Hochpass ≈ 200 Hz) → +0,8 bei L (≈ 2,4 kHz) |
| A Fader | 1 bis L − 2 → 0 bei L |
| B Fader | 0 → 1 von Takt 0 bis 2 |
| B Tief | Kill bis S − 1 Beat, dann → 0 dB bei S |
| B Filter | −0,45 bei 0 (Tiefpass ≈ 1,7 kHz) → 0 bei S |

Passt, wenn: Δ ≤ 6 %; K ≥ 0,5. Wirkt gut, wenn A melodisch ist und „weggedreht" werden soll.

**T3 – Harter Schnitt auf die Eins** · L = 0

| Regler | Verlauf |
|---|---|
| A Fader | 1 → 0 bei T0 (Glättung 5 ms) |
| B Fader | 0 → 1 bei T0 |
| Variante „Lücke" | A wird 1 Beat oder 2 Beats vor T0 stumm; B setzt auf der Eins ein |

Passt, wenn: B am Einstieg sofort Kick und volle Energie hat (Drop oder Hauptteil); Energie von B am Einstieg ≥ Energie von A am Ausstieg − 0,1. Tonart und Tempo müssen nicht passen; ein Temposprung ist erlaubt.

**T4 – Echo-Out / Reverb-Out** · Vorlauf 1 Takt, Ausklang 2–4 Takte

| Regler | Verlauf |
|---|---|
| A Effekt-Send (Delay 3/4 Beat, Feedback 0,65, Hochpass im Rückweg 300 Hz) | 0 bei T0 − 1 Takt → 1 bei T0 |
| A Eingang in den Kanal | zu bei T0; der Effekt klingt aus |
| B Fader | 0 → 1 bei T0 |
| Variante Reverb | statt Delay: Hall mit 4 s Abklingzeit, Send-Verlauf gleich |

Passt wie T3, wirkt weicher. Bevorzugt bei unverträglichen Tonarten oder Temposprung; die Delay-Zeit bleibt auf dem Tempo von A stehen.

**T5 – Loop-Out mit sich verkürzendem Loop** · L = 8 oder 16

| Regler | Verlauf (für L = 8; bei L = 16 alle Zeiten doppelt) |
|---|---|
| A Loop-Länge in Beats | 4 (Takt 0–2), 2 (2–4), 1 (4–6), 1/2 (6–7), 1/4 (7–7,5), 1/8 (7,5–8) |
| A Filter | 0 bei 0 → +0,6 bei L |
| A Fader | 1 → 0 bei L (Schnitt) |
| B | Variante a: Schnitt-Einstieg bei L (wie T3). Variante b: läuft ab 0 mit Tief-Kill und Fader 0 → 1 über 2 Takte mit, Tief → 0 dB bei L |

Passt, wenn: A am Ausstieg Kick hat; B bei L einen Drop oder Kick-Einsatz hat. Für Variante b zusätzlich Δ ≤ 6 % und K ≥ 0,5.

**T6 – Einstieg im Breakdown, Wechsel am Drop** · L = n Takte, n = Vielfaches von 8, höchstens 32

Der Drop von B wird auf den Zeitpunkt des Drops von A gelegt. T0 = Drop − n, wobei n die kürzere der beiden Längen ist: Breakdown von A oder Vorlauf von B bis zu seinem Drop.

| Regler | Verlauf |
|---|---|
| B Fader | 0 → 1 von Takt 0 bis n/2 |
| B Tief | Kill bis zum Drop, dann 0 dB |
| A Filter | 0 bis n/2 → +0,5 bei n |
| A Fader | 1 bis n − 1 Beat → 0 bei n (der Drop von A erklingt nicht) |

Passt, wenn: A einen Breakdown von ≥ 8 Takten hat; B einen Aufbau mit Drop hat; Δ ≤ 3 %; K ≥ 0,6, und K ≥ 0,9, wenn beide Abschnitte melodisch sind.

**T7 – Double Drop** · Vorlauf 8–16 Takte, gemeinsame Phase D = 8 oder 16 Takte

| Regler | Verlauf |
|---|---|
| B Fader | 0 → 0,7 im Vorlauf (Aufbau von B unter dem Aufbau von A), 1 ab Drop |
| B Tief | Kill im Vorlauf, 0 dB ab Drop |
| A Tief | 0 dB bis Drop − 1 Beat, dann Kill |
| A Mitte | 0 dB → −3 dB ab Drop |
| A Fader | 1 bis Drop + D − 4 → 0 bei Drop + D (oder Schnitt bei Drop + D) |

Passt, wenn: beide Tracks Aufbau und Drop haben; K ≥ 0,9; Δ ≤ 2 %; Energiewert beider Tracks ≥ 6. Wird sparsam eingesetzt (höchstens einmal je 30 Minuten, einstellbar).

**T8 – Anteasern** · Dauer 4 oder 8 Takte, 16–32 Takte vor dem eigentlichen Übergang

| Regler | Verlauf |
|---|---|
| B Position | markanteste Phrase von B (höchste Tonalität im Hauptteil) |
| B Tief | Kill |
| B Filter | +0,5 (Hochpass ≈ 400 Hz) |
| B Fader | 0 → 0,7 über 1 Takt, am Ende Schnitt oder Echo-Out |

Danach springt B zurück auf seinen Einstiegspunkt. Passt, wenn: K ≥ 0,8; ein Deck frei ist. Wird mit T1, T2 oder T6 kombiniert.

**T9 – Stem-Tausch** · L = 16, 32 oder 64 · Q = L/4

| Zeitpunkt | Aktion |
|---|---|
| 0 → Q | B Melodie 0 → 0,8; alle anderen Stems von B stumm |
| Q | Drums tauschen: A Drums → 0, B Drums → 1 (innerhalb 1 Beat) |
| 2Q | Bass tauschen: A Bass → 0, B Bass → 1 (innerhalb 1 Beat) |
| 2Q → 3Q | A Melodie 1 → 0; B Melodie 0,8 → 1 |
| 3Q | A Vocals aus, B Vocals an; A ist damit vollständig aus |

Passt, wenn: für beide Tracks Stems vorliegen; Δ ≤ 6 %; K ≥ 0,8 (Melodien und Bässe überlagern sich länger als bei T1).

**T10 – Tempo-Angleichung** (Zusatz zu T1, T2, T5b, T6, T9)

- Δ ≤ 0,5 %: Das Master-Tempo springt am Tauschpunkt S unhörbar auf das Tempo von B.
- 0,5 % < Δ ≤ 6 %: Das Master-Tempo läuft linear vom Tempo von A zum Tempo von B, mit höchstens 0,25 % Änderung je Takt. Reicht die Übergangslänge dafür nicht, beginnt der Verlauf bis zu 16 Takte vor T0 und endet bis zu 16 Takte nach T0 + L. Key Lock ist für beide Tracks aktiv.
- 6 % < Δ ≤ 8 %: wie oben, zulässig nur für T1 mit L ≥ 32; sonst T3 oder T4 mit Temposprung.
- Δ > 8 %: nur T3 oder T4 mit Temposprung. Vor dem Vergleich wird geprüft, ob halbes oder doppeltes Tempo besser passt.

### 5.11 Bewertung eines Track-Paars

Gesamtwert **P(A → B) = 0,40 · K + 0,25 · T + 0,20 · E + 0,15 · U**, jeweils 0–1. Die Gewichte sind einstellbar.

**K – harmonische Verträglichkeit** (Camelot-Zahl n, Buchstabe A/B):

| Beziehung | Beispiel | K |
|---|---|---|
| gleiche Tonart | 8A → 8A | 1,0 |
| Nachbar, gleicher Buchstabe | 8A → 7A, 9A | 0,9 |
| Paralleltonart | 8A → 8B | 0,9 |
| diagonal | 8A → 9B, 8B → 7A | 0,6 |
| +2 (Energiesprung) | 8A → 10A | 0,6 |
| +7 (ein Halbton höher, bewusster Energiesprung) | 8A → 3A | 0,5 |
| alles andere | | 0,1 |

- Hat einer der Tracks einen Tonart-Sicherheitswert unter 0,3 (perkussiv), gilt K = 0,7.
- Ist K < 0,6 und würde Transponieren von B um ±1 oder ±2 Halbtöne K ≥ 0,9 ergeben, wird transponiert (mit Key Lock). Abzug 0,1 je Halbton. Mehr als ±2 Halbtöne werden nicht automatisch gewählt (**Annahme:** darüber werden Artefakte hörbar).

**T – Tempo:** 1,0 bei Δ ≤ 2 %; linear fallend auf 0 bei Δ = 8 %; darüber 0. Verglichen wird auch mit halbem und doppeltem Tempo.

**E – Energie:** 1 − |Soll-Energie an dieser Stelle des Sets − Energiewert von B| / 9. Zusätzlich Abzug 0,2, wenn der Sprung zwischen A und B mehr als 3 Stufen beträgt und der Soll-Verlauf dort keinen Sprung vorsieht.

**U – Übergangsqualität:** Wert der besten anwendbaren Technik für dieses Paar (siehe unten).

**Abzüge:** gleicher Künstler direkt hintereinander −0,1; Track in den letzten 30 Tagen im selben Plan-Nachbar-Paar verwendet −0,05 (abschaltbar).

**Wahl der Technik:** Für jede Kombination aus Mix-Out-Kandidat von A, Mix-In-Kandidat von B und Technik werden zuerst die Voraussetzungen geprüft. Die verbleibenden Kombinationen werden bewertet mit

U = Grundwert der Technik × Eignung Mix-Out × Eignung Mix-In × Passung − Wiederholungsabzug.

| Technik | Grundwert | Passung steigt, wenn … |
|---|---|---|
| T1 Blend | 1,0 | „Bass setzt ein" in B auf S liegt; A am Ausstieg wenig Melodie hat |
| T9 Stem-Tausch | 0,95 | K hoch ist und beide Tracks melodisch sind |
| T6 Breakdown-Einstieg | 0,9 | Breakdown von A und Vorlauf von B gleich lang sind |
| T2 Filter | 0,85 | A am Ausstieg melodisch ist |
| T5 Loop-Out | 0,75 | B mit Drop einsteigt und die Soll-Energie steigt |
| T4 Echo-Out | 0,7 | K niedrig oder Δ hoch ist |
| T7 Double Drop | 0,7 | Soll-Energie nahe am Höhepunkt des Sets liegt |
| T3 Schnitt | 0,6 | K niedrig oder Δ hoch ist; Soll-Energie springt |

Wiederholungsabzug: 0,15, wenn dieselbe Technik im vorherigen Übergang verwendet wurde, 0,3, wenn in den beiden vorherigen. T1 ist davon ausgenommen. T8 (Anteasern) wird nachträglich bei höchstens jedem dritten Übergang ergänzt, wenn die Voraussetzungen erfüllt sind.

### 5.12 Planung des Sets

**Eingabe:** Trackmenge, Zieldauer, Energie-Sollkurve, feste Positionen. **Ausgabe:** Mix-Plan (6.3).

1. **Spieldauer je Track:** Zieldauer geteilt durch Trackzahl, begrenzt auf mindestens 2 Minuten und höchstens die mixbare Länge des Tracks.
2. **Reihenfolge:** Gesucht ist die Reihenfolge mit der höchsten Summe aller P(A → B). Bis 10 Tracks wird die beste Lösung exakt bestimmt (dynamische Programmierung über Teilmengen). Darüber: Aufbau durch bestes Einfügen, danach Verbesserung durch Vertauschen und Umhängen von Teilstücken, mit festem Startwert für den Zufall und einem Zeitlimit von 5 s. Feste Positionen sind harte Bedingungen.
3. **Auswahl „beste Auswahl":** Passen nicht alle Tracks in die Zieldauer, werden die Tracks mit dem geringsten Beitrag zur Gesamtbewertung entfernt.
4. **Übergänge:** Für jedes Nachbar-Paar wird die Kombination mit dem höchsten U gewählt, bei der der Mix-Out-Punkt der gewünschten Spieldauer am nächsten liegt (Abweichung geht mit −0,05 je Phrase in U ein).
5. **Feinabstimmung der Dauer:** Weicht die Gesamtdauer um mehr als 2 % ab, werden Mix-Out-Punkte um einzelne Phrasen verschoben, beginnend bei den Übergängen mit dem geringsten Bewertungsverlust.
6. **Prüfung:** Jeder Übergang wird gegen TRANS-02 und TRANS-03 geprüft. Verstöße führen zur nächstbesten Kombination.

### 5.13 Tempo- und Tonhöhenänderung

| Betriebsart | Verfahren | Qualitätsziel |
|---|---|---|
| Key Lock aus | Abtastratenwandlung mit gefenstertem Sinc-Filter (32 Stützstellen), Tempo und Tonhöhe ändern sich gemeinsam | Sperrdämpfung ≥ 80 dB, kein hörbares Aliasing |
| Key Lock an oder Transposition | Zeitdehnung im Frequenzbereich (Phase-Vocoder-Familie) | Tonhöhenfehler ≤ 3 Cent; bei ±8 % Tempo verlängert sich die Anstiegszeit einer Kick um ≤ 2 ms |
| Tempo-Abweichung < 0,1 % und keine Transposition | unveränderte Wiedergabe | bitgenau |

Die Verzögerung des Zeitdehners wird ausgeglichen, indem das Deck entsprechend vorausliest. Die hörbare Position bleibt sample-genau am Beatgrid. Der Wechsel zwischen den Betriebsarten wird über 10 ms überblendet.

### 5.14 Kennlinien von EQ und Filter

- **Isolator-EQ:** Frequenzweiche mit Linkwitz-Riley-Filtern 4. Ordnung (24 dB/Oktave) bei 200 Hz und 2,5 kHz; die Bänder werden einzeln gewichtet und summiert. Reglerweg: Mitte = 0 dB, rechts +6 dB, links stufenlos bis −∞; die unteren 5 % des Wegs sind Kill.
- **Filter-Regler** p von −1 bis +1, Ruhezone |p| < 0,02 (Bypass):
  - Tiefpass (p < 0): Grenzfrequenz = 20 000 Hz · (80 / 20 000)^|p|
  - Hochpass (p > 0): Grenzfrequenz = 20 Hz · (8 000 / 20)^p
  - zustandsvariables Filter 12 dB/Oktave, Güte 0,7–4 (Standard 1,2)

### 5.15 Effektkatalog

P1–P3 sind die drei Regler im Einzel-Modus. Im Gruppen-Modus steuert ein Regler den fett markierten Parameter, die anderen stehen auf dem Standardwert.

| Effekt | P1 | P2 | P3 | Verhalten |
|---|---|---|---|---|
| Reverb | **Größe** 0,3–10 s (2,5 s) | Dämpfung Höhen 1–16 kHz (6 kHz) | Hochpass 20–800 Hz (200 Hz) | Hall mit Ausklang nach Abschalten |
| Delay | **Zeit** 1/16–1 Takt, auch punktiert und triolisch (3/4 Beat) | Feedback 0–0,95 (0,5) | Filter im Rückweg −1…+1 (+0,2) | temposynchron; „Freeze"-Taste hält den Inhalt und schließt den Eingang |
| Filter | **Stellung** −1…+1 | Resonanz 0,7–8 (1,5) | LFO-Tiefe 0–1 (0), Rate 1/8–8 Takte | wie 5.14, zusätzlich temposynchrone Modulation |
| Flanger | **Rate** 1/4–16 Takte (4) | Tiefe 0–1 (0,7) | Feedback −0,9…+0,9 (0,5) | Verzögerung 0,5–10 ms, moduliert |
| Phaser | **Rate** 1/4–16 Takte (4) | Tiefe 0–1 (0,7) | Feedback 0–0,9 (0,5) | 6 Allpass-Stufen |
| Gater | **Rate** 1/32–1/2 Takt (1/8) | Form (Rechteck ↔ weich) 0–1 (0,3) | Tastverhältnis 0,1–0,9 (0,5) | schaltet das Signal im Takt |
| Beat-Repeat | **Länge** 1/32–1 Beat (1/4) | Abklingen 0–1 (0) | Tonhöhe je Wiederholung −12…+12 Halbtöne (0) | zeichnet beim Einschalten auf und wiederholt; arbeitet wie Flux, der Track läuft unhörbar weiter |
| Bitcrusher | **Abtastrate** 44,1 kHz–500 Hz | Bittiefe 16–2 (12) | Glättung 0–1 (0,2) | Reduktion von Rate und Auflösung |
| Echo-Out | **Zeit** 1/4–1 Beat (3/4) | Feedback 0,3–0,9 (0,65) | Ausklang-Hochpass 100–1 000 Hz (300 Hz) | Einschalten schließt den Kanal-Eingang und lässt das Echo ausklingen (Technik T4) |
| Tape-Stop (S) | **Dauer** 1/4–4 Beats (1) | – | – | bremst auf null, Loslassen startet wieder im Takt |
| Ringmodulator (S) | **Frequenz** 20 Hz–4 kHz | Mischung roh/geglättet | – | – |
| Verzerrung (S) | **Antrieb** 0–36 dB | Klangfarbe 0–1 | Ausgang −24…0 dB | weiche Sättigung mit zweifacher Überabtastung |

### 5.16 Stem-Zerlegung

- **Modell:** Hybrid Transformer Demucs (htdemucs, vier Quellen), ausgeführt über ONNX Runtime. Rechenwege: CPU überall; GPU über DirectML (Windows), CoreML (macOS), CUDA (Linux).
- **Ablauf:** Umrechnung auf 44,1 kHz stereo, Verarbeitung in Abschnitten von 7,8 s mit 25 % Überlappung und linearer Überblendung, Rückrechnung auf die Original-Abtastrate.
- **Restsignal:** Original minus Summe der vier Stems wird dem Stem „Melodie/Rest" zugeschlagen. Die Summe aller Stems ergibt damit wieder das Original (STEM-01).
- **Speicherung:** je Stem eine FLAC-Datei mit 16 bit im Cache, zugeordnet über den Inhalts-Hash (ca. 150–250 MB je Track). Option „platzsparend": OGG Vorbis in hoher Qualität (ca. 40 MB je Track).
- **Modell-Datei:** wird beim ersten Gebrauch geladen (ca. 80–300 MB je nach Variante) oder mit dem Installer ausgeliefert.

### 5.17 Open-Source-Bausteine

| Zweck | Baustein | Lizenz | Bemerkung |
|---|---|---|---|
| Dekodierung MP3, AAC, ALAC, FLAC, Vorbis, WAV, AIFF | Symphonia | MPL-2.0 | reines Rust |
| Dekodierung Opus | libopus | BSD | über Bindung |
| Audio-Ein-/Ausgabe | cpal | Apache-2.0 | WASAPI, ASIO, CoreAudio, ALSA, JACK; PipeWire über dessen ALSA-/JACK-Schicht |
| Abtastratenwandlung | rubato | MIT | für das Laden; die Wiedergabe nutzt einen eigenen Sinc-Interpolator |
| Fourier-Transformation | rustfft / realfft | MIT / Apache-2.0 | |
| Zeitdehnung | Signalsmith Stretch | MIT | Alternative: Rubber Band (GPL oder Kauflizenz), SoundTouch (LGPL) |
| Lautheit | ebur128 (Rust) | MIT | |
| Stems | ONNX Runtime + htdemucs | MIT | Lizenz der Modellgewichte vor Auslieferung prüfen (12.1) |
| Datenbank | SQLite über rusqlite | gemeinfrei / MIT | |
| Metadaten | lofty | MIT / Apache-2.0 | |
| XML (NML, Rekordbox) | quick-xml | MIT | |
| MP3-Export | LAME | LGPL | dynamisch gebunden |
| FLAC-Export | flacenc oder libFLAC | Apache-2.0 / BSD | |
| Anwendungsrahmen | Tauri 2 | MIT / Apache-2.0 | |
| Datenaustausch Audio-Thread | rtrb (Ringpuffer ohne Sperren) | MIT / Apache-2.0 | |

Tempo-, Tonart-, Struktur- und Ereignis-Analyse werden selbst umgesetzt. Bausteine mit GPL-Lizenz werden nicht verwendet, damit eine spätere Weitergabe ohne Offenlegung des Quelltexts möglich bleibt (**Annahme**, siehe 12.2).

---

## 6. Datenmodell und Dateiformate

### 6.1 Ablage

| Inhalt | Ort | Format |
|---|---|---|
| Bibliothek | Benutzerdaten-Ordner `deckwerk/library.db` | SQLite |
| Sicherungen | `deckwerk/backups/` | SQLite-Kopien, die letzten 10 |
| Cache (Wellenformen, Stems) | Cache-Ordner `deckwerk/cache/<hash>/` | Binärdateien, FLAC |
| Einstellungen, Tastenbelegung | `deckwerk/settings.json`, `keymaps/*.json` | JSON |
| Mix-Pläne | frei wählbar | `.dwmix` (JSON) |
| Aufnahmen | einstellbarer Ordner | WAV, FLAC |

Die Musikdateien selbst werden nie verändert, außer beim ausdrücklichen Zurückschreiben von Metadaten (BIB-04).

### 6.2 Tabellen der Bibliothek

| Tabelle | Wichtige Felder |
|---|---|
| `track` | id, Pfad, Inhalts-Hash, Dateigröße, Änderungsdatum, Format, Abtastrate, Kanäle, Dauer, Titel, Künstler, Album, Label, Genre, Jahr, Kommentar, Bewertung, Farbe, hinzugefügt am, zuletzt gespielt, Spielzähler, fehlt (ja/nein) |
| `analysis` | track_id, Analyse-Version, gesperrt, BPM, BPM-Sicherheit, Tonart (0–23), Tonart-Sicherheit, Stimmung in Cent, Lautheit LUFS, LRA, True Peak, Energiewert, Quelle (eigene Analyse / Import / Hand) |
| `grid_anchor` | track_id, Position in s, BPM ab hier, Beat-Nummer im Takt (1–4) |
| `segment` | track_id, Typ, Start in s, Ende in s, Start-Takt, Länge in Takten, Quelle |
| `event` | track_id, Typ, Position in s, Stärke, Quelle |
| `cue` | track_id, Nummer (Hotcue 1–8 oder leer), Typ, Position in s, Länge in s (bei Loops), Name, Farbe, Eignungswert, Quelle |
| `energy_curve` | track_id, Werte je Takt (Binärfeld) |
| `playlist` | id, Eltern-Ordner, Name, Typ (Liste / Ordner / intelligent), Regeln (JSON), Position |
| `playlist_item` | playlist_id, track_id, Position |
| `history` | Sitzung, track_id, Deck, Startzeit, Dauer hörbar |
| `stem_set` | track_id, Modell-Version, Format, Speicherort, Größe |
| `sample` | id, Pfad oder Cache-Ort, Typ (Loop / One-Shot), BPM, Tonart, Länge in Beats, Herkunfts-Track |
| `remix_set` | id, Name, 16 Zellen (sample_id, Modus, Lautstärke, Farbe) |
| `fx_preset` | id, Name, Effekt, Parameter (JSON) |
| `transition_template` | id, Name, Basis-Technik, Parameter und Kurven (JSON) |

„Quelle" unterscheidet überall: eigene Analyse, Import, von Hand. Rangfolge bei Konflikten: von Hand > Import > eigene Analyse.

### 6.3 Mix-Plan `.dwmix`

JSON-Dokument mit Versionsnummer:

- **Kopf:** Name, Zieldauer, Energie-Sollkurve (Stützpunkte Zeit/Wert), Gewichte der Bewertung, Zufalls-Startwert.
- **Tracks:** Liste in Reihenfolge; je Eintrag Inhalts-Hash, Pfad, Deck, Einstiegs- und Ausstiegsposition, Transposition, fixiert (ja/nein).
- **Übergänge:** je Nachbar-Paar Technik, Parameter (L, S, Varianten), Startposition in A und B, Tempo-Verlauf, Teilbewertungen K, T, E, U mit Begründungstext, „von Hand geändert" (ja/nein).
- **Automation:** je Übergang und Regler eine Liste von Stützpunkten (Master-Beat relativ zu T0, Wert, Kurvenform linear / weich / Stufe). Regler werden über dieselben Namen angesprochen wie in der Befehlsschicht (7.4).

Ein Plan verweist auf Tracks über Hash und Pfad und lässt sich deshalb auf einem anderen Rechner mit derselben Musik öffnen.

### 6.4 Import und Export

| Format | Richtung | Inhalt | Besonderheit |
|---|---|---|---|
| Traktor `collection.nml` (XML) | Import | Tracks, Playlists, Cue-Punkte (Typen Cue, Fade-In, Fade-Out, Load, Grid, Loop), BPM, Tonart | Cue-Positionen in Millisekunden; bei MP3 Versatz durch unterschiedliche Decoder-Verzögerung ausgleichen (bekannt: rund 26 ms je nach Datei) |
| Rekordbox-XML | Import, Export (K) | Tracks, Playlists, Positionsmarken (Hotcues, Memory-Cues, Loops), Tempo-Abschnitte | Tonart als Text in mehreren Schreibweisen |
| `.stem.mp4` | Import (S) | vier Stems plus Master in einem MP4-Container | |
| `.cue` | Export | Trackliste mit Zeitstempeln zum Mix | |
| WAV, FLAC, MP3 | Export | Mix und Aufnahme | Kapitelmarken, wo das Format sie kennt |

---

## 7. Architektur und Technologie

### 7.1 Empfehlung

**Kern in Rust, Oberfläche als Web-Technik in einer nativen Hülle (Tauri 2), Wellenformen in WebGL 2.**

| Schicht | Technik | Begründung |
|---|---|---|
| Audio-Engine, Analyse, Bibliothek, Auto-Mix | Rust | Echtzeit-tauglich ohne Speicherbereinigung, speichersicher, ein Quelltext für alle drei Systeme, gute Audio-Bibliotheken |
| Anwendungshülle | Tauri 2 | kleine Installer, nutzt die Web-Ansicht des Systems, Rust-Kern läuft im selben Prozess |
| Oberfläche | TypeScript, React, Vite | schnelle Entwicklung komplexer Layouts; die Oberfläche lässt sich im Browser mit simuliertem Kern starten und automatisch per Screenshot prüfen |
| Wellenformen, Pegel, Zeitleiste | WebGL 2 und Canvas, außerhalb des React-Renderings | 60 fps unabhängig von der übrigen Oberfläche |

Der Ton läuft vollständig im Rust-Kern in einem eigenen Echtzeit-Thread. Die Oberfläche zeigt nur an und sendet Befehle; ruckelt sie, bleibt der Ton unberührt.

**Alternativen**

| Alternative | Vorteil | Nachteil |
|---|---|---|
| C++ mit JUCE | Standard für Audio-Software, sehr ausgereift | Lizenz GPL oder kostenpflichtig; Oberfläche aufwendiger; Speicherfehler-Risiko |
| C++ mit Qt/QML | ausgereifte native Oberfläche | LGPL-Auflagen, großer Lieferumfang, Audio-Teil komplett selbst |
| Electron | sehr verbreitet | hoher Speicherbedarf, Audio-Kern als nativer Zusatz umständlicher |
| Oberfläche rein in Rust (egui, iced) | ein Sprachraum | komplexe Layouts und automatische Sichtprüfung deutlich mühsamer |

### 7.2 Module

```
deckwerk/
  crates/
    dw-core       Grundtypen: Zeit, Beats, Tonart, Kennungen, Befehle
    dw-decode     Dateien dekodieren, Abtastrate wandeln
    dw-dsp        Filter, EQ, Limiter, Interpolation, Zeitdehnung, Effekte
    dw-analysis   Tempo, Beatgrid, Tonart, Lautheit, Struktur, Ereignisse, Cues
    dw-engine     Decks, Mixer, Clock, Sync, Automation, Offline-Render
    dw-audio      Geräte, Ausgabe (cpal)
    dw-library    Datenbank, Suche, Import/Export
    dw-automix    Bewertung, Planung, Übergangsvorlagen
    dw-stems      Stem-Zerlegung
    dw-cli        Kommandozeile für Analyse, Render und Tests
  app/            Tauri-Hülle und Befehls-Anbindung
  ui/             Oberfläche (TypeScript)
  testdaten/      Referenzsammlung (nicht im Repository, außer synthetische Tracks)
```

`dw-cli` ist kein Nebenprodukt: Alle Kernfunktionen sind ohne Oberfläche ausführbar (`deckwerk analyze`, `deckwerk render plan.dwmix`). Darauf stützen sich die automatischen Tests.

### 7.3 Threads

| Thread | Aufgabe | Regeln |
|---|---|---|
| Audio | berechnet je Puffer alle Decks, Mixer, Effekte | keine Speicheranforderung, keine Sperren, keine Datei- oder Systemaufrufe (ENG-06) |
| Steuerung | nimmt Befehle an, bereitet Tracks vor, übergibt fertige Objekte an den Audio-Thread | Übergabe über Ringpuffer ohne Sperren |
| Lader | dekodiert Tracks in den Speicher | ein Track liegt vollständig als 32-bit-Gleitkomma im RAM (ca. 160 MB für 7 Minuten bei 48 kHz) – **Annahme**, passend zum Referenzrechner |
| Analyse | Warteschlange mit niedriger Priorität, Anzahl Kerne einstellbar | gibt bei laufender Wiedergabe Rechenzeit frei |
| Oberfläche | Darstellung und Eingabe | erhält 60-mal je Sekunde einen Zustands-Schnappschuss |

### 7.4 Befehlsschicht

Jede Bedienhandlung ist ein Befehl mit Ziel, Aktion und Wert, zum Beispiel `deck.a.play`, `deck.b.loop.set(4)`, `mixer.b.eq.low = -inf`, `fx.1.drywet = 0.4`, `master.tempo = 135.0`.

- Maus, Tastatur, Automation, Autopilot und ein späteres MIDI-Mapping erzeugen dieselben Befehle.
- Befehle tragen wahlweise einen Zeitpunkt in Master-Beats. Der Audio-Thread führt sie sample-genau aus. So entstehen quantisierte Aktionen und Automationskurven.
- Jeder stetige Parameter hat Name, Wertebereich, Standardwert, Einheit und Glättungszeit. Diese Liste ist die einzige Quelle für Oberfläche, Tastenbelegung, Automation und `.dwmix`.
- Der Zustand fließt als Schnappschuss zurück: Positionen, Tempi, Phasen, Pegel, Reglerstellungen, Last.

### 7.5 Signalfluss

```
Deck ─ Tempo/Tonhöhe ─ Gain ─ EQ ─ Filter ─ Insert-Effekte ─┬─ Fader ─ Crossfader ─┬─ Master-Summe
                                                            │                      └─ Effekt-Sends
                                                            └─ Cue-Abgriff (vor Fader) ─ Cue-Summe

Master-Summe ─ Master-Effekte ─ Master-Lautstärke ─ Limiter ─┬─ Master-Ausgang
                                                             ├─ Aufnahme
                                                             └─ Kopfhörer-Mischer ← Cue-Summe ─ Kopfhörer-Ausgang
```

Stem-Decks haben vor dem Gain vier parallele Zweige (Lautstärke, Filter, Send je Stem). Remix-Decks summieren ihre vier Spuren an derselben Stelle.

### 7.6 Clock und Sync

- Die Master-Clock zählt Beats als 64-bit-Gleitkommazahl und hat ein Tempo in BPM.
- Ein synchronisiertes Deck erhält seine Abspielrate aus Master-Tempo ÷ Track-Tempo an der aktuellen Stelle. Ein Regler gleicht den Phasenversatz aus: Abweichungen unter 1/8 Beat werden innerhalb von höchstens 2 Beats durch eine Ratenänderung von höchstens ±1 % unhörbar korrigiert; größere Abweichungen nur auf Befehl (Sync-Taste) durch einen Sprung.
- Modus „Tempo": Nur das Tempo wird angeglichen, die Phase schiebe ich selbst. Modus „Beat": Tempo und Phase.
- Ist kein Deck Master, läuft die freie Clock weiter. Effekte, Remix-Decks und Automation richten sich immer nach der Master-Clock.

### 7.7 Offline-Render

Dieselbe Engine wird ohne Audiogerät in festen Blöcken von 256 Samples aufgerufen. Geladen wird vorab statt nebenläufig, Befehle kommen aus dem Mix-Plan mit Beat-Zeitstempeln. Das Ergebnis ist deterministisch (ENG-07) und dient zugleich als Messgrundlage für die Tests in Kapitel 10.

### 7.8 Fehlerverhalten

| Fall | Verhalten |
|---|---|
| Datei nicht lesbar oder beschädigt | Track wird markiert, Meldung in der Liste, kein Abbruch des Imports |
| Audiogerät fällt aus | Meldung, automatischer Wiederverbindungsversuch alle 2 s, Decks bleiben geladen (ENG-08) |
| Überlast (Xruns) | Zähler und Warnung; Analyse und Stem-Berechnung pausieren automatisch |
| Absturz | Protokolldatei; beim nächsten Start Angebot, Decks und Positionen wiederherzustellen (BED-09); laufende Aufnahme bleibt lesbar (REC-01) |
| Datenbank beschädigt | Start aus der letzten Sicherung (BIB-18) |
| Speicher knapp | Laden eines weiteren Tracks wird mit Hinweis abgelehnt, statt Aussetzer zu riskieren |

---

## 8. Oberfläche

### 8.1 Gestaltungsrichtung

- Dunkle Grundfläche, wenige kräftige Signalfarben, die ausschließlich Bedeutung tragen: Deck-Farben, Frequenzbänder, Tonart-Verträglichkeit, Warnungen.
- Die Wellenformen sind das größte Element. Alles Übrige ordnet sich unter.
- Zahlen in einer Schrift mit festen Ziffernbreiten, damit BPM und Zeiten nicht springen.
- Jedes Deck hat eine eigene Farbe (A–D), die in Wellenform, Mixer-Kanal, Zeitleiste und Bibliothek wiederkehrt.
- Kein Nachbau bekannter DJ-Programme: eigene Anordnung, eigene Formen, eigener Name.

Farben, Schriften, Abstände und Komponenten werden vor dem ersten Oberflächen-Code als Designsystem festgelegt (Stufe 0) und sind danach die einzige Quelle für alle Ansichten.

### 8.2 Ansicht „Mixen" (Standard)

```
┌──────────────────────────────────────────────────────────────────────────────┐
│ Kopfzeile: Ansichten │ Master-Clock BPM │ Q S │ Aufnahme │ Last │ Uhr │ ⚙    │
├──────────────────────────────────────────────────────────────────────────────┤
│ Parallele Wellenformen (alle laufenden Decks übereinander, Abspielmarke      │
│ in der Mitte, Beat- und Phrasenlinien, Abschnittsbänder)                     │
├───────────────────────────┬──────────────────┬───────────────────────────────┤
│ Deck A                    │ Mixer            │ Deck B                        │
│ Titel · Künstler · Cover  │ Gain             │ (gespiegelt)                  │
│ BPM · Tonart · Zeit       │ EQ H/M/T         │                               │
│ Übersichts-Wellenform     │ Filter           │                               │
│ Transport · Sync · Tempo  │ Cue · Pegel      │                               │
│ Hotcues 1–8               │ Fader            │                               │
│ Loop · Beatjump · Flux    │ Crossfader       │                               │
├───────────────────────────┴──────────────────┴───────────────────────────────┤
│ Effekteinheiten 1–4 (einklappbar)                                            │
├──────────────────────────────────────────────────────────────────────────────┤
│ Bibliothek: Playlists links │ Trackliste mit Verträglichkeit │ Vorhören      │
└──────────────────────────────────────────────────────────────────────────────┘
```

- Im 4-Deck-Layout stehen C und D unter A und B; der Mixer hat vier Kanäle.
- Die Bibliothek lässt sich auf volle Höhe aufziehen; dann schrumpfen die Decks auf eine Zeile mit Wellenform, BPM, Tonart und Restzeit.
- Jedes Deck zeigt dauerhaft die Entfernung zum nächsten Abschnitt („Drop in 16 Takten").

### 8.3 Ansicht „Bibliothek"

Trackliste mit Spalten: Cover, Titel, Künstler, BPM, Tonart (farbig), Energie, Länge, Bewertung, Verträglichkeit zum Master, Mini-Wellenform, Kommentar, hinzugefügt am. Links der Playlist-Baum, oben Suche und Filterleiste (BPM-Bereich als Schieber, Tonart als Camelot-Rad zum Anklicken, Energie, „passt zum laufenden Track"). Rechts ein Detailbereich mit Cover, großer Wellenform, Abschnitten und Cue-Liste. Unten der Analyse-Fortschritt.

### 8.4 Ansicht „Vorbereiten"

Ein Track in voller Breite: große Wellenform mit Zoom, darüber Abschnittsbänder und Ereignis-Marker zum Ziehen, darunter das Beatgrid mit Griffen an den Ankern. Seitlich: BPM-Feld mit ×2, ÷2 und Tap, Tonart-Auswahl mit Vorhör-Ton, Cue-Liste zum Bearbeiten, Schloss, „neu analysieren". Wiedergabe mit Metronom-Klick zur Kontrolle des Rasters.

### 8.5 Ansicht „Auto-Mix"

```
┌──────────────────────────────────────────────────────────────────────────────┐
│ Plan: Name │ Zieldauer │ Planen │ Vorhören │ Rendern │ Autopilot starten     │
├──────────────────────────────────────────────────────────────────────────────┤
│ Energie-Sollkurve (zum Zeichnen) mit Ist-Verlauf darüber                     │
├──────────────────────────────────────────────────────────────────────────────┤
│ Spur 1: ▓▓▓ Track 1 ▓▓▓▓▓▓╲                ╱▓▓▓ Track 3 ▓▓▓▓▓               │
│ Spur 2:                    ╲▓▓▓ Track 2 ▓▓╱                                  │
│          (Wellenformen, Abschnitte, Phrasenraster; Übergänge als Überlappung)│
├──────────────────────────────────────────────────────────────────────────────┤
│ Automationsspuren des gewählten Übergangs: Fader, EQ, Filter, Effekt, Tempo  │
├──────────────────────────────────────────────────────────────────────────────┤
│ Detail: Technik │ Länge │ Tauschpunkt │ Bewertung K/T/E/U mit Begründung     │
└──────────────────────────────────────────────────────────────────────────────┘
```

Ziehen eines Tracks rastet im Phrasenraster. Die Übergangszone zeigt beide Wellenformen übereinander wie im Live-Modus. Verstöße gegen Bass-Regel oder Phrasenraster werden rot markiert.

### 8.6 Weitere Ansichten

- **Stem-Deck:** vier gestapelte Wellenformen mit je Lautstärke, Stumm, Filter, Send.
- **Remix-Deck:** Raster 4 × 4 mit Farbfeldern, Fortschrittsanzeige je Zelle, Spur-Regler darunter.
- **Einstellungen:** Audio (Gerät, Kanäle, Abtastrate, Puffer, Latenz-Anzeige, Testton), Analyse (BPM-Bereich, Kerne, automatisch beim Import), Mixer (EQ-Typ, Kurven, Referenz-Lautheit), Tastatur (Belegungs-Editor mit „Taste drücken zum Zuweisen"), Bibliothek (Ordner, Import, Sicherung, Cache), Darstellung (Design, Skalierung, Sprache), Aufnahme.

### 8.7 Standard-Tastenbelegung

Tasten sind als Positionen auf dem US-Layout benannt und an die physische Position gebunden (BED-03). Auf einer deutschen Tastatur liegt z. B. „Z" dort, wo „Y" aufgedruckt ist.

| Funktion | Deck A | Deck B |
|---|---|---|
| Play/Pause | Q | U |
| Cue | W | I |
| Sync | E | O |
| Abbremsen / Anschieben | A / S | J / K |
| Tempo − / + (fein mit Shift) | D / F | L / ; |
| Hotcue 1–4 (Shift = setzen, Alt = löschen) | 1 2 3 4 | 7 8 9 0 |
| Loop an/aus | C | , |
| Loop halbieren / verdoppeln | Z / X | N / M |
| Beatjump zurück / vor | V / B | . / / |
| Flux | R | P |
| Kopfhörer-Cue | T | Y |
| Laden aus der Liste | Shift + ← | Shift + → |

| Funktion | Taste |
|---|---|
| Crossfader links / rechts (Alt = ganz) | ← / → |
| Kanalfader A auf / zu, B auf / zu | Alt + D/F, Alt + L/; |
| Liste hoch / runter | ↑ / ↓ |
| Suche | Strg/Cmd + F |
| Ansichten Mixen, Bibliothek, Vorbereiten, Auto-Mix | F1–F4 |
| Decks C/D statt A/B steuern (Umschalt-Ebene) | Caps Lock |
| Tief-Kill Deck A / B | G / H |
| Filter A zu / auf, B zu / auf | Strg + A/S, Strg + J/K |
| Aufnahme | Strg/Cmd + R |
| Quantisierung, Snap | F9, F10 |
| Rückgängig / Wiederholen | Strg/Cmd + Z / Shift + Strg/Cmd + Z |
| Leertaste | Play/Pause des Decks mit Fokus (das zuletzt bediente Deck, sichtbar markiert) bzw. Vorhören in Bibliothek und Auto-Mix |
| Fokus eines Reglers lösen | Esc |

---

## 9. Nicht-funktionale Anforderungen

### 9.1 Leistung (Referenzrechner, 48 kHz, 256 Samples)

| Messgröße | Zielwert |
|---|---|
| Ausgabelatenz | ≤ 10 ms (ASIO, CoreAudio, JACK); ≤ 25 ms (WASAPI gemeinsam, ALSA Standard) |
| DSP-Last mit 4 Decks, Key Lock an, 4 Effekteinheiten | ≤ 40 % der Pufferzeit im Mittel, ≤ 70 % in der Spitze |
| Aussetzer | 0 in 4 Stunden Dauerbetrieb |
| Bildrate | 60 fps mit 4 Decks; nie unter 30 fps |
| Track laden bis spielbereit | ≤ 2 s (7 Minuten, MP3 320) |
| Analyse ohne Stems | ≤ 15 s je 7-Minuten-Track |
| Stem-Zerlegung | ≤ 7 min CPU, ≤ 1 min GPU je 7-Minuten-Track |
| Offline-Render | ≥ 10-fache Echtzeit |
| Programmstart mit 20 000 Tracks | ≤ 3 s bis zur bedienbaren Oberfläche |
| Suche und Sortierung bei 20 000 Tracks | ≤ 100 ms |
| Arbeitsspeicher mit 4 geladenen Tracks | ≤ 1,5 GB |

### 9.2 Zuverlässigkeit und Datensicherheit

- Musikdateien werden nur gelesen (Ausnahme BIB-04, mit Sicherheitskopie der Datei-Tags).
- Jede Änderung an der Bibliothek ist eine abgeschlossene Datenbank-Transaktion.
- Tägliche automatische Sicherung, Wiederherstellung per Dialog.
- Kein Netzwerkzugriff im Betrieb, außer dem einmaligen Laden des Stem-Modells und einer abschaltbaren Update-Prüfung. Keine Nutzungsdaten.

### 9.3 Bedienbarkeit

- Erster Start: Auswahl des Audiogeräts mit Testton, Auswahl des Musikordners, danach direkt die Ansicht „Mixen".
- Alle Bedienelemente haben Kurzhilfen mit Name, aktuellem Wert und Tastenkürzel.
- Gefährliche Aktionen im Live-Betrieb sind abgesichert, die Absicherung ist abschaltbar. Als gefährlich gilt, was ein hörbares Deck unterbricht (Fader offen und Crossfader nicht abgewandt): Stoppen, Cue, Sprung in der Übersicht, Neuladen. Solche Aktionen brauchen ein Halten der Taste (ca. 400 ms) oder einen zweiten Druck innerhalb 1 s; der Sprung in der Übersicht braucht Shift. Ein unhörbares Deck wird ohne Hürde bedient.
- In ein geladenes, stehendes Deck lädt nur der gezielte Befehl (Lade-Taste, Shift + Pfeil, Ablegen); der Doppelklick in der Liste lädt nur in ein leeres Deck.
- Doppelklick = Standardwert gilt nicht für Kanalfader und Master-Lautstärke: Ein geschlossener Kanal öffnet sich nie mit einem Griff.
- Bedienziele im Live-Betrieb sind mindestens 28 px hoch, auch bei 1280×720.
- Kontraste nach VIS-11; keine Information nur über Farbe (VIS-12).

### 9.4 Wartbarkeit

- Kernfunktionen sind ohne Oberfläche testbar (7.2, `dw-cli`).
- Analyse-Version je Track (ANA-17), Versionsnummern in Datenbank-Schema und `.dwmix` mit automatischer Migration.
- Automatischer Bau für Windows, macOS und Linux bei jedem Stand.

---

## 10. Teststrategie

### 10.1 Referenzsammlung

| Teil | Inhalt | Zweck | Im Repository |
|---|---|---|---|
| A – synthetisch | Vom Testgenerator erzeugte Tracks mit exakt bekanntem Tempo, Beatgrid, Tonart, Struktur und Ereignissen (Kick, Bass, Hi-Hat, Akkorde, Breakdown, Riser, Roll); dazu Messsignale | bitgenaue Grundwahrheit, läuft in jedem Testlauf | ja (Generator, nicht die Audiodaten) |
| B – frei lizenziert | 30–50 Techno-Tracks unter freier Lizenz, von Hand annotiert | realistische Klänge | Verweise und Annotationen |
| C – privat | 100+ eigene Tracks mit Annotationen (BPM, Tonart, Abschnitte, Mix-Punkte) | Abnahme der Zielwerte | nein, nur Annotationen lokal |

Die Abnahmekriterien ANA-02 bis ANA-13 gelten für Teil B + C. Teil A muss zu 100 % bestanden werden und dient als Rückschritts-Schutz.

### 10.2 Testarten

| Art | Gegenstand | Beispiel |
|---|---|---|
| Einzeltests | Bausteine der Signalverarbeitung | EQ in Mittelstellung ist neutral; Limiter hält die Decke; Loop-Rücksprung ohne Sprungstelle |
| Messtests gegen Referenz | Filter, Effekte, Zeitdehnung | Frequenzgang und Impulsantwort gegen gespeicherte Referenz (Toleranz je Test) |
| Analyse-Tests | Tempo, Raster, Tonart, Struktur, Ereignisse | Trefferquoten je Kennzahl, Bericht als Tabelle, Vergleich mit dem vorherigen Stand |
| Engine-Tests per Offline-Render | Sync, Quantisierung, Flux, Automation | zwei synchronisierte Klick-Tracks: Abstand der Klicks ≤ 1 ms über 10 min |
| Übergangs-Tests | alle Techniken aus 5.10 | Reglerverläufe entsprechen der Vorlage; Bass-Regel; Phrasenraster |
| Auto-Mix-Tests | Planung und Render | festes Test-Set: reproduzierbarer Plan, alle Regeln erfüllt, Zieldauer ±2 % |
| Echtzeit-Tests | Audio-Thread | keine Speicheranforderung im Audio-Pfad; Lastmessung mit 4 Decks |
| Oberflächen-Tests | Ansichten und Abläufe | Oberfläche im Browser mit simuliertem Kern; Screenshots aller Ansichten in definierten Zuständen und vier Auflösungen; Vergleich mit freigegebenem Stand |
| Import-Tests | NML, Rekordbox-XML | Testsammlungen mit bekannten Inhalten |
| Dauertest | ganze Anwendung | 4 Stunden Autopilot ohne Aussetzer und ohne Speicherzuwachs |

### 10.3 Abnahme am Gerät

Nicht automatisch prüfbar und deshalb je Stufe von Hand abzunehmen: Klangqualität von EQ, Filter, Effekten und Zeitdehnung; Latenz und Stabilität am echten Audio-Interface; Kopfhörer-Vorhören; Bediengefühl mit Tastatur; musikalische Qualität der automatischen Übergänge. Zu jeder Stufe gibt es dafür eine Abnahmeliste mit genauen Schritten.

---

## 11. Umsetzungsplan

Jede Stufe endet mit einer startbaren, für sich nutzbaren Anwendung, grünen Tests und Installern für alle drei Systeme.

| Stufe | Inhalt | Anforderungen | Danach kann ich … |
|---|---|---|---|
| 0 – Fundament | Projektgerüst, Befehlsschicht, Offline-Engine-Rahmen, automatischer Bau, Testgenerator, Designsystem, leere Anwendungshülle | VIS-10–12, BED-07, ENG-04, ENG-07 | die Anwendung starten und das Design beurteilen |
| 1 – Kern-Mixing | Dekodierung aller Formate, Bibliothek mit Suche und Playlists, Analyse von Tempo, Beatgrid, Tonart, Lautheit, Wellenform; zwei Decks, Mixer mit EQ, Filter, Fader, Crossfader, Limiter; Sync; Kopfhörer-Cue; Wellenformen | BIB-01–03, 05, 06, 08, 09, 18; ANA-01–04, 06–08, 16; DECK-01, 04–09, 18, 20; MIX-01, 02, 04–10, 13; VIS-01–07; ENG-01–07, 09; BED-05, 08 | zwei Tracks von Hand sauber ineinander mixen |
| 2 – Decks vollständig | vier Decks, Key Lock, Transposition, Hotcues, Loops, Beatjump, Flux, Rückwärts, Quantisierung, Tastenbelegung mit Editor, Mitschnitt, Vorhören aus der Liste | DECK-02, 10–17, 19, 21, 22; BIB-12, 17; REC-01; ENG-08, 10; BED-02–04, 09 | ein komplettes Set live spielen und aufnehmen |
| 3 – Struktur und Vorbereitung | Abschnitte, Phrasen, Energie, Ereignisse, Cue-Vorschläge, Ansicht „Vorbereiten", Verträglichkeit in der Bibliothek, intelligente Playlists | ANA-05, 09–15, 17; BIB-04, 07, 10, 11; BED-06 | sehen, wo ein Track gemixt werden will, und passende Tracks finden |
| 4 – Effekte | Effekteinheiten, Effektkatalog, Mixer-Effekte, weitere EQ-Typen, Spektrum | FX-01–07; MIX-03, 11, 12; VIS-07 | mit Effekten arbeiten, Echo-Out spielen |
| 5 – Auto-Mix | Übergangskatalog, Bewertung, Planung, Zeitleisten-Editor, Offline-Render, Export, Autopilot | TRANS-01–05; AUTO-01–11; REC-02–05; VIS-09 | aus einer Playlist einen fertigen Mix erzeugen |
| 6 – Stems | Zerlegung, Stem-Decks, Stem-Übergang, Stem-Import | STEM-01–07; DECK-03; VIS-08 | einzelne Spuren mixen, Bässe sauber tauschen |
| 7 – Remix-Decks | Zellen-Raster, Sample-Aufnahme, Sample-Bibliothek, Sets | RMX-01–07 | Loops und Samples einwerfen |
| 8 – Import und Auslieferung | Traktor- und Rekordbox-Import, fehlende Dateien, signierte Installer, Handbuch, Dauertest | BIB-13–16 | meine bestehende Sammlung übernehmen und das Programm weitergeben |

---

## 12. Risiken und offene Fragen

### 12.1 Risiken

| Risiko | Auswirkung | Gegenmaßnahme |
|---|---|---|
| Tonart-Erkennung erreicht bei Techno nur rund 70 % | falsche Verträglichkeits-Hinweise, unpassende Auto-Übergänge | Sicherheitswert einbeziehen, Handkorrektur einfach machen, importierte Tonarten bevorzugen, perkussive Tracks neutral bewerten |
| Struktur-Erkennung bei untypischen Tracks (kein klarer Breakdown, ungerade Phrasen) | Cue-Vorschläge neben der Phrase | Rasterversatz erkennen, Eignungswerte anzeigen, Handkorrektur in „Vorbereiten" |
| Klang der Zeitdehnung bei Kicks | weiche Anschläge bei Key Lock | Bypass bei < 0,1 % Abweichung, Messziel aus 5.13, bei Bedarf Wechsel des Bausteins (Schnittstelle bleibt gleich) |
| Rechenzeit und Speicherbedarf der Stems | lange Wartezeiten ohne GPU, große Caches | Hintergrund-Warteschlange, Zerlegung nur auf Wunsch je Track oder Playlist, Cache-Grenze, platzsparendes Format |
| Lizenz der Stem-Modellgewichte | Einschränkung bei kommerzieller Weitergabe | vor Stufe 6 rechtlich prüfen, Modell austauschbar halten |
| ASIO-Unterstützung | Steinberg-SDK hat eigene Lizenzbedingungen | Bedingungen vor Stufe 1 prüfen; WASAPI exklusiv als Rückfallebene |
| Web-Ansicht unter Linux (WebKitGTK) langsamer bei WebGL | Bildrate unter Ziel | früh messen (Stufe 0); Rückfall: Wellenformen im Kern rendern und als Bild übergeben |
| Latenz zwischen Ton und Bild | Wellenform wirkt versetzt | Ausgabelatenz des Geräts in die Anzeige einrechnen, Abgleich-Regler in den Einstellungen |
| Versatz importierter Cue-Punkte bei MP3 | Cues liegen ca. 26 ms daneben | Encoder-Verzögerung auswerten, importierte Grid-Marker gegen eigene Analyse prüfen und angleichen |
| Musikalische Qualität der Auto-Übergänge ist nicht messbar | Regeln erfüllt, klingt trotzdem unpassend | Hörabnahme je Stufe, Bewertungsgewichte einstellbar, eigene Vorlagen (TRANS-04) |
| Umfang | lange Bauzeit | Stufen sind einzeln nutzbar; Stufen 6–8 sind unabhängig voneinander und verschiebbar |

### 12.2 Offene Fragen an dich

| Nr. | Frage | Bis dahin gilt die Annahme |
|---|---|---|
| F1 | Wie soll das Programm heißen? | Arbeitstitel „Deckwerk" |
| F2 | Nur für dich oder zur Weitergabe bzw. zum Verkauf? Das entscheidet, ob Bausteine mit GPL-Lizenz erlaubt sind. | Weitergabe soll möglich bleiben, keine GPL-Bausteine |
| F3 | Auf welchem System arbeitest du hauptsächlich und mit welchem Audio-Interface? | alle drei Systeme gleichrangig; Abnahme am Gerät zuerst auf deinem System |
| F4 | Hat dein Rechner eine Grafikkarte, die für die Stem-Berechnung nutzbar ist? | CPU-Berechnung muss ausreichen |
| F5 | Wie groß ist deine Sammlung? | bis 20 000 Tracks |
| F6 | Welche BPM-Spanne spielst du? | 110–170 BPM, einstellbar |
| F7 | Reichen Deutsch und Englisch als Sprachen? | ja |
| F8 | Sollen Mikrofon- oder Line-Eingänge (z. B. für externe Geräte) unterstützt werden? | nein |
| F9 | Kannst du für die Referenzsammlung 100 eigene Tracks mit bekannten BPM- und Tonart-Werten bereitstellen (z. B. aus deiner Traktor-Sammlung)? | bis dahin nur synthetische und frei lizenzierte Tracks |
