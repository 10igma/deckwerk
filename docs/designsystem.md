# Deckwerk – Designsystem

Stand: Stufe 0/1 · Quelle der Wahrheit: `ui/src/design/tokens.css` · Lebende Übersicht: `/#/designsystem` in der App

## 1. Leitidee „Prüfstand"

Deckwerk sieht aus wie ein Messplatz in einer Werkhalle, nicht wie ein nachgebautes Mischpult: rußschwarze, matte Flächen, Haarlinien, Typenschilder in Versalien, keine Rundungen, keine Verläufe, kein Glanz. Farbe trägt ausschließlich Bedeutung. Die Wellenformen sind das größte Element und bestehen aus drei gestapelten **Metallschichten** – Kupfer (Tief), Messing (Mitte), Zinn (Hoch) –, die von einer gemeinsamen **Beat-Schiene** aus wachsen: Deck A nach oben, Deck B nach unten. Liegen die Beats übereinander, ist das Bild spiegelgleich; läuft ein Deck weg, bricht die Spiegelung sichtbar. Die Decks selbst tragen kalte Signalfarben (Eis, Malve), die nie in der Wellenform-Füllung, sondern nur an Schildern, Marken und aktiven Tasten erscheinen.

Vier Regeln folgen daraus:

1. **Eine Farbe, eine Bedeutung.** Warm = Klang (Frequenzbänder), kalt = Deck, Gelb/Rot = Warnung. Neutrale Zustände sind Kalkweiß auf Ruß.
2. **Fase statt Radius.** Tasten, Schilder und Fader-Kappen haben eine abgeschrägte Ecke (links oben, auf der B-Seite rechts oben). Drehregler haben eine sechskantige Kappe, deren Drehung man an der Form sieht.
3. **Nichts nur über Farbe.** Aktiv = gefüllt + Lämpchen, Kill = Schraffur + „−∞", Auswahl = Balken links, Tonart = Code + Form, Verträglichkeit = Raute / halbe Raute / Kreuz.
4. **Zahlen stehen still.** Alle Zahlen laufen in einer Schrift mit festen Ziffernbreiten; Vorzeichen und Trennzeichen sind immer vorhanden.

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
| `--band-high` | `#EDE8D6` | Zinn | Hoch > 2,5 kHz |
| `--status-ok` | `#5EDB81` | – | „passt" |
| `--status-warn` | `#F9E03F` | – | Achtung: Pegel ab −9 dB, Hinweise, „geht", Limiter greift |
| `--status-danger` | `#FC4447` | – | Übersteuerung, „reibt" |
| `--status-rec` | `#FC4447` | – | Aufnahme läuft (ab Stufe 2) |

Alle Signalfarben erreichen ≥ 4,5 : 1 auf `--bg-0` und `--bg-1`; `--ink` darauf ebenfalls. Die drei Bänder unterscheiden sich zusätzlich in der Helligkeit (Kupfer < Messing < Zinn) und in der Lage (unten, Mitte, oben), bleiben also auch bei Rot-Grün-Sehschwäche unterscheidbar (VIS-12).

### 2.3 Tonarten

`--key-1a … --key-12a` (Moll) und `--key-1b … --key-12b` (Dur), dazu `--key-none`. Zwölf Farbtöne im Kreis nach Camelot-Zahl: Farbton = 25° + (Zahl − 1) · 30° in OKLCH. Moll: L 0,69 / C 0,14 (dunkler, volles Rechteck). Dur: L 0,84 / C 0,11 (heller, Kerbe oben rechts). Der Camelot-Code steht immer in `--ink` auf der Farbe (≥ 6,5 : 1). Berechnung: `ui/src/design/keyColors.ts`; ein Test hält Tokens und Berechnung gleich.

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
| Bedienelemente | Höhe `--ctl-h-sm` 1,5 · `--ctl-h-md` 2 · `--ctl-h-lg` 2,6 rem · Regler `--knob-sm` 2,5 · `--knob-md` 3,1 · `--knob-lg` 4 rem · `--fader-cap` 1,3 rem |
| Raster | `--header-h` 2,9 rem · `--row-h` 1,8 rem |
| Bewegung | `--t-fast` 70 ms · `--t-med` 140 ms (nur Farbwechsel; nichts gleitet) |

### 2.6 Zustände

| Token | Wert | Wirkung |
|---|---|---|
| `--state-hover-line` | `--text-2` | Umriss unter dem Zeiger |
| `--state-held-bg` | `--bg-3` | Fläche, solange gehalten |
| `--state-disabled-opacity` | 0,38 | Regler und Fader deaktiviert |
| `--focus-ring` | 2 px `--text-1` mit 2 px Abstand | Tastaturfokus (Tasten: Umriss wird 2 px und hell) |
| `--played-dim` | 0,45 | Helligkeit des gespielten Teils der Wellenform |
| `--hatch`, `--hatch-fine` | 135°-Schraffur | leer, deaktiviert, gekilltes Band |

### 2.7 Skalierung und Designs

Alle Längen sind rem. Die Wurzelgröße folgt dem Fenster (`ui/src/app/scale.ts`): 14 px bei 1280×720, wachsend mit der 0,75-ten Potenz des Maßstabs (1920×1080: 19 px, 2560×1440: 23,5 px, 3840×2160: 32 px). Größere Fenster bekommen also größere Elemente *und* mehr Platz für Wellenform und Liste. `:root[data-theme='contrast']` überschreibt Flächen, Linien und Schrift für das kontrastreiche Design (VIS-11).

