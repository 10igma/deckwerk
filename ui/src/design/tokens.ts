// Zugriff auf Tokens aus Canvas- und WebGL-Code. Werte kommen immer aus den CSS-Variablen.
import { hexToRgb } from '../lib/color';
import type { DeckId } from '../core/types';

const cache = new Map<string, string>();
let cacheTheme: string | null = null;

/** Wert einer CSS-Variablen von :root, z. B. cssVar('--deck-a'). */
export function cssVar(name: string): string {
  const theme = document.documentElement.dataset.theme ?? '';
  if (theme !== cacheTheme) {
    cache.clear();
    cacheTheme = theme;
  }
  let v = cache.get(name);
  if (v === undefined) {
    v = getComputedStyle(document.documentElement).getPropertyValue(name).trim();
    cache.set(name, v);
  }
  return v;
}

export type Rgb = [number, number, number];

/** Farb-Token als RGB 0…1. */
export function cssRgb(name: string): Rgb {
  const [r, g, b] = hexToRgb(cssVar(name));
  return [r / 255, g / 255, b / 255];
}

export const cssNumber = (name: string): number => parseFloat(cssVar(name));

export const deckColorVar = (deck: DeckId): string => `var(--deck-${deck})`;
export const deckTokenName = (deck: DeckId): string => `--deck-${deck}`;

/** Aktuelle Wurzelgröße in CSS-Pixeln (1 rem). */
export const remPx = (): number => parseFloat(getComputedStyle(document.documentElement).fontSize) || 14;
