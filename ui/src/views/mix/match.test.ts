import { describe, expect, it } from 'vitest';
import type { TrackInfo } from '../../core/types';
import { fromCamelot } from '../../lib/key';
import { MATCH_TEMPO, matchDeck, matchTracks } from './LibraryPanel';

const track = (id: string, bpm: number, camelotNr: number | null, letter: 'A' | 'B' = 'A'): TrackInfo => ({
  id,
  title: id,
  artist: '',
  bpm,
  key: camelotNr === null ? null : fromCamelot(camelotNr, letter),
  duration: 400,
  analyzed: true,
  missing: false,
  incomplete: false,
});

describe('Filter „passt zum laufenden Track" (BIB-11)', () => {
  const ref = track('bezug', 138, 8, 'A');
  const ids = (list: TrackInfo[]): string[] => list.map((t) => t.id);

  it('Tonart: gleiche, Nachbar, Parallele, diagonal und +2 passen; alles andere nicht (5.11, K ≥ 0,6)', () => {
    const tracks = [
      ref,
      track('gleich', 138, 8, 'A'),
      track('nachbar-tief', 138, 7, 'A'),
      track('nachbar-hoch', 138, 9, 'A'),
      track('parallel', 138, 8, 'B'),
      track('diagonal', 138, 9, 'B'),
      track('plus2', 138, 10, 'A'),
      track('plus7', 138, 3, 'A'),
      track('minus2', 138, 6, 'A'),
      track('fremd', 138, 2, 'B'),
      track('ohne-tonart', 138, null),
    ];
    expect(ids(matchTracks(tracks, ref))).toEqual(['gleich', 'nachbar-tief', 'nachbar-hoch', 'parallel', 'diagonal', 'plus2']);
  });

  it('Tempo: höchstens 6 % neben dem Bezug', () => {
    expect(MATCH_TEMPO).toBe(0.06);
    const tracks = [track('gleich', 138, 8), track('rand-oben', 138 * 1.06, 8), track('rand-unten', 138 * 0.94, 8), track('zu-schnell', 147, 8), track('zu-langsam', 129, 8), track('halb', 69, 8), track('ohne-tempo', 0, 8)];
    expect(ids(matchTracks(tracks, ref))).toEqual(['gleich', 'rand-oben', 'rand-unten']);
  });

  it('ohne brauchbaren Bezug bleibt die Liste, wie sie ist', () => {
    const tracks = [track('a', 120, 1), track('b', 150, 6)];
    expect(matchTracks(tracks, null)).toBe(tracks);
    expect(matchTracks(tracks, track('x', 0, 8))).toBe(tracks);
    expect(matchTracks(tracks, track('x', 138, null))).toBe(tracks);
  });

  it('Bezugs-Deck: Master, sonst das laufende, sonst das erste geladene', () => {
    const both = { a: true, b: true };
    expect(matchDeck('b', both, { a: true, b: true })).toBe('b');
    expect(matchDeck(null, both, { a: false, b: true })).toBe('b');
    expect(matchDeck(null, both, { a: false, b: false })).toBe('a');
    expect(matchDeck('a', { a: false, b: true }, { a: false, b: false })).toBe('b');
    expect(matchDeck(null, { a: false, b: false }, { a: false, b: false })).toBe(null);
  });
});
