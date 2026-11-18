// Laufende Anzeigen: schreiben in der Bild-Schleife direkt ins DOM.
import { useRef, type CSSProperties } from 'react';
import type { Snapshot } from '../../core/types';
import { useFrame } from '../../app/frame';

/** Text, der sich mit jedem Bild ändern kann (Zeit, BPM, Zähler). */
export function LiveText({ read, className, style }: { read: (s: Snapshot) => string; className?: string; style?: CSSProperties }) {
  const el = useRef<HTMLSpanElement | null>(null);
  const last = useRef('');
  useFrame((s) => {
    const text = read(s);
    if (text !== last.current && el.current) {
      last.current = text;
      el.current.textContent = text;
    }
  });
  return <span ref={el} className={className} style={style} />;
}

export const PHASE_LOCK = 0.012;

/**
 * Phasenanzeige (VIS-05): Versatz zur Master-Clock in Beats, −0,5 … +0,5.
 * Lage und Balken zeigen den Versatz; eingerastet wird die Marke zur Raute.
 */
export function PhaseMeter({ read, ariaLabel }: { read: (s: Snapshot) => number | null; ariaLabel?: string }) {
  const root = useRef<HTMLDivElement | null>(null);
  const marker = useRef<HTMLDivElement | null>(null);
  const bar = useRef<HTMLDivElement | null>(null);
  const last = useRef<string>('');
  useFrame((s) => {
    const phase = read(s);
    const key = phase === null ? 'off' : phase.toFixed(3);
    if (key === last.current || !root.current || !marker.current || !bar.current) return;
    last.current = key;
    if (phase === null) {
      root.current.dataset.off = 'true';
      root.current.dataset.lock = 'false';
      return;
    }
    const p = Math.max(-0.5, Math.min(0.5, phase));
    root.current.dataset.off = 'false';
    root.current.dataset.lock = Math.abs(p) < PHASE_LOCK ? 'true' : 'false';
    marker.current.style.left = `${(50 + p * 100).toFixed(2)}%`;
    bar.current.style.transform = `scaleX(${(p * 2).toFixed(4)})`;
  });
  return (
    <div className="dw-phase" ref={root} role="img" aria-label={ariaLabel} data-off="true">
      <span className="dw-phase__tick" style={{ left: '25%' }} />
      <span className="dw-phase__tick" style={{ left: '75%' }} />
      <div className="dw-phase__bar" ref={bar} />
      <span className="dw-phase__center" />
      <div className="dw-phase__marker" ref={marker} />
    </div>
  );
}
