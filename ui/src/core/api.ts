import type {
  Command,
  DeckId,
  EngineStatus,
  LoadedTracks,
  Snapshot,
  TrackAnalysis,
  TrackId,
  TrackInfo,
  Waveform,
} from './types';

export type Unsubscribe = () => void;

/**
 * Einzige Schnittstelle zwischen Oberfläche und Kern.
 * Der MockCore und die spätere Tauri-Anbindung implementieren sie gleich.
 */
export interface CoreApi {
  /** Bedienbefehl an die Engine senden. */
  send(cmd: Command): void;
  /** Zustands-Schnappschüsse (ca. 60 je Sekunde). Der letzte Stand wird sofort geliefert. */
  subscribe(listener: (s: Snapshot) => void): Unsubscribe;
  /** Engine-Kennzahlen (Last, Xruns), ca. 4 je Sekunde. */
  subscribeStatus(listener: (s: EngineStatus) => void): Unsubscribe;
  listTracks(): Promise<TrackInfo[]>;
  /** Lädt einen Track auf ein Deck; erfüllt, sobald das Deck spielbereit ist. */
  loadTrack(deck: DeckId, trackId: TrackId): Promise<void>;
  /** Welche Tracks liegen auf den Decks (z. B. nach einem Neustart, BED-08). */
  getLoadedTracks(): Promise<LoadedTracks>;
  getWaveform(trackId: TrackId): Promise<Waveform>;
  getAnalysis(trackId: TrackId): Promise<TrackAnalysis>;
}
