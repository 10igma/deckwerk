import { describe, expect, it } from 'vitest';
import { emptyDeckState } from '../core/types';
import { CONFIRM_MS, ConfirmGuard, HOLD_MS, isAudible, loadVerdict, seekVerdict } from './guard';

const deck = (over: Partial<ReturnType<typeof emptyDeckState>> = {}) => ({ ...emptyDeckState(), loaded: true, playing: true, fader: 1, xf_assign: 'left' as const, ...over });

describe('Live-Schutz: hörbar (9.3)', () => {
  it('läuft, Fader offen, Crossfader nicht ganz abgewandt', () => {
    expect(isAudible(deck(), 0)).toBe(true);
    expect(isAudible(deck(), -1)).toBe(true);
    expect(isAudible(deck(), 0.5)).toBe(true);
  });
  it('unhörbar: steht, Fader unten oder Crossfader ganz auf der anderen Seite', () => {
    expect(isAudible(deck({ playing: false }), 0)).toBe(false);
    expect(isAudible(deck({ fader: 0 }), 0)).toBe(false);
    expect(isAudible(deck(), 1)).toBe(false);
    expect(isAudible(deck({ xf_assign: 'right' }), -1)).toBe(false);
    expect(isAudible(deck({ loaded: false }), 0)).toBe(false);
  });
  it('ohne Crossfader-Zuweisung zählt nur der Fader', () => {
    expect(isAudible(deck({ xf_assign: 'thru' }), 1)).toBe(true);
    expect(isAudible(deck({ xf_assign: 'thru', fader: 0 }), 1)).toBe(false);
  });
});

describe('Live-Schutz: zweiter Druck innerhalb 1 s', () => {
  it('Fristen wie in der Spezifikation', () => {
    expect(HOLD_MS).toBe(400);
    expect(CONFIRM_MS).toBe(1000);
  });
  it('erster Druck schärft, zweiter löst aus', () => {
    const g = new ConfirmGuard();
    expect(g.press('stop.a', 1000)).toBe('armed');
    expect(g.armedId(1500)).toBe('stop.a');
    expect(g.press('stop.a', 1900)).toBe('run');
    expect(g.armedId(1901)).toBeNull();
  });
  it('nach Ablauf der Frist beginnt es von vorn', () => {
    const g = new ConfirmGuard();
    expect(g.press('stop.a', 0)).toBe('armed');
    expect(g.armedId(1001)).toBeNull();
    expect(g.press('stop.a', 1001)).toBe('armed');
    expect(g.press('stop.a', 2001)).toBe('run');
  });
  it('eine andere Aktion bestätigt nicht', () => {
    const g = new ConfirmGuard();
    g.press('stop.a', 0);
    expect(g.press('stop.b', 100)).toBe('armed');
    expect(g.press('stop.a', 200)).toBe('armed');
    expect(g.press('cue.a', 300)).toBe('armed');
    g.clear('cue.a');
    expect(g.press('cue.a', 400)).toBe('armed');
  });
});

describe('Live-Schutz: Sprung in der Übersicht (DECK-20, 9.3)', () => {
  const base = { playing: true, audible: true, snap: true, shift: false, guard: true };
  it('stehendes Deck springt immer', () => {
    expect(seekVerdict({ ...base, playing: false, snap: false })).toBe('ok');
  });
  it('laufendes Deck ohne Snap springt nie', () => {
    expect(seekVerdict({ ...base, snap: false })).toBe('need-snap');
    expect(seekVerdict({ ...base, snap: false, shift: true })).toBe('need-snap');
    expect(seekVerdict({ ...base, snap: false, guard: false, audible: false })).toBe('need-snap');
  });
  it('hörbares Deck springt nur mit Shift', () => {
    expect(seekVerdict(base)).toBe('need-shift');
    expect(seekVerdict({ ...base, shift: true })).toBe('ok');
  });
  it('unhörbares laufendes Deck springt ohne Hürde; abgeschalteter Schutz auch', () => {
    expect(seekVerdict({ ...base, audible: false })).toBe('ok');
    expect(seekVerdict({ ...base, guard: false })).toBe('ok');
  });
});

describe('Live-Schutz: Laden (DECK-04, 9.3)', () => {
  it('Doppelklick lädt nur in ein leeres Deck', () => {
    expect(loadVerdict({ loaded: false, audible: false, targeted: false, guard: true })).toBe('ok');
    expect(loadVerdict({ loaded: true, audible: false, targeted: false, guard: true })).toBe('occupied');
    expect(loadVerdict({ loaded: true, audible: true, targeted: false, guard: false })).toBe('occupied');
  });
  it('gezielt: stehendes oder unhörbares Deck ohne Hürde, hörbares geschützt', () => {
    expect(loadVerdict({ loaded: true, audible: false, targeted: true, guard: true })).toBe('ok');
    expect(loadVerdict({ loaded: true, audible: true, targeted: true, guard: true })).toBe('guard');
    expect(loadVerdict({ loaded: true, audible: true, targeted: true, guard: false })).toBe('ok');
  });
});
