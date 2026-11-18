// Letzter Snapshot des Kerns und die Bild-Schleife. Animierte Elemente zeichnen hier
// direkt aus dem Snapshot; React rendert nur, wenn sich ein ausgewählter Wert ändert.
import { useEffect, useRef, useSyncExternalStore } from 'react';
import type { CoreApi } from '../core/api';
import { emptyEngineStatus, emptySnapshot, type EngineStatus, type Snapshot } from '../core/types';

export type FrameCallback = (s: Snapshot, timeMs: number, dtMs: number) => void;

let current: Snapshot = emptySnapshot();
let status: EngineStatus = emptyEngineStatus();
const snapListeners = new Set<() => void>();
const statusListeners = new Set<() => void>();
const frameCallbacks = new Set<FrameCallback>();
let rafId = 0;
let lastTime = 0;

function loop(time: number): void {
  const dt = lastTime === 0 ? 16 : Math.min(100, time - lastTime);
  lastTime = time;
  for (const cb of frameCallbacks) cb(current, time, dt);
  rafId = requestAnimationFrame(loop);
}

/** Kern anschließen; liefert eine Funktion zum Trennen. */
export function attachCore(core: CoreApi): () => void {
  const offSnap = core.subscribe((s) => {
    current = s;
    for (const l of snapListeners) l();
  });
  const offStatus = core.subscribeStatus((s) => {
    status = s;
    for (const l of statusListeners) l();
  });
  if (!rafId) rafId = requestAnimationFrame(loop);
  return () => {
    offSnap();
    offStatus();
    cancelAnimationFrame(rafId);
    rafId = 0;
    lastTime = 0;
  };
}

export const latestSnapshot = (): Snapshot => current;

function subscribeSnap(l: () => void): () => void {
  snapListeners.add(l);
  return () => snapListeners.delete(l);
}

/**
 * Einen einzelnen Wert aus dem Snapshot beobachten. Der Selektor muss einen
 * primitiven Wert liefern; nur dessen Änderung löst ein Rendern aus.
 */
export function useSnap<T extends string | number | boolean | null>(select: (s: Snapshot) => T): T {
  return useSyncExternalStore(subscribeSnap, () => select(current));
}

export function useEngineStatus(): EngineStatus {
  return useSyncExternalStore(
    (l) => {
      statusListeners.add(l);
      return () => statusListeners.delete(l);
    },
    () => status,
  );
}

/** Rückruf in der Bild-Schleife (requestAnimationFrame), außerhalb von React. */
export function useFrame(cb: FrameCallback): void {
  const ref = useRef(cb);
  ref.current = cb;
  useEffect(() => {
    const fn: FrameCallback = (s, t, dt) => ref.current(s, t, dt);
    frameCallbacks.add(fn);
    return () => {
      frameCallbacks.delete(fn);
    };
  }, []);
}
