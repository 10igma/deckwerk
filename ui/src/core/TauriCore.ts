// Anschluss an den echten Kern in der Desktop-Hülle (Tauri 2). Bildet `CoreApi` auf die
// Befehle aus app/src/commands.rs ab (docs/kern-schnittstelle.md).
import type { CoreApi, Unsubscribe } from './api';
import { decodeWaveform } from './dwwf';
import {
  emptyEngineStatus,
  emptySnapshot,
  type AudioDevice,
  type Command,
  type DeckId,
  type EngineStatus,
  type AppSettings,
  type ImportResult,
  type RecordingInfo,
  type RefreshResult,
  type PairTransition,
  type Transition,
  type TransitionRun,
  type TransitionWhen,
  type LibraryStatus,
  type LoadedTracks,
  type Poll,
  type Snapshot,
  type TrackAnalysis,
  type TrackId,
  type TrackInfo,
  type Waveform,
} from './types';

export type Invoke = <T>(cmd: string, args?: Record<string, unknown>) => Promise<T>;
/** Taktgeber für den Abruf: liefert eine Funktion zum Abbrechen. Standard: requestAnimationFrame. */
export type Schedule = (tick: () => void) => () => void;

const frameSchedule: Schedule = (tick) => {
  let id = requestAnimationFrame(function loop() {
    tick();
    id = requestAnimationFrame(loop);
  });
  return () => cancelAnimationFrame(id);
};

function sameStatus(a: EngineStatus, b: EngineStatus): boolean {
  return (
    a.dsp_load === b.dsp_load &&
    a.xruns === b.xruns &&
    a.sample_rate === b.sample_rate &&
    a.buffer_size === b.buffer_size &&
    a.has_device === b.has_device &&
    a.device_error === b.device_error &&
    a.device === b.device
  );
}

export class TauriCore implements CoreApi {
  private snapshot: Snapshot = emptySnapshot();
  private status: EngineStatus = emptyEngineStatus();
  private library: LibraryStatus = { rev: 0, pending_analysis: 0 };
  private snapListeners = new Set<(s: Snapshot) => void>();
  private statusListeners = new Set<(s: EngineStatus) => void>();
  private libraryListeners = new Set<(s: LibraryStatus) => void>();
  private polling = false;
  private stop: () => void;
  private lastError = '';
  /** Zählt die Abrufe (Diagnose und Tests). */
  polls = 0;

  constructor(
    private invoke: Invoke,
    schedule: Schedule = frameSchedule,
  ) {
    this.stop = schedule(() => this.poll());
  }

  dispose(): void {
    this.stop();
  }

  /** Ein Abruf je Bild; solange der vorige läuft, wird keiner gestartet. */
  private poll(): void {
    if (this.polling) return;
    this.polling = true;
    this.polls += 1;
    this.invoke<Poll>('core_poll')
      .then((p) => this.apply(p))
      .catch((err) => this.report('core_poll', err))
      .finally(() => {
        this.polling = false;
      });
  }

  private apply(p: Poll): void {
    this.snapshot = p.snapshot;
    for (const l of this.snapListeners) l(p.snapshot);
    if (!sameStatus(p.status, this.status)) {
      this.status = p.status;
      for (const l of this.statusListeners) l(p.status);
    }
    if (p.library_rev !== this.library.rev || p.pending_analysis !== this.library.pending_analysis) {
      this.library = { rev: p.library_rev, pending_analysis: p.pending_analysis };
      for (const l of this.libraryListeners) l(this.library);
    }
  }

  /** Fehler der Hülle einmal je Text melden, nicht in jedem Bild. */
  private report(where: string, err: unknown): void {
    const text = `${where}: ${typeof err === 'string' ? err : err instanceof Error ? err.message : String(err)}`;
    if (text === this.lastError) return;
    this.lastError = text;
    console.error(text);
  }

  // --- CoreApi -------------------------------------------------------------

  send(cmd: Command): void {
    this.invoke<void>('core_send', { cmd }).catch((err) => this.report('core_send', err));
  }

  subscribe(listener: (s: Snapshot) => void): Unsubscribe {
    this.snapListeners.add(listener);
    listener(this.snapshot);
    return () => this.snapListeners.delete(listener);
  }

