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
  /** Umriss-Grau: Abspielmarke, solange kein Deck geladen ist. */
  line: Rgb;
  /** Warnfarbe: Versatz zwischen den Beat-Marken beider Decks. */
  warn: Rgb;
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
  /** Punktindex in der Bildmitte und Punkte je Pixel. */
  idxCenter: number;
  idxPerPx: number;
  /** Abspielstelle in Pixeln ab dem linken Rand der Bahn; links davon ist „gespielt". Ohne Vorausblick und bei einem stehenden Deck die Bildmitte. */
  headX: number;
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
  /** Abspielstelle der laufenden Decks in Pixeln; mit Vorausblick neben der Bildmitte, auch außerhalb des Bilds. */
  headX: number;
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
  /**
   * Vorausblick in Beats bei Clock-Tempo (VIS-13): Die Bildmitte zeigt, was dann an der Abspielmarke
   * liegt. Gilt für alle laufenden Decks gemeinsam, damit sichtbar bleibt, was zusammen erklingt. Ein
   * stehendes Deck bleibt an seiner Stelle, solange ein anderes läuft: So lässt sich darin die Stelle
   * suchen, die zu dem passt, was das laufende Deck dann spielt. Läuft keines, wandern alle. 0 = aus.
   */
  peekBeats?: number;
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

/** Deckkraft der Rasterlinien in den Bahnen: Beat ≈ 2 : 1, Takt ≥ 3 : 1, Phrase kräftiger (VIS-03). */
export const LINE_ALPHA = { beat: 0.24, bar: 0.36, phrase: 0.52 };

