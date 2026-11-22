import type {
  AudioDevice,
  Command,
  DeckId,
  EngineStatus,
  AppSettings,
  ImportResult,
  RecordingInfo,
  RefreshResult,
  Transition,
  TransitionRun,
  TransitionWhen,
  LibraryStatus,
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
 * MockCore (Browser) und TauriCore (Desktop-Hülle) implementieren sie gleich.
 */
export interface CoreApi {
  /** Bedienbefehl an die Engine senden. */
  send(cmd: Command): void;
  /** Zustands-Schnappschüsse (einer je Bild). Der letzte Stand wird sofort geliefert. */
  subscribe(listener: (s: Snapshot) => void): Unsubscribe;
  /** Engine-Kennzahlen (Last, Xruns, Gerätestatus); nur bei Änderung. */
  subscribeStatus(listener: (s: EngineStatus) => void): Unsubscribe;
  /** Stand der Bibliothek: Ändert sich `rev`, muss `listTracks` neu abgerufen werden. */
  subscribeLibrary(listener: (s: LibraryStatus) => void): Unsubscribe;
  listTracks(): Promise<TrackInfo[]>;
  /** Dateien und Ordner importieren (Ordner rekursiv). */
  importPaths(paths: string[]): Promise<ImportResult>;
  /** Musikordner nach neuen Dateien durchsuchen; bekannte Tracks bleiben unberührt (BIB-19). */
  refreshLibrary(): Promise<RefreshResult>;
  getSettings(): Promise<AppSettings>;
  /** Beim Start die Musikordner durchsuchen. */
  setRefreshOnStart(on: boolean): Promise<void>;
  /**
   * Lädt einen Track auf ein Deck; erfüllt, sobald das Deck spielbereit ist (kann Sekunden dauern).
   * Abgelehnt mit einem Klartext (string oder Error), wenn das Laden scheitert.
   */
  loadTrack(deck: DeckId, trackId: TrackId): Promise<void>;
  /** Welche Tracks liegen auf den Decks (z. B. nach einem Neustart, BED-08). */
  getLoadedTracks(): Promise<LoadedTracks>;
  getWaveform(trackId: TrackId): Promise<Waveform>;
  getAnalysis(trackId: TrackId): Promise<TrackAnalysis>;
  /** Eingebaute Vorlagen für Übergänge in der gewünschten Länge (TRANS-01). */
  listTransitions(bars: number): Promise<Transition[]>;
  /**
   * Plant einen Übergang vom laufenden Deck `from` zum Deck `to` ein (TRANS-05); der Kern fährt die Regler.
   * `when` legt fest, ob er an der nächsten Taktgrenze beginnt oder an einer vorgegebenen Stelle endet. Steht
   * `to`, wird es gestellt und gestartet; läuft es schon im Takt mit, werden nur die Regler gefahren.
   * Abgelehnt mit Klartext, wenn die Voraussetzungen fehlen.
   */
  startTransition(from: DeckId, to: DeckId, transition: Transition, when: TransitionWhen): Promise<TransitionRun>;
  /** Selbst gebaute Übergänge (TRANS-04). */
  listSavedTransitions(): Promise<Transition[]>;
  /** Speichert einen Übergang unter seinem Namen (ersetzt einen gleichnamigen); liefert die neue Liste. */
  saveTransition(transition: Transition): Promise<Transition[]>;
  deleteTransition(name: string): Promise<Transition[]>;
  /** Bricht den eingeplanten oder laufenden Übergang ab; die Regler bleiben stehen. */
  cancelTransition(): Promise<void>;
  /** Mitschnitt des Master-Signals beginnen (REC-01); `stamp` ist der Zeitstempel für den Dateinamen. */
  startRecording(stamp: string): Promise<RecordingInfo>;
  /** Mitschnitt beenden; liefert Pfad und Länge der Datei. */
  stopRecording(): Promise<RecordingInfo>;
  /** Laufender Mitschnitt, sonst null. */
  recordingStatus(): Promise<RecordingInfo | null>;
  /** Ausgabegeräte des Systems (ENG-01). */
  listAudioDevices(): Promise<AudioDevice[]>;
  /**
   * Wechselt das Ausgabegerät; null = Standardgerät des Systems. Geladene Decks bleiben erhalten.
   * Abgelehnt mit einem Klartext, wenn sich das Gerät nicht öffnen lässt; dann läuft das bisherige weiter.
   */
  selectAudioDevice(name: string | null): Promise<void>;
}

/** Fehlertext aus einer Ablehnung der Hülle (dort ein string) oder einem Error. */
export function errorText(err: unknown): string {
  if (typeof err === 'string') return err;
  if (err instanceof Error) return err.message;
  return String(err);
}
