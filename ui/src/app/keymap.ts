// Standard-Tastenbelegung (Spezifikation 8.7), gebunden an KeyboardEvent.code – also an die
// physische Position, unabhängig vom Tastaturlayout (BED-03). Reine Daten und Logik.
import type { DeckId } from '../core/types';
import type { Lang } from '../lib/format';

export type ActionId =
  | 'deck.play'
  | 'deck.play_focus'
  | 'deck.fader_up'
  | 'deck.fader_down'
  | 'focus.release'
  | 'deck.cue'
  | 'deck.sync'
  | 'deck.bend_down'
  | 'deck.bend_up'
  | 'deck.tempo_down'
  | 'deck.tempo_up'
  | 'deck.headphone'
  | 'deck.load'
  | 'deck.kill_low'
  | 'deck.filter_close'
  | 'deck.filter_open'
  | 'deck.hotcue'
  | 'deck.loop_toggle'
  | 'deck.loop_halve'
  | 'deck.loop_double'
  | 'deck.jump_back'
  | 'deck.jump_fwd'
  | 'deck.flux'
  | 'xf.left'
  | 'xf.right'
  | 'xf.full_left'
  | 'xf.full_right'
  | 'list.up'
  | 'list.down'
  | 'search.focus'
  | 'view.mixen'
  | 'view.bibliothek'
  | 'view.vorbereiten'
  | 'view.automix'
  | 'stage.peek_fwd'
  | 'stage.peek_back'
  | 'stage.peek_reset'
  | 'global.quantize'
  | 'global.snap'
  | 'global.record';

export interface Binding {
  /** KeyboardEvent.code, z. B. 'KeyQ'. */
  code: string;
  /** true = Shift nötig; 'fine' = Shift erlaubt und bedeutet Feineinstellung. */
  shift?: boolean | 'fine';
  /** Strg (Windows/Linux) oder Cmd (macOS). */
  mod?: boolean;
  alt?: boolean;
  action: ActionId;
  deck?: DeckId;
  arg?: number;
  /** Auslösen beim Drücken oder Halten bis zum Loslassen (BED-04). */
  mode: 'trigger' | 'hold';
  /** Tastenwiederholung des Systems zulassen. */
  repeat?: boolean;
  /** Ausbaustufe, ab der die Funktion existiert. */
  stage: number;
}

/** Stand der Oberfläche: Funktionen späterer Stufen sind belegt, aber noch nicht aktiv. */
export const CURRENT_STAGE = 1;

function deckBindings(deck: DeckId, k: Record<string, string>): Binding[] {
  const hot = k.hot.split(' ');
  return [
    { code: k.play, action: 'deck.play', deck, mode: 'trigger', stage: 1 },
    { code: k.cue, action: 'deck.cue', deck, mode: 'hold', stage: 1 },
    { code: k.sync, action: 'deck.sync', deck, mode: 'trigger', stage: 1 },
    { code: k.bendDown, action: 'deck.bend_down', deck, mode: 'hold', stage: 1 },
    { code: k.bendUp, action: 'deck.bend_up', deck, mode: 'hold', stage: 1 },
    { code: k.tempoDown, shift: 'fine', action: 'deck.tempo_down', deck, mode: 'trigger', repeat: true, stage: 1 },
    { code: k.tempoUp, shift: 'fine', action: 'deck.tempo_up', deck, mode: 'trigger', repeat: true, stage: 1 },
    { code: k.headphone, action: 'deck.headphone', deck, mode: 'trigger', stage: 1 },
    { code: k.load, shift: true, action: 'deck.load', deck, mode: 'trigger', stage: 1 },
    { code: k.killLow, action: 'deck.kill_low', deck, mode: 'trigger', stage: 1 },
    { code: k.bendDown, mod: true, action: 'deck.filter_close', deck, mode: 'trigger', repeat: true, stage: 1 },
    { code: k.bendUp, mod: true, action: 'deck.filter_open', deck, mode: 'trigger', repeat: true, stage: 1 },
    // 8.7 legt die Kanalfader auf Strg + D/F bzw. Strg + L/; – Strg + F ist dort aber auch die Suche.
    // Die Suche behält Strg/Cmd + F; die Kanalfader liegen auf Alt (wie „Crossfader ganz").
    { code: k.tempoDown, alt: true, shift: 'fine', action: 'deck.fader_up', deck, mode: 'trigger', repeat: true, stage: 1 },
    { code: k.tempoUp, alt: true, shift: 'fine', action: 'deck.fader_down', deck, mode: 'trigger', repeat: true, stage: 1 },
    // Ab Stufe 2 (Hotcues, Loops, Beatjump, Flux):
    ...hot.map((code, i): Binding => ({ code, action: 'deck.hotcue', deck, arg: i + 1, mode: 'trigger', stage: 2 })),
    { code: k.loop, action: 'deck.loop_toggle', deck, mode: 'trigger', stage: 2 },
    { code: k.loopHalve, action: 'deck.loop_halve', deck, mode: 'trigger', stage: 2 },
    { code: k.loopDouble, action: 'deck.loop_double', deck, mode: 'trigger', stage: 2 },
    { code: k.jumpBack, action: 'deck.jump_back', deck, mode: 'trigger', repeat: true, stage: 2 },
    { code: k.jumpFwd, action: 'deck.jump_fwd', deck, mode: 'trigger', repeat: true, stage: 2 },
    { code: k.flux, action: 'deck.flux', deck, mode: 'trigger', stage: 2 },
  ];
}

