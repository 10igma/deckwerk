// Zahlenformate. Deutsch mit Dezimalkomma, Englisch mit Punkt.

export type Lang = 'de' | 'en';

const MINUS = '−';
export const decimalSep = (lang: Lang): string => (lang === 'de' ? ',' : '.');

export function formatNumber(value: number, digits: number, lang: Lang, forceSign = false): string {
  const rounded = Math.abs(value) < Math.pow(10, -digits) / 2 ? 0 : value;
  const body = Math.abs(rounded).toFixed(digits).replace('.', decimalSep(lang));
  if (rounded < 0) return MINUS + body;
  return forceSign && rounded > 0 ? '+' + body : forceSign ? '±' + body : body;
}

export const formatBpm = (bpm: number, lang: Lang, digits = 2): string =>
  bpm > 0 ? formatNumber(bpm, digits, lang) : digits === 2 ? `---${decimalSep(lang)}--` : `---${decimalSep(lang)}-`;

/** Tempo-Abweichung als Faktor → „+0,75 %“. */
export const formatPercent = (factor: number, lang: Lang, digits = 2): string =>
  `${formatNumber(factor * 100, digits, lang, true)} %`;

export function formatDb(db: number, lang: Lang, digits = 1): string {
  if (db === -Infinity) return `${MINUS}∞ dB`;
  return `${formatNumber(db, digits, lang, true)} dB`;
}

export function formatHz(hz: number, lang: Lang): string {
  if (hz >= 1000) return `${formatNumber(hz / 1000, hz >= 10000 ? 0 : 1, lang)} kHz`;
  return `${Math.round(hz)} Hz`;
}

/** Phase in Beats, −0,5 … +0,5. */
export const formatPhase = (phase: number, lang: Lang): string => formatNumber(phase, 2, lang, true);
