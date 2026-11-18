// Beatgrid-Rechnung (Spiegel von dw-core/src/grid.rs).
import type { BeatGrid } from '../core/types';

export function constantGrid(bpm: number, firstBeat: number, barOffset = 0): BeatGrid {
  return { anchors: [{ pos: firstBeat, bpm }], anchor_beats: [0], bar_offset: ((barOffset % 4) + 4) % 4 };
}

function segmentForTime(grid: BeatGrid, sec: number): number {
  for (let i = grid.anchors.length - 1; i >= 0; i--) if (grid.anchors[i].pos <= sec) return i;
  return 0;
}

function segmentForBeat(grid: BeatGrid, beat: number): number {
  for (let i = grid.anchor_beats.length - 1; i >= 0; i--) if (grid.anchor_beats[i] <= beat) return i;
  return 0;
}

/** Beat-Nummer (mit Nachkommaanteil) an der Zeit `sec`. */
export function beatAt(grid: BeatGrid, sec: number): number {
  const i = segmentForTime(grid, sec);
  const a = grid.anchors[i];
  return grid.anchor_beats[i] + ((sec - a.pos) * a.bpm) / 60;
}

/** Zeit in Sekunden des Beats `beat`. */
export function secAt(grid: BeatGrid, beat: number): number {
  const i = segmentForBeat(grid, beat);
  const a = grid.anchors[i];
  return a.pos + ((beat - grid.anchor_beats[i]) * 60) / a.bpm;
}

export const bpmAt = (grid: BeatGrid, sec: number): number => grid.anchors[segmentForTime(grid, sec)].bpm;

export function isDownbeat(grid: BeatGrid, beat: number): boolean {
  return (((beat - grid.bar_offset) % 4) + 4) % 4 === 0;
}

export interface BarBeat {
  /** Takt, 1-basiert (vor dem ersten Downbeat ≤ 0). */
  bar: number;
  /** Schlag im Takt, 1–4. */
  beat: number;
}

/** Takt-/Beat-Zähler für die Anzeige (DECK-18). */
export function barBeat(grid: BeatGrid, beat: number): BarBeat {
  const whole = Math.floor(beat + 1e-9) - grid.bar_offset;
  const bar = Math.floor(whole / 4);
  return { bar: bar + 1, beat: whole - bar * 4 + 1 };
}

/** Sprungziel, das die Phase erhält: ganze Beats Abstand zur aktuellen Position (DECK-20). */
export function snapSeek(grid: BeatGrid, currentSec: number, targetSec: number): number {
  const delta = Math.round(beatAt(grid, targetSec) - beatAt(grid, currentSec));
  return secAt(grid, beatAt(grid, currentSec) + delta);
}