export const DEFAULT_KEYMAP: Binding[] = [
  ...deckBindings('a', {
    play: 'KeyQ', cue: 'KeyW', sync: 'KeyE', bendDown: 'KeyA', bendUp: 'KeyS', tempoDown: 'KeyD', tempoUp: 'KeyF',
    headphone: 'KeyT', load: 'ArrowLeft', killLow: 'KeyG', hot: 'Digit1 Digit2 Digit3 Digit4',
    loop: 'KeyC', loopHalve: 'KeyZ', loopDouble: 'KeyX', jumpBack: 'KeyV', jumpFwd: 'KeyB', flux: 'KeyR',
  }),
  ...deckBindings('b', {
    play: 'KeyU', cue: 'KeyI', sync: 'KeyO', bendDown: 'KeyJ', bendUp: 'KeyK', tempoDown: 'KeyL', tempoUp: 'Semicolon',
    headphone: 'KeyY', load: 'ArrowRight', killLow: 'KeyH', hot: 'Digit7 Digit8 Digit9 Digit0',
    loop: 'Comma', loopHalve: 'KeyN', loopDouble: 'KeyM', jumpBack: 'Period', jumpFwd: 'Slash', flux: 'KeyP',
  }),
  { code: 'ArrowLeft', action: 'xf.left', mode: 'trigger', repeat: true, stage: 1 },
  { code: 'ArrowRight', action: 'xf.right', mode: 'trigger', repeat: true, stage: 1 },
  // Alt = ganz nach links/rechts (8.7).
  { code: 'ArrowLeft', alt: true, action: 'xf.full_left', mode: 'trigger', stage: 1 },
  { code: 'ArrowRight', alt: true, action: 'xf.full_right', mode: 'trigger', stage: 1 },
  { code: 'ArrowUp', action: 'list.up', mode: 'trigger', repeat: true, stage: 1 },
  { code: 'ArrowDown', action: 'list.down', mode: 'trigger', repeat: true, stage: 1 },
  { code: 'KeyF', mod: true, action: 'search.focus', mode: 'trigger', stage: 1 },
  { code: 'F1', action: 'view.mixen', mode: 'trigger', stage: 0 },
  { code: 'F2', action: 'view.bibliothek', mode: 'trigger', stage: 0 },
  { code: 'F3', action: 'view.vorbereiten', mode: 'trigger', stage: 0 },
  { code: 'F4', action: 'view.automix', mode: 'trigger', stage: 0 },
  { code: 'F10', action: 'global.snap', mode: 'trigger', stage: 1 },
  // Leertaste: Play/Pause des Decks mit Fokus (das zuletzt bediente).
  { code: 'Space', action: 'deck.play_focus', mode: 'trigger', stage: 1 },
  { code: 'Escape', action: 'focus.release', mode: 'trigger', stage: 1 },
  // Vorausblick in den Wellenformen (VIS-13): vier Takte, mit Shift ein Takt; Pos1 oder Esc zurück.
  { code: 'PageDown', shift: 'fine', action: 'stage.peek_fwd', mode: 'trigger', repeat: true, stage: 1 },
  { code: 'PageUp', shift: 'fine', action: 'stage.peek_back', mode: 'trigger', repeat: true, stage: 1 },
  { code: 'Home', action: 'stage.peek_reset', mode: 'trigger', stage: 1 },
  { code: 'F9', action: 'global.quantize', mode: 'trigger', stage: 2 },
  { code: 'KeyR', mod: true, action: 'global.record', mode: 'trigger', stage: 1 },
];

