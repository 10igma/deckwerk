// Rechnen mit Übergängen für den Editor (TRANS-04): Kurven auswerten wie der Kern, Reglerwerte auf
// die Höhe einer Spur abbilden, Punkte setzen, verschieben und löschen. Alles ohne Seiteneffekte.
import type { Transition, TransitionLane, TransitionPoint } from '../core/types';

export type Param = TransitionLane['param'];
export type Side = TransitionLane['side'];

export const PARAMS: Param[] = ['fader', 'eq_low', 'eq_mid', 'eq_high', 'filter'];
/** EQ: Unter diesem Wert in dB gilt der Regler als Kill. */
export const KILL_BELOW_DB = -26;
const EQ_MAX_DB = 6;

const isEq = (p: Param): boolean => p !== 'fader' && p !== 'filter';
const dbToLin = (db: number | null): number => (db === null ? 0 : 10 ** (db / 20));
const linToDb = (lin: number): number | null => (lin <= 0 ? null : 20 * Math.log10(lin));

/** Ruhestellung des Reglers. */
export function neutral(param: Param): number {
  return param === 'fader' ? 1 : 0;
}

/** Wert zwischen zwei Stützpunkten, `t` 0 … 1 – dieselbe Regel wie im Kern (`Lane::between`). */
export function between(param: Param, from: TransitionPoint, to: TransitionPoint, t: number): number | null {
  const shaped = to.curve === 'smooth' ? 0.5 - 0.5 * Math.cos(Math.PI * t) : to.curve === 'step' ? (t >= 1 ? 1 : 0) : t;
  if (isEq(param) && (from.value === null || to.value === null)) {
    const [a, b] = [dbToLin(from.value), dbToLin(to.value)];
    return linToDb(a + (b - a) * shaped);
  }
  const [a, b] = [from.value ?? 0, to.value ?? 0];
  return a + (b - a) * shaped;
}

/**
 * Stellung des Reglers zum Zeitpunkt `beat`: vor dem ersten Stützpunkt `undefined` (die Spur fasst
 * den Regler noch nicht an), nach dem letzten dessen Wert. `null` = Kill.
 */
export function valueAt(lane: TransitionLane, beat: number): number | null | undefined {
  const pts = lane.points;
  if (pts.length === 0 || beat < pts[0].beat) return undefined;
  for (let i = 0; i + 1 < pts.length; i++) {
    if (beat < pts[i + 1].beat) return between(lane.param, pts[i], pts[i + 1], (beat - pts[i].beat) / (pts[i + 1].beat - pts[i].beat));
  }
  return pts[pts.length - 1].value;
}

/** Höhe 0 (unten) … 1 (oben) für einen Reglerwert. EQ: Kill ganz unten, 0 dB bei 0,8, +6 dB oben. */
export function toHeight(param: Param, value: number | null): number {
  if (param === 'fader') return Math.min(1, Math.max(0, value ?? 0));
  if (param === 'filter') return Math.min(1, Math.max(0, ((value ?? 0) + 1) / 2));
  if (value === null || value <= KILL_BELOW_DB) return 0;
  return Math.min(1, 0.05 + ((value - KILL_BELOW_DB) / (EQ_MAX_DB - KILL_BELOW_DB)) * 0.95);
}

/** Reglerwert für eine Höhe 0 … 1; Umkehrung von `toHeight`. EQ rastet nahe 0 dB ein. */
export function fromHeight(param: Param, height: number): number | null {
  const h = Math.min(1, Math.max(0, height));
  if (param === 'fader') return Math.round(h * 100) / 100;
  if (param === 'filter') {
    const v = Math.round((h * 2 - 1) * 100) / 100;
    return Math.abs(v) < 0.04 ? 0 : v;
  }
  if (h < 0.05) return null;
  const db = KILL_BELOW_DB + ((h - 0.05) / 0.95) * (EQ_MAX_DB - KILL_BELOW_DB);
  return Math.abs(db) < 0.6 ? 0 : Math.round(db * 2) / 2;
}

/** Text für einen Reglerwert. */
export function valueText(param: Param, value: number | null, kill: string): string {
  if (param === 'fader') return `${Math.round((value ?? 0) * 100)} %`;
  if (param === 'filter') return (value ?? 0) === 0 ? '0' : `${(value ?? 0) > 0 ? '+' : '−'}${Math.abs(value ?? 0).toFixed(2)}`;
  if (value === null) return kill;
  return `${value > 0 ? '+' : value < 0 ? '−' : '±'}${Math.abs(value).toFixed(1)} dB`;
}

/** Spur eines Reglers, falls der Übergang sie hat. */
export function findLane(t: Transition, side: Side, param: Param): TransitionLane | undefined {
  return t.lanes.find((l) => l.side === side && l.param === param);
}

/** Zeitbereich des Editors in Beats relativ zu T0: vom ersten Eingriff (höchstens −4) bis zum Ende. */
export function timeRange(t: Transition): [number, number] {
  const first = Math.min(0, ...t.lanes.flatMap((l) => l.points.slice(0, 1).map((p) => p.beat)));
  return [Math.min(-4, Math.floor(first / 4) * 4), t.bars * 4];
}

const sortPoints = (points: TransitionPoint[]): TransitionPoint[] => [...points].sort((a, b) => a.beat - b.beat);

