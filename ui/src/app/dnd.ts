// Drag-and-drop von Tracks aus der Liste auf ein Deck (DECK-04).
import { useState } from 'react';
import type { DeckId, TrackId } from '../core/types';
import { loadTrack } from './actions';

export const TRACK_MIME = 'application/x-deckwerk-track';

export function startTrackDrag(e: React.DragEvent, trackId: TrackId): void {
  e.dataTransfer.setData(TRACK_MIME, trackId);
  e.dataTransfer.setData('text/plain', trackId);
  e.dataTransfer.effectAllowed = 'copy';
}

/** Ablageziel für ein Deck. `over` ist wahr, solange ein Track darüber schwebt. */
export function useTrackDrop(deck: DeckId) {
  const [over, setOver] = useState(false);
  const accepts = (e: React.DragEvent) => e.dataTransfer.types.includes(TRACK_MIME);
  return {
    over,
    handlers: {
      onDragEnter: (e: React.DragEvent) => {
        if (accepts(e)) setOver(true);
      },
      onDragOver: (e: React.DragEvent) => {
        if (!accepts(e)) return;
        e.preventDefault();
        e.dataTransfer.dropEffect = 'copy';
        setOver(true);
      },
      onDragLeave: (e: React.DragEvent) => {
        if (!e.currentTarget.contains(e.relatedTarget as Node | null)) setOver(false);
      },
      onDrop: (e: React.DragEvent) => {
        e.preventDefault();
        setOver(false);
        const id = e.dataTransfer.getData(TRACK_MIME);
        if (id) void loadTrack(deck, id);
      },
    },
  };
}
