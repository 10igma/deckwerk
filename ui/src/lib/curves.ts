// Regler-Kennlinien: Reglerweg 0…1 ↔ Wert in der Einheit des Befehls (Spezifikation 5.14, MIX-01–05).

export const clamp = (v: number, lo: number, hi: number): number => Math.min(hi, Math.max(lo, v));

export interface DbCurve {
  /** Reglerweg 0…1 → dB (−Infinity im Kill-Bereich). */
  toDb(pos: number): number;
  /** dB → Reglerweg 0…1. */
  toPos(db: number): number;
  /** Reglerweg des Standardwerts (0 dB). */
  zeroPos: number;
  killZone: number;
}

interface DbCurveSpec {
  maxDb: number;
  /** Letzter endlicher Wert direkt über dem Kill-Bereich. */
  floorDb: number;
  zeroPos: number;
  killZone: number;
  exponent: number;
}

export function makeDbCurve(spec: DbCurveSpec): DbCurve {
  const { maxDb, floorDb, zeroPos, killZone, exponent } = spec;
  return {
    zeroPos,
    killZone,
    toDb(pos) {
      const p = clamp(pos, 0, 1);
      if (p < killZone) return -Infinity;
      if (p >= zeroPos) return ((p - zeroPos) / (1 - zeroPos)) * maxDb;
      const x = (zeroPos - p) / (zeroPos - killZone);
      return floorDb * Math.pow(x, exponent);
    },
    toPos(db) {
      if (!(db > -Infinity)) return 0;
      if (db >= 0) return clamp(zeroPos + (db / maxDb) * (1 - zeroPos), 0, 1);
      if (db <= floorDb) return killZone;
      const x = Math.pow(db / floorDb, 1 / exponent);
      return zeroPos - x * (zeroPos - killZone);
    },
  };
}

/** Isolator-EQ: Mitte = 0 dB, rechts +6 dB, links stufenlos bis −∞; untere 5 % = Kill. */
export const EQ_CURVE = makeDbCurve({ maxDb: 6, floorDb: -36, zeroPos: 0.5, killZone: 0.05, exponent: 1.6 });
/** Gain: −∞ … +12 dB, Mitte = 0 dB. */
export const GAIN_CURVE = makeDbCurve({ maxDb: 12, floorDb: -48, zeroPos: 0.5, killZone: 0.05, exponent: 1.6 });
/** Master- und Kopfhörer-Lautstärke: −∞ … +6 dB, 0 dB bei 3/4 des Wegs. */
export const VOLUME_CURVE = makeDbCurve({ maxDb: 6, floorDb: -60, zeroPos: 0.75, killZone: 0.03, exponent: 1.6 });

/** Kanalfader: im Kern quadratisch (Faktor = Stellung²), ganz unten stumm (MIX-05). */
export const faderGain = (value: number): number => {
  const v = clamp(value, 0, 1);
  return v * v;
};

/** Anzeige des Kanalfaders: 40 · log10(Stellung) dB; halber Weg = −12 dB. */
export function faderToDb(value: number): number {
  const v = clamp(value, 0, 1);
  return v <= 0 ? -Infinity : 40 * Math.log10(v);
}

/** Stellung des Kanalfaders für einen Pegel in dB (Skalenstriche, Zahleneingabe). */
export function faderFromDb(db: number): number {
  if (!(db > -Infinity)) return 0;
  return clamp(Math.pow(10, db / 40), 0, 1);
}

export const FILTER_DEAD_ZONE = 0.02;

export interface FilterSetting {
  type: 'off' | 'lowpass' | 'highpass';
  /** Grenzfrequenz in Hz (0 bei „aus“). */
  hz: number;
}

/** Filter-Regler −1 … +1 → Typ und Grenzfrequenz (5.14). */
export function filterSetting(pos: number): FilterSetting {
  const p = clamp(pos, -1, 1);
  if (Math.abs(p) < FILTER_DEAD_ZONE) return { type: 'off', hz: 0 };
  if (p < 0) return { type: 'lowpass', hz: 20000 * Math.pow(80 / 20000, -p) };
  return { type: 'highpass', hz: 20 * Math.pow(8000 / 20, p) };
}

/** Bipolarer Regler −1 … +1 ↔ Reglerweg 0 … 1. */
export const bipolarToPos = (v: number): number => (clamp(v, -1, 1) + 1) / 2;
export const posToBipolar = (p: number): number => clamp(p, 0, 1) * 2 - 1;

export const TEMPO_RANGES = [0.02, 0.04, 0.06, 0.08, 0.1, 0.16, 0.25, 0.5, 1] as const;
export const TEMPO_RESOLUTION = 0.0001;

/** Reglerweg 0 … 1 → Tempo-Abweichung als Faktor, gerastet auf 0,01 %. */
export function tempoPosToOffset(pos: number, range: number): number {
  const raw = posToBipolar(pos) * range;
  return Math.round(raw / TEMPO_RESOLUTION) * TEMPO_RESOLUTION;
}

export function tempoOffsetToPos(offset: number, range: number): number {
  return bipolarToPos(offset / range);
}

/** Nächstgrößerer Tempo-Bereich (zyklisch). */
export function nextTempoRange(range: number): number {
  const i = TEMPO_RANGES.findIndex((r) => Math.abs(r - range) < 1e-9);
  return TEMPO_RANGES[(i + 1) % TEMPO_RANGES.length];
}

/** Kleinster Bereich, in den die Abweichung passt. */
export function tempoRangeFor(offset: number, current: number): number {
  if (Math.abs(offset) <= current + 1e-9) return current;
  return TEMPO_RANGES.find((r) => Math.abs(offset) <= r + 1e-9) ?? 1;
}

/** Angezeigtes Tempo = Original-BPM × (1 + Regler) (DECK-06). */
export const bpmWithOffset = (trackBpm: number, offset: number): number => trackBpm * (1 + offset);

/** Winkel des Drehreglers in Grad: −135° (links) … +135° (rechts). */
export const KNOB_SWEEP = 270;
export const knobAngle = (pos: number): number => (clamp(pos, 0, 1) - 0.5) * KNOB_SWEEP;

/** Crossfader „weich“: gleiche Leistung (MIX-06). Liefert [links, rechts]. */
export function crossfaderGains(pos: number): [number, number] {
  const x = (clamp(pos, -1, 1) + 1) / 2;
  return [Math.cos((x * Math.PI) / 2), Math.sin((x * Math.PI) / 2)];
}
