// Globaler Tastatur-Handler: Tastenereignis → Belegung → Bedienhandlung.
import type { DeckId } from '../core/types';
import * as act from './actions';
import { resolveKey, type ActionId, type Binding } from './keymap';
import { navigate } from './router';
import { getUi } from './store';

type Handler = (deck: DeckId | undefined, fine: boolean) => void;

function focusSearch(): void {
  // Sofort fokussieren, damit die nächsten Tasten schon im Suchfeld landen.
  const focus = () => {
    const input = document.querySelector<HTMLInputElement>('[data-search-input]');
    input?.focus();
    input?.select();
    return !!input;
  };
  if (!focus()) {
    navigate('mixen');
    requestAnimationFrame(focus);
  }
}

/** Esc: Fokus eines Reglers oder einer Taste lösen, damit die Pfeiltasten wieder global wirken (8.7). */
function releaseFocus(): void {
  const el = document.activeElement;
  if (el instanceof HTMLElement && el !== document.body) el.blur();
}

const onDown: Partial<Record<ActionId, Handler>> = {
  'deck.play': (d) => d && act.playDown(d),
  'deck.play_focus': () => act.playDown(getUi().focusDeck),
  'deck.cue': (d) => d && act.cueDown(d),
  'deck.sync': (d) => d && act.toggleSync(d),
  'deck.bend_down': (d) => d && act.bend(d, -1),
  'deck.bend_up': (d) => d && act.bend(d, 1),
  'deck.tempo_down': (d, fine) => d && act.tempoStep(d, -1, fine),
  'deck.tempo_up': (d, fine) => d && act.tempoStep(d, 1, fine),
  'deck.fader_up': (d, fine) => d && act.faderStep(d, 1, fine),
  'deck.fader_down': (d, fine) => d && act.faderStep(d, -1, fine),
  'deck.headphone': (d) => d && act.toggleHeadphone(d),
  'deck.load': (d) => d && act.loadSelectedDown(d),
  'deck.kill_low': (d) => d && act.toggleKill(d, 'low'),
  'deck.filter_close': (d) => d && act.filterStep(d, -1),
  'deck.filter_open': (d) => d && act.filterStep(d, 1),
  'xf.left': () => act.crossfaderStep(-1, false),
  'xf.right': () => act.crossfaderStep(1, false),
  'xf.full_left': () => act.crossfaderStep(-1, true),
  'xf.full_right': () => act.crossfaderStep(1, true),
  'list.up': () => act.moveSelection(-1),
  'list.down': () => act.moveSelection(1),
  'search.focus': focusSearch,
  'focus.release': releaseFocus,
  'view.mixen': () => navigate('mixen'),
  'view.bibliothek': () => navigate('bibliothek'),
  'view.vorbereiten': () => navigate('vorbereiten'),
  'view.automix': () => navigate('automix'),
  'global.snap': () => act.toggleSnap(),
};

// Loslassen: beendet Halten-Tasten und bricht das „Halten" des Live-Schutzes ab.
const onUp: Partial<Record<ActionId, Handler>> = {
  'deck.play': (d) => d && act.playUp(d),
  'deck.play_focus': () => {
    act.playUp('a');
    act.playUp('b');
  },
  'deck.cue': (d) => d && act.cueUp(d),
  'deck.load': (d) => d && act.loadUp(d),
  'deck.bend_down': (d) => d && act.bend(d, 0),
  'deck.bend_up': (d) => d && act.bend(d, 0),
};

function isTextInput(el: EventTarget | null): boolean {
  return el instanceof HTMLInputElement || el instanceof HTMLTextAreaElement || (el instanceof HTMLElement && el.isContentEditable);
}

export function installKeyboard(): () => void {
  // Gedrückte Tasten: beim Loslassen zählt die Belegung vom Drücken, auch wenn Shift inzwischen losgelassen wurde.
  const held = new Map<string, Binding>();

  const keydown = (ev: KeyboardEvent) => {
    if (isTextInput(ev.target)) {
      // In Eingabefeldern: nur Esc (verlassen), ↑/↓ (Liste), F-Tasten und Strg+F wirken.
      if (ev.code === 'Escape') (ev.target as HTMLElement).blur();
      const passes = ev.code === 'ArrowUp' || ev.code === 'ArrowDown' || /^F\d+$/.test(ev.code) || ((ev.ctrlKey || ev.metaKey) && ev.code === 'KeyF');
      if (!passes) return;
    }
    const target = ev.target instanceof HTMLElement ? ev.target : null;
    // Regler mit Tastaturfokus behandeln die bloßen Pfeiltasten selbst; mit Shift/Alt gilt die globale Belegung.
    if (target?.getAttribute('role') === 'slider' && ev.code.startsWith('Arrow') && !ev.shiftKey && !ev.altKey) return;
    // Eine Taste mit Tastaturfokus löst mit Leertaste/Enter sich selbst aus.
    if (target instanceof HTMLButtonElement && (ev.code === 'Space' || ev.code === 'Enter')) return;
    if (target instanceof HTMLAnchorElement && ev.code === 'Enter') return;

    const hit = resolveKey(ev);
    if (!hit) return;
    ev.preventDefault();
    const b = hit.binding;
    if (ev.repeat && !b.repeat) return;
    if (!ev.repeat) held.set(ev.code, b);
    onDown[b.action]?.(b.deck, hit.fine);
  };

  const keyup = (ev: KeyboardEvent) => {
    const b = held.get(ev.code);
    if (!b) return;
    held.delete(ev.code);
    onUp[b.action]?.(b.deck, false);
  };

  const releaseAll = () => {
    for (const b of held.values()) onUp[b.action]?.(b.deck, false);
    held.clear();
  };

  window.addEventListener('keydown', keydown);
  window.addEventListener('keyup', keyup);
  window.addEventListener('blur', releaseAll);
  return () => {
    window.removeEventListener('keydown', keydown);
    window.removeEventListener('keyup', keyup);
    window.removeEventListener('blur', releaseAll);
  };
}