export interface KeyLike {
  code: string;
  shiftKey: boolean;
  ctrlKey: boolean;
  metaKey: boolean;
  altKey: boolean;
}

export interface ResolvedKey {
  binding: Binding;
  /** Shift als Feineinstellung gedrückt. */
  fine: boolean;
}

function matches(b: Binding, ev: KeyLike): boolean {
  if (b.code !== ev.code) return false;
  if (!!b.mod !== (ev.ctrlKey || ev.metaKey)) return false;
  if (!!b.alt !== ev.altKey) return false;
  if (b.shift === 'fine') return true;
  return !!b.shift === ev.shiftKey;
}

/** Belegung für ein Tastenereignis finden. Funktionen späterer Stufen liefern null. */
export function resolveKey(ev: KeyLike, map: Binding[] = DEFAULT_KEYMAP, stage = CURRENT_STAGE): ResolvedKey | null {
  const b = map.find((x) => matches(x, ev));
  if (!b || b.stage > stage) return null;
  return { binding: b, fine: b.shift === 'fine' && ev.shiftKey };
}

const CODE_LABELS: Record<string, string> = {
  Semicolon: ';',
  Comma: ',',
  Period: '.',
  Slash: '/',
  ArrowLeft: '←',
  ArrowRight: '→',
  ArrowUp: '↑',
  ArrowDown: '↓',
  Space: '␣',
  Escape: 'Esc',
  PageDown: 'PgDn',
  PageUp: 'PgUp',
};

/** Beschriftung einer Taste nach US-Layout (wie in 8.7). */
export function codeLabel(code: string): string {
  if (CODE_LABELS[code]) return CODE_LABELS[code];
  if (code.startsWith('Key')) return code.slice(3);
  if (code.startsWith('Digit')) return code.slice(5);
  return code;
}

/** Tastenkürzel als Liste von Kappen, z. B. ['Shift', '←']. */
export function bindingKeys(b: Binding, lang: Lang): string[] {
  const out: string[] = [];
  if (b.mod) out.push(lang === 'de' ? 'Strg' : 'Ctrl');
  if (b.alt) out.push('Alt');
  if (b.shift === true) out.push('Shift');
  out.push(codeLabel(b.code));
  return out;
}

export function findBinding(action: ActionId, deck?: DeckId, map: Binding[] = DEFAULT_KEYMAP): Binding | undefined {
  return map.find((b) => b.action === action && b.deck === deck);
}

/** Kürzel einer Funktion für die Kurzhilfe; leer, wenn nicht belegt. */
export function shortcutKeys(action: ActionId, deck: DeckId | undefined, lang: Lang): string[] {
  const b = findBinding(action, deck);
  return b ? bindingKeys(b, lang) : [];
}

/** Doppelt belegte Tastenkombinationen finden (Prüfung der Belegung). */
export function findConflicts(map: Binding[] = DEFAULT_KEYMAP): string[] {
  const seen = new Map<string, number>();
  for (const b of map) {
    const shift = b.shift === true ? 'S' : b.shift === 'fine' ? '*' : '-';
    const id = `${b.code}|${b.mod ? 'M' : '-'}|${b.alt ? 'A' : '-'}|${shift}`;
    seen.set(id, (seen.get(id) ?? 0) + 1);
    // 'fine' deckt auch die Kombination mit Shift ab.
    if (b.shift === 'fine') {
      const withShift = `${b.code}|${b.mod ? 'M' : '-'}|${b.alt ? 'A' : '-'}|S`;
      seen.set(withShift, (seen.get(withShift) ?? 0) + 1);
    }
  }
  return [...seen.entries()].filter(([, n]) => n > 1).map(([id]) => id);
}
