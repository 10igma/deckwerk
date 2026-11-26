// Erfundene Bibliothek für den simulierten Kern: 36 Techno-Tracks, 126–148 BPM.
import type { BeatGrid, Key, TrackAnalysis, TrackInfo } from '../types';
import { fromCamelot } from '../../lib/key';
import { makeRng } from './prng';

export const MOCK_SAMPLE_RATE = 44100;

export type SectionKind = 'intro' | 'groove' | 'main' | 'break' | 'build' | 'drop' | 'outro';

export interface Section {
  kind: SectionKind;
  /** Erster Takt (0-basiert) und Länge in Takten. */
  startBar: number;
  bars: number;
}

export interface MockTrack extends TrackInfo {
  seed: number;
  firstBeat: number;
  grid: BeatGrid;
  lufs: number;
  sections: Section[];
  totalBars: number;
}

// Titel, Künstler, BPM, Camelot-Code (leer = perkussiv, keine Tonart), Takte.
const TABLE: [string, string, number, string, number][] = [
  ['Kaltwalzwerk', 'Mara Volt', 134, '8A', 224],
  ['Schicht III', 'Hektor Blei', 136, '9A', 240],
  ['Hallendach', 'Nordkai', 132, '5A', 208],
  ['Ferrit', 'Ilse Draht', 138, '11A', 240],
  ['Druckkammer', 'Tenzo Ruhr', 140, '2A', 256],
  ['Null Uhr Vier', 'Okular', 128, '7B', 192],
  ['Brennofen', 'Liv Brandt', 142, '4A', 256],
  ['Relais', 'Kontor 9', 133, '8B', 224],
  ['Asphaltblüte', 'Jonte Maar', 126, '10B', 192],
  ['Hochspannung', 'Mara Volt', 145, '1A', 272],
  ['Taktstraße', 'Stanzwerk Ost', 135, '9A', 224],
  ['Schwungrad', 'Hektor Blei', 137.5, '10A', 240],
  ['Nachtschicht', 'Vera Kessel', 130, '6A', 208],
  ['Lot und Blei', 'Okular', 139, '12A', 240],
  ['Kiesbett', 'Nordkai', 129, '3B', 208],
  ['Unterstrom', 'Aino Quarz', 134, '7A', 224],
  ['Stahlseil', 'Tenzo Ruhr', 144, '5A', 272],
  ['Vakuum', 'Ilse Draht', 131, '', 208],
  ['Rohrpost', 'Kontor 9', 136, '8A', 240],
  ['Gleis 13', 'Jonte Maar', 127, '11B', 192],
  ['Zunder', 'Liv Brandt', 148, '3A', 288],
  ['Messing', 'Vera Kessel', 133, '6A', 224],
  ['Lastwechsel', 'Stanzwerk Ost', 141, '2A', 256],
  ['Sinter', 'Aino Quarz', 135, '10A', 224],
  ['Halle 7', 'Mara Volt', 138, '12A', 240],
  ['Kranbahn', 'Rufus Zeche', 132.5, '4A', 208],
  ['Dampfhammer', 'Rufus Zeche', 146, '1A', 272],
  ['Feinblech', 'Okular', 130, '9B', 208],
  ['Nieten', 'Dunja Flöz', 137, '11A', 240],
  ['Schacht Vier', 'Dunja Flöz', 143, '6A', 256],
  ['Gegenstrom', 'Nordkai', 134, '9A', 240],
  ['Wolfram', 'Hektor Blei', 139, '3A', 240],
  ['Leerlauf', 'Ilse Draht', 128, '5B', 192],
  ['Kupferstich', 'Vera Kessel', 136, '7A', 224],
  ['Plattenbau Dub', 'Kontor 9', 126, '12B', 192],
  ['Morgengrauen am Werkstor (Frühschicht-Fassung)', 'Tenzo Ruhr & Aino Quarz', 140, '4A', 256],
];

function parseCamelot(code: string): Key | null {
  if (!code) return null;
  const n = parseInt(code, 10);
  const letter = code.endsWith('A') ? 'A' : 'B';
  return fromCamelot(n, letter);
}

/** Abschnitte im 16-Takt-Raster: Intro, Groove, Hauptteil, Breakdown mit Aufbau, Drop, Outro. */
function buildSections(totalBars: number, rnd: () => number): Section[] {
  const out: Section[] = [];
  let bar = 0;
  const push = (kind: SectionKind, bars: number) => {
    out.push({ kind, startBar: bar, bars });
    bar += bars;
  };
  const outroBars = 32;
  push('intro', 16);
  push('groove', 16);
  push('main', rnd() < 0.5 ? 32 : 48);
  const breakBars = rnd() < 0.6 ? 16 : 24;
  push('break', breakBars);
  push('build', 8);
  // Rest bis zum Outro: Drop, bei langen Tracks ein zweiter kurzer Breakdown.
  let rest = totalBars - bar - outroBars;
  if (rest >= 96) {
    push('drop', 48);
    push('break', 8);
    push('build', 8);
    rest = totalBars - bar - outroBars;
  }
  push('drop', Math.max(16, rest));
  push('outro', totalBars - bar);
  return out;
}

function build(): MockTrack[] {
  return TABLE.map(([title, artist, bpm, code, bars], i) => {
    const seed = 0x51ed + i * 7919;
    const rnd = makeRng(seed);
    const firstBeat = 0.03 + Math.round(rnd() * 20) / 100;
    const grid: BeatGrid = { anchors: [{ pos: firstBeat, bpm }], anchor_beats: [0], bar_offset: 0 };
    const sections = buildSections(bars, rnd);
    const duration = firstBeat + (bars * 4 * 60) / bpm + 1.5;
    return {
      id: `mock-${String(i + 1).padStart(2, '0')}`,
      title,
      artist,
      bpm,
      key: parseCamelot(code),
      duration,
      analyzed: true,
      missing: false,
      incomplete: false,
      seed,
      firstBeat,
      grid,
      lufs: -11 + Math.round(rnd() * 50) / 10,
      sections,
      totalBars: bars,
    };
  });
}

export const MOCK_TRACKS: MockTrack[] = build();

export function trackInfo(t: MockTrack): TrackInfo {
  return { id: t.id, title: t.title, artist: t.artist, bpm: t.bpm, key: t.key, duration: t.duration, analyzed: true, missing: false, incomplete: false };
}

export function trackAnalysis(t: MockTrack): TrackAnalysis {
  return {
    version: 1,
    duration: t.duration,
    bpm: t.bpm,
    bpm_confidence: 0.96,
    grid: t.grid,
    key: t.key,
    key_confidence: t.key === null ? 0.18 : 0.84,
    tuning_cents: 0,
    lufs: t.lufs,
    lra: 5.5,
    true_peak_db: -0.4,
  };
}
