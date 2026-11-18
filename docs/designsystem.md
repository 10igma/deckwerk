# Deckwerk – Designsystem

Stand: Stufe 1 nach der ersten Prüfrunde (Art Direction, UX) · Quelle der Wahrheit: `ui/src/design/tokens.css` · Lebende Übersicht: `/#/designsystem` in der App

## 1. Leitidee „Prüfstand"

Deckwerk sieht aus wie ein Messplatz in einer Werkhalle, nicht wie ein nachgebautes Mischpult: rußschwarze, matte Flächen, Haarlinien, Typenschilder in Versalien, keine Rundungen, keine Verläufe, kein Glanz. Farbe trägt ausschließlich Bedeutung. Die Wellenformen sind das größte Element und bestehen aus drei gestapelten **Metallschichten** – Kupfer (Tief), Messing (Mitte), Zinn (Hoch) –, die von einer gemeinsamen **Beat-Schiene** aus wachsen: Deck A nach oben, Deck B nach unten. Liegen die Beats übereinander, ist das Bild spiegelgleich; läuft ein Deck weg, bricht die Spiegelung sichtbar. Die Decks selbst tragen kalte Signalfarben (Eis, Malve), die nie in der Wellenform-Füllung, sondern nur an Schildern, Marken und aktiven Tasten erscheinen.

Sechs Regeln folgen daraus:

1. **Eine Farbe, eine Bedeutung.** Warm = Klang (Frequenzbänder), kalt = Deck, Gelb/Rot = Warnung. Neutrale Zustände sind Kalkweiß auf Ruß. Gefüllte Farbflächen gehören den Deck-Schildern und aktiven Tasten; die Tonart zeigt ihre Farbe nur als Balken.
2. **Fase statt Radius, nach Seite.** Tasten, Schilder und Fader-Kappen haben eine abgeschrägte Ecke: in der linken Hälfte des Pults (Deck A, Kanal A) und bei allem Mittigen (Kopfzeile, Master, Bühne, Bibliothek) oben links, in der rechten Hälfte (Deck B, Kanal B) oben rechts. Die Seite setzt der umschließende Bereich (`CutSide`), nicht die einzelne Taste. Angedockte Tasten (Sync + Modus) teilen sich eine Fase. Drehregler haben eine sechskantige Kappe, deren Drehung man an der Form sieht.
3. **Nichts nur über Farbe.** Aktiv = gefüllt bzw. kräftiger Umriss + Lämpchen, Kill = Schraffur + „−∞", Auswahl = Balken links, Tonart = Code + Kerbe, Verträglichkeit = Raute / halbe Raute / Kreuz, Deck mit Fokus = doppelt starke Kante + Balken im Schild.
4. **Zahlen stehen still.** Alle Zahlen laufen in einer Schrift mit festen Ziffernbreiten; Vorzeichen und Trennzeichen sind immer vorhanden. Der unruhige Teil einer Zahl (Zehntelsekunde, Schlag im Takt) ist kleiner und gedämpft. Wo nichts geladen ist, steht ein gedämpfter Strich, kein Scheinwert.
5. **Die Bühne führt.** Die Wellenformen bekommen mindestens 40 % der Fensterhöhe. Das Pult ist fest (13,8 rem), die Bibliothek nimmt bis zu acht Zeilen, jede weitere Höhe geht an die Bühne. Decks sind höchstens 32 rem breit; Überschuss geht an den Mixer.
6. **Live-Ziele sind mindestens 28 px hoch**, auch bei 1280×720 (Spezifikation 9.3): Tasten, Lade-Tasten, Listenzeilen, Regler, Fader, Übersicht, Suche. Wo die sichtbare Marke kleiner ist (Lade-Taste, Zahlenwert), ist die Trefferfläche größer.

## 2. Tokens

### 2.1 Flächen, Linien, Schrift

| Token | Wert | Verwendung | Kontrast auf `--bg-1` |
|---|---|---|---|
| `--bg-0` | `#0D0C0B` | Grund, Wellenform-Bühne, Schlitze | – |
| `--bg-1` | `#151412` | Platten: Decks, Mixer, Liste, Kopfzeile | – |
| `--bg-2` | `#201E1B` | erhabene Teile: Tasten, Felder | – |
| `--bg-3` | `#2C2925` | gedrückt, Zeile unter dem Zeiger, Regler-Kappen | – |
| `--line-1` | `#35322D` | Haarlinie, rein gliedernd | 1,4 : 1 |
| `--line-2` | `#6E6960` | Umriss von Bedienelementen | 3,4 : 1 |
| `--text-1` | `#F2EEE6` | Werte, Titel, Abspielmarke | 15,9 : 1 |
| `--text-2` | `#B8B2A6` | Fließtext, Nebenwerte | 8,7 : 1 |
| `--text-3` | `#958F84` | Beschriftungen | 5,7 : 1 (auf `--bg-3`: 4,5 : 1) |
| `--ink` | `#0D0C0B` | Schrift auf farbigen Flächen | – |

