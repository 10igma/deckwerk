# Schnittstelle zwischen Oberfläche und Desktop-Hülle

Die Oberfläche spricht den Kern nur über `CoreApi` (`ui/src/core/api.ts`) an. In der Desktop-Anwendung bildet ein Adapter diese Schnittstelle auf die folgenden Tauri-Befehle ab (`app/src/commands.rs`). Aufruf mit `invoke(name, args)` aus `@tauri-apps/api/core`; Argumentnamen in camelCase.

| Befehl | Argumente | Ergebnis | Zweck |
|---|---|---|---|
| `core_send` | `cmd: Command` | – | Bedienbefehl an die Engine |
| `core_poll` | – | `Poll` | Schnappschuss, Engine-Kennzahlen, Stand der Bibliothek; je Bild einmal aufrufen |
| `library_list` | – | `TrackInfo[]` | alle Tracks, nach Titel sortiert |
| `library_import` | `paths: string[]` | `ImportResult` | Dateien und Ordner importieren (rekursiv) |
| `deck_load` | `deck: DeckId`, `trackId: string` | – | Track laden; erfüllt, wenn das Deck spielbereit ist |
| `deck_loaded` | – | `{ a, b, c, d: string \| null }` | Tracks auf den Decks |
| `track_waveform` | `trackId: string` | `ArrayBuffer` | Wellenform im Binärformat „DWWF" |
| `track_analysis` | `trackId: string` | `TrackAnalysis` | Analyseergebnis |
| `audio_devices` | – | `DeviceEntry[]` | Ausgabegeräte |

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
  };
  library_rev: number;           // ändert sich → library_list neu abrufen
  pending_analysis: number;      // Tracks in der Analyse-Warteschlange
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

## Umgebungsvariablen

| Variable | Wirkung |
|---|---|
| `DECKWERK_DATA` | Ordner für Bibliothek und Cache statt des Benutzerdaten-Ordners |
| `DECKWERK_MUSIC` | Ordner, der beim Start importiert wird |
