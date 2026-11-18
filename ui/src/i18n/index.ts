// Sprachdateien (BED-07): alle Texte der Oberfläche liegen in de.json und en.json.
import de from './de.json';
import en from './en.json';
import type { Lang } from '../lib/format';
import { useUi } from '../app/store';

export type MsgKey = keyof typeof de;
export type Messages = Record<MsgKey, string>;

export const MESSAGES: Record<Lang, Messages> = { de, en: en as Messages };

export type Params = Record<string, string | number>;

/** Text holen; Platzhalter in der Form {name} werden ersetzt. */
export function translate(lang: Lang, key: MsgKey, params?: Params): string {
  const raw = MESSAGES[lang][key] ?? MESSAGES.de[key] ?? key;
  if (!params) return raw;
  return raw.replace(/\{(\w+)\}/g, (m, name: string) => (name in params ? String(params[name]) : m));
}

export type TFn = (key: MsgKey, params?: Params) => string;

export function useT(): TFn {
  const lang = useUi((s) => s.lang);
  return (key, params) => translate(lang, key, params);
}

export const useLang = (): Lang => useUi((s) => s.lang);
