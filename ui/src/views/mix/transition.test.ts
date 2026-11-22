import { describe, expect, it } from 'vitest';
import { previewRoles, transitionRoles, transitionWhen } from '../../app/actions';
import { MockCore } from '../../core/mock/MockCore';
import { MOCK_TRACKS } from '../../core/mock/tracks';
import { stageTarget, transitionProgress, transitionZone } from './WaveStage';

describe('Übergang auf Knopfdruck (TRANS-05)', () => {
  it('Rollen: das laufende Deck geht, das stehende kommt; sonst keine', () => {
    const d = (loaded: boolean, playing: boolean) => ({ loaded, playing });
    expect(transitionRoles(d(true, true), d(true, false))).toEqual({ from: 'a', to: 'b' });
    expect(transitionRoles(d(true, false), d(true, true))).toEqual({ from: 'b', to: 'a' });
    // Laufen beide, geht der Master; ohne Master-Deck gibt es keine Rollen.
    expect(transitionRoles(d(true, true), d(true, true))).toBe(null);
    expect(transitionRoles(d(true, true), d(true, true), 'b')).toEqual({ from: 'b', to: 'a' });
    expect(transitionRoles(d(true, true), d(true, true), 'a')).toEqual({ from: 'a', to: 'b' });
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

  it('mit Vorausblick endet der Übergang an der Linie, sonst beginnt er an der nächsten Vier-Takt-Grenze', () => {
    expect(transitionWhen(0)).toEqual({ type: 'next', align_bars: 4 });
    expect(transitionWhen(-8)).toEqual({ type: 'next', align_bars: 4 });
    expect(transitionWhen(128)).toEqual({ type: 'end_in', beats: 128 });
  });

  it('Zone in der Wellenform: eingeplant ab T0, als Vorschau endet sie am Vorausblick', () => {
    // Eingeplant: 20 Beats bis T0, 16 Takte lang.
    expect(transitionZone({ t0_beat: 520, bars: 16 }, 500, 0, 8, true)).toEqual({ startBeats: 20, endBeats: 84, preview: false });
    // Läuft schon: Beginn liegt hinter der Abspielstelle.
    expect(transitionZone({ t0_beat: 520, bars: 16 }, 530, 0, 8, true)).toEqual({ startBeats: -10, endBeats: 54, preview: false });
    // Vorschau: endet am Vorausblick, beginnt um die gewählte Länge davor.
    expect(transitionZone(null, 500, 128, 16, true)).toEqual({ startBeats: 64, endBeats: 128, preview: true });
    expect(transitionZone(null, 500, 128, 16, false)).toBe(null);
    expect(transitionZone(null, 500, 0, 16, true)).toBe(null);
  });

  it('simulierter Kern mit vorgegebenem Ende: Beginn zurückgerechnet, das stehende Deck erreicht seine Stelle am Ende', async () => {
    const core = new MockCore({ manualClock: true });
    const step = (seconds: number) => {
      for (let t = 0; t < seconds; t += 1 / 60) core.step(1 / 60);
    };
    core.loadNow('a', MOCK_TRACKS[0].id);
    core.loadNow('b', MOCK_TRACKS[0].id);
    core.apply({ type: 'play', deck: 'a', on: true });
    step(1.3);
    const [template] = await core.listTransitions(4);
    await expect(core.startTransition('a', 'b', template, { type: 'end_in', beats: 10 })).rejects.toContain('Bis zum gewählten Ende');
    const before = core.snapshot();
    const markBeat = Math.round((before.decks[1].pos / 60) * 134 / 4) * 4;
    const run = await core.startTransition('a', 'b', template, { type: 'end_in', beats: 40 });
    expect(Math.abs(run.t0_beat + 16 - before.clock_beats - 40)).toBeLessThanOrEqual(2 + 1e-9);
    // B steht jetzt um die Länge vor seiner Marke – hier vor dem Anfang des Tracks.
    expect(core.snapshot().decks[1].pos).toBeLessThan(0);
    step((run.t0_beat + 16 - before.clock_beats) * (60 / before.clock_bpm));
    const after = core.snapshot();
    expect(Math.abs((after.decks[1].pos / 60) * 134 - markBeat)).toBeLessThan(0.1);
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
    await expect(core.startTransition('a', 'b', template, { type: 'next', align_bars: 1 })).rejects.toContain('Deck A läuft nicht');
    core.apply({ type: 'play', deck: 'a', on: true });
    step(1.3);
    const run = await core.startTransition('a', 'b', template, { type: 'next', align_bars: 1 });
    const before = core.snapshot();
    expect(run.t0_beat - before.clock_beats).toBeGreaterThanOrEqual(3 - 1e-9);
    expect(run.t0_beat - before.clock_beats).toBeLessThan(7 + 1e-9);
    expect(before.decks[1].playing).toBe(false);
    await expect(core.startTransition('a', 'b', template, { type: 'next', align_bars: 1 })).rejects.toContain('Es läuft schon ein Übergang');
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
    const second = await core.startTransition('b', 'a', template, { type: 'next', align_bars: 1 });
    await core.cancelTransition();
    step((second.t0_beat - core.snapshot().clock_beats + 4) * secPerBeat);
    expect(core.snapshot().decks[0].playing).toBe(false);
  });

  it('Klick in die Wellenform: laufendes Deck = Stelle auf der Zeitachse, stehendes Deck = Stelle im Track', () => {
    const grid = { anchors: [{ pos: 0.25, bpm: 120 }], anchor_beats: [0], bar_offset: 0 };
    const base = { width: 1000, zoomBeats: 8, clockBpm: 120, grid };
    const playing = { playing: true, pos: 50, tempo_offset: 0, duration: 400 };
    // 8 Beats füllen 1000 px: Die Mitte ist „jetzt" plus Vorausblick, 125 px sind ein Beat.
    expect(stageTarget({ ...base, x: 500, peekBeats: 0, deck: 'a', state: playing, anyPlaying: true })).toEqual({ kind: 'time', beats: 0 });
    expect(stageTarget({ ...base, x: 750, peekBeats: 0, deck: 'a', state: playing, anyPlaying: true })).toEqual({ kind: 'time', beats: 2 });
    expect(stageTarget({ ...base, x: 250, peekBeats: 128, deck: 'a', state: playing, anyPlaying: true })).toEqual({ kind: 'time', beats: 126 });
    // Stehendes Deck bei 60,3 s, ein anderes läuft: Der Vorausblick verschiebt es nicht. 250 px rechts
    // der Mitte sind 2 Beats = 1 s weiter; gerückt wird auf den nächsten Beat des Rasters (61,25 s).
    const stopped = { playing: false, pos: 60.3, tempo_offset: 0, duration: 400 };
    expect(stageTarget({ ...base, x: 750, peekBeats: 128, deck: 'b', state: stopped, anyPlaying: true })).toEqual({ kind: 'deck', deck: 'b', pos: 61.25 });
    // Läuft keines, verschiebt der Vorausblick auch das stehende Deck: 128 Beats = 64 s weiter.
    expect(stageTarget({ ...base, x: 500, peekBeats: 128, deck: 'b', state: stopped, anyPlaying: false })).toEqual({ kind: 'deck', deck: 'b', pos: 124.25 });
    // Tempo-Regler +10 %: In derselben Echtzeit legt das Deck mehr Strecke zurück.
    const fast = { playing: false, pos: 60.25, tempo_offset: 0.1, duration: 400 };
    expect(stageTarget({ ...base, x: 1000, peekBeats: 0, deck: 'b', state: fast, anyPlaying: true })).toEqual({ kind: 'deck', deck: 'b', pos: 62.25 });
    // Nie vor den Anfang oder hinter das Ende.
    expect(stageTarget({ ...base, x: 0, peekBeats: 0, deck: 'b', state: { ...stopped, pos: 0.5 }, anyPlaying: true })).toEqual({ kind: 'deck', deck: 'b', pos: 0 });
  });
});

describe('Vorhören im Editor (TRANS-06)', () => {
  it('Rollen wie live; stehen beide Decks, geht der Master, sonst Deck A', () => {
    const d = (loaded: boolean, playing: boolean) => ({ loaded, playing });
    expect(previewRoles(d(true, false), d(true, true))).toEqual({ from: 'b', to: 'a' });
    expect(previewRoles(d(true, false), d(true, false))).toEqual({ from: 'a', to: 'b' });
    expect(previewRoles(d(true, false), d(true, false), 'b')).toEqual({ from: 'b', to: 'a' });
    expect(previewRoles(d(true, false), d(false, false))).toBe(null);
  });

  it('simulierter Kern: spielt ab dem gewählten Takt und stellt die Decks danach zurück', async () => {
    const core = new MockCore({ scene: 'loaded', manualClock: true });
    const step = (seconds: number) => {
      for (let s = 0; s < seconds; s += 1 / 60) core.step(1 / 60);
    };
    for (const deck of ['a', 'b'] as const) core.send({ type: 'play', deck, on: false });
    core.send({ type: 'seek', deck: 'a', pos: 30 });
    core.send({ type: 'seek', deck: 'b', pos: 45 });
    step(0.1);
    const before = core.snapshot();
    expect(before.decks[0].loaded && before.decks[1].loaded).toBe(true);
    const [swap] = await core.listTransitions(4);
    // Ab Takt 3: Beginn ein Takt davor (Beat 4), das neue Deck läuft dort schon mit.
    const run = await core.previewTransition('a', 'b', swap, { type: 'next', align_bars: 4 }, 8);
    expect(run.t0_beat).toBeCloseTo(core.snapshot().clock_beats - 4, 6);
    expect(run.end_beat - run.t0_beat).toBe(16 + 8);
    step(0.5);
    expect(core.snapshot().decks.slice(0, 2).map((d) => d.playing)).toEqual([true, true]);
    // Bis über das Ende: beide stehen wieder an ihrer Stelle.
    step(((run.end_beat - core.snapshot().clock_beats + 1) * 60) / core.snapshot().clock_bpm);
    const after = core.snapshot();
    expect(after.decks.slice(0, 2).map((d) => d.playing)).toEqual([false, false]);
    expect(after.decks[0].pos).toBeCloseTo(30, 6);
    expect(after.decks[1].pos).toBeCloseTo(45, 6);
    // Ohne Track auf einem Deck: Absage im Klartext.
    await expect(new MockCore({ scene: 'empty', manualClock: true }).previewTransition('a', 'b', swap, { type: 'next', align_bars: 4 }, 0)).rejects.toMatch(/liegt kein Track/);
  });
});