### 2.2 Signalfarben

| Token | Wert | Name | Bedeutung |
|---|---|---|---|
| `--deck-a` | `#36DCEC` | Eis | Deck A: Schild, Kante, aktive Tasten, Fader, Marke in der Übersicht |
| `--deck-b` | `#D78FF6` | Malve | Deck B |
| `--deck-c` | `#B6E551` | Lind | Deck C (ab Stufe 2) |
| `--deck-d` | `#6A9CFB` | Azur | Deck D (ab Stufe 2) |
| `--band-low` | `#D77343` | Kupfer | Tief < 200 Hz: Wellenform, EQ-Regler, Kill-Taste |
| `--band-mid` | `#E1B75C` | Messing | Mitte 200 Hz–2,5 kHz |
| `--band-high` | `#C8D3D4` | Zinn | Hoch > 2,5 kHz – kühles Grau, deutlich unter Kalkweiß (Schrift, Abspielmarke, neutrale Tasten) |
| `--status-ok` | `#5EDB81` | – | „passt" |
| `--status-warn` | `#F9E03F` | – | Achtung: Pegel ab −9 dB, Hinweise, „geht", Limiter greift, Versatz der Beat-Marken, „wird analysiert", kein Audiogerät, Live-Schutz scharf |
| `--status-danger` | `#FC4447` | – | Übersteuerung, „reibt", Fehler (Laden gescheitert, Datei fehlt) |
| `--status-rec` | `#FC4447` | – | Aufnahme läuft (ab Stufe 2) |

Alle Signalfarben erreichen ≥ 4,5 : 1 auf `--bg-0` und `--bg-1`; `--ink` darauf ebenfalls. Die drei Bänder unterscheiden sich zusätzlich in der Helligkeit (Kupfer < Messing < Zinn; relative Helligkeit 0,27 / 0,51 / 0,64) und in der Lage (unten, Mitte, oben), bleiben also auch bei Rot-Grün-Sehschwäche unterscheidbar (VIS-12). Zinn liegt bewusst unter Kalkweiß (0,85): Eine aktive Hoch-Kill-Taste ist damit von einer neutralen aktiven Taste zu unterscheiden.

### 2.3 Tonarten

`--key-1a … --key-12a` (Moll) und `--key-1b … --key-12b` (Dur), dazu `--key-none`. Zwölf Farbtöne im Kreis nach Camelot-Zahl: Farbton = 25° + (Zahl − 1) · 30° in OKLCH. Moll: L 0,69 / C 0,14 (dunkler). Dur: L 0,84 / C 0,11 (heller). Die Farbe erscheint nur als Balken links an der Tonart-Marke; der Camelot-Code steht in `--text-1` auf `--bg-2`. So bleibt die gefüllte Farbfläche den Decks vorbehalten, auch wo ein Farbton einer Signalfarbe nahekommt (9A ≈ Azur, 7B ≈ Eis, 2A ≈ Kupfer). Dur trägt zusätzlich eine Kerbe oben rechts, Moll ist ein volles Rechteck. Berechnung: `ui/src/design/keyColors.ts`; ein Test hält Tokens und Berechnung gleich.

### 2.4 Schrift

| Token | Wert | Verwendung |
|---|---|---|
| `--font-text` | Barlow Semi Condensed 400/500/600 | Titel, Künstler, Fließtext |
| `--font-label` | Barlow Condensed 600/700, Versalien, Sperrung `--track-label` (0,08 em) | Beschriftungen, Tasten |
| `--font-num` | Martian Mono (variabel), Breite `--num-stretch` 80 %, feste Ziffernbreite | alle Zahlen und Tastenkappen |

| Token | rem | bei 1280×720 | Verwendung |
|---|---|---|---|
| `--fs-1` | 0,786 | 11 px | kleinste Schrift, Nebenwerte |
| `--fs-2` | 0,857 | 12 px | Beschriftung |
| `--fs-3` | 1 | 14 px | Text |
| `--fs-4` | 1,214 | 17 px | Titel im Deck |
| `--fs-5` | 1,5 | 21 px | Clock-BPM, Takt |
| `--fs-6` | 2 | 28 px | Hauptwerte: BPM, Restzeit |
| `--fs-7` | 2,857 | 40 px | Überschriften |

