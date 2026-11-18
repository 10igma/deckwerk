# Auftrag: Spezifikation für eine DJ-Mixing-Software mit Schwerpunkt Techno

## Deine Aufgabe

Schreibe eine vollständige fachliche und technische Spezifikation für die unten beschriebene Software. Die Spezifikation soll so präzise sein, dass ein Entwicklerteam (oder ein KI-Coding-Agent) sie ohne Rückfragen Schritt für Schritt umsetzen kann. Schreibe keinen Programmcode, sondern das Dokument, nach dem später programmiert wird.

Wenn dir für eine Entscheidung Informationen fehlen, stelle mir zuerst gezielte Fragen. Wo du trotzdem eine Annahme triffst, kennzeichne sie sichtbar als „Annahme“, damit ich sie prüfen kann.

## Das Produkt in einem Absatz

Eine Desktop-Anwendung (Windows, macOS, Linux), mit der ich Techno-Tracks so mixen kann, wie es ein DJ im Club macht. Sie hat zwei Betriebsarten, die auf derselben Audio-Engine und denselben Analysedaten aufbauen:

1. **Live-Modus:** Ich mixe selbst in Echtzeit an virtuellen Decks mit Mixer, Effekten, Loops und Cue-Punkten.
2. **Auto-Mix-Modus:** Ich gebe eine Trackauswahl vor, die Software plant Reihenfolge, Einstiegs- und Ausstiegspunkte und Übergänge selbst, zeigt mir den Plan zum Bearbeiten und rendert daraus einen fertigen Mix als Audiodatei.

Funktional orientiert sich das Programm am Umfang professioneller DJ-Software wie Native Instruments Traktor Pro. Das ist eine Referenz für den Funktionsumfang, keine Vorlage zum Kopieren: Name, Oberfläche, Layout und Gestaltung sollen eigenständig sein.

## Feste Rahmenbedingungen

- **Plattform:** native Desktop-App, lokal und offline nutzbar, Musik liegt als Dateien auf der Festplatte.
- **Bedienung:** vollständig per Maus und Tastatur, mit frei belegbaren Tastenkürzeln. MIDI-Controller und Timecode-Vinyl gehören nicht zur ersten Version; die Architektur soll eine spätere MIDI-Anbindung aber nicht verbauen.
- **Audio-Ausgabe:** Mehrkanal-Audio-Interface mit getrenntem Master- und Kopfhörer-Ausgang zum Vorhören (Cue), dazu ein Fallback für Geräte mit nur einem Stereo-Ausgang.
- **Genre-Fokus:** Techno und verwandte Stile (ca. 120–160 BPM, durchgehende Kick im 4/4-Takt, lange Intros und Outros, Phrasen von 16 oder 32 Takten). Analyse und Übergangslogik dürfen auf diese Eigenschaften optimiert sein.
- **Dateiformate:** mindestens MP3, WAV, AIFF, FLAC, AAC/M4A, ALAC, OGG Vorbis und Opus, in gängigen Abtastraten und Bittiefen.

## Funktionsbereiche, die die Spezifikation abdecken muss

### 1. Musikbibliothek
Import von Dateien und Ordnern, Auslesen von Metadaten und Covern, Playlists, Suche, Sortieren und Filtern nach BPM, Tonart, Energie, Genre und Bewertung. Analyseergebnisse werden dauerhaft gespeichert. Import bestehender Bibliotheken aus Traktor (collection.nml) und Rekordbox (XML) inklusive Cue-Punkten, Beatgrids, Loops und Playlists.

### 2. Track-Analyse
Jeder Track wird beim Import automatisch analysiert. Beschreibe für jede Analyse das Verfahren, Ein- und Ausgaben, die erwartete Genauigkeit und wie ich das Ergebnis von Hand korrigieren kann.

