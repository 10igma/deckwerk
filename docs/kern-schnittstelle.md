# Schnittstelle zwischen Oberfläche und Desktop-Hülle

Die Oberfläche spricht den Kern nur über `CoreApi` (`ui/src/core/api.ts`) an. In der Desktop-Anwendung bildet ein Adapter diese Schnittstelle auf die folgenden Tauri-Befehle ab (`app/src/commands.rs`). Aufruf mit `invoke(name, args)` aus `@tauri-apps/api/core`; Argumentnamen in camelCase.

| Befehl | Argumente | Ergebnis | Zweck |
|---|---|---|---|
| `core_send` | `cmd: Command` | – | Bedienbefehl an die Engine |
| `core_poll` | – | `Poll` | Schnappschuss, Engine-Kennzahlen, Stand der Bibliothek; je Bild einmal aufrufen |
| `library_list` | – | `TrackInfo[]` | alle Tracks, nach Titel sortiert |
| `library_import` | `paths: string[]` | `ImportResult` | Dateien und Ordner importieren (rekursiv) |
| `library_refresh` (zählt eine am selben Ort ersetzte Datei unter `added`; `TrackInfo.incomplete` meldet unvollständige Dateien, BIB-20) | – | `RefreshResult` | Musikordner erneut durchsuchen (BIB-19): neue Dateien aufnehmen, verschobene neu zuordnen, fehlende markieren. Musikordner sind die als Ordner importierten Orte; kennt die Hülle noch keine, übernimmt sie einmalig die Ordner der vorhandenen Tracks |
| `settings_get` | – | `AppSettings` (`refresh_on_start`, `music_folders`) | Einstellungen der Hülle |
| `settings_refresh_on_start` | `on: boolean` | – | Beim Start die Musikordner durchsuchen (Vorgabe an) |
| `transition_templates` | `bars: number` | `Transition[]` | eingebaute Vorlagen für Übergänge in dieser Länge (`bass_swap`, `fade`; TRANS-01) |
| `transition_start` | `from: DeckId`, `to: DeckId`, `transition: Transition`, `alignBars: number` | `TransitionRun` | Übergang vom laufenden zum stehenden Deck einplanen (TRANS-05): Beginn an der nächsten Taktgrenze des laufenden Tracks im Raster von `alignBars` Takten, frühestens zwei Beats nach dem Aufruf. Das stehende Deck rückt auf den Taktanfang neben seiner Position und bekommt Beat-Sync; Start, Reglerverläufe und Stopp fährt die Engine. Abgelehnt mit Klartext, wenn das laufende Deck weder Master noch im Beat-Sync ist, der Tempo-Abstand über 8 % liegt, ein Track zu kurz ist oder schon ein Übergang läuft |
| `transition_cancel` | – | – | verwirft alle vorgemerkten Befehle; die Regler bleiben stehen |
| `deck_load` | `deck: DeckId`, `trackId: string` | – | Track laden; erfüllt, wenn das Deck spielbereit ist |
| `deck_loaded` | – | `{ a, b, c, d: string \| null }` | Tracks auf den Decks |
| `track_waveform` | `trackId: string` | `ArrayBuffer` | Wellenform im Binärformat „DWWF" |
| `track_analysis` | `trackId: string` | `TrackAnalysis` | Analyseergebnis |
| `audio_devices` | – | `AudioDevice[]` | Ausgabegeräte aller Schnittstellen des Systems |
| `audio_select` | `device: string \| null` | – | Ausgabegerät wechseln (`null` = Standardgerät des Systems); wirkt sofort, geladene Decks bleiben erhalten. Lässt sich das Gerät nicht öffnen, wird abgelehnt und das bisherige läuft weiter |

Fehler kommen als abgelehntes Promise mit einem deutschen Klartext (`string`).

## Typen

