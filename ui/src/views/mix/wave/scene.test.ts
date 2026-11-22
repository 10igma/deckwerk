import { describe, expect, it } from 'vitest';
import { PEEK_MAX, PEEK_MIN, peekAfter } from '../../../app/actions';
import { emptyDeckState, type BeatGrid, type DeckState, type Waveform } from '../../../core/types';
import { buildScene, type LaneInput, type Rect, type StageColors, type StageInput } from './scene';

const COLORS: StageColors = {
  bg: [0, 0, 0],
  rail: [0.1, 0.1, 0.1],
  text: [1, 1, 1],
  line: [0.5, 0.5, 0.5],
  warn: [1, 0.8, 0],
  low: [1, 0, 0],
  mid: [0, 1, 0],
  high: [0, 0, 1],
  decks: { a: [0, 0.5, 1], b: [1, 0.5, 0], c: [0, 1, 0.5], d: [1, 0, 0.5] },
  playedDim: 0.4,
};

const WIDTH = 1000;
const HEIGHT = 400;

/** Raster mit festem Tempo ab Sekunde 0, die Eins auf Beat 0. */
const grid = (bpm: number): BeatGrid => ({ anchors: [{ pos: 0, bpm }], anchor_beats: [0], bar_offset: 0 });

/** Wellenform mit einer Stufe (64 Samples je Punkt bei 48 kHz = 750 Punkte je Sekunde). */
const waveform = (seconds: number): Waveform => {
  const level = { samples_per_point: 64, data: new Float32Array(Math.ceil(seconds * 750) * 9) };
  return { sample_rate: 48000, frames: seconds * 48000, overview: level, levels: [level] };
};

function lane(deck: 'a' | 'b', trackBpm: number, over: Partial<DeckState>): LaneInput {
  const state: DeckState = { ...emptyDeckState(), loaded: true, playing: true, duration: 600, track_bpm: trackBpm, bpm: trackBpm, ...over };
  return { deck, state, waveform: waveform(600), grid: grid(trackBpm) };
}

function input(lanes: [LaneInput, LaneInput], peekBeats: number, zoomBeats = 8): StageInput {
  return { width: WIDTH, height: HEIGHT, dpr: 1, railH: 28, zoomBeats, peekBeats, clockBpm: 120, lanes, colors: COLORS };
}

/** Mittelpunkte der Rasterlinien einer Bahn: volle Bahnhöhe, weiß, schmal. */
function gridLines(rects: Rect[], y: number, laneH: number): number[] {
  return rects.filter((r) => r.y === y && r.h === laneH && r.w <= 3 && r.color[0] === 1 && r.color[1] === 1 && r.color[3] < 1).map((r) => r.x + r.w / 2);
}

/** Abspielmarke: weiße, deckende Linie über die ganze Höhe oder, im Vorausblick, über eine Bahn. */
const playheads = (rects: Rect[]): Rect[] => rects.filter((r) => r.h >= 100 && r.w === 2 && r.color[0] === 1 && r.color[2] === 1 && r.color[3] === 1);
/** Striche der Mittellinie im Vorausblick: Warnfarbe. */
const centreDashes = (rects: Rect[]): Rect[] => rects.filter((r) => r.color[0] === 1 && r.color[1] === 0.8 && r.color[2] === 0 && r.w === 2 && r.x === WIDTH / 2 - 1);

