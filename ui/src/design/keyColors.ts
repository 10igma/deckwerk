// Tonart-Farben: 12 Farbtöne nach Camelot-Position, Moll (A) dunkler, Dur (B) heller.
// Die Werte stehen als Tokens in tokens.css; ein Test hält beide Quellen gleich.
import type { Key } from '../core/types';
import { camelot } from '../lib/key';
import { oklchToHex } from '../lib/color';

const HUE_START = 25;
const HUE_STEP = 30;
const MAJOR = { L: 0.84, C: 0.11 };
const MINOR = { L: 0.69, C: 0.14 };

export function camelotColorHex(number: number, letter: 'A' | 'B'): string {
  const hue = HUE_START + (number - 1) * HUE_STEP;
  const tone = letter === 'A' ? MINOR : MAJOR;
  return oklchToHex(tone.L, tone.C, hue);
}

export function keyColorHex(key: Key): string {
  const c = camelot(key);
  return camelotColorHex(c.number, c.letter);
}

/** Name des Tokens, z. B. `--key-8a`. */
export function keyTokenName(key: Key | null): string {
  if (key === null) return '--key-none';
  const c = camelot(key);
  return `--key-${c.number}${c.letter.toLowerCase()}`;
}

export const keyColorVar = (key: Key | null): string => `var(${keyTokenName(key)})`;
