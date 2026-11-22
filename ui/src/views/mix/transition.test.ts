import { describe, expect, it } from 'vitest';
import { transitionRoles } from '../../app/actions';
import { MockCore } from '../../core/mock/MockCore';
import { MOCK_TRACKS } from '../../core/mock/tracks';
import { transitionProgress } from './WaveStage';

describe('Übergang auf Knopfdruck (TRANS-05)', () => {
  it('Rollen: das laufende Deck geht, das stehende kommt; sonst keine', () => {
    const d = (loaded: boolean, playing: boolean) => ({ loaded, playing });
    expect(transitionRoles(d(true, true), d(true, false))).toEqual({ from: 'a', to: 'b' });
    expect(transitionRoles(d(true, false), d(true, true))).toEqual({ from: 'b', to: 'a' });
    expect(transitionRoles(d(true, true), d(true, true))).toBe(null);
    expect(transitionRoles(d(true, false), d(true, false))).toBe(null);
    expect(transitionRoles(d(true, true), d(false, false))).toBe(null);
  });

  it('Anzeige: Beats bis zum Start, dann der laufende Takt, dann fertig', () => {
    expect(transitionProgress(90, 100, 16)).toEqual({ phase: 'waiting', beats: 10 });
    expect(transitionProgress(99.2, 100, 16)).toEqual({ phase: 'waiting', beats: 1 });
    expect(transitionProgress(100, 100, 16)).toEqual({ phase: 'running', bar: 1 });
    expect(transitionProgress(103.9, 100, 16)).toEqual({ phase: 'running', bar: 1 });
    expect(transitionProgress(104, 100, 16)).toEqual({ phase: 'running', bar: 2 });
    expect(transitionProgress(163.9, 100, 16)).toEqual({ phase: 'running', bar: 16 });
    expect(transitionProgress(164, 100, 16)).toEqual({ phase: 'done' });
  });

  it('simulierter Kern: Start an der Taktgrenze, danach läuft nur das neue Deck; Abbruch und Absagen', async () => {
    const core = new MockCore({ manualClock: true });
    const step = (seconds: number) => {
      for (let t = 0; t < seconds; t += 1 / 60) core.step(1 / 60);
    };
    core.loadNow('a', MOCK_TRACKS[0].id);
    core.loadNow('b', MOCK_TRACKS[1].id);
    const [template] = await core.listTransitions(4);
    expect(template.bars).toBe(4);
    await expect(core.startTransition('a', 'b', template, 1)).rejects.toContain('Deck A läuft nicht');
    core.apply({ type: 'play', deck: 'a', on: true });
    step(1.3);
    const run = await core.startTransition('a', 'b', template, 1);
    const before = core.snapshot();
    expect(run.t0_beat - before.clock_beats).toBeGreaterThanOrEqual(3 - 1e-9);
    expect(run.t0_beat - before.clock_beats).toBeLessThan(7 + 1e-9);
    expect(before.decks[1].playing).toBe(false);
    await expect(core.startTransition('a', 'b', template, 1)).rejects.toContain('Es läuft schon ein Übergang');
    // Mitten im Übergang laufen beide; am Ende nur noch B, Fader offen.
    const secPerBeat = 60 / before.clock_bpm;
    step((run.t0_beat - before.clock_beats + 8) * secPerBeat);
    expect(core.snapshot().decks.slice(0, 2).map((d) => d.playing)).toEqual([true, true]);
    step(12 * secPerBeat);
    const after = core.snapshot();
    expect(after.decks.slice(0, 2).map((d) => d.playing)).toEqual([false, true]);
    expect(after.decks[1].fader).toBe(1);
    expect(after.master).toBe('b');
    // Abbrechen vor dem Start: nichts geschieht.
    core.apply({ type: 'seek', deck: 'a', pos: 20 });
    const second = await core.startTransition('b', 'a', template, 1);
    await core.cancelTransition();
    step((second.t0_beat - core.snapshot().clock_beats + 4) * secPerBeat);
    expect(core.snapshot().decks[0].playing).toBe(false);
  });
});