describe('Vorausblick in den Wellenformen (VIS-13)', () => {
  // 8 Beats bei 120 BPM füllen 1000 px: 125 px je Beat, 4 ms je Pixel.
  const a = lane('a', 120, { pos: 10 });
  const idle = lane('b', 120, { loaded: false, playing: false });

  it('ohne Vorausblick liegt die Abspielstelle in der Bildmitte', () => {
    const scene = buildScene(input([a, idle], 0));
    expect(scene.headX).toBe(500);
    expect(scene.lanes[0].headX).toBe(500);
    expect(scene.lanes[0].idxCenter).toBeCloseTo(10 * 750, 6);
    expect(playheads(scene.rects).map((r) => r.x + r.w / 2)).toEqual([500]);
    expect(centreDashes(scene.rects).length).toBe(0);
  });

  it('zeigt in der Bildmitte die Stelle, die in so vielen Beats an der Abspielmarke liegt', () => {
    // +2 Beats = 1 s: Die Mitte zeigt Sekunde 11, die Abspielstelle wandert 250 px nach links.
    const scene = buildScene(input([a, idle], 2));
    expect(scene.lanes[0].idxCenter).toBeCloseTo(11 * 750, 6);
    expect(scene.headX).toBeCloseTo(250, 6);
    expect(playheads(scene.rects).map((r) => r.x + r.w / 2)).toEqual([250]);
    expect(centreDashes(scene.rects).length).toBeGreaterThan(10);
    // Beat 22 liegt bei Sekunde 11 und damit in der Bildmitte; Beat 20 (Abspielstelle) bei 250 px.
    const lines = gridLines(scene.rects, 0, scene.lanes[0].h);
    expect(lines.some((x) => Math.abs(x - 500) <= 1)).toBe(true);
    expect(lines.some((x) => Math.abs(x - 250) <= 1)).toBe(true);
  });

  it('weit voraus steht die Abspielmarke außerhalb des Bilds und wird nicht gezeichnet', () => {
    // +128 Beats = 64 s (eine gute Minute voraus).
    const scene = buildScene(input([a, idle], 128));
    expect(scene.lanes[0].idxCenter).toBeCloseTo(74 * 750, 6);
    expect(scene.headX).toBeCloseTo(500 - 128 * 125, 6);
    expect(playheads(scene.rects).length).toBe(0);
    expect(centreDashes(scene.rects).length).toBeGreaterThan(10);
  });

  it('nach hinten wandert die Abspielstelle nach rechts', () => {
    const scene = buildScene(input([a, idle], -2));
    expect(scene.lanes[0].idxCenter).toBeCloseTo(9 * 750, 6);
    expect(scene.headX).toBeCloseTo(750, 6);
  });

  it('beide Decks verschieben sich um dieselbe Echtzeit: Was im Takt liegt, bleibt untereinander', () => {
    // B ist ein Track mit 126 BPM, auf 120 BPM heruntergeregelt und im Takt mit A.
    const rate = 120 / 126;
    const b = lane('b', 126, { pos: 30 * (60 / 126), tempo_offset: rate - 1, bpm: 120 });
    for (const peek of [0, 16, 128]) {
      const scene = buildScene(input([a, b], peek));
      const [la, lb] = scene.lanes;
      const xa = gridLines(scene.rects, la.y, la.h);
      const xb = gridLines(scene.rects, lb.y, lb.h);
      expect(xa.length).toBeGreaterThanOrEqual(8);
      // Jede Beat-Linie von B liegt höchstens einen Pixel neben einer von A (VIS-04). Linien genau
      // am Bildrand bleiben außen vor: Ob sie noch gezeichnet werden, hängt an der Rundung.
      const inner = xb.filter((x) => x > 5 && x < WIDTH - 5);
      expect(inner.length).toBeGreaterThanOrEqual(7);
      for (const x of inner) expect(Math.min(...xa.map((v) => Math.abs(v - x)))).toBeLessThanOrEqual(1);
      // B zeigt in der Mitte die Stelle, die es in dieser Zeit erreicht: 0,5 s Echtzeit je Beat mal Abspielrate.
      expect(lb.idxCenter).toBeCloseTo((b.state.pos + peek * 0.5 * rate) * 750, 3);
    }
  });

  it('ein stehendes Deck bleibt an seiner Stelle, solange das andere läuft', () => {
    // B steht am Einsatz bei Sekunde 30; A läuft und zeigt die Stelle in 128 Beats.
    const cued = lane('b', 120, { pos: 30, playing: false });
    const scene = buildScene(input([a, cued], 128));
    expect(scene.lanes[0].idxCenter).toBeCloseTo(74 * 750, 6);
    expect(scene.lanes[1].idxCenter).toBeCloseTo(30 * 750, 6);
    // Die Abspielmarke von B bleibt in der Bildmitte, in seiner Bahn; die von A liegt außerhalb.
    expect(scene.lanes[1].headX).toBe(500);
    const heads = playheads(scene.rects);
    expect(heads.map((r) => r.x + r.w / 2)).toEqual([500]);
    expect(heads[0].y).toBe(scene.lanes[1].y);
    expect(heads[0].h).toBe(scene.lanes[1].h);
    // Das stehende Deck lässt sich weiter verschieben (Suchen), die Ansicht folgt seiner Stelle.
    const moved = buildScene(input([a, lane('b', 120, { pos: 45, playing: false })], 128));
    expect(moved.lanes[1].idxCenter).toBeCloseTo(45 * 750, 6);
    expect(moved.lanes[0].idxCenter).toBeCloseTo(74 * 750, 6);
  });

  it('läuft kein Deck, verschieben sich alle', () => {
    const stopped = lane('a', 120, { pos: 10, playing: false });
    const cued = lane('b', 120, { pos: 0, playing: false });
    const scene = buildScene(input([stopped, cued], 128));
    expect(scene.lanes[0].idxCenter).toBeCloseTo(74 * 750, 6);
    expect(scene.lanes[1].idxCenter).toBeCloseTo(64 * 750, 6);
  });

  it('der Schritt bleibt auf ganzen Beats und in den Grenzen', () => {
    expect(peekAfter(0, 16)).toBe(16);
    expect(peekAfter(16, -16)).toBe(0);
    expect(peekAfter(0, 0.25)).toBe(0);
    expect(peekAfter(3, 1.5)).toBe(5);
    expect(peekAfter(PEEK_MAX - 4, 16)).toBe(PEEK_MAX);
    expect(peekAfter(PEEK_MIN + 4, -16)).toBe(PEEK_MIN);
  });

  it('Zone eines Übergangs: Balken von Beginn bis Ende, im Vorausblick mitverschoben (TRANS-05)', () => {
    // 8 Beats füllen 1000 px: 125 px je Beat. Zone von +1 bis +3 Beats.
    const zoneRects = (peek: number, preview: boolean) => {
      const scene = buildScene({ ...input([a, idle], peek), zone: { startBeats: 1, endBeats: 3, preview } });
      return scene.rects.filter((r) => r.color[0] === 1 && r.color[1] === 0.8 && r.color[2] === 0 && r.y === 0 && r.h < 20);
    };
    const [bar] = zoneRects(0, false);
    expect([bar.x, bar.x + bar.w]).toEqual([625, 875]);
    expect(bar.color[3]).toBeCloseTo(0.9, 6);
    // Vorausblick +2 Beats: Die Zone rückt um 250 px nach links, ihr Ende liegt 125 px rechts der Mitte.
    const [shifted] = zoneRects(2, true);
    expect([shifted.x, shifted.x + shifted.w]).toEqual([375, 625]);
    expect(shifted.color[3]).toBeCloseTo(0.45, 6);
    expect(buildScene(input([a, idle], 0)).rects.filter((r) => r.y === 0 && r.h < 20 && r.color[1] === 0.8).length).toBe(0);
  });
});
