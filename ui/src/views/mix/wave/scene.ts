// Szene der parallelen Wellenformen: Geometrie der Bahnen, Rasterlinien und Abspielmarke.
// Reine Berechnung aus Snapshot und Analyse; die Zeichner (WebGL 2, Canvas 2D) setzen sie um.
import type { BeatGrid, DeckId, DeckState, Waveform } from '../../../core/types';
import { VALUES_PER_POINT } from '../../../core/types';
import { dbToLin, fromWire } from '../../../lib/db';
import { beatAt, isDownbeat, secAt } from '../../../lib/grid';

export type Rgb = [number, number, number];
export type Rgba = [number, number, number, number];

export interface StageColors {
  bg: Rgb;
  rail: Rgb;
  text: Rgb;
  low: Rgb;
  mid: Rgb;
  high: Rgb;
  decks: Record<DeckId, Rgb>;
  playedDim: number;
}

export interface LaneInput {
  deck: DeckId;
  state: DeckState;
  waveform: Waveform | null;
  grid: BeatGrid | null;
}

export interface LaneScene {
  deck: DeckId;
  /** Rechteck in Geräte-Pixeln, Ursprung oben links. */
  x: number;
  y: number;
  w: number;
  h: number;
  /** +1: Grundlinie unten (Bahn über der Schiene), −1: Grundlinie oben. */
  dir: 1 | -1;
  loaded: boolean;
  /** Gewählte Stufe der Pyramide. */
  level: number;
  points: number;
  /** Punktindex an der Abspielmarke und Punkte je Pixel. */
  idxCenter: number;
  idxPerPx: number;
  /** Sichtbarkeit je Band 0 … 1; 0 = Kill (schraffiert). */
  bandGain: [number, number, number];
}

export interface Rect {
  x: number;
  y: number;
  w: number;
  h: number;
  color: Rgba;
}

export interface StageScene {
  width: number;
  height: number;
  dpr: number;
  lanes: LaneScene[];
  /** Linien und Flächen über den Wellenformen, in Zeichenreihenfolge. */
  rects: Rect[];
}

export interface StageInput {
  width: number;
  height: number;
  dpr: number;
  /** Höhe der Beat-Schiene zwischen den Bahnen in Geräte-Pixeln. */
  railH: number;
  /** Strichstärke in Geräte-Pixeln (wächst mit der Skalierung der Oberfläche). */
  unit?: number;
  /** Sichtbare Beats bei Clock-Tempo. */
  zoomBeats: number;
  clockBpm: number;
  lanes: [LaneInput, LaneInput];
  colors: StageColors;
}

/** Anteil der Bahnhöhe je Band bei Vollausschlag (gestapelt: Tief unten, darüber Mitte, Hoch). */
export const BAND_SHARE: [number, number, number] = [0.5, 0.27, 0.2];
export const FALLBACK_BPM = 130;
/** Raster der Phrasenmarken in Takten (Annahme, bis die Struktur-Analyse aus Stufe 3 vorliegt). */
export const PHRASE_BARS = 8;

/** Stufe wählen: die gröbste, die noch mindestens 0,6 Punkte je Pixel liefert. */
export function pickLevel(waveform: Waveform, trackSecPerPx: number): number {
  let best = 0;
  for (let i = 0; i < waveform.levels.length; i++) {
    const perPx = (trackSecPerPx * waveform.sample_rate) / waveform.levels[i].samples_per_point;
    if (perPx >= 0.6) best = i;
  }
  return best;
}

/** Sekunden Echtzeit je Pixel: `zoomBeats` Beats füllen die Breite. */
export function secondsPerPx(width: number, zoomBeats: number, bpm: number): number {
  return (zoomBeats * 60) / (bpm > 0 ? bpm : FALLBACK_BPM) / Math.max(1, width);
}

/** Sichtbarkeit eines Bands aus dem EQ-Wert: Kill = 0, Absenkung dunkler, ab 0 dB voll. */
export function bandVisibility(db: number): number {
  if (db === -Infinity) return 0;
  return Math.max(0.03, Math.min(1, dbToLin(db)));
}