- **Tempo und Beatgrid:** BPM auf zwei Nachkommastellen, Position jedes Beats, Erkennung des ersten Schlags im Takt (Downbeat), Schutz vor Halb- und Doppeltempo-Fehlern, Umgang mit leichten Temposchwankungen.
- **Tonart:** Erkennung der Tonart mit Anzeige in musikalischer Schreibweise sowie als Camelot- und Open-Key-Code, plus Angabe, wie sicher die Erkennung ist.
- **Lautheit:** integrierte Lautheit (LUFS) und Spitzenpegel für automatische Pegelanpassung.
- **Struktur:** Gliederung in Abschnitte wie Intro, Aufbau, Breakdown, Drop, Hauptteil und Outro sowie Phrasengrenzen im 16-/32-Takt-Raster.
- **Energieverlauf:** Kurve über die Tracklänge und ein Gesamt-Energiewert pro Track.
- **Ereignisse im Track:** Stellen, an denen Kick oder Bass einsetzen oder wegfallen (Veränderungen im Tiefenbereich), an denen Hi-Hats und Höhen dazukommen oder wegfallen (Veränderungen im Höhenbereich), Verdopplungen der Kick und Snare-/Kick-Rolls vor einem Drop, Riser, Breaks und Stille.
- **Automatische Cue-Punkte:** Aus Struktur und Ereignissen werden Vorschläge für Einstiegspunkte (Mix-In), Ausstiegspunkte (Mix-Out), Drop und Breakdown abgeleitet, jeweils auf Phrasengrenzen eingerastet und mit einer Bewertung, wie gut sich die Stelle zum Mixen eignet.

### 3. Decks
Vier Decks, je Deck umschaltbar zwischen Track-, Stem- und Remix-Deck. Wiedergabe, Cue, Tempo-Regler mit wählbarem Bereich, Tonhöhenerhalt bei Tempoänderung (Key Lock), Transponieren in Halbtönen, Tempo- und Phasen-Sync mit Master-Clock, Hotcues, manuelle und automatische Loops mit Längen von 1/32 bis 32 Beats, Loop verschieben, Beatjump, Slip-/Flux-Modus, Rückwärtslauf, Quantisierung, Anschieben und Abbremsen per Tastatur.

### 4. Mixer
Pro Kanal Gain mit Auto-Gain, Equalizer mit drei oder vier Bändern und vollständiger Auslöschung (Kill), ein kombinierter Hoch-/Tiefpassfilter auf einem Regler, Kanalfader, Pegelanzeige und Cue-Taste. Dazu Crossfader mit einstellbarer Kurve, Master-Ausgang mit Limiter, Kopfhörer-Mix zwischen Cue und Master.

### 5. Effekte
Mehrere Effekteinheiten, frei auf Kanäle zuweisbar, mit temposynchronen Parametern. Mindestens: Reverb, Delay/Echo, Filter, Flanger, Phaser, Gater, Beat-Repeat, Bitcrusher, Echo-Out-Effekt. Beschreibe für jeden Effekt die Parameter und ihr Verhalten.

### 6. Stems
Tracks werden per KI-Modell offline in vier Spuren zerlegt (Drums, Bass, Melodie/Rest, Vocals) und das Ergebnis wird gespeichert. Im Stem-Deck ist jede Spur einzeln regelbar, stummschaltbar, filterbar und an Effekte sendbar. Stems stehen auch der Übergangslogik zur Verfügung, zum Beispiel für einen sauberen Tausch der Bassline.

### 7. Remix- und Sample-Decks
Raster aus Slots für Loops und One-Shots, die taktsynchron und quantisiert starten. Samples lassen sich direkt aus einem laufenden Deck aufnehmen.

### 8. Übergänge
Lege einen Katalog der Übergangstechniken an, die in der Techno-Szene tatsächlich gespielt werden. Gib für jede Technik an: wann sie passt, welche Voraussetzungen die beiden Tracks erfüllen müssen, Dauer in Takten und den genauen zeitlichen Verlauf aller beteiligten Regler (EQ, Filter, Fader, Effekte, Tempo). Mindestens:

- langer Blend über 16–64 Takte mit Tausch der Bässe am Phrasenwechsel
- Filter-Übergang
- harter Schnitt auf die Eins am Phrasenwechsel
- Echo- oder Reverb-Out
- Loop-Out mit sich verkürzendem Loop
- Einstieg des neuen Tracks im Breakdown des alten, Wechsel am Drop
- Double Drop
- Anteasern des neuen Tracks vor dem eigentlichen Übergang
- Stem-basierter Übergang (einzelne Spuren nacheinander tauschen)
- Tempo-Angleichung über mehrere Takte bei unterschiedlichen BPM

### 9. Auto-Mix
Die Software plant aus einer Trackauswahl ein komplettes Set:

