import { describe, expect, it, vi } from 'vitest';

// Der Ablauf wird ohne Dokument geprüft; das Laden selbst gehört zu den Aktionen.
vi.mock('./actions', () => ({ loadByDrop: vi.fn() }));

import { DRAG_THRESHOLD, TrackDrag } from './dnd';

describe('Ziehen aus der Liste aufs Deck (DECK-04)', () => {
  it('unter der Schwelle bleibt es ein Klick: kein Ziehen, kein Laden', () => {
    const d = new TrackDrag();
    d.down('t1', 100, 100);
    expect(d.move(100 + DRAG_THRESHOLD - 1, 100, 'a')).toBe(false);
    expect(d.view).toBeNull();
    expect(d.up('a')).toBeNull();
  });

  it('ab der Schwelle wird gezogen; das Deck unter dem Zeiger wird gemeldet', () => {
    const d = new TrackDrag();
    d.down('t1', 100, 100);
    // Schräg: 3 und 4 Pixel ergeben genau 5.
    expect(d.move(103, 104, null)).toBe(true);
    expect(d.view).toEqual({ trackId: 't1', over: null });
    expect(d.move(300, 40, 'b')).toBe(true);
    expect(d.view).toEqual({ trackId: 't1', over: 'b' });
    // Gleiches Deck: keine Änderung, also kein neues Zeichnen.
    expect(d.move(310, 42, 'b')).toBe(false);
    expect(d.move(500, 400, null)).toBe(true);
    expect(d.view?.over).toBeNull();
  });

  it('Loslassen über einem Deck lädt genau diesen Track dorthin', () => {
    const d = new TrackDrag();
    d.down('t7', 0, 0);
    d.move(0, 60, 'a');
    expect(d.up('b')).toEqual({ deck: 'b', trackId: 't7' });
    expect(d.view).toBeNull();
  });

  it('Loslassen neben den Decks oder Abbruch lädt nichts', () => {
    const d = new TrackDrag();
    d.down('t7', 0, 0);
    d.move(0, 60, 'a');
    expect(d.up(null)).toBeNull();

    d.down('t7', 0, 0);
    d.move(0, 60, 'a');
    d.cancel();
    expect(d.view).toBeNull();
    expect(d.up('a')).toBeNull();
    // Ohne neuen Druck löst eine Bewegung nichts aus.
    expect(d.move(0, 200, 'a')).toBe(false);
  });
});