export function buildScene(input: StageInput): StageScene {
  const { width, height, dpr, colors } = input;
  const railH = Math.max(4, Math.round(input.railH / 2) * 2);
  const laneH = Math.max(1, Math.floor((height - railH) / 2));
  const railY = laneH;
  const center = Math.round(width / 2);
  const refBpm =
    input.clockBpm > 0 ? input.clockBpm : input.lanes.find((l) => l.state.loaded)?.state.bpm || FALLBACK_BPM;
  const secPerPx = secondsPerPx(width, input.zoomBeats, refBpm);
  // Vorausblick als Echtzeit; die Abspielstelle wandert um diese Strecke aus der Bildmitte.
  const peekSec = ((input.peekBeats ?? 0) * 60) / refBpm;
  const headX = center - peekSec / secPerPx;
  const anyPlaying = input.lanes.some((l) => l.state.loaded && l.state.playing);
  const px = Math.max(1, Math.round(input.unit ?? dpr));
  // Breite der Marken in der Schiene: Beat 0,2 rem, Takt 0,4 rem, Phrase 0,7 rem.
  const rem = (input.unit ?? dpr) * 14;
  const markW = { beat: Math.max(2, Math.round(rem * 0.2)), bar: Math.max(4, Math.round(rem * 0.4)), phrase: Math.max(7, Math.round(rem * 0.7)) };
  const half = railH / 2;
  const seam = px; // Fuge zwischen den Hälften der Schiene

  const lanes: LaneScene[] = [];
  const lines: Rect[] = [];
  const marks: Rect[] = [];
  // Beat-Positionen je Bahn, um den Versatz zwischen den Decks zu zeigen.
  const beatXs: number[][] = [[], []];
  const beatSpacing: number[] = [0, 0];

  input.lanes.forEach((lane, i) => {
    const top = i === 0;
    const y = top ? 0 : railY + railH;
    const s = lane.state;
    const loaded = s.loaded && lane.waveform !== null;
    const rate = 1 + s.tempo_offset;
    const trackSecPerPx = secPerPx * rate;
    // Ein stehendes Deck bleibt stehen, solange ein anderes läuft; seine Abspielstelle ist die Bildmitte.
    const lanePeek = !s.loaded || s.playing || !anyPlaying ? peekSec : 0;
    // Stelle des Tracks in der Bildmitte: In `lanePeek` Echtzeit legt das Deck `lanePeek · rate` zurück.
    const viewPos = s.pos + lanePeek * rate;
    let level = 0;
    let points = 0;
    let idxCenter = 0;
    let idxPerPx = 1;
    if (loaded && lane.waveform) {
      level = pickLevel(lane.waveform, trackSecPerPx);
      const lv = lane.waveform.levels[level];
      points = Math.floor(lv.data.length / VALUES_PER_POINT);
      const perSec = lane.waveform.sample_rate / lv.samples_per_point;
      idxCenter = viewPos * perSec;
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
      headX: lanePeek === 0 ? center : headX,
      bandGain: [bandVisibility(fromWire(s.eq_db[0])), bandVisibility(fromWire(s.eq_db[1])), bandVisibility(fromWire(s.eq_db[2]))],
    });

    const deckColor = colors.decks[lane.deck];
    // Grundlinie an der Schiene.
    lines.push({ x: 0, y: top ? railY - px : railY + railH, w: width, h: px, color: [...deckColor, loaded ? 0.7 : 0.25] });
    if (!loaded || !lane.grid || trackSecPerPx <= 0) return;

    // Beat- und Taktlinien in der Bahn; in der Schiene füllt jede Marke ihre Hälfte in der Deck-Farbe.
    const grid = lane.grid;
    const tLeft = viewPos - center * trackSecPerPx;
    const tRight = viewPos + (width - center) * trackSecPerPx;
    const first = Math.ceil(beatAt(grid, Math.max(tLeft, -1)));
    const last = Math.floor(beatAt(grid, Math.min(tRight, s.duration)));
    const pxPerBeat = 60 / (s.track_bpm || FALLBACK_BPM) / trackSecPerPx;
    const showBeats = pxPerBeat >= markW.beat * 3;
    beatSpacing[i] = pxPerBeat;
    for (let b = Math.max(0, first); b <= last; b++) {
      const down = isDownbeat(grid, b);
      if (!down && !showBeats) continue;
      const bar = Math.floor((b - grid.bar_offset) / 4);
      const phrase = down && bar % PHRASE_BARS === 0;
      const x = center + (secAt(grid, b) - viewPos) / trackSecPerPx;
      if (showBeats) beatXs[i].push(x);
      const lw = phrase ? 3 * px : down ? 2 * px : px;
      lines.push({
        x: Math.round(x - lw / 2),
        y,
        w: lw,
        h: laneH,
        color: [...colors.text, phrase ? LINE_ALPHA.phrase : down ? LINE_ALPHA.bar : LINE_ALPHA.beat],
      });
      const tw = phrase ? markW.phrase : down ? markW.bar : markW.beat;
      const mx = Math.round(x - tw / 2);
      marks.push({ x: mx, y: top ? railY : railY + half + seam / 2, w: tw, h: half - seam / 2, color: [...deckColor, 1] });
      if (phrase) {
        // Phrasenmarke: zusätzlich eine Fahne in die eigene Bahn hinein.
        const flag = Math.round(rem * 0.4);
        marks.push({ x: mx, y: top ? railY - flag : railY + railH, w: tw, h: flag, color: [...deckColor, 1] });
      }
    }
  });

  const rects: Rect[] = [...lines];
  rects.push({ x: 0, y: railY, w: width, h: railH, color: [...colors.rail, 1] });
  // Laufen beide Decks, wird der Versatz zwischen ihren Beat-Marken als gelbe Fläche sichtbar:
  // keine Fläche = im Takt, breite Fläche = läuft weg.
  const [la, lb] = input.lanes;
  if (la.state.playing && lb.state.playing && beatXs[0].length > 0 && beatXs[1].length > 0) {
    const limit = Math.min(beatSpacing[0], beatSpacing[1]) / 2;
    const bandH = Math.max(2 * px, Math.round(railH * 0.5));
    for (const xb of beatXs[1]) {
      let best = Infinity;
      for (const xa of beatXs[0]) if (Math.abs(xa - xb) < Math.abs(best)) best = xa - xb;
      if (Math.abs(best) > 1.5 && Math.abs(best) < limit) {
        const x0 = Math.min(xb, xb + best);
        rects.push({ x: Math.round(x0), y: railY + (railH - bandH) / 2, w: Math.max(1, Math.round(Math.abs(best))), h: bandH, color: [...colors.warn, 1] });
      }
    }
  }
  rects.push(...marks);

  // Abspielmarke über alles, mit Kappen oben und unten; ohne geladenes Deck nur als graue Linie.
  const anyLoaded = lanes.some((l) => l.loaded);
  const head: Rgba = anyLoaded ? [...colors.text, 1] : [...colors.line, 1];
  const pw = 2 * px;
  const peeking = peekSec !== 0;
  const unit = input.unit ?? dpr;
  const cap = Math.round(5 * unit);
  const capH = Math.round(3 * unit);
  if (!peeking) {
    rects.push({ x: center - pw / 2 - px, y: 0, w: pw + 2 * px, h: height, color: [...colors.bg, 0.55] });
    rects.push({ x: center - pw / 2, y: 0, w: pw, h: height, color: head });
    if (anyLoaded) {
      rects.push({ x: center - cap, y: 0, w: cap * 2, h: capH, color: head });
      rects.push({ x: center - cap, y: height - capH, w: cap * 2, h: capH, color: head });
    }
  } else {
    // Vorausblick: Die Bildmitte ist nicht die Abspielstelle. Sie bekommt eine gestrichelte Linie in
    // der Warnfarbe. Die Abspielmarke steht je Bahn an ihrer wirklichen Stelle, solange sie im Bild
    // liegt: bei einem laufenden Deck verschoben, bei einem stehenden weiter in der Mitte.
    const dash = Math.round(6 * unit);
    for (let y0 = 0; y0 < height; y0 += dash * 2) {
      rects.push({ x: center - pw / 2, y: y0, w: pw, h: Math.min(dash, height - y0), color: [...colors.warn, 1] });
    }
    for (const lane of lanes) {
      const hx = Math.round(lane.headX);
      if (!lane.loaded || hx < 0 || hx > width) continue;
      rects.push({ x: hx - pw / 2 - px, y: lane.y, w: pw + 2 * px, h: lane.h, color: [...colors.bg, hx === center ? 1 : 0.55] });
      rects.push({ x: hx - pw / 2, y: lane.y, w: pw, h: lane.h, color: head });
      rects.push({ x: hx - cap, y: lane.dir === 1 ? lane.y : lane.y + lane.h - capH, w: cap * 2, h: capH, color: head });
    }
  }

  return { width, height, dpr, lanes, rects, headX };
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