function withLane(t: Transition, side: Side, param: Param, change: (points: TransitionPoint[]) => TransitionPoint[]): Transition {
  const existing = findLane(t, side, param);
  const points = change(existing ? existing.points : []);
  const lanes = t.lanes.filter((l) => !(l.side === side && l.param === param));
  if (points.length > 0) lanes.push({ side, param, points: sortPoints(points) });
  // Feste Reihenfolge, damit gespeicherte Übergänge vergleichbar bleiben.
  const order = (l: TransitionLane) => (l.side === 'in' ? 0 : 10) + PARAMS.indexOf(l.param);
  return { ...t, lanes: lanes.sort((a, b) => order(a) - order(b)) };
}

/** Rückt einen Zeitpunkt auf das Raster (`step` Beats) und in den erlaubten Bereich. */
export function snapBeat(t: Transition, beat: number, step = 1): number {
  const [lo, hi] = timeRange(t);
  return Math.min(hi, Math.max(lo, Math.round(beat / step) * step));
}

/** Setzt einen Punkt; liegt auf diesem Beat schon einer, bekommt er den neuen Wert. */
export function addPoint(t: Transition, side: Side, param: Param, beat: number, value: number | null): Transition {
  return withLane(t, side, param, (points) => [...points.filter((p) => p.beat !== beat), { beat, value, curve: 'linear' }]);
}

/**
 * Verschiebt den Punkt `index` einer Spur. Er bleibt zwischen seinen Nachbarn (mindestens einen
 * Rasterschritt Abstand), damit die Reihenfolge erhalten bleibt.
 */
export function movePoint(t: Transition, side: Side, param: Param, index: number, beat: number, value: number | null, step = 1): Transition {
  return withLane(t, side, param, (points) => {
    if (index < 0 || index >= points.length) return points;
    const lo = index > 0 ? points[index - 1].beat + step : -Infinity;
    const hi = index + 1 < points.length ? points[index + 1].beat - step : Infinity;
    const next = [...points];
    next[index] = { ...points[index], beat: Math.min(hi, Math.max(lo, beat)), value };
    return next;
  });
}

/** Löscht den Punkt `index`; mit dem letzten Punkt verschwindet die Spur. */
export function removePoint(t: Transition, side: Side, param: Param, index: number): Transition {
  return withLane(t, side, param, (points) => points.filter((_, i) => i !== index));
}

/** Kurvenform des Wegs zu diesem Punkt. */
export function setCurve(t: Transition, side: Side, param: Param, index: number, curve: NonNullable<TransitionPoint['curve']>): Transition {
  return withLane(t, side, param, (points) => points.map((p, i) => (i === index ? { ...p, curve } : p)));
}

/**
 * Ändert die Länge: Alles ab T0 wird gestreckt oder gestaucht und auf ganze Beats gerückt; Punkte
 * vor T0 (Vorbereitung) bleiben, wo sie sind. Start und Stopp wandern mit.
 */
export function rescale(t: Transition, bars: number): Transition {
  if (bars === t.bars || bars < 1) return t;
  const f = bars / t.bars;
  const scale = (beat: number) => (beat <= 0 ? beat : Math.round(beat * f));
  const lanes = t.lanes.map((l) => {
    const points: TransitionPoint[] = [];
    for (const p of l.points) {
      const beat = scale(p.beat);
      // Fallen zwei Punkte auf denselben Beat, zählt der spätere.
      if (points.length > 0 && points[points.length - 1].beat === beat) points[points.length - 1] = { ...p, beat };
      else points.push({ ...p, beat });
    }
    return { ...l, points };
  });
  return { ...t, bars, start_in: t.start_in === null ? null : scale(t.start_in), stop_out: t.stop_out === null ? null : scale(t.stop_out), lanes };
}

/**
 * Bass-Regel (TRANS-03) als Hinweis im Editor: Zeitabschnitte, in denen beide Decks hörbar sind und
 * beide Bässe offen – länger als ein Takt. Rückgabe: [Beginn, Ende] in Beats.
 */
export function bassClashes(t: Transition): [number, number][] {
  const open = (side: Side, beat: number): boolean => {
    const low = findLane(t, side, 'eq_low');
    const fader = findLane(t, side, 'fader');
    // Ohne Spur oder vor ihrem ersten Punkt: Regler in Ruhestellung. Das kommende Deck läuft erst ab seinem Start.
    if (side === 'in' && t.start_in !== null && beat < t.start_in) return false;
    if (side === 'out' && t.stop_out !== null && beat >= t.stop_out) return false;
    const lowDb = low ? valueAt(low, beat) : 0;
    const level = fader ? valueAt(fader, beat) : 1;
    return (lowDb === undefined ? 0 : (lowDb ?? -Infinity)) > -6 && (level === undefined ? 1 : (level ?? 0)) > 0.5;
  };
  const out: [number, number][] = [];
  let from: number | null = null;
  const end = t.bars * 4;
  for (let b = 0; b <= end; b += 0.25) {
    const both = b < end && open('out', b) && open('in', b);
    if (both && from === null) from = b;
    if (!both && from !== null) {
      if (b - from > 4) out.push([from, b]);
      from = null;
    }
  }
  return out;
}

/** Name für eine Kopie, die es unter `taken` noch nicht gibt: „Übergang 1", „Übergang 2" … */
export function freeName(base: string, taken: string[]): string {
  for (let n = 1; ; n++) {
    const name = `${base} ${n}`;
    if (!taken.includes(name)) return name;
  }
}
