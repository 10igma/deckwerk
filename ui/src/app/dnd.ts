// Ziehen von Tracks aus der Liste auf ein Deck (DECK-04).
//
// Bewusst mit Zeiger-Ereignissen statt mit dem Drag-and-drop des Browsers: Die Tauri-Hülle
// fängt mit `dragDropEnabled` (nötig für den Import durch Ablegen von Dateien) jedes Ziehen
// über dem Fenster ab, unter macOS und Windows erreichen `dragover` und `drop` die Seite dann
// nicht mehr. Zeiger-Ereignisse sind davon unberührt.
import { useSyncExternalStore } from 'react';
import type { DeckId, TrackId } from '../core/types';
import { loadByDrop } from './actions';

/** Weg in CSS-Pixeln, ab dem aus einem Druck ein Ziehen wird; darunter bleibt es ein Klick. */
export const DRAG_THRESHOLD = 5;

/** Attribut, das ein Element als Ablageziel eines Decks ausweist. */
export const DROP_ATTR = 'data-drop-deck';

export interface DragView {
  trackId: TrackId;
  /** Deck unter dem Zeiger, solange gezogen wird. */
  over: DeckId | null;
}

/**
 * Ablauf eines Zugs ohne Bezug zum Dokument: Druck → (Weg über der Schwelle) Ziehen → Loslassen.
 * Gibt beim Loslassen das Zieldeck zurück, wenn über einem Deck losgelassen wurde.
 */
export class TrackDrag {
  private pending: { trackId: TrackId; x: number; y: number } | null = null;
  private active: DragView | null = null;

  /** Zustand für die Anzeige; `null`, solange nicht gezogen wird. */
  get view(): DragView | null {
    return this.active;
  }

  down(trackId: TrackId, x: number, y: number): void {
    this.pending = { trackId, x, y };
    this.active = null;
  }

  /** Meldet `true`, wenn sich der sichtbare Zustand geändert hat. */
  move(x: number, y: number, over: DeckId | null): boolean {
    if (!this.pending) return false;
    if (!this.active) {
      if (Math.hypot(x - this.pending.x, y - this.pending.y) < DRAG_THRESHOLD) return false;
      this.active = { trackId: this.pending.trackId, over };
      return true;
    }
    if (this.active.over === over) return false;
    this.active = { ...this.active, over };
    return true;
  }

  /** Loslassen: Ziel und Track, falls ein Zug über einem Deck endet. */
  up(over: DeckId | null): { deck: DeckId; trackId: TrackId } | null {
    const drag = this.active;
    this.cancel();
    return drag && over ? { deck: over, trackId: drag.trackId } : null;
  }

  cancel(): void {
    this.pending = null;
    this.active = null;
  }
}

const drag = new TrackDrag();
const listeners = new Set<() => void>();
let ghost: HTMLElement | null = null;

function publish(): void {
  const view = drag.view;
  document.documentElement.toggleAttribute('data-track-drag', view !== null);
  if (!view) {
    ghost?.remove();
    ghost = null;
  }
  listeners.forEach((fn) => fn());
}

function deckAt(x: number, y: number): DeckId | null {
  const target = document.elementFromPoint(x, y)?.closest(`[${DROP_ATTR}]`);
  return (target?.getAttribute(DROP_ATTR) as DeckId | null | undefined) ?? null;
}

function moveGhost(x: number, y: number, label: string): void {
  if (!ghost) {
    ghost = document.createElement('div');
    ghost.className = 'drag-ghost dw-label';
    ghost.textContent = label;
    document.body.appendChild(ghost);
  }
  ghost.style.transform = `translate(${x}px, ${y}px)`;
}

/** Beginnt einen möglichen Zug; aufzurufen beim Druck auf eine Zeile der Liste. */
export function beginTrackDrag(e: React.PointerEvent, trackId: TrackId, label: string): void {
  if (e.button !== 0) return;
  drag.down(trackId, e.clientX, e.clientY);

  const onMove = (ev: PointerEvent) => {
    const changed = drag.move(ev.clientX, ev.clientY, deckAt(ev.clientX, ev.clientY));
    if (drag.view) moveGhost(ev.clientX, ev.clientY, label);
    if (changed) publish();
  };
  const finish = () => {
    window.removeEventListener('pointermove', onMove);
    window.removeEventListener('pointerup', onUp);
    window.removeEventListener('pointercancel', onCancel);
    window.removeEventListener('keydown', onKey, true);
    publish();
  };
  const onUp = (ev: PointerEvent) => {
    const hit = drag.up(deckAt(ev.clientX, ev.clientY));
    finish();
    if (hit) loadByDrop(hit.deck, hit.trackId);
  };
  const onCancel = () => {
    drag.cancel();
    finish();
  };
  const onKey = (ev: KeyboardEvent) => {
    if (ev.key !== 'Escape' || !drag.view) return;
    ev.stopPropagation();
    onCancel();
  };
  window.addEventListener('pointermove', onMove);
  window.addEventListener('pointerup', onUp);
  window.addEventListener('pointercancel', onCancel);
  window.addEventListener('keydown', onKey, true);
}

function subscribe(fn: () => void): () => void {
  listeners.add(fn);
  return () => listeners.delete(fn);
}

/** Kennung des Tracks, der gerade gezogen wird. */
export function useDraggedTrack(): TrackId | null {
  return useSyncExternalStore(subscribe, () => drag.view?.trackId ?? null);
}

/** Ablageziel für ein Deck. `over` ist wahr, solange ein Track darüber schwebt. */
export function useTrackDrop(deck: DeckId) {
  const over = useSyncExternalStore(subscribe, () => drag.view?.over === deck);
  return { over, handlers: { [DROP_ATTR]: deck } };
}