Die Schriften liegen im Paket (`@fontsource`), es wird nichts aus dem Netz geladen.

### 2.5 Abstände, Formen, Maße

| Gruppe | Tokens |
|---|---|
| Abstände | `--sp-1` 0,25 · `--sp-2` 0,5 · `--sp-3` 0,75 · `--sp-4` 1 · `--sp-5` 1,5 · `--sp-6` 2,5 rem |
| Formen | `--radius-0` 0 · `--radius-1` 1 px · Fase `--cut-1` 0,3 rem, `--cut-2` 0,5 rem · Strich `--stroke-1` 1 px, `--stroke-2` 2 px |
| Bedienelemente | Höhe `--ctl-h-sm` 2 (= 28 px bei 1280×720, Mindestmaß) · `--ctl-h-md` 2,2 · `--ctl-h-lg` 2,6 rem · Regler `--knob-sm` 2,4 · `--knob-md` 3,1 · `--knob-lg` 4 rem · `--fader-cap` 1,3 rem |
| Raster | `--header-h` 2,6 rem · `--row-h` max(28 px, 1,8 rem) |
| Bewegung | `--t-fast` 70 ms · `--t-med` 140 ms (nur Farbwechsel; nichts gleitet) |

### 2.6 Zustände

| Token | Wert | Wirkung |
|---|---|---|
| `--state-hover-line` | `--text-2` | Umriss unter dem Zeiger |
| `--state-held-bg` | `--bg-3` | Fläche, solange gehalten |
| `--state-armed-line` | `--status-warn` | Live-Schutz wartet: Umriss gelb, beim Halten läuft ein Balken (0,4 s) |
| `--state-disabled-opacity` | 0,38 | Regler und Fader deaktiviert |
| `--focus-ring` | 2 px `--text-1` mit 2 px Abstand | Tastaturfokus (Tasten: Umriss wird 2 px und hell) |
| `--played-dim` | 0,45 | Helligkeit des gespielten Teils der Wellenform |
| `--hatch`, `--hatch-fine` | 135°-Schraffur | leer, deaktiviert, gekilltes Band |

### 2.7 Skalierung und Designs

Alle Längen sind rem (Mindestmaße in px, wo 9.3 sie verlangt). Die Wurzelgröße folgt dem Fenster (`ui/src/app/scale.ts`): 14 px bei 1280×720, wachsend mit der 0,75-ten Potenz des Maßstabs (1920×1080: 19 px, 2560×1440: 23,5 px, 3840×2160: 32 px). Größere Fenster bekommen also größere Elemente *und* mehr Platz für Wellenform und Liste. `:root[data-theme='contrast']` überschreibt Flächen, Linien und Schrift für das kontrastreiche Design (VIS-11).

## 3. Komponenten

Quelle: `ui/src/design/components/`. Jede Komponente nimmt eine Kurzhilfe (`tip`: Name, Wert, Tastenkürzel).

