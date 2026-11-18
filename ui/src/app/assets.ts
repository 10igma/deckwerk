// Wellenform und Analyse der geladenen Tracks – außerhalb von React, für die Zeichner.
import type { CoreApi } from '../core/api';
import type { DeckId, TrackAnalysis, TrackId, Waveform } from '../core/types';

export interface DeckAssets {
  trackId: TrackId;
  waveform: Waveform;
  analysis: TrackAnalysis;
}

const assets: Record<DeckId, DeckAssets | null> = { a: null, b: null, c: null, d: null };
const pending: Record<DeckId, TrackId | null> = { a: null, b: null, c: null, d: null };

export const getDeckAssets = (deck: DeckId): DeckAssets | null => assets[deck];

export function clearDeckAssets(deck: DeckId): void {
  assets[deck] = null;
  pending[deck] = null;
}

/** Daten eines Tracks für ein Deck holen. Ein späterer Ladevorgang gewinnt. */
export async function loadDeckAssets(core: CoreApi, deck: DeckId, trackId: TrackId): Promise<DeckAssets | null> {
  pending[deck] = trackId;
  const [waveform, analysis] = await Promise.all([core.getWaveform(trackId), core.getAnalysis(trackId)]);
  if (pending[deck] !== trackId) return null;
  const a: DeckAssets = { trackId, waveform, analysis };
  assets[deck] = a;
  return a;
}
