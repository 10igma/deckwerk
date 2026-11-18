// Gemeinsame Regler-Bedienung (BED-05): Ziehen, Mausrad, Shift = fein, Doppelklick = Standardwert,
// Pfeiltasten bei Tastaturfokus. Der Wert ist immer der Reglerweg 0 … 1.
import { useCallback, useEffect, useRef, useState } from 'react';

export const FINE_FACTOR = 0.1;
export const WHEEL_STEP = 0.02;
export const KEY_STEP = 0.02;

export interface DragValueOptions {
  value: number;
  onChange: (v: number) => void;
  defaultValue?: number;
  disabled?: boolean;
  /** Zieh-Achse: 'y' = nach oben größer, 'x' = nach rechts größer. */
  axis: 'x' | 'y';
  /** Zieh-Strecke in Pixeln für den ganzen Weg. */
  travel: () => number;
  /** Schrittweite je Mausrad-Raste (Anteil des Wegs). */
  wheelStep?: number;
  onHeldChange?: (held: boolean) => void;
}

/** Neuer Reglerweg nach einer Zieh-Bewegung. */
export function dragDelta(deltaPx: number, travelPx: number, fine: boolean): number {
  return (deltaPx / Math.max(1, travelPx)) * (fine ? FINE_FACTOR : 1);
}

/** Neuer Reglerweg nach einer Mausrad-Raste (deltaY < 0 = hoch). */
export function wheelDelta(deltaY: number, fine: boolean, step = WHEEL_STEP): number {
  if (deltaY === 0) return 0;
  return -Math.sign(deltaY) * step * (fine ? FINE_FACTOR : 1);
}

const clamp01 = (v: number): number => Math.min(1, Math.max(0, v));

export function useDragValue<E extends HTMLElement>(opts: DragValueOptions) {
  const ref = useRef<E | null>(null);
  const [held, setHeld] = useState(false);
  const o = useRef(opts);
  o.current = opts;
  // Laufender Wert während des Ziehens, unabhängig vom Takt der Rückmeldung aus dem Kern.
  const live = useRef(opts.value);
  const last = useRef<{ x: number; y: number } | null>(null);
  if (!held) live.current = opts.value;

  const commit = useCallback((v: number) => {
    const next = clamp01(v);
    live.current = next;
    o.current.onChange(next);
  }, []);

  const onPointerDown = useCallback((e: React.PointerEvent<E>) => {
    if (o.current.disabled || e.button !== 0) return;
    // Kein Fokus per Maus: Pfeiltasten bleiben beim Crossfader und der Liste.
    e.preventDefault();
    e.currentTarget.setPointerCapture(e.pointerId);
    last.current = { x: e.clientX, y: e.clientY };
    live.current = o.current.value;
    setHeld(true);
    o.current.onHeldChange?.(true);
  }, []);

  const onPointerMove = useCallback(
    (e: React.PointerEvent<E>) => {
      if (!last.current) return;
      const d = o.current.axis === 'y' ? last.current.y - e.clientY : e.clientX - last.current.x;
      last.current = { x: e.clientX, y: e.clientY };
      if (d !== 0) commit(live.current + dragDelta(d, o.current.travel(), e.shiftKey));
    },
    [commit],
  );

  const end = useCallback((e: React.PointerEvent<E>) => {
    if (!last.current) return;
    last.current = null;
    if (e.currentTarget.hasPointerCapture(e.pointerId)) e.currentTarget.releasePointerCapture(e.pointerId);
    setHeld(false);
    o.current.onHeldChange?.(false);
  }, []);

  const onDoubleClick = useCallback(() => {
    if (o.current.disabled || o.current.defaultValue === undefined) return;
    commit(o.current.defaultValue);
  }, [commit]);

  const onKeyDown = useCallback(
    (e: React.KeyboardEvent<E>) => {
      if (o.current.disabled) return;
      const step = KEY_STEP * (e.shiftKey ? FINE_FACTOR : 1);
      if (e.key === 'ArrowUp' || e.key === 'ArrowRight') commit(o.current.value + step);
      else if (e.key === 'ArrowDown' || e.key === 'ArrowLeft') commit(o.current.value - step);
      else if (e.key === 'Home' || e.key === 'Delete') {
        if (o.current.defaultValue !== undefined) commit(o.current.defaultValue);
      } else return;
      e.preventDefault();
      e.stopPropagation();
    },
    [commit],
  );

  // Mausrad braucht einen nicht-passiven Listener, damit die Seite nicht rollt.
  useEffect(() => {
    const el = ref.current;
    if (!el) return;
    const onWheel = (e: WheelEvent) => {
      if (o.current.disabled) return;
      e.preventDefault();
      // Mit Shift liefern manche Systeme die Bewegung als deltaX.
      const d = wheelDelta(e.deltaY || e.deltaX, e.shiftKey, o.current.wheelStep);
      if (d !== 0) commit(o.current.value + d);
    };
    el.addEventListener('wheel', onWheel, { passive: false });
    return () => el.removeEventListener('wheel', onWheel);
  }, [commit]);

  return {
    ref,
    held,
    handlers: { onPointerDown, onPointerMove, onPointerUp: end, onPointerCancel: end, onDoubleClick, onKeyDown },
  };
}
