// Typen der Befehlsschicht – Spiegel von crates/dw-core/src/command.rs und analysis.rs.
// Feldnamen und Werte entsprechen dem serde-Format (JSON) des Kerns.

export type DeckId = 'a' | 'b' | 'c' | 'd';
export const DECK_IDS: readonly DeckId[] = ['a', 'b', 'c', 'd'];
export const deckIndex = (d: DeckId): number => DECK_IDS.indexOf(d);

export type Band = 'low' | 'mid' | 'high';
export const BANDS: readonly Band[] = ['low', 'mid', 'high'];
export const bandIndex = (b: Band): number => BANDS.indexOf(b);

export type SyncMode = 'off' | 'tempo' | 'beat';
export type XfAssign = 'left' | 'right' | 'thru';

/** Pegel in dB. `null` steht für −∞ (serde_json überträgt nicht-endliche Zahlen als null). */
export type Db = number | null;

/** Bedienbefehl (Rust: `Command`, `#[serde(tag = "type", rename_all = "snake_case")]`). */
export type Command =
  | { type: 'play'; deck: DeckId; on: boolean }
  /** Start mit Vorlauf (VIS-13): Das Deck erreicht seine jetzige Stelle in `beats` Beats der Clock. */
  | { type: 'play_in'; deck: DeckId; beats: number }
  /** Position in Sekunden. */
  | { type: 'seek'; deck: DeckId; pos: number }
  /** Sprung im Takt (DECK-20): Der Kern verschiebt das Ziel im laufenden Deck um höchstens einen halben Beat, sodass die Phase bleibt. */
  | { type: 'seek_snap'; deck: DeckId; pos: number }
  /** Tempo-Regler als Faktor, 0.02 = +2 %. */
  | { type: 'tempo'; deck: DeckId; offset: number }
  /** Anschieben/Abbremsen, 0 = losgelassen. */
  | { type: 'bend'; deck: DeckId; amount: number }
  | { type: 'sync'; deck: DeckId; mode: SyncMode }
  /** `null` = freie Clock. */
  | { type: 'set_master'; deck: DeckId | null }
  | { type: 'clock_tempo'; bpm: number }
  | { type: 'gain'; deck: DeckId; db: Db }
  /** `null` = Kill, Maximum +6. */
  | { type: 'eq'; deck: DeckId; band: Band; db: Db }
  /** −1 … +1, 0 = aus. */
  | { type: 'filter'; deck: DeckId; pos: number }
  /** 0 … 1. */
  | { type: 'fader'; deck: DeckId; value: number }
  /** Vorhören auf dem Kopfhörer. */
  | { type: 'cue'; deck: DeckId; on: boolean }
  | { type: 'xf_assign'; deck: DeckId; assign: XfAssign }
  /** −1 (links) … +1 (rechts). */
  | { type: 'crossfader'; pos: number }
  | { type: 'master_volume'; db: Db }
  | { type: 'limiter'; on: boolean }
  /** 0 = nur Cue, 1 = nur Master. */
  | { type: 'cue_mix'; value: number }
  | { type: 'cue_volume'; db: Db };

export type CommandType = Command['type'];

/** Rust: `DeckState`. */
export interface DeckState {
  loaded: boolean;
  playing: boolean;
  /** Position in Sekunden. */
  pos: number;
  duration: number;
  /** Beat-Nummer im Beatgrid des Tracks (mit Nachkommaanteil). */
  beat: number;
  track_bpm: number;
  /** Aktuell hörbares Tempo. */
  bpm: number;
  /** Tempo-Abweichung als Faktor (Regler bzw. Sync). */
  tempo_offset: number;
  sync: SyncMode;
  is_master: boolean;
  /** Versatz zur Master-Clock in Beats, −0,5 … +0,5. */
  phase: number;
  /** Kanalpegel vor dem Fader, Spitze links/rechts (linear). */
  peak: [number, number];
  gain_db: Db;
  /** Reihenfolge wie `Band`: Tief, Mitte, Hoch. */
  eq_db: [Db, Db, Db];
  filter: number;
  fader: number;
  cue: boolean;
  xf_assign: XfAssign;
}

/** Rust: `MixerState`. */
export interface MixerState {
  crossfader: number;
  master_db: Db;
  limiter_on: boolean;
  /** Pegelreduktion des Limiters in dB (≤ 0). */
  limiter_gr_db: number;
  master_peak: [number, number];
  cue_mix: number;
  cue_db: Db;
}

/** Rust: `Snapshot`. */
export interface Snapshot {
  clock_bpm: number;
  clock_beats: number;
  master: DeckId | null;
  decks: [DeckState, DeckState, DeckState, DeckState];
  mixer: MixerState;
  /** Bisher verarbeitete Frames. */
  frames: number;
}

/** Rust: `Key(u8)` – Index 0–11 = Dur (C … H), 12–23 = Moll (c … h). */
export type Key = number;

/** Rust: `GridAnchor`. */
export interface GridAnchor {
  pos: number;
  bpm: number;
}

/** Rust: `BeatGrid`. */
export interface BeatGrid {
  anchors: GridAnchor[];
  anchor_beats: number[];
  bar_offset: number;
}

