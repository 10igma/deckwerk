// Live-Schutz (Spezifikation 9.3): Was ein hörbares Deck unterbricht, braucht ein Halten
// (ca. 400 ms) oder einen zweiten Druck innerhalb 1 s. Reine Logik, Zeit kommt von außen.
import type { DeckState } from '../core/types';
import { channelOpen } from '../lib/audible';

export const HOLD_MS = 400;
export const CONFIRM_MS = 1000;

/** Hörbar: Das Deck läuft, der Kanalfader ist offen und der Crossfader ihm nicht ganz abgewandt. */
export function isAudible(deck: Pick<DeckState, 'loaded' | 'playing' | 'fader' | 'xf_assign'>, crossfader: number): boolean {
  return deck.loaded && deck.playing && channelOpen(deck, crossfader);
}

export type GuardResult = 'run' | 'armed';

/** Merkt sich den ersten Druck; der zweite innerhalb der Frist löst aus. */
export class ConfirmGuard {
  private armed: { id: string; at: number } | null = null;

  constructor(private windowMs = CONFIRM_MS) {}

  /** Druck auf eine geschützte Aktion. `run` = jetzt ausführen, `armed` = erster Druck. */
  press(id: string, now: number): GuardResult {
    const a = this.armed;
    if (a && a.id === id && now - a.at <= this.windowMs) {
      this.armed = null;
      return 'run';
    }
    this.armed = { id, at: now };
    return 'armed';
  }

  /** Aktion, die gerade auf den zweiten Druck wartet (oder null). */
  armedId(now: number): string | null {
    const a = this.armed;
    return a && now - a.at <= this.windowMs ? a.id : null;
  }

  clear(id?: string): void {
    if (!id || this.armed?.id === id) this.armed = null;
  }
}

export type SeekVerdict = 'ok' | 'need-snap' | 'need-shift';

/**
 * Sprung in der Übersicht (DECK-20, 9.3): Im laufenden Deck nur mit Snap (phasenerhaltend);
 * ist das Deck hörbar, zusätzlich nur mit Shift.
 */
export function seekVerdict(opts: { playing: boolean; audible: boolean; snap: boolean; shift: boolean; guard: boolean }): SeekVerdict {
  if (!opts.playing) return 'ok';
  if (!opts.snap) return 'need-snap';
  if (opts.guard && opts.audible && !opts.shift) return 'need-shift';
  return 'ok';
}

export type LoadVerdict = 'ok' | 'guard' | 'occupied' | 'none-free';

/**
 * Laden (DECK-04, 9.3). `targeted`: Lade-Taste, Shift + Pfeil oder Ablegen auf das Deck.
 * Der Doppelklick in der Liste (`targeted = false`) lädt nur in ein leeres Deck.
 */
export function loadVerdict(opts: { loaded: boolean; audible: boolean; targeted: boolean; guard: boolean }): LoadVerdict {
  if (!opts.targeted && opts.loaded) return 'occupied';
  if (opts.guard && opts.audible) return 'guard';
  return 'ok';
}
