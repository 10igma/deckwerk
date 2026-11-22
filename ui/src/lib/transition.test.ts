import { describe, expect, it } from 'vitest';
import type { Transition } from '../core/types';
import { addPoint, bassClashes, between, findLane, freeName, fromHeight, movePoint, removePoint, rescale, setCurve, snapBeat, timeRange, toHeight, valueAt, valueText } from './transition';

/** Bass-Tausch über 16 Takte wie die Vorlage des Kerns (nur Fader und Bass). */
const swap = (): Transition => ({
  name: 'test',
  bars: 16,
  start_in: 0,
  stop_out: 64,
  reset_out: true,
  lanes: [
    { side: 'in', param: 'fader', points: [{ beat: -1, value: 0 }, { beat: 0, value: 0 }, { beat: 8, value: 1 }] },
    { side: 'in', param: 'eq_low', points: [{ beat: -1, value: null }, { beat: 31, value: null }, { beat: 32, value: 0 }] },
    { side: 'out', param: 'eq_low', points: [{ beat: 31, value: 0 }, { beat: 32, value: null }] },
    { side: 'out', param: 'fader', points: [{ beat: 48, value: 1 }, { beat: 64, value: 0 }] },
  ],
});

describe('Übergänge im Editor (TRANS-04)', () => {
  it('wertet Kurven aus wie der Kern: linear, weich, Stufe, Kill linear im Pegel', () => {
    const t = swap();
    const fader = findLane(t, 'in', 'fader')!;
    expect(valueAt(fader, -2)).toBe(undefined);
    expect(valueAt(fader, 0)).toBe(0);
    expect(valueAt(fader, 4)).toBe(0.5);
    expect(valueAt(fader, 100)).toBe(1);
    // Bass-Tausch: in der Mitte des Beats beide auf halbem Pegel (−6,02 dB).
    expect(valueAt(findLane(t, 'out', 'eq_low')!, 31.5)).toBeCloseTo(-6.0206, 3);
    expect(valueAt(findLane(t, 'in', 'eq_low')!, 31.5)).toBeCloseTo(-6.0206, 3);
    expect(valueAt(findLane(t, 'in', 'eq_low')!, 10)).toBe(null);
    // Kurvenformen
    const a = { beat: 0, value: 0 };
    expect(between('fader', a, { beat: 8, value: 1, curve: 'smooth' }, 0.5)).toBeCloseTo(0.5, 9);
    expect(between('fader', a, { beat: 8, value: 1, curve: 'smooth' }, 0.25)).toBeLessThan(0.25);
    expect(between('fader', a, { beat: 8, value: 1, curve: 'step' }, 0.99)).toBe(0);
    expect(between('fader', a, { beat: 8, value: 1, curve: 'step' }, 1)).toBe(1);
    expect(between('eq_mid', { beat: 0, value: -6 }, { beat: 8, value: 0 }, 0.5)).toBe(-3);
  });

  it('bildet Reglerwerte auf die Spurhöhe ab und zurück', () => {
    expect(toHeight('fader', 0.25)).toBe(0.25);
    expect(fromHeight('fader', 0.333)).toBe(0.33);
    expect(toHeight('filter', 0)).toBe(0.5);
    expect(fromHeight('filter', 0.51)).toBe(0);
    expect(fromHeight('filter', 1)).toBe(1);
    // EQ: Kill unten, +6 dB oben, 0 dB dazwischen; nahe 0 dB rastet es ein.
    expect(toHeight('eq_low', null)).toBe(0);
    expect(toHeight('eq_low', 6)).toBe(1);
    expect(fromHeight('eq_low', 0)).toBe(null);
    expect(fromHeight('eq_low', 1)).toBe(6);
    expect(fromHeight('eq_low', toHeight('eq_low', 0) + 0.01)).toBe(0);
    for (const db of [-24, -12, -6, -3, 3]) expect(fromHeight('eq_mid', toHeight('eq_mid', db))).toBe(db);
    expect(valueText('eq_low', null, 'Kill')).toBe('Kill');
    expect(valueText('eq_low', -6, 'Kill')).toBe('−6.0 dB');
    expect(valueText('fader', 0.5, 'Kill')).toBe('50 %');
  });

  it('setzt, verschiebt und löscht Punkte; die Reihenfolge bleibt', () => {
    let t = swap();
    expect(timeRange(t)).toEqual([-4, 64]);
    expect(snapBeat(t, 12.4)).toBe(12);
    expect(snapBeat(t, 99)).toBe(64);
    expect(snapBeat(t, -9)).toBe(-4);
    expect(snapBeat(t, 12.4, 0.25)).toBe(12.5);
    // Neue Spur durch den ersten Punkt.
    t = addPoint(t, 'out', 'filter', 40, 0);
    t = addPoint(t, 'out', 'filter', 56, 0.6);
    expect(findLane(t, 'out', 'filter')!.points.map((p) => [p.beat, p.value])).toEqual([[40, 0], [56, 0.6]]);
    // Punkt zwischen zwei vorhandene setzen: sortiert einreihen.
    t = addPoint(t, 'in', 'fader', 4, 0.8);
    expect(findLane(t, 'in', 'fader')!.points.map((p) => p.beat)).toEqual([-1, 0, 4, 8]);
    // Verschieben stößt an die Nachbarn (ein Beat Abstand).
    t = movePoint(t, 'in', 'fader', 2, 20, 0.7);
    expect(findLane(t, 'in', 'fader')!.points[2]).toEqual({ beat: 7, value: 0.7, curve: 'linear' });
    t = movePoint(t, 'in', 'fader', 2, -3, 0.7);
    expect(findLane(t, 'in', 'fader')!.points[2].beat).toBe(1);
    t = setCurve(t, 'in', 'fader', 3, 'smooth');
    expect(findLane(t, 'in', 'fader')!.points[3].curve).toBe('smooth');
    t = removePoint(t, 'in', 'fader', 2);
    expect(findLane(t, 'in', 'fader')!.points.map((p) => p.beat)).toEqual([-1, 0, 8]);
    // Mit dem letzten Punkt verschwindet die Spur.
    t = removePoint(removePoint(t, 'out', 'filter', 0), 'out', 'filter', 0);
    expect(findLane(t, 'out', 'filter')).toBe(undefined);
    // Die Vorlage selbst bleibt unberührt.
    expect(findLane(swap(), 'in', 'fader')!.points.length).toBe(3);
  });

  it('ändert die Länge: ab T0 gestreckt, die Vorbereitung davor bleibt', () => {
    const t = rescale(swap(), 32);
    expect((t.bars, [t.start_in, t.stop_out])).toEqual([0, 128]);
    expect(findLane(t, 'in', 'fader')!.points.map((p) => p.beat)).toEqual([-1, 0, 16]);
    expect(findLane(t, 'out', 'eq_low')!.points.map((p) => p.beat)).toEqual([62, 64]);
    // Stauchen: Der Bass-Tausch (ein Beat) fällt auf einen Punkt zusammen; es bleibt der spätere.
    const short = rescale(swap(), 4);
    expect(findLane(short, 'out', 'eq_low')!.points).toEqual([{ beat: 8, value: null }]);
    expect(rescale(swap(), 16)).toEqual(swap());
  });

  it('warnt, wenn zwei offene Bässe länger als einen Takt zusammen laufen (TRANS-03)', () => {
    expect(bassClashes(swap())).toEqual([]);
    // Bass von A bleibt bis Takt 12 offen, B hat seinen ab Takt 8: 16 Beats doppelt.
    let late = swap();
    late = movePoint(late, 'out', 'eq_low', 1, 48, null);
    late = movePoint(late, 'out', 'eq_low', 0, 47, 0);
    expect(bassClashes(late)).toEqual([[31.75, 47.5]]);
    // Ohne Bass-Spuren und mit beiden Fadern offen: doppelt über die ganze Mitte.
    const none: Transition = { ...swap(), lanes: swap().lanes.filter((l) => l.param === 'fader') };
    expect(bassClashes(none)).toEqual([[4.25, 56]]);
  });

  it('findet einen freien Namen', () => {
    expect(freeName('Übergang', [])).toBe('Übergang 1');
    expect(freeName('Übergang', ['Übergang 1', 'Übergang 2'])).toBe('Übergang 3');
  });
});