/** Rust: `TrackAnalysis`. */
export interface TrackAnalysis {
  version: number;
  duration: number;
  bpm: number;
  bpm_confidence: number;
  grid: BeatGrid;
  key: Key | null;
  key_confidence: number;
  tuning_cents: number;
  /** `null` = −∞ (Stille). */
  lufs: number | null;
  lra: number;
  true_peak_db: number | null;
}

/** Rust: `WaveformLevel`. serde liefert `number[]`; ein Adapter darf ein Float32Array übergeben. */
export interface WaveformLevel {
  samples_per_point: number;
  /** Je Punkt 9 Werte: Tief (min, max, rms), Mitte (min, max, rms), Hoch (min, max, rms). */
  data: ArrayLike<number>;
}
export const VALUES_PER_POINT = 9;

/** Rust: `Waveform`. */
export interface Waveform {
  sample_rate: number;
  frames: number;
  /** Feste Übersicht mit 2 000 Punkten. */
  overview: WaveformLevel;
  /** Pyramide mit 64, 256, 1 024 und 4 096 Samples je Punkt. */
  levels: WaveformLevel[];
}

// ---------------------------------------------------------------------------
// Ab hier: Typen der Desktop-Hülle (docs/kern-schnittstelle.md, app/src/commands.rs).
// ---------------------------------------------------------------------------

export type TrackId = string;

/** Zeile der Bibliothek (`library_list`). */
export interface TrackInfo {
  id: TrackId;
  title: string;
  artist: string;
  /** 0, solange unbekannt (noch nicht analysiert und keine Angabe in der Datei). */
  bpm: number;
  key: Key | null;
  /** Dauer in Sekunden. */
  duration: number;
  analyzed: boolean;
  /** Datei liegt nicht mehr am gespeicherten Ort. */
  missing: boolean;
  /** Die Datei enthält weniger Audio, als sie ankündigt (abgebrochene Kopie, BIB-20). */
  incomplete: boolean;
}

/** Engine-Kennzahlen (`Poll.status`, ENG-09). */
export interface EngineStatus {
  /** DSP-Last 0–1, höchster Wert seit dem letzten Abruf. */
  dsp_load: number;
  xruns: number;
  sample_rate: number;
  /** 0 = Vorgabe des Geräts. */
  buffer_size: number;
  /** false: kein Audiogerät, die Engine läuft stumm. */
  has_device: boolean;
  device_error: string | null;
  /** Gewähltes Ausgabegerät; null = Standardgerät des Systems. */
  device: string | null;
}

/** Ausgabegerät für die Einstellungen (`audio_devices`, ENG-01). */
export interface AudioDevice {
  /** Schnittstelle, z. B. „WASAPI", „CoreAudio", „ALSA". */
  host: string;
  name: string;
  /** Standardgerät des Systems. */
  is_default: boolean;
  max_channels: number;
  sample_rates: number[];
}

/** Antwort von `core_poll`. */
export interface Poll {
  snapshot: Snapshot;
  status: EngineStatus;
  /** Ändert sich → Bibliothek neu abrufen. */
  library_rev: number;
  /** Tracks in der Analyse-Warteschlange. */
  pending_analysis: number;
}

/** Stand der Bibliothek, wie ihn die Oberfläche beobachtet. */
export interface LibraryStatus {
  rev: number;
  pending_analysis: number;
}

/** Ergebnis von `library_import`. */
export interface ImportResult {
  added: number;
  duplicates: number;
  relinked: number;
  failed: [path: string, reason: string][];
}

/** Ergebnis von `library_refresh`. */
export interface RefreshResult {
  /** Durchsuchte Musikordner. */
  folders: number;
  /** Musikordner, die gerade nicht erreichbar sind. */
  unreachable: number;
  added: number;
  relinked: number;
  /** Tracks der Bibliothek, deren Datei fehlt. */
  missing: number;
  failed: [path: string, reason: string][];
}

/** Einstellungen der Hülle (`settings_get`). */
export interface AppSettings {
  refresh_on_start: boolean;
  music_folders: string[];
}

export type LoadedTracks = Record<DeckId, TrackId | null>;

export const emptyEngineStatus = (): EngineStatus => ({
  dsp_load: 0,
  xruns: 0,
  sample_rate: 48000,
  buffer_size: 0,
  has_device: true,
  device_error: null,
  device: null,
});

export function emptyDeckState(): DeckState {
  return {
    loaded: false,
    playing: false,
    pos: 0,
    duration: 0,
    beat: 0,
    track_bpm: 0,
    bpm: 0,
    tempo_offset: 0,
    sync: 'off',
    is_master: false,
    phase: 0,
    peak: [0, 0],
    gain_db: 0,
    eq_db: [0, 0, 0],
    filter: 0,
    fader: 1,
    cue: false,
    xf_assign: 'thru',
  };
}

export function emptySnapshot(): Snapshot {
  return {
    clock_bpm: 0,
    clock_beats: 0,
    master: null,
    decks: [emptyDeckState(), emptyDeckState(), emptyDeckState(), emptyDeckState()],
    mixer: {
      crossfader: 0,
      master_db: 0,
      limiter_on: true,
      limiter_gr_db: 0,
      master_peak: [0, 0],
      cue_mix: 0.5,
      cue_db: 0,
    },
    frames: 0,
  };
}