- **Reihenfolge:** nach harmonischer Verträglichkeit (Camelot-Regeln: gleiche Tonart, benachbarte Tonart, Dur-/Moll-Parallele, bewusster Energiesprung), Tempo-Nähe und einem von mir vorgegebenen Energieverlauf über das Set. Tracks lassen sich fest an Positionen setzen.
- **Übergangsplanung:** Auswahl von Ausstiegs- und Einstiegspunkt, Übergangstechnik und Dauer anhand der Analysedaten; bei Bedarf Transponieren um wenige Halbtöne oder Tempo-Angleichung. Lege die Bewertungslogik offen, nach der entschieden wird.
- **Bearbeitung:** Der Plan erscheint als Zeitleiste mit allen Tracks, Übergängen und Reglerverläufen. Ich kann jeden Punkt verschieben, die Technik wechseln, Kurven nachzeichnen und einzelne Übergänge vorhören.
- **Ausgabe:** Rendern schneller als Echtzeit in eine Datei. Zusätzlich ein Autopilot im Live-Modus, der Übergänge selbst spielt und den ich jederzeit übernehmen kann.

### 10. Aufnahme und Export
Mitschnitt des Live-Mixes und Export gerenderter Mixes als WAV, FLAC und MP3, mit wählbarer Ziel-Lautheit, Trackliste mit Zeitstempeln und Cue-Sheet.

### 11. Visuelle Darstellung
Beschreibe jede Ansicht und jedes Element mit Zweck, Inhalt und Verhalten:

- Wellenformen als Gesamtübersicht und als scrollender Ausschnitt, nach Frequenzbereichen eingefärbt
- Beatgrid, Takt- und Phrasenmarker, Cue-Punkte, Loops und erkannte Abschnitte in der Wellenform
- parallele Wellenformen der laufenden Decks übereinander zum Angleichen der Beats
- Phasenanzeige, BPM, Tonart mit farblicher Kennzeichnung der Verträglichkeit zwischen den Decks, Restlaufzeit
- Pegelanzeigen pro Kanal und Master, Spektrum
- Bibliotheksansicht mit Verträglichkeitshinweisen zum laufenden Track
- Zeitleisten-Editor für den Auto-Mix mit Automationskurven

### 12. Audio-Engine
Niedrige Latenz, Anbindung an die Audio-Schnittstellen der Betriebssysteme (ASIO und WASAPI, CoreAudio, JACK/PipeWire/ALSA), freie Zuordnung der Ausgänge, hochwertiges Time-Stretching und Pitch-Shifting, aussetzerfreie Wiedergabe unter Last. Nenne messbare Zielwerte für Latenz, CPU-Last und Analysedauer pro Track.

## Aufbau des Dokuments

1. Überblick, Ziele und Abgrenzung (was gehört nicht zur ersten Version)
2. Glossar der DJ- und Audio-Fachbegriffe
3. Nutzungsszenarien von Anfang bis Ende (Bibliothek aufbauen, live mixen, Set automatisch erstellen und exportieren)
4. Funktionale Anforderungen je Bereich, jede mit eindeutiger Kennung, Priorität (Muss / Soll / Kann) und prüfbarem Abnahmekriterium
5. Analyse- und Auto-Mix-Verfahren im Detail, mit Eingaben, Ausgaben, Qualitätsmaßen und empfohlenen Open-Source-Bausteinen samt Lizenz
6. Datenmodell (Track, Analysedaten, Cue-Punkte, Playlists, Mix-Plan, Einstellungen) und Dateiformate für Import und Export
7. Architektur und Technologie-Empfehlung mit Begründung und Alternativen
8. Oberfläche: jede Ansicht beschrieben, mit skizziertem Aufbau und Standard-Tastenbelegung
9. Nicht-funktionale Anforderungen: Leistung, Stabilität, Datensicherheit der Bibliothek, Bedienbarkeit
10. Teststrategie, einschließlich einer Referenzsammlung von Tracks mit bekannten BPM- und Tonart-Werten zur Messung der Analysequalität
11. Umsetzungsplan in Ausbaustufen, wobei schon die erste Stufe für sich benutzbar ist
12. Risiken und offene Fragen

## Hinweise zur Ausführung

- Sprache: Deutsch; gängige englische DJ-Begriffe (Cue, Loop, Drop, Breakdown) bleiben englisch.
- Schreibe konkret statt allgemein: Zahlen, Wertebereiche, Einheiten und Standardwerte statt Formulierungen wie „schnell“ oder „genau“.
- Wo es mehrere sinnvolle Lösungen gibt, empfiehl eine und begründe sie kurz.
- Vollständigkeit geht vor Kürze. Wenn das Dokument für eine Antwort zu lang wird, liefere es kapitelweise und sag mir, wo du stehst.
