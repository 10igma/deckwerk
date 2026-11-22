# Handbuch

Stand: Stufe 1. Beschrieben ist nur, was gebaut ist. Was noch fehlt, steht am Ende.

## Starten

Bauen und Starten beschreibt die [README](../README.md#schnellstart). Beim ersten Start ist die Bibliothek leer.

![Leere Ansicht nach dem ersten Start](bilder/mixen-leer.png)

Das Ausgabegerät wählst du in den Einstellungen (Zahnrad oben rechts) unter „Audio-Ausgabe": „Systemstandard" oder eines der aufgeführten Geräte. Der Wechsel wirkt sofort, geladene Decks laufen weiter, und die Wahl bleibt über einen Neustart erhalten. Lässt sich ein Gerät nicht öffnen, steht der Grund unter der Liste und das bisherige Gerät läuft weiter.

Findet die Anwendung kein Audiogerät, erscheint in der Kopfzeile ein Warnhinweis. Die Engine läuft dann stumm weiter: Alles lässt sich bedienen, es ist nur nichts zu hören.

## Musik importieren

Ordner oder einzelne Dateien auf das Fenster ziehen. Ordner werden mit allen Unterordnern durchsucht.

**Passende Tracks:** Die Taste „Passend" in der Kopfzeile der Bibliothek zeigt nur Tracks, die zum Bezugs-Deck passen: Tempo höchstens 6 % daneben und eine verträgliche Tonart (gleiche, Nachbar im Camelot-Kreis, Paralleltonart, diagonal oder zwei Schritte höher). Bezug ist der Master, sonst das laufende, sonst das erste geladene Deck; welches gilt und mit welchen Werten, steht im Hinweis der Taste. Verglichen werden die analysierten Werte der Tracks. Suche und Sortierung wirken weiter.

**Neue Dateien im Ordner:** Ordner, die du auf das Fenster ziehst, merkt sich Deckwerk als Musikordner. Die Taste „Aktualisieren" in der Kopfzeile der Bibliothek durchsucht sie erneut: Neue Dateien kommen in die Liste und werden im Hintergrund analysiert, verschobene oder umbenannte Dateien werden am Inhalt wiedererkannt und behalten ihre Analyse, fehlende werden als „Datei fehlt" markiert. Bekannte Tracks bleiben unberührt. Dasselbe geschieht bei jedem Start; abschalten lässt es sich in den Einstellungen unter „Bibliothek". Dort stehen auch die gemerkten Musikordner. Einzeln hereingezogene Dateien machen ihren Ordner nicht zum Musikordner. Ersetzt du eine Datei am selben Ort durch eine andere Fassung, erkennt das Aktualisieren das und analysiert sie neu.

**Unvollständige Dateien:** Enthält eine Datei weniger Audio, als sie ankündigt – typisch für eine abgebrochene Kopie oder einen abgebrochenen Download –, steht in der Liste statt der Tonart „Unvollständig", und beim Laden erscheint ein Hinweis. Der Track bleibt spielbar, endet aber früher, als die Länge in der Liste sagt; Tempo und Tonart stammen dann nur aus dem vorhandenen Teil. Abhilfe: die Datei neu kopieren und „Aktualisieren" drücken. Tracks, die schon vor dieser Prüfung analysiert waren, werden erst beim Laden markiert.

Eine Überwachung im laufenden Betrieb gibt es nicht; entfernen lässt sich ein Musikordner noch nicht.

- **Formate:** MP3, WAV, AIFF, FLAC, AAC/M4A, ALAC, OGG Vorbis, Opus.
- **Sofort sichtbar:** Die Tracks erscheinen gleich in der Liste, zunächst mit den Angaben aus der Datei.
- **Analyse im Hintergrund:** Tempo, Beatgrid, Tonart, Lautheit und Wellenform werden nacheinander berechnet. Die Kopfzeile der Bibliothek zeigt, wie viele Tracks noch warten. Ein Track von sieben Minuten braucht ein bis zwei Sekunden.
- **Doppelte Dateien:** Was schon in der Bibliothek ist, wird nicht noch einmal angelegt, auch nicht unter anderem Namen.
- **Verschobene Dateien:** Wird ein Track verschoben oder umbenannt und der neue Ort importiert, behält er seine Analyse.

![Bibliothek, während Tracks analysiert werden](bilder/bibliothek-analyse.png)

Die Musikdateien selbst werden nur gelesen, nie verändert.

## Die Ansicht „Mixen"

![Ansicht „Mixen" mit zwei geladenen Decks](bilder/mixen-geladen.png)

Von oben nach unten:

| Bereich | Inhalt |
|---|---|
| Kopfzeile | Ansichten, Tempo der Master-Clock mit Taktgeber, Hinweis „Kein Audio", Snap (S), DSP-Last, Uhr, Einstellungen |
| Bühne | Wellenformen beider Decks an einer gemeinsamen Beat-Schiene, Abspielmarke in der Mitte |
| Decks A und B | Titel, Tonart, BPM, Takt, Restzeit, Übersichts-Wellenform, Phase, Tempo, Transport |
| Mixer | je Kanal Gain, EQ mit Kill, Filter, Kopfhörer-Cue, Pegel, Fader; in der Mitte Master und Kopfhörer; unten der Crossfader |
| Bibliothek | Suche und Trackliste; die Taste „Liste" zieht sie auf und schrumpft die Decks auf eine Zeile |

### Wellenformen lesen

- **Drei Schichten:** Kupfer zeigt die Tiefen (Kick, Bass), Messing die Mitten, das helle Zinn die Höhen.
- **Spiegelung:** Deck A wächst von der Schiene nach oben, Deck B nach unten. Liegen die Beats beider Decks übereinander, stehen sich die Kicks genau gegenüber.
- **Schiene:** Marken für Beats, betonte Marken für Takte und für jeden achten Takt.
- **Schraffur:** Ein herausgenommenes Band (Kill) ist schraffiert statt gefüllt. So ist zu sehen, welches Deck gerade den Bass spielt.
- **Versatz:** Laufen zwei Decks auseinander, füllt sich die Lücke zwischen ihren Marken auf der Schiene gelb.
- **Bedienung:** Ziehen in der Wellenform schiebt das Deck an oder bremst es, das Mausrad ändert den Ausschnitt.

**Mitschnitt:** Die Taste „Aufn." oben rechts (oder `Strg` + `R`) schneidet mit, was auf dem Master zu hören ist – nach dem Limiter, also genau das, was an die Lautsprecher geht. Solange aufgenommen wird, leuchtet die Taste rot und zeigt die Laufzeit. Ein zweiter Druck beendet die Aufnahme; die Meldung nennt Länge und Speicherort. Die Datei ist eine WAV mit 24 bit und heißt `Deckwerk <Datum> <Uhrzeit>.wav`; sie liegt im Musik-Ordner deines Benutzers im Unterordner „Deckwerk". Während der Aufnahme wird laufend geschrieben: Stürzt das Programm ab, ist die Datei bis zur letzten Sekunde lesbar. Der Kopfhörer-Weg wird nicht aufgenommen.

**Übergang auf Knopfdruck:** Unten rechts in der Bühne, links neben dem Vorausblick, wählst du eine Vorlage („Bass-Tausch" oder „Überblenden") und eine Länge (4 bis 32 Takte). Es gibt zwei Wege, ihn zu legen:

- **Das Ende bestimmen (empfohlen):** Schieb die Ansicht mit dem Vorausblick dorthin, wo der Übergang fertig sein soll. Die gestrichelte Linie in der Bildmitte ist dann das Ende, und ein blasser Balken am oberen Rand zeigt, von wo bis wo der Übergang laufen würde. Steht das zweite Deck, stell es auf die Stelle, die es an dieser Linie erreicht haben soll (zum Beispiel seinen ersten vollen Takt). Die Taste heißt dann „Übergang A → B endet hier". Deckwerk rechnet zurück: Der Übergang beginnt um seine Länge vor der Linie, und das zweite Deck startet so viel früher, dass es seine Stelle genau am Ende erreicht – notfalls vor seinem Anfang, dann läuft es bis dorthin stumm. Liegt die Linie zu nah für die gewählte Länge, ist die Taste gesperrt.
- **Per Klick in die Wellenform:** Ein Klick (ohne zu ziehen) auf die Wellenform eines laufenden Decks öffnet ein Menü für genau diese Stelle: „Übergang endet hier", „Übergang beginnt hier" und „Ansicht hierher". Die Stelle wird auf den nächsten Taktanfang gerückt; im Kopf des Menüs steht, wie weit sie voraus liegt. Vorlage und Länge kommen aus der Leiste unten rechts. Ein Klick auf die Wellenform eines stehenden Decks bietet „Deck hierher stellen" an – so legst du den Einstieg des zweiten Tracks auf den Beat genau. Um die Stelle zu finden, zieh die Ansicht mit der Maus: Ist der Vorausblick schon verschoben oder hältst du Shift, bewegt Ziehen in einem laufenden Deck nur die Ansicht und nicht die Wiedergabe.
- **Sofort beginnen:** Ohne Vorausblick beginnt der Übergang an der nächsten Vier-Takt-Grenze des laufenden Tracks; ein stehendes Deck steigt an seiner jetzigen Stelle ein.

Beide Wege gehen auch, wenn das zweite Deck schon im Beat-Sync mitläuft; dann werden nur noch die Regler gefahren, und der Übergang geht vom Master aus. Nach dem Drücken springt die Ansicht zurück zur Abspielstelle, und der Übergang ist als kräftiger Balken mit Linien an Beginn, Mitte und Ende eingezeichnet. Die Leiste zeigt, wie viele Beats es bis zum Start sind und in welchem Takt er ist. Ein stehendes Deck rückt Deckwerk auf seine Eins und schaltet es auf Beat-Sync; es startet von selbst. Die Regler beider Decks fahren im Takt: Fader, Bässe, Mitten, Höhen. Am Ende stoppt das alte Deck, und seine Regler gehen zurück in die Ruhestellung. „Abbrechen" hält die Regler dort an, wo sie gerade stehen; beide Decks laufen weiter. Beim Bass-Tausch wechselt der Bass in der Mitte innerhalb eines Beats, beim Überblenden weich über wenige Takte.

**Vorausblick:** Mit `Shift` + Mausrad, den Doppelpfeilen unten rechts in der Bühne oder `PgDn` / `PgUp` (vier Takte, mit `Shift` ein Takt) schiebst du die Ansicht nach vorn oder zurück. Die Wiedergabe läuft dabei unverändert weiter. Laufende Decks verschieben sich gemeinsam; ein stehendes Deck bleibt an seiner Stelle. So suchst du den Einsatz: Der laufende Track zeigt, was er in einer Minute spielt, und im stehenden Track ziehst du die Stelle heran, die dazu passen soll. Laufen beide Decks, siehst du, was dann zusammen erklingt. Läuft keines, verschieben sich beide.

**Start aus dem Vorausblick:** Drückst du im Vorausblick Play am stehenden Deck, startet es nicht an der eingestellten Stelle, sondern um den Vorausblick früher. Es erreicht die eingestellte Stelle also genau dann, wenn der laufende Track bei der Stelle ankommt, die du darüber gesehen hast – das Bild trifft so ein, wie du es eingestellt hast. Liegt der Start vor dem Anfang des Tracks, läuft das Deck bis dorthin stumm (die Zeit zählt negativ); so bekommst du einen Vorlauf, den der Track selbst nicht hat. Die Cue-Taste spielt weiter ab der eingestellten Stelle vor.

 Solange die Ansicht verschoben ist, steht in der Bildmitte eine gestrichelte gelbe Linie; die Abspielmarke bleibt je Deck an ihrer wirklichen Stelle, beim stehenden Deck also in der Mitte. Unten rechts zeigt die Taste „Zurück" den Abstand in Beats und Minuten. Ein Druck darauf, `Pos1` oder `Esc` bringt die Ansicht zurück zur Abspielstelle.

### Regler bedienen

| Handgriff | Wirkung |
|---|---|
| Ziehen | Wert ändern |
| Mausrad | Wert in Schritten ändern |
| Shift beim Ziehen | Feineinstellung |
| Doppelklick | Standardwert (nicht bei Kanalfader und Master-Lautstärke) |
| Klick auf die Zahl | Wert eintippen; Enter übernimmt, Esc verwirft (Gain, EQ, Filter, Tempo, Master) |

Wer mit der Maus über einem Bedienelement bleibt, sieht Name, aktuellen Wert und Tastenkürzel.

## Ein Übergang Schritt für Schritt

![Übergang: Deck A läuft aus, Bass bei B herausgenommen](bilder/mixen-uebergang.png)

1. **Ersten Track laden und starten.** In der Liste auswählen, mit `Shift + ←` auf Deck A laden, `Q` startet. Das Deck wird Taktgeber; seine Taste „Clock" leuchtet, und sein Schild steht in der Kopfzeile neben dem Tempo.
2. **Zweiten Track laden.** `Shift + →` lädt auf Deck B. Die Tonart-Marke zeigt, ob er zum laufenden Track passt (passt / geht / reibt).
3. **Vorhören.** `Y` legt Deck B auf den Kopfhörer. Der Regler „Mix" im Kopfhörer-Teil blendet zwischen Vorhör-Signal und Master.
4. **Angleichen.** `O` schaltet Sync für Deck B ein: Tempo und Phase folgen dem Master. Die Phasenanzeige steht dann in der Mitte, die Kicks in der Bühne stehen sich gegenüber.
5. **Bass herausnehmen.** `H` nimmt bei B die Tiefen heraus (Kill).
6. **Einsteigen.** Deck B am Phrasenwechsel von A mit `U` starten, den Kanalfader hochziehen.
7. **Bässe tauschen.** Am nächsten Phrasenwechsel `G` (Tiefen bei A heraus) und `H` (Tiefen bei B hinein).
8. **Ausblenden.** Kanalfader von A herunterziehen oder den Crossfader zu B schieben.

Deck B übernimmt die Rolle des Taktgebers von selbst, sobald A nicht mehr zu hören ist. Tempo und Phase bleiben dabei unverändert.

## Schutz vor Fehlgriffen

Was ein hörbares Deck unterbrechen würde, braucht einen bewussten zweiten Griff. Hörbar heißt: Kanalfader offen und Crossfader dem Kanal nicht ganz abgewandt.

| Aktion am hörbaren Deck | So geht es trotzdem |
|---|---|
| Stoppen (Play) | Taste halten oder ein zweites Mal drücken |
| Cue | Taste halten oder ein zweites Mal drücken |
| Sprung in der Übersichts-Wellenform | mit Shift klicken |
| Neuen Track laden | erst stoppen |

![Stopp-Versuch am hörbaren Deck: Die Taste ist scharf, ein Hinweis erklärt den zweiten Griff](bilder/schutz-stopp.png)

Ein laufendes Deck springt außerdem nur bei eingeschaltetem Snap, damit es im Takt bleibt. Ein Deck, das nicht zu hören ist, lässt sich ohne Hürde bedienen. Der Schutz ist in den Einstellungen abschaltbar. Ein Doppelklick in der Liste lädt nur in ein leeres Deck; in ein geladenes Deck lädt die Lade-Taste der Zeile, `Shift + Pfeil` oder das Ablegen auf dem Deck.

## Tastenbelegung

Die Tasten sind an ihre Position auf der Tastatur gebunden, nicht an den aufgedruckten Buchstaben. Die Namen unten gelten für das US-Layout; auf einer deutschen Tastatur liegt zum Beispiel `Y` dort, wo `Z` aufgedruckt ist.

| Funktion | Deck A | Deck B |
|---|---|---|
| Play/Pause | Q | U |
| Cue | W | I |
| Sync | E | O |
| Abbremsen / Anschieben | A / S | J / K |
| Tempo − / + (fein mit Shift) | D / F | L / ; |
| Kopfhörer-Cue | T | Y |
| Tiefen-Kill | G | H |
| Filter zu / auf | Strg + A / S | Strg + J / K |
| Kanalfader auf / zu | Alt + D / F | Alt + L / ; |
| Track aus der Liste laden | Shift + ← | Shift + → |

| Funktion | Taste |
|---|---|
| Crossfader links / rechts | ← / → |
| Crossfader ganz links / rechts | Alt + ← / → |
| Liste hoch / runter | ↑ / ↓ |
| Suche | Strg + F (macOS: Cmd + F) |
| Play/Pause des zuletzt bedienten Decks | Leertaste |
| Ansichten | F1–F4 |
| Snap | F10 |
| Fokus eines Reglers lösen | Esc |

## Kommandozeile

Das Programm `deckwerk` nutzt denselben Kern wie die Anwendung, ohne Oberfläche.

**Analysieren**

```bash
deckwerk analyze track1.mp3 track2.flac
```

```
track1.mp3
  Tempo 132.00 BPM · Tonart 8A (a-Moll) · Lautheit -14.5 LUFS · Spitze -1.0 dBTP · Länge 3:53 · 1.4 s Rechenzeit
```

Mit `--json` kommt das vollständige Ergebnis samt Beatgrid als JSON.

**Übergang rendern**

```bash
deckwerk mix track1.mp3 track2.flac -o mix.wav --bars 32
```

```
A: 132.00 BPM, 8A (a-Moll)
B: 134.50 BPM, 9A (e-Moll)
Tonarten: 8A → 9A: passt (K = 0.9)
Übergang: Blend mit Bass-Tausch über 32 Takte, Tausch nach 16 Takten; A ab 130.9 s, Übergang ab 160.0 s, B ab 0.0 s
Tempo: 132.00 → 134.50 BPM (1.9 %), angeglichen über 32 Takte. Ohne Key Lock ändert sich dabei die Tonhöhe.
Gerendert: 115.3 s in 2.0 s · Übergang von 29.1 s bis 86.7 s, Bass-Tausch bei 58.0 s · größter Beat-Versatz 0.0000 Beats · Limiter bis -0.2 dB
```

| Option | Bedeutung | Standard |
|---|---|---|
| `--bars` | Länge des Übergangs in Takten: 16, 32 oder 64 | 32 |
| `--lead` | Takte von A vor dem Übergang | 16 |
| `--tail` | Takte von B nach dem Übergang | 16 |

Der Ausstieg aus A liegt auf der letzten 8-Takt-Grenze, nach der der Übergang noch Platz hat; B steigt an seinem ersten Downbeat ein. Bis 6 % Tempo-Abstand (8 % ab 32 Takten) wird das Tempo angeglichen, darüber lehnt das Programm den Blend ab.

**Testmaterial erzeugen**

```bash
deckwerk testtrack a.wav --bpm 132 --key 8A --seed 1
```

Erzeugt einen synthetischen Techno-Track mit Intro, Hauptteil, Breakdown, Drop und Outro, dessen Tempo und Tonart exakt bekannt sind.

## Wo die Daten liegen

| Inhalt | Ort |
|---|---|
| Bibliothek | Benutzerdaten-Ordner des Systems, Unterordner `library/` |
| Sicherungen der Bibliothek | `library/backups/`, die letzten zehn, bei jedem Start eine neue |
| Wellenformen | Unterordner `cache/wave/` |

Mit der Umgebungsvariable `DECKWERK_DATA` lässt sich ein anderer Ordner wählen. Ist die Bibliothek beschädigt, spielt die Anwendung beim Start die neueste Sicherung ein.

## Was noch fehlt

- **Ton am Gerät ist ungeprüft.** Die Ausgabe ist geschrieben, lief aber noch an keinem Audio-Interface. Das Gerät ist in den Einstellungen wählbar; Kanäle und Puffergröße lassen sich noch nicht einstellen. Ab vier Kanälen liegt der Kopfhörer auf Kanal 3/4.
- **Analyse an echter Musik ist ungemessen.** Die Trefferquoten stammen von synthetischen Tracks.
- **Import nur durch Ablegen** auf das Fenster; einen Ordner-Dialog gibt es noch nicht. Unter Linux ist das Ablegen selbst noch nicht von Hand geprüft.
- **Nur zwei Decks in der Oberfläche**, obwohl der Kern vier kann.
- **Kein Key Lock:** Wer das Tempo ändert, ändert die Tonhöhe mit.
- **Noch nicht gebaut:** Loops, Hotcues, Beatjump, Effekte, Playlists in der Oberfläche, die Ansichten „Bibliothek", „Vorbereiten" und „Auto-Mix", Stems, Remix-Decks, Import aus Traktor und Rekordbox.

Die Reihenfolge, in der das kommt, steht im [Umsetzungsplan](spezifikation.md#11-umsetzungsplan).
