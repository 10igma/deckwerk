// Globaler Tastatur-Handler: Tastenereignis → Belegung → Bedienhandlung.
import type { DeckId } from '../core/types';
import * as act from './actions';
import { resolveKey, type ActionId, type Binding } from './keymap';
import { navigate } from './router';

type Handler = (deck: DeckId | undefined, fine: boolean) => void;

const onDown: Partial<Record<ActionId, Handler>> = {
  'deck.play': (d) => d && act.togglePlay(d),
  'deck.cue': (d) => d && act.cueDown(d),
  'deck.sync': (d) => d && act.toggleSync(d),
  'deck.bend_down': (d) => d && act.bend(d, -1),
  'deck.bend_up': (d) => d && act.bend(d, 1),
  'deck.tempo_down': (d, fine) => d && act.tempoStep(d, -1, fine),
  'deck.tempo_up': (d, fine) => d && act.tempoStep(d, 1, fine),
  'deck.headphone': (d) => d && act.toggleHeadphone(d),
  'deck.load': (d) => d && act.loadSelected(d),
  'deck.kill_low': (d) => d && act.toggleKill(d, 'low'),
  'deck.filter_close': (d) => d && act.filterStep(d, -1),
  'deck.filter_open': (d) => d && act.filterStep(d, 1),
  'xf.left': () => act.crossfaderStep(-1, false),
  'xf.right': () => act.crossfaderStep(1, false),
  'xf.full_left': () => act.crossfaderStep(-1, true),
  'xf.full_right': () => act.crossfaderStep(1, true),
  'list.up': () => act.moveSelection(-1),
  'list.down': () => act.moveSelection(1),
  'search.focus': () => {
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
  },
  'view.mixen': () => navigate('mixen'),
  'view.bibliothek': () => navigate('bibliothek'),
  'view.vorbereiten': () => navigate('vorbereiten'),
  'view.automix': () => navigate('automix'),
  'global.snap': () => act.toggleSnap(),
};

const onUp: Partial<Record<ActionId, Handler>> = {
  'deck.cue': (d) => d && act.cueUp(d),
  'deck.bend_down': (d) => d && act.bend(d, 0),
  'deck.bend_up': (d) => d && act.bend(d, 0),
};

function isTextInput(el: EventTarget | null): boolean {
  return el instanceof HTMLInputElement || el instanceof HTMLTextAreaElement || (el instanceof HTMLElement && el.isContentEditable);
}

export function installKeyboard(): () => void {
  // Gehaltene Tasten: beim Loslassen zählt die Belegung vom Drücken, auch wenn Shift inzwischen losgelassen wurde.
  const held = new Map<string, Binding>();

  const keydown = (ev: KeyboardEvent) => {
    if (isTextInput(ev.target)) {
      // Im Suchfeld: nur Esc (verlassen), ↑/↓ (Liste) und F-Tasten wirken.
      if (ev.code === 'Escape') (ev.target as HTMLElement).blur();
      const passes = ev.code === 'ArrowUp' || ev.code === 'ArrowDown' || /^F\d+$/.test(ev.code) || ((ev.ctrlKey || ev.metaKey) && ev.code === 'KeyF');
      if (!passes) return;
    }
    // Regler mit Tastaturfokus behandeln Pfeiltasten selbst.
    if (ev.target instanceof HTMLElement && ev.target.getAttribute('role') === 'slider' && ev.code.startsWith('Arrow')) return;

    const hit = resolveKey(ev);
    if (!hit) return;
    ev.preventDefault();
    const b = hit.binding;
    if (ev.repeat && !b.repeat) return;
    if (b.mode === 'hold') {
      if (held.has(ev.code)) return;
      held.set(ev.code, b);
    }
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
