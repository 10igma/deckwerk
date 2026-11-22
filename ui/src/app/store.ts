// Zustand der Oberfläche (alles, was nicht aus dem Kern kommt). Kleiner eigener Speicher
// auf Basis von useSyncExternalStore.
import { useSyncExternalStore } from 'react';
import type { DeckId, RecordingInfo, TrackId, TrackInfo, Transition, TransitionRun } from '../core/types';
import type { Lang } from '../lib/format';

export type SortKey = 'title' | 'artist' | 'bpm' | 'key' | 'duration';
export type Theme = 'dark' | 'contrast';
export type PerDeck<T> = Record<DeckId, T>;

export interface Notice {
  id: number;
  text: string;
  /** 'warn' = Schutz und Hinweise, 'error' = Fehler (z. B. beim Laden). */
  tone: 'warn' | 'error';
}

export interface UiState {
  lang: Lang;
  theme: Theme;
  /** Sichtbare Beats in den parallelen Wellenformen (1–128). */
  zoomBeats: number;
  /**
   * Vorausblick (VIS-13): Die Wellenformen aller Decks zeigen die Stelle, die in so vielen Beats
   * (bei Clock-Tempo) an der Abspielmarke liegt. 0 = Ansicht an der Abspielstelle. Die Wiedergabe
   * bleibt unberührt.
   */
  peekBeats: number;
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
  /** Live-Schutz nach 9.3 (abschaltbar). */
  liveGuard: boolean;
  /** Deck mit Fokus: das zuletzt bediente; die Leertaste wirkt darauf. */
  focusDeck: DeckId;
  /** Geschützte Aktion, die auf Halten oder den zweiten Druck wartet (z. B. 'stop.a'). */
  armedId: string | null;
  /** Track, der gerade auf ein Deck geladen wird (Dekodieren dauert). */
  loading: PerDeck<TrackInfo | null>;
  /** Bibliothek aufgezogen: Decks schrumpfen auf eine Zeile (8.2). */
  libraryBig: boolean;
  /** Liste zeigt nur, was zum Bezugs-Deck passt (BIB-11). */
  matchFilter: boolean;
  /** Laufender Mitschnitt und der Zeitpunkt (Uhr des Rechners, ms), zu dem er begann (REC-01). */
  recording: { info: RecordingInfo; since: number } | null;
  /** Eingeplanter oder laufender Übergang (TRANS-05). */
  transition: TransitionRun | null;
  /** Selbst gebaute Übergänge (TRANS-04). */
  savedTransitions: Transition[];
  /** Übergang im Editor: der Entwurf und der Name, unter dem er gespeichert ist (null = neu). */
  editor: { draft: Transition; original: string | null } | null;
  /** Laufendes Vorhören im Editor (TRANS-06). */
  preview: TransitionRun | null;
  /** Ab hier wird vorgehört: Beats ab T0, immer ein Taktanfang. */
  previewFrom: number;
  /** Stehen beide Decks: Ihre Stellen sind Beginn oder Ende des Übergangs. */
  previewAnchor: 'start' | 'end';
  /** Gewählte Vorlage und Länge für den nächsten Übergang. */
  transitionName: string;
  transitionBars: number;
  /** Tracks in der Analyse-Warteschlange. */
  pendingAnalysis: number;
  /** Die Liste wurde mindestens einmal vom Kern geholt. */
  tracksReady: boolean;
  /** Womit die Wellenformen gezeichnet werden (Anzeige in den Einstellungen). */
  rendererInfo: string;
}

const perDeck = <T,>(make: () => T): PerDeck<T> => ({ a: make(), b: make(), c: make(), d: make() });

function readStored(key: string): string | null {
  try {
    return localStorage.getItem(key);
  } catch {
    return null; // kein Speicher verfügbar
  }
}

function store(key: string, value: string): void {
  try {
    localStorage.setItem(key, value);
  } catch {
    /* kein Speicher verfügbar */
  }
}

function readLang(): Lang {
  const stored = readStored('dw.lang');
  return stored === 'en' ? 'en' : 'de';
}

let state: UiState = {
  lang: 'de',
  theme: 'dark',
  zoomBeats: 8,
  peekBeats: 0,
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
  liveGuard: true,
  focusDeck: 'a',
  armedId: null,
  loading: perDeck(() => null),
  libraryBig: false,
  matchFilter: false,
  recording: null,
  transition: null,
  savedTransitions: [],
  editor: null,
  preview: null,
  previewFrom: 0,
  previewAnchor: 'start',
  transitionName: 'bass_swap',
  transitionBars: 16,
  pendingAnalysis: 0,
  tracksReady: false,
  rendererInfo: '',
};

const listeners = new Set<() => void>();

export const getUi = (): UiState => state;

export function setUi(patch: Partial<UiState> | ((s: UiState) => Partial<UiState>)): void {
  const p = typeof patch === 'function' ? patch(state) : patch;
  state = { ...state, ...p };
  for (const l of listeners) l();
}

/** Einen Eintrag je Deck ändern. */
export function setDeckUi<K extends 'deckTracks' | 'cuePoints' | 'cueHeld' | 'tempoRange' | 'syncPref' | 'eqBeforeKill' | 'loading'>(
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
    liveGuard: params.get('guard') === 'off' ? false : readStored('dw.guard') !== 'off',
    libraryBig: params.get('lib') === 'big',
  });
}

export function setLang(lang: Lang): void {
  setUi({ lang });
  store('dw.lang', lang);
}

export function setLiveGuard(on: boolean): void {
  setUi({ liveGuard: on });
  store('dw.guard', on ? 'on' : 'off');
}

let noticeId = 0;
let noticeTimer: ReturnType<typeof setTimeout> | null = null;

/** Kurzen Hinweis zeigen (verschwindet von selbst). */
export function showNotice(text: string, tone: Notice['tone'] = 'warn', ms = tone === 'error' ? 7000 : 3200): void {
  noticeId += 1;
  setUi({ notice: { id: noticeId, text, tone } });
  if (noticeTimer) clearTimeout(noticeTimer);
  noticeTimer = setTimeout(() => setUi({ notice: null }), ms);
}
