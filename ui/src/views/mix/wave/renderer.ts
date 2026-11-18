import type { DeckId, Waveform } from '../../../core/types';
import type { StageColors, StageScene } from './scene';

/** Gemeinsame Schnittstelle der Zeichner für die parallelen Wellenformen. */
export interface WaveRenderer {
  readonly kind: 'webgl2' | 'canvas2d';
  /** Kurzbeschreibung für die Einstellungen (z. B. Name des Grafiktreibers). */
  readonly info: string;
  /** Wellenform eines Decks übernehmen (null = entladen). */
  setTrack(deck: DeckId, waveform: Waveform | null): void;
  draw(scene: StageScene, colors: StageColors): void;
  dispose(): void;
}
