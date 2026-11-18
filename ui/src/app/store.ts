// Zustand der Oberfläche (alles, was nicht aus dem Kern kommt). Kleiner eigener Speicher
// auf Basis von useSyncExternalStore.
import { useSyncExternalStore } from 'react';
import type { DeckId, TrackId, TrackInfo } from '../core/types';
import type { Lang } from '../lib/format';

export type SortKey = 'title' | 'artist' | 'bpm' | 'key' | 'duration';
export type Theme = 'dark' | 'contrast';
export type PerDeck<T> = Record<DeckId, T>;

export interface Notice {
  id: number;
  text: string;
}

export interface UiState {
  lang: Lang;
  theme: Theme;
  /** Sichtbare Beats in den parallelen Wellenformen (1–32). */
  zoomBeats: number;
  /** Snap: Sprünge im laufenden Deck rasten auf Beats (DECK-20). */
  snap: boolean;
  tracks: TrackInfo[];
  deckTracks: PerDeck<TrackInfo | null>;
  /** Cue-Punkt je Deck in Sekunden. */
  cuePoints: PerDeck<number>;
  /** Cue-Taste wird gehalten und spielt vor. */
  cueHeld: PerDeck<boolean>;
  tempoRange: PerDeck<number>;
  syncPref: PerDeck<'tempo' | 'beat'>;
  /** EQ-Werte vor dem Kill, zum Zurückschalten. */
  eqBeforeKill: PerDeck<[number, number, number]>;
  selectedId: TrackId | null;
  search: string;
  sortKey: SortKey;
  sortDir: 1 | -1;
  notice: Notice | null;
  settingsOpen: boolean;
}

const perDeck = <T,>(make: () => T): PerDeck<T> => ({ a: make(), b: make(), c: make(), d: make() });

function readLang(): Lang {
  try {
    const stored = localStorage.getItem('dw.lang');
    if (stored === 'de' || stored === 'en') return stored;
  } catch {
    /* kein Speicher verfügbar */
  }
  return 'de';
}

let state: UiState = {
  lang: 'de',
  theme: 'dark',
  zoomBeats: 8,
  snap: true,
  tracks: [],
  deckTracks: perDeck(() => null),
  cuePoints: perDeck(() => 0),
  cueHeld: perDeck(() => false),
  tempoRange: perDeck(() => 0.08),
  syncPref: perDeck(() => 'beat' as const),
  eqBeforeKill: perDeck(() => [0, 0, 0] as [number, number, number]),
  selectedId: null,
  search: '',
  sortKey: 'title',
  sortDir: 1,
  notice: null,
  settingsOpen: false,
};

const listeners = new Set<() => void>();

export const getUi = (): UiState => state;

export function setUi(patch: Partial<UiState> | ((s: UiState) => Partial<UiState>)): void {
  const p = typeof patch === 'function' ? patch(state) : patch;
  state = { ...state, ...p };
  for (const l of listeners) l();
}

/** Einen Eintrag je Deck ändern. */
export function setDeckUi<K extends 'deckTracks' | 'cuePoints' | 'cueHeld' | 'tempoRange' | 'syncPref' | 'eqBeforeKill'>(
  key: K,
  deck: DeckId,
  value: UiState[K][DeckId],
): void {
  setUi((s) => ({ [key]: { ...s[key], [deck]: value } }) as Partial<UiState>);
}

function subscribe(l: () => void): () => void {
  listeners.add(l);
  return () => listeners.delete(l);
}

export function useUi<T>(select: (s: UiState) => T): T {
  return useSyncExternalStore(subscribe, () => select(state));
}

/** Gespeicherte Einstellungen laden (Sprache, Design). URL-Parameter haben Vorrang. */
export function initUi(search: string): void {
  const params = new URLSearchParams(search);
  const lang = params.get('lang');
  const theme = params.get('theme');
  setUi({
    lang: lang === 'de' || lang === 'en' ? lang : readLang(),
    theme: theme === 'contrast' ? 'contrast' : 'dark',
  });
}

export function setLang(lang: Lang): void {
  setUi({ lang });
  try {
    localStorage.setItem('dw.lang', lang);
  } catch {
    /* kein Speicher verfügbar */
  }
}

let noticeId = 0;
let noticeTimer: ReturnType<typeof setTimeout> | null = null;

/** Kurzen Hinweis zeigen (verschwindet von selbst). */
export function showNotice(text: string, ms = 3200): void {
  noticeId += 1;
  setUi({ notice: { id: noticeId, text } });
  if (noticeTimer) clearTimeout(noticeTimer);
  noticeTimer = setTimeout(() => setUi({ notice: null }), ms);
}