```ts
interface Poll {
  snapshot: Snapshot;            // wie dw-core
  status: {
    dsp_load: number;            // 0–1, höchster Wert seit dem letzten Abruf
    xruns: number;
    sample_rate: number;
    buffer_size: number;         // 0 = Vorgabe des Geräts
    has_device: boolean;         // false: kein Audiogerät, Engine läuft stumm
    device_error: string | null;
    device: string | null;       // gewähltes Ausgabegerät; null = Standardgerät des Systems
  };
  library_rev: number;           // ändert sich → library_list neu abrufen
  pending_analysis: number;      // Tracks in der Analyse-Warteschlange
}

interface AudioDevice {
  host: string;                  // Schnittstelle, z. B. "WASAPI", "CoreAudio", "ALSA"
  name: string;
  is_default: boolean;           // Standardgerät des Systems
  max_channels: number;
  sample_rates: number[];
}

interface TrackInfo {
  id: string;
  title: string;
  artist: string;
  bpm: number;                   // 0, solange unbekannt
  key: number | null;            // 0–11 Dur (C … H), 12–23 Moll
  duration: number;              // Sekunden
  analyzed: boolean;
  missing: boolean;              // Datei liegt nicht mehr am gespeicherten Ort
}

interface ImportResult {
  added: number;
  duplicates: number;
  relinked: number;
  failed: [path: string, reason: string][];
}
```

Pegel von −∞ dB (Kill, Stille) stehen in JSON als `null`, in beiden Richtungen.

## Wellenform „DWWF"

Little Endian. Kopf: `"DWWF"`, Format-Version `u32` (= 1), Abtastrate `u32`, Frames `u64`, Anzahl Stufen `u32`. Danach je Stufe: Samples je Punkt `u32`, Anzahl Punkte `u32`, dann 9 Bytes je Punkt. Die erste Stufe ist die Übersicht mit 2 000 Punkten, danach folgt die Pyramide mit 64, 256, 1 024 und 4 096 Samples je Punkt.

Die 9 Bytes eines Punkts: Tief (min, max, rms), Mitte (min, max, rms), Hoch (min, max, rms). `min` und `max` sind vorzeichenbehaftete Bytes (Wert ÷ 127), `rms` ist vorzeichenlos (Wert ÷ 255).

Bei der Übersicht ist „Samples je Punkt" abgerundet; die genaue Zuordnung ergibt sich aus Frames ÷ Punkte.

## Ereignisse des Fensters

Dateien und Ordner, die auf das Fenster gezogen werden, importiert die Hülle selbst. Die Oberfläche bemerkt das über `library_rev`.

## Einstellungen

Das gewählte Ausgabegerät steht in `settings.json` im Datenordner (`{ "audio_device": string | null }`) und wird beim Start wieder geöffnet. Fehlt das Gerät dann, nimmt die Anwendung das Standardgerät des Systems. Die Engine behält über einen Gerätewechsel hinweg die Abtastrate, mit der sie gestartet ist.

## Umgebungsvariablen

| Variable | Wirkung |
|---|---|
| `DECKWERK_DATA` | Ordner für Bibliothek und Cache statt des Benutzerdaten-Ordners |
| `DECKWERK_MUSIC` | Ordner, der beim Start importiert wird |

## Übergang als Datenobjekt

`Transition` beschreibt einen Übergang unabhängig davon, ob er offline gerendert oder live gefahren wird (Format wie der Abschnitt „Automation" des Mix-Plans, Spezifikation 6.3):

| Feld | Bedeutung |
|---|---|
| `name` | Kennung der Vorlage oder frei gewählter Name |
| `bars` | Länge in Takten (1 … 256) |
| `start_in` | Start des kommenden Decks in Beats relativ zu T0; `null` = es läuft schon |
| `stop_out` | Stopp des gehenden Decks in Beats relativ zu T0; `null` = es läuft weiter |
| `reset_out` | nach dem Stopp die gefahrenen Regler des gehenden Decks in Ruhestellung bringen |
| `lanes` | Automationsspuren: `side` (`out` = gehendes, `in` = kommendes Deck), `param` (`fader` 0 … 1, `eq_low` / `eq_mid` / `eq_high` in dB mit `null` = Kill, `filter` −1 … +1), `points` |

Ein Stützpunkt hat `beat` (Master-Beats relativ zu T0, negativ = davor), `value` und `curve` (`linear`, `smooth`, `step`) für den Weg vom vorigen Punkt. Vor dem ersten Stützpunkt fasst die Spur den Regler nicht an. EQ-Wege von oder zu Kill laufen linear im Pegelfaktor, alle anderen linear im Wert. Gesendet wird in Schritten von einem Viertel-Beat; die Engine glättet dazwischen.

`TransitionRun`: `from`, `to`, `name`, `bars`, `t0_beat`, `end_beat` (Clock-Beats). Den Stand liest die Oberfläche aus `Snapshot.clock_beats`.