  subscribeStatus(listener: (s: EngineStatus) => void): Unsubscribe {
    this.statusListeners.add(listener);
    listener(this.status);
    return () => this.statusListeners.delete(listener);
  }

  subscribeLibrary(listener: (s: LibraryStatus) => void): Unsubscribe {
    this.libraryListeners.add(listener);
    listener(this.library);
    return () => this.libraryListeners.delete(listener);
  }

  listTracks(): Promise<TrackInfo[]> {
    return this.invoke<TrackInfo[]>('library_list');
  }

  importPaths(paths: string[]): Promise<ImportResult> {
    return this.invoke<ImportResult>('library_import', { paths });
  }

  loadTrack(deck: DeckId, trackId: TrackId): Promise<void> {
    return this.invoke<void>('deck_load', { deck, trackId });
  }

  getLoadedTracks(): Promise<LoadedTracks> {
    return this.invoke<LoadedTracks>('deck_loaded');
  }

  async getWaveform(trackId: TrackId): Promise<Waveform> {
    const raw = await this.invoke<ArrayBuffer | Uint8Array | number[]>('track_waveform', { trackId });
    // Je nach Weg der Übertragung kommt ein ArrayBuffer, ein Uint8Array oder ein Zahlenfeld an.
    const bytes = raw instanceof ArrayBuffer || raw instanceof Uint8Array ? raw : Uint8Array.from(raw);
    return decodeWaveform(bytes);
  }

  getAnalysis(trackId: TrackId): Promise<TrackAnalysis> {
    return this.invoke<TrackAnalysis>('track_analysis', { trackId });
  }

  refreshLibrary(): Promise<RefreshResult> {
    return this.invoke<RefreshResult>('library_refresh');
  }

  getSettings(): Promise<AppSettings> {
    return this.invoke<AppSettings>('settings_get');
  }

  setRefreshOnStart(on: boolean): Promise<void> {
    return this.invoke<void>('settings_refresh_on_start', { on });
  }

  listTransitions(bars: number): Promise<Transition[]> {
    return this.invoke<Transition[]>('transition_templates', { bars });
  }

  startTransition(from: DeckId, to: DeckId, transition: Transition, when: TransitionWhen): Promise<TransitionRun> {
    return this.invoke<TransitionRun>('transition_start', { from, to, transition, when });
  }

  previewTransition(from: DeckId, to: DeckId, transition: Transition, when: TransitionWhen, fromBeat: number): Promise<TransitionRun> {
    return this.invoke<TransitionRun>('transition_preview', { from, to, transition, when, fromBeat });
  }

  stopPreview(): Promise<void> {
    return this.invoke<void>('transition_preview_stop');
  }

  listPairTransitions(a: DeckId, b: DeckId): Promise<PairTransition[]> {
    return this.invoke<PairTransition[]>('pair_list', { a, b });
  }

  savePairTransition(from: DeckId, to: DeckId, transition: Transition, when: TransitionWhen): Promise<PairTransition[]> {
    return this.invoke<PairTransition[]>('pair_save', { from, to, transition, when });
  }

  deletePairTransition(id: number): Promise<void> {
    return this.invoke<void>('pair_delete', { id });
  }

  cancelTransition(): Promise<void> {
    return this.invoke<void>('transition_cancel');
  }

  listSavedTransitions(): Promise<Transition[]> {
    return this.invoke<Transition[]>('transition_saved');
  }

  saveTransition(transition: Transition): Promise<Transition[]> {
    return this.invoke<Transition[]>('transition_save', { transition });
  }

  deleteTransition(name: string): Promise<Transition[]> {
    return this.invoke<Transition[]>('transition_delete', { name });
  }

  startRecording(stamp: string): Promise<RecordingInfo> {
    return this.invoke<RecordingInfo>('record_start', { stamp });
  }

  stopRecording(): Promise<RecordingInfo> {
    return this.invoke<RecordingInfo>('record_stop');
  }

  recordingStatus(): Promise<RecordingInfo | null> {
    return this.invoke<RecordingInfo | null>('record_status');
  }

  listAudioDevices(): Promise<AudioDevice[]> {
    return this.invoke<AudioDevice[]>('audio_devices');
  }

  selectAudioDevice(name: string | null): Promise<void> {
    return this.invoke<void>('audio_select', { device: name });
  }
}