| Komponente | Zweck | Zustände und Varianten |
|---|---|---|
| **Taste** `KeyBtn` | Auslösen, Umschalten, Halten (BED-04) | normal · Zeiger darüber (heller Umriss) · gehalten (Fläche `--bg-3`, Umriss hell, Inhalt 1 px tiefer) · aktiv · aktiv + gehalten · scharf (Live-Schutz: gelber Umriss, Halte-Balken) · deaktiviert (blass, schraffiert) · Fokus (Umriss 2 px, hell). „Aktiv" in zwei Darstellungen: `fill` = Fläche in Akzentfarbe mit Schrift `--ink` (kleine Tasten: Sync, Clock, Kill, Cue), `outline` = Umriss 2 px, Lämpchen und Zeichen in Akzentfarbe (Play – keine Leuchtfläche). Die Beschriftung wechselt nie mit dem Zustand. Größen sm/md/lg. Nur Umschalt-Tasten tragen ein Lämpchen. |
| **Drehregler** `Knob` | stetiger Wert | einseitig oder ab der Mitte (bipolar) · Kerbe = Bezugswert (0 dB) · dunkles Stück links = Kill-Bereich · Zeiger darüber · gehalten · deaktiviert · Fokus. Größen sm/md/lg, Bogenfarbe frei (EQ: Bandfarbe). |
| **Fader** `Fader` | Kanalfader, Tempo | senkrecht/waagrecht · Füllung ab 0 oder ab der Mitte · Skalenstriche · gehalten · deaktiviert · geführt (Kappe hohl: Wert kommt vom Sync) · Fokus |
| **Crossfader** `Crossfader` | Überblenden | Fader waagrecht zwischen zwei Deck-Schildern, Mitte markiert und leicht gerastet |
| **Zahleneingabe** `ValueField` | Wert eines Reglers tippen (BED-05) | Anzeige (gepunktete Linie unter dem Zeiger) · Eingabe offen (Feld mit hellem Umriss) · unlesbar (roter Umriss, bleibt offen). Enter übernimmt, Esc verwirft; Komma und Punkt gelten beide. |
| **Pegelanzeige** `Meter` | Kanal, Master, Limiter | Lamellen −36 … +6 dB · bis −9 dB Kalk, darüber Gelb, ab 0 dB Rot · Spitzenhalter 1,2 s · Übersteuerungs-Block hält 2,5 s · Variante „Reduktion" wächst von oben |
| **Zahlenanzeige** `Readout` | Werte mit Beschriftung | lg / md / sm · links- oder rechtsbündig · gedämpft · optionale Einheit |
| **Listen-Zeile** `ListRow` | Bibliothek | Kopf · normal · Wechselzeile (`--bg-0`) · Zeiger darüber · ausgewählt (Fläche + Balken links) · wird gezogen · gedämpft (Datei fehlt). Statt der Tonart steht bei Bedarf der Zustand: „wird analysiert" (gelb, laufender Balken) oder „Datei fehlt" (rot, Warnzeichen). |
| **Kurzhilfe** `TipLayer` | Name, Wert, Kürzel (9.3) | dunkel (`--bg-3`) mit Umriss `--line-2`, damit sie im dunklen Raum nicht das Hellste auf dem Schirm ist. Erscheint nach 0,45 s über dem Element und bei Tastaturfokus. Beim Ziehen eines Drehreglers bleibt sie weg (der Wert steht daneben), beim Ziehen eines Faders erscheint sie sofort. |
| **Deck-Schild** `DeckPlate` | Deck-Kennung | A–D · sm/md/lg · Fase nach Seite · hohl · mit Balken = Deck mit Fokus |
| **Tonart-Marke** `KeyChip` | Tonart | Code auf dunklem Grund mit Farbbalken · Moll (volles Rechteck) · Dur (Kerbe) · ohne Tonart (Umriss, „–") · mit/ohne Namen |
| **Phasenanzeige** `PhaseMeter` | Versatz zur Clock ±0,5 Beat | Marke und Balken zeigen Richtung und Größe · eingerastet: Marke wird zur Raute · ohne Clock: nur Mittellinie. Im Deck liegt sie als schmaler Streifen direkt über dem Tempo-Regler, auf derselben Achse. |
| **Hinweis** `.dw-notice` | Schutzmeldungen, Fehler | gelbe Fahne mit Warnzeichen (3 s) · rote Fahne für Fehler (7 s), mit dem Klartext der Hülle |
| **Zeichen** `Icon` | 20 eigene Zeichen aus geraden Kanten | – |

### Regler-Bedienung (BED-05, 9.3)

Ziehen (Drehregler senkrecht über 12 rem Weg; Fader 1:1 an der Kappe), Mausrad (2 % je Raste), **Shift = fein** (ein Zehntel), **Doppelklick = Standardwert**, Zahl anklicken = eintippen. Kanalfader und Master-Lautstärke haben keinen Doppelklick-Sprung: Ein geschlossener Kanal öffnet sich nie mit einem Griff. Fader werden relativ gezogen – ein Klick neben die Kappe lässt den Wert nie springen. Mausklicks nehmen der Tastatursteuerung nicht den Fokus; mit Tab erreichte Regler reagieren auf Pfeiltasten, Esc gibt sie wieder frei.

### Live-Schutz (9.3)

Was ein hörbares Deck unterbricht (Stoppen, Cue, Neuladen), führt beim ersten Druck nur zu einem gelben Hinweis und zum Zustand „scharf" an der Taste. Ausgeführt wird nach 0,4 s Halten (ein Balken läuft an der Unterkante der Taste) oder beim zweiten Druck innerhalb 1 s. Der Sprung in der Übersicht braucht am hörbaren Deck Shift. Ein unhörbares Deck (Fader zu oder Crossfader ganz abgewandt) wird ohne Hürde bedient. Der Schutz ist in den Einstellungen abschaltbar.

## 4. Ansicht „Mixen" und Wellenformen

- **Bühne** (`ui/src/views/mix/wave/`): WebGL 2, Rückfall Canvas 2D mit demselben Bild. Deck A oben, Deck B unten, dazwischen die **Beat-Schiene** (2 rem). Jede Beat-Marke füllt die Hälfte ihres Decks in der Deck-Farbe: Beat 0,2 rem breit, Takt 0,4 rem, alle 8 Takte 0,7 rem mit einer Fahne in die eigene Bahn. Laufen beide Decks und liegen ihre Marken nicht übereinander, füllt Gelb die Lücke dazwischen – keine Fläche heißt im Takt, eine breite Fläche heißt läuft weg.
- **Rasterlinien in den Bahnen:** Beat rund 2 : 1 gegen den Grund, Takt mindestens 3 : 1, 8-Takt-Linie kräftiger (VIS-03).
- **Stapelung:** je Spalte von der Schiene weg Kupfer, darüber Messing, darüber Zinn; Anteile der Bahnhöhe 50 / 27 / 20 %. Abspielmarke in der Mitte, gespielter Teil auf 45 % Helligkeit.
- **EQ in der Wellenform:** Ein abgesenktes Band wird dunkler, ein gekilltes Band schraffiert (135°, in beiden Zeichnern gleich) statt gefüllt – man sieht, wessen Bass gerade spielt.
- **Schild, Titel und Zoom** liegen ohne deckende Fläche über der Wellenform (Schrift mit Ruß-Kontur) und schneiden keine Spitzen ab.
- **Leerzustand:** ein Hinweis je Deck in der Bühne, Abspielmarke grau, keine Schraffur; Zahlen als gedämpfter Strich.
- **Decks:** Kennung außen; Tonart, Verträglichkeit und BPM innen am Mixer (bei A rechts, bei B links), Restzeit außen. Darunter Übersicht, Gleichlauf-Block (Phase über Tempo, Anschieben/Abbremsen links und rechts) und Transport – in beiden Decks in derselben Reihenfolge: Anfang · Cue · Play ¦ Sync+Modus · (fester Abstand) · Clock. Cue steht nicht neben Sync; die Clock-Taste ist in jeder Auflösung um denselben Abstand abgerückt.
- **Mixer:** je Kanal Gain und Filter nebeneinander, darunter Hoch/Mitte/Tief jeweils als Regler · Wert · Kill-Taste in voller Zeilenhöhe; Fader und Pegel innen am Master. Werte stehen ohne Einheit rechtsbündig in fester Spalte (alles dB; die Einheit steht in Kurzhilfe und Eingabe).
- **Übersicht:** dieselben Schichten über den ganzen Track, Position als Strich in der Deck-Farbe, Cue-Punkt als Dreieck.
- **Bibliothek:** Titel und Künstler sind in der Breite begrenzt, BPM und Tonart schließen direkt an. Lade-Tasten sind neutral; die Deck-Farbe erscheint erst unter dem Zeiger und als gefülltes Schild in der Zeile des geladenen Tracks. Die Liste lässt sich aufziehen – dann schrumpfen die Decks auf je eine Zeile (Kennung, Titel, Übersicht, BPM, Tonart, Restzeit).
- **Kopfzeile:** Ohne Audiogerät steht dauerhaft „Kein Audio" als gelber Umriss mit Warnzeichen; die Meldung des Systems steht in der Kurzhilfe.

## 5. Dateien

| Pfad | Inhalt |
|---|---|
| `ui/src/design/tokens.css` | alle Tokens (einzige Quelle) |
| `ui/src/design/fonts.css` | lokale Schriften |
| `ui/src/design/base.css` | Reset, Schriftklassen `.dw-label`, `.dw-num`, Fokus |
| `ui/src/design/components/` | Grundkomponenten und ihr CSS |
| `ui/src/design/keyColors.ts`, `tokens.ts` | Tonart-Farben, Token-Zugriff für Canvas/WebGL |
| `ui/src/design/design.test.ts` | prüft Kontraste (≥ 4,5 : 1 Schrift, ≥ 3 : 1 Umrisse), Helligkeitsstufen der Bänder und Tonart-Farben |
| `ui/scripts/shots.cjs` | Screenshots aller Zustände; misst Bühnenhöhe (≥ 40 %), Bedienziele (≥ 28 px) und sichtbare Listenzeilen nach |
| `ui/src/views/DesignSystemView.tsx` | Seite `/#/designsystem` |
