// Tonarten: Camelot-/Open-Key-Schreibweise und Verträglichkeit (Spiegel von dw-core/src/key.rs, 5.11).
import type { Key } from '../core/types';
import type { Lang } from './format';

const NAMES_DE_MAJOR = ['C-Dur', 'Des-Dur', 'D-Dur', 'Es-Dur', 'E-Dur', 'F-Dur', 'Fis-Dur', 'G-Dur', 'As-Dur', 'A-Dur', 'B-Dur', 'H-Dur'];
const NAMES_DE_MINOR = ['c-Moll', 'cis-Moll', 'd-Moll', 'es-Moll', 'e-Moll', 'f-Moll', 'fis-Moll', 'g-Moll', 'as-Moll', 'a-Moll', 'b-Moll', 'h-Moll'];
const NAMES_EN_MAJOR = ['C major', 'D♭ major', 'D major', 'E♭ major', 'E major', 'F major', 'F♯ major', 'G major', 'A♭ major', 'A major', 'B♭ major', 'B major'];
const NAMES_EN_MINOR = ['C minor', 'C♯ minor', 'D minor', 'E♭ minor', 'E minor', 'F minor', 'F♯ minor', 'G minor', 'A♭ minor', 'A minor', 'B♭ minor', 'B minor'];

export const isMinor = (key: Key): boolean => key >= 12;
export const pitchClass = (key: Key): number => key % 12;
export const makeKey = (pc: number, minor: boolean): Key => (((pc % 12) + 12) % 12) + (minor ? 12 : 0);

export interface Camelot {
  /** 1–12 */
  number: number;
  /** 'A' = Moll, 'B' = Dur */
  letter: 'A' | 'B';
}

export function camelot(key: Key): Camelot {
  const minor = isMinor(key);
  // Moll liegt über die Paralleltonart (kleine Terz höher) auf derselben Zahl.
  const majorPc = minor ? (pitchClass(key) + 3) % 12 : pitchClass(key);
  const number = (((majorPc * 7) % 12) + 7) % 12 + 1;
  return { number, letter: minor ? 'A' : 'B' };
}

export function camelotString(key: Key): string {
  const c = camelot(key);
  return `${c.number}${c.letter}`;
}

export function openKeyString(key: Key): string {
  const c = camelot(key);
  return `${((c.number + 4) % 12) + 1}${c.letter === 'A' ? 'm' : 'd'}`;
}

export function fromCamelot(number: number, letter: 'A' | 'B'): Key | null {
  for (let k = 0; k < 24; k++) {
    const c = camelot(k);
    if (c.number === number && c.letter === letter) return k;
  }
  return null;
}

export function keyName(key: Key, lang: Lang): string {
  const pc = pitchClass(key);
  if (lang === 'de') return isMinor(key) ? NAMES_DE_MINOR[pc] : NAMES_DE_MAJOR[pc];
  return isMinor(key) ? NAMES_EN_MINOR[pc] : NAMES_EN_MAJOR[pc];
}

/** Harmonische Verträglichkeit K für den Wechsel von a nach b (Tabelle in 5.11). */
export function harmonicCompat(a: Key, b: Key): number {
  const ca = camelot(a);
  const cb = camelot(b);
  const up = (((cb.number - ca.number) % 12) + 12) % 12;
  const same = ca.letter === cb.letter;
  if (up === 0) return same ? 1.0 : 0.9;
  if (up === 1 || up === 11) return same ? 0.9 : 0.6;
  if (up === 2 && same) return 0.6;
  if (up === 7 && same) return 0.5;
  return 0.1;
}

export type CompatClass = 'match' | 'ok' | 'clash';

/** Dreistufige Anzeige (VIS-06): passt / geht / reibt. */
export function compatClass(k: number): CompatClass {
  if (k >= 0.9) return 'match';
  if (k >= 0.5) return 'ok';
  return 'clash';
}

/** Sortierwert: erst Camelot-Zahl, dann Moll vor Dur. */
export function camelotSortValue(key: Key | null): number {
  if (key === null) return 999;
  const c = camelot(key);
  return c.number * 2 + (c.letter === 'A' ? 0 : 1);
}