## 3. Komponenten

Quelle: `ui/src/design/components/`. Jede Komponente nimmt eine Kurzhilfe (`tip`: Name, Wert, Tastenkürzel).

| Komponente | Zweck | Zustände und Varianten |
|---|---|---|
| **Taste** `KeyBtn` | Auslösen, Umschalten, Halten (BED-04) | normal · Zeiger darüber (heller Umriss) · gehalten (Fläche `--bg-3`, Umriss hell, Inhalt 1 px tiefer) · aktiv (gefüllt in Akzentfarbe, Schrift `--ink`, Lämpchen gefüllt) · aktiv + gehalten · deaktiviert (blass, schraffiert) · Fokus. Größen sm/md/lg, Fase links oder rechts. Nur Umschalt-Tasten tragen ein Lämpchen. |
| **Drehregler** `Knob` | stetiger Wert | einseitig oder ab der Mitte (bipolar) · Kerbe = Standardwert · dunkles Stück links = Kill-Bereich · gehalten · deaktiviert · Fokus. Größen sm/md/lg, Bogenfarbe frei (EQ: Bandfarbe). |
| **Fader** `Fader` | Kanalfader, Tempo | senkrecht/waagrecht · Füllung ab 0 oder ab der Mitte · Skalenstriche · gehalten · deaktiviert · geführt (Kappe hohl: Wert kommt vom Sync) · Fokus |
| **Crossfader** `Crossfader` | Überblenden | Fader waagrecht zwischen zwei Deck-Schildern, Mitte markiert und leicht gerastet |
| **Pegelanzeige** `Meter` | Kanal, Master, Limiter | Lamellen −36 … +6 dB · bis −9 dB Kalk, darüber Gelb, ab 0 dB Rot · Spitzenhalter 1,2 s · Übersteuerungs-Block hält 2,5 s · Variante „Reduktion" wächst von oben |
| **Zahlenanzeige** `Readout` | BPM, Zeit, Takt | lg / md / sm · links- oder rechtsbündig · gedämpft · optionale Einheit |
| **Listen-Zeile** `ListRow` | Bibliothek | Kopf · normal · Zeiger darüber · ausgewählt (Fläche + Balken links) · wird gezogen |
| **Kurzhilfe** `TipLayer` | Name, Wert, Kürzel (9.3) | erscheint nach 0,45 s über dem Element, sofort beim Ziehen eines Reglers und bei Tastaturfokus |
| **Deck-Schild** `DeckPlate` | Deck-Kennung | A–D · sm/md/lg · Fase links/rechts · hohl |
| **Tonart-Marke** `KeyChip` | Tonart | Moll (volles Rechteck) · Dur (Kerbe) · ohne Tonart (Umriss, „–") · mit/ohne Namen |
| **Phasenanzeige** `PhaseMeter` | Versatz zur Clock ±0,5 Beat | Marke und Balken zeigen Richtung und Größe · eingerastet: Marke wird zur Raute · ohne Clock: nur Mittellinie |
| **Hinweis** `.dw-notice` | Schutzmeldungen | gelbe Fahne mit Warnzeichen, verschwindet nach 3 s |
| **Zeichen** `Icon` | 16 eigene Zeichen aus geraden Kanten | – |

### Regler-Bedienung (BED-05)

Ziehen (Drehregler senkrecht über 12 rem Weg; Fader 1:1 an der Kappe), Mausrad (2 % je Raste), **Shift = fein** (ein Zehntel), **Doppelklick = Standardwert**. Fader werden relativ gezogen – ein Klick neben die Kappe lässt den Wert nie springen. Mausklicks nehmen der Tastatursteuerung nicht den Fokus; mit Tab erreichte Regler reagieren auf Pfeiltasten.

## 4. Wellenformen

- **Parallele Wellenformen** (`ui/src/views/mix/wave/`): WebGL 2, Rückfall Canvas 2D. Deck A oben, Deck B unten, dazwischen die Beat-Schiene mit Marken in der Deck-Farbe (Beat kurz, Takt lang, alle 8 Takte breit). Abspielmarke in der Mitte, gespielter Teil auf 45 % Helligkeit.
- **Stapelung:** je Spalte von der Schiene weg Kupfer, darüber Messing, darüber Zinn; Anteile der Bahnhöhe 50 / 27 / 20 %.
- **EQ in der Wellenform:** Ein abgesenktes Band wird dunkler, ein gekilltes Band schraffiert statt gefüllt – man sieht, wessen Bass gerade spielt.
- **Übersicht:** dieselben Schichten über den ganzen Track, Position als Strich in der Deck-Farbe, Cue-Punkt als Dreieck.

## 5. Dateien

| Pfad | Inhalt |
|---|---|
| `ui/src/design/tokens.css` | alle Tokens (einzige Quelle) |
| `ui/src/design/fonts.css` | lokale Schriften |
| `ui/src/design/base.css` | Reset, Schriftklassen `.dw-label`, `.dw-num`, Fokus |
| `ui/src/design/components/` | Grundkomponenten und ihr CSS |
| `ui/src/design/keyColors.ts`, `tokens.ts` | Tonart-Farben, Token-Zugriff für Canvas/WebGL |
| `ui/src/design/design.test.ts` | prüft Kontraste (≥ 4,5 : 1 Schrift, ≥ 3 : 1 Umrisse) und Tonart-Farben |
| `ui/src/views/DesignSystemView.tsx` | Seite `/#/designsystem` |