export function buildScene(input: StageInput): StageScene {
  const { width, height, dpr, colors } = input;
  const railH = Math.max(4, Math.round(input.railH / 2) * 2);
  const laneH = Math.max(1, Math.floor((height - railH) / 2));
  const railY = laneH;
  const center = Math.round(width / 2);
  const refBpm =
    input.clockBpm > 0 ? input.clockBpm : input.lanes.find((l) => l.state.loaded)?.state.bpm || FALLBACK_BPM;
  const secPerPx = secondsPerPx(width, input.zoomBeats, refBpm);
  const px = Math.max(1, Math.round(input.unit ?? dpr));

  const lanes: LaneScene[] = [];
  const rects: Rect[] = [];
  rects.push({ x: 0, y: railY, w: width, h: railH, color: [...colors.rail, 1] });

  input.lanes.forEach((lane, i) => {
    const top = i === 0;
    const y = top ? 0 : railY + railH;
    const s = lane.state;
    const loaded = s.loaded && lane.waveform !== null;
    const rate = 1 + s.tempo_offset;
    const trackSecPerPx = secPerPx * rate;
    let level = 0;
    let points = 0;
    let idxCenter = 0;
    let idxPerPx = 1;
    if (loaded && lane.waveform) {
      level = pickLevel(lane.waveform, trackSecPerPx);
      const lv = lane.waveform.levels[level];
      points = Math.floor(lv.data.length / VALUES_PER_POINT);
      const perSec = lane.waveform.sample_rate / lv.samples_per_point;
      idxCenter = s.pos * perSec;
      idxPerPx = trackSecPerPx * perSec;
    }
    lanes.push({
      deck: lane.deck,
      x: 0,
      y,
      w: width,
      h: laneH,
      dir: top ? 1 : -1,
      loaded,
      level,
      points,
      idxCenter,
      idxPerPx,
      bandGain: [bandVisibility(fromWire(s.eq_db[0])), bandVisibility(fromWire(s.eq_db[1])), bandVisibility(fromWire(s.eq_db[2]))],
    });

    const deckColor = colors.decks[lane.deck];
    // Grundlinie an der Schiene.
    rects.push({ x: 0, y: top ? railY - px : railY + railH, w: width, h: px, color: [...deckColor, loaded ? 0.55 : 0.25] });
    if (!loaded || !lane.grid || trackSecPerPx <= 0) return;

    // Beat- und Taktlinien: in der Bahn dezent, in der Schiene kräftig in der Deck-Farbe.
    const grid = lane.grid;
    const half = railH / 2;
    const tLeft = s.pos - center * trackSecPerPx;
    const tRight = s.pos + (width - center) * trackSecPerPx;
    const first = Math.ceil(beatAt(grid, Math.max(tLeft, -1)));
    const last = Math.floor(beatAt(grid, Math.min(tRight, s.duration)));
    const pxPerBeat = (60 / (s.track_bpm || FALLBACK_BPM)) / trackSecPerPx;
    const showBeats = pxPerBeat >= 7 * dpr;
    for (let b = Math.max(0, first); b <= last; b++) {
      const down = isDownbeat(grid, b);
      if (!down && !showBeats) continue;
      const bar = Math.floor((b - grid.bar_offset) / 4);
      const phrase = down && bar % PHRASE_BARS === 0;
      const x = center + (secAt(grid, b) - s.pos) / trackSecPerPx;
      const lw = phrase ? 3 * px : down ? 2 * px : px;
      const xi = Math.round(x - lw / 2);
      rects.push({ x: xi, y, w: lw, h: laneH, color: [...colors.text, phrase ? 0.34 : down ? 0.2 : 0.08] });
      const th = phrase || down ? half - px : Math.round(half * 0.5);
      const tw = phrase ? 6 * px : down ? 3 * px : 2 * px;
      rects.push({
        x: Math.round(x - tw / 2),
        y: top ? railY + (half - px - th) : railY + half + px,
        w: tw,
        h: th,
        color: [...deckColor, 1],
      });
    }
  });

  // Abspielmarke über alles, mit Kappen oben und unten.
  const pw = 2 * px;
  rects.push({ x: center - pw / 2 - px, y: 0, w: pw + 2 * px, h: height, color: [...colors.bg, 0.55] });
  rects.push({ x: center - pw / 2, y: 0, w: pw, h: height, color: [...colors.text, 1] });
  const cap = Math.round(5 * dpr);
  rects.push({ x: center - cap, y: 0, w: cap * 2, h: Math.round(3 * dpr), color: [...colors.text, 1] });
  rects.push({ x: center - cap, y: height - Math.round(3 * dpr), w: cap * 2, h: Math.round(3 * dpr), color: [...colors.text, 1] });

  return { width, height, dpr, lanes, rects };
}

/** Amplituden (Tief, Mitte, Hoch; 0 … 1) einer Pixelspalte – wie im Shader. */
export function sampleColumn(data: ArrayLike<number>, points: number, c: number, idxPerPx: number, out: Float32Array): void {
  const peak = (i: number, b: number): number => {
    if (i < 0 || i >= points) return 0;
    const o = i * VALUES_PER_POINT + b * 3;
    return Math.max(-data[o], data[o + 1]);
  };
  if (idxPerPx < 1) {
    const f = c - 0.5;
    const i = Math.floor(f);
    const t = f - i;
    for (let b = 0; b < 3; b++) out[b] = peak(i, b) * (1 - t) + peak(i + 1, b) * t;
    return;
  }
  const i0 = Math.floor(c - idxPerPx / 2);
  const i1 = Math.min(i0 + 5, Math.floor(c + idxPerPx / 2));
  out[0] = out[1] = out[2] = 0;
  for (let i = i0; i <= i1; i++) for (let b = 0; b < 3; b++) out[b] = Math.max(out[b], peak(i, b));
}

/** Stufe als RGBA-Bytes packen (R = Tief, G = Mitte, B = Hoch), zeilenweise in fester Breite. */
export function packLevel(data: ArrayLike<number>, texWidth: number): { bytes: Uint8Array; width: number; height: number; points: number } {
  const points = Math.floor(data.length / VALUES_PER_POINT);
  const height = Math.max(1, Math.ceil(points / texWidth));
  const bytes = new Uint8Array(texWidth * height * 4);
  for (let i = 0; i < points; i++) {
    const o = i * VALUES_PER_POINT;
    for (let b = 0; b < 3; b++) {
      const a = Math.max(-data[o + b * 3], data[o + b * 3 + 1]);
      bytes[i * 4 + b] = Math.max(0, Math.min(255, Math.round(a * 255)));
    }
    bytes[i * 4 + 3] = 255;
  }
  return { bytes, width: texWidth, height, points };
}
