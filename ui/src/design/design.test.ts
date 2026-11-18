// Prüft die Tokens selbst: Tonart-Farben, Kontraste (VIS-11) und Vollständigkeit.
import { readFileSync } from 'node:fs';
import { describe, expect, it } from 'vitest';
import { contrast, luminance } from '../lib/color';
import { camelot, fromCamelot } from '../lib/key';
import { camelotColorHex, keyColorHex, keyTokenName } from './keyColors';

const css = readFileSync(new URL('./tokens.css', import.meta.url), 'utf8');

function tokens(block: string): Map<string, string> {
  const out = new Map<string, string>();
  for (const m of block.matchAll(/(--[\w-]+)\s*:\s*([^;]+);/g)) out.set(m[1], m[2].trim());
  return out;
}

const rootBlock = css.slice(css.indexOf(':root {'), css.indexOf(":root[data-theme='contrast']"));
const contrastBlock = css.slice(css.indexOf(":root[data-theme='contrast']"));
const dark = tokens(rootBlock);
const high = new Map([...dark, ...tokens(contrastBlock)]);
const hex = (t: Map<string, string>, name: string): string => {
  const v = t.get(name);
  if (!v || !/^#[0-9a-fA-F]{6}$/.test(v)) throw new Error(`Token ${name} fehlt oder ist keine Hex-Farbe: ${v}`);
  return v;
};

describe('Tonart-Farben', () => {
  it('Tokens entsprechen der Berechnung aus der Camelot-Position', () => {
    for (let n = 1; n <= 12; n++) {
      for (const letter of ['A', 'B'] as const) {
        const key = fromCamelot(n, letter)!;
        expect(hex(dark, keyTokenName(key)).toUpperCase(), `${n}${letter}`).toBe(camelotColorHex(n, letter));
        expect(keyColorHex(key)).toBe(camelotColorHex(n, letter));
      }
    }
    expect(keyTokenName(null)).toBe('--key-none');
  });
  it('Moll ist bei gleicher Position dunkler als Dur', () => {
    for (let n = 1; n <= 12; n++) {
      expect(luminance(camelotColorHex(n, 'A'))).toBeLessThan(luminance(camelotColorHex(n, 'B')) - 0.1);
    }
  });
  it('zwölf verschiedene Farbtöne je Reihe', () => {
    for (const letter of ['A', 'B'] as const) {
      const set = new Set(Array.from({ length: 12 }, (_, i) => camelotColorHex(i + 1, letter)));
      expect(set.size).toBe(12);
    }
  });
  it('dunkle Schrift auf jeder Tonart-Marke ist lesbar (≥ 4,5:1)', () => {
    for (let k = 0; k < 24; k++) {
      const c = camelot(k);
      expect(contrast(camelotColorHex(c.number, c.letter), hex(dark, '--ink'))).toBeGreaterThanOrEqual(4.5);
    }
  });
});

describe.each([
  ['dunkel', dark],
  ['kontrastreich', high],
])('Kontraste, Design %s (VIS-11)', (_name, t) => {
  const surfaces = ['--bg-0', '--bg-1', '--bg-2', '--bg-3'];
  it('Schrift ≥ 4,5:1 auf allen Flächen', () => {
    for (const text of ['--text-1', '--text-2', '--text-3']) {
      for (const bg of surfaces) {
        expect(contrast(hex(t, text), hex(t, bg)), `${text} auf ${bg}`).toBeGreaterThanOrEqual(4.5);
      }
    }
  });
  it('Umriss von Bedienelementen ≥ 3:1 auf den Flächen, auf denen sie stehen', () => {
    for (const bg of ['--bg-0', '--bg-1', '--bg-2']) {
      expect(contrast(hex(t, '--line-2'), hex(t, bg)), `--line-2 auf ${bg}`).toBeGreaterThanOrEqual(3);
    }
  });
  it('Signalfarben ≥ 4,5:1 auf Grund und Platte, dunkle Schrift darauf ≥ 4,5:1', () => {
    const signals = ['--deck-a', '--deck-b', '--deck-c', '--deck-d', '--band-low', '--band-mid', '--band-high', '--status-ok', '--status-warn', '--status-danger'];
    for (const s of signals) {
      expect(contrast(hex(t, s), hex(t, '--bg-0')), `${s} auf --bg-0`).toBeGreaterThanOrEqual(4.5);
      expect(contrast(hex(t, s), hex(t, '--bg-1')), `${s} auf --bg-1`).toBeGreaterThanOrEqual(4.5);
      expect(contrast(hex(t, '--ink'), hex(t, s)), `--ink auf ${s}`).toBeGreaterThanOrEqual(4.5);
    }
  });
  it('Frequenzbänder unterscheiden sich auch in der Helligkeit (VIS-12)', () => {
    const [lo, mid, hi] = ['--band-low', '--band-mid', '--band-high'].map((n) => luminance(hex(t, n)));
    expect(mid / lo).toBeGreaterThan(1.5);
    expect(hi / mid).toBeGreaterThan(1.4);
  });
});
