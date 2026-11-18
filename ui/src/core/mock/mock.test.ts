import { describe, expect, it } from 'vitest';
import { VALUES_PER_POINT, type Snapshot } from '../types';
import { MockCore } from './MockCore';
import { MOCK_TRACKS } from './tracks';
import { synthWaveform } from './waveform';

const run = (core: MockCore, seconds: number, dt = 1 / 60): void => {
  for (let t = 0; t < seconds; t += dt) core.step(dt);
};
const last = (core: MockCore): Snapshot => core.snapshot();

describe('Simulierte Bibliothek', () => {
  it('30–40 Tracks, 126–148 BPM, eindeutige Kennungen', async () => {
    const core = new MockCore({ manualClock: true });
    const tracks = await core.listTracks();
    expect(tracks.length).toBeGreaterThanOrEqual(30);
    expect(tracks.length).toBeLessThanOrEqual(40);
    for (const t of tracks) {
      expect(t.bpm).toBeGreaterThanOrEqual(126);
      expect(t.bpm).toBeLessThanOrEqual(148);
      expect(t.duration).toBeGreaterThan(240);
    }
    expect(new Set(tracks.map((t) => t.id)).size).toBe(tracks.length);
    expect(tracks.some((t) => t.key === null)).toBe(true);
  });
  it('Abschnitte füllen den Track lückenlos', () => {
    for (const t of MOCK_TRACKS) {
      let bar = 0;
      for (const s of t.sections) {
        expect(s.startBar).toBe(bar);
        expect(s.bars).toBeGreaterThan(0);
        bar += s.bars;
      }
      expect(bar).toBe(t.totalBars);
      expect(t.sections.some((s) => s.kind === 'break')).toBe(true);
      expect(t.sections.some((s) => s.kind === 'drop')).toBe(true);
    }
  });
});

describe('Synthetische Wellenform', () => {
  const track = MOCK_TRACKS[0];
  const wf = synthWaveform(track);
  it('Pyramide mit 64/256/1024/4096 Samples je Punkt und 2000 Punkten Übersicht (5.5)', () => {
    expect(wf.levels.map((l) => l.samples_per_point)).toEqual([64, 256, 1024, 4096]);
    expect(wf.overview.data.length).toBe(2000 * VALUES_PER_POINT);
    expect(wf.levels[0].data.length / VALUES_PER_POINT).toBe(Math.ceil(wf.frames / 64));
  });
  it('Werte bleiben im Bereich: min ≤ 0 ≤ max ≤ 1, RMS ≤ max', () => {
    const d = wf.levels[1].data;
    for (let i = 0; i < d.length; i += 3) {
      expect(d[i]).toBeLessThanOrEqual(0);
      expect(d[i + 1]).toBeGreaterThanOrEqual(0);
      expect(d[i + 1]).toBeLessThanOrEqual(1);
      expect(d[i + 2]).toBeLessThanOrEqual(d[i + 1] + 1e-6);
    }
  });
  it('Kick liegt auf dem Beatgrid; im Breakdown fehlt der Bass', () => {
    const lvl = wf.levels[0];
    const lowAt = (beat: number): number => {
      const sec = track.firstBeat + (beat * 60) / track.bpm;
      const i = Math.round((sec * wf.sample_rate) / lvl.samples_per_point);
      let mx = 0;
      for (let j = i; j < i + 12; j++) mx = Math.max(mx, lvl.data[j * VALUES_PER_POINT + 1]);
      return mx;
    };
    expect(lowAt(40)).toBeGreaterThan(0.8);
    expect(lowAt(40.85)).toBeLessThan(0.45);
    const brk = track.sections.find((s) => s.kind === 'break')!;
    expect(lowAt((brk.startBar + 2) * 4)).toBeLessThan(0.15);
    const drop = track.sections.find((s) => s.kind === 'drop')!;
    expect(lowAt(drop.startBar * 4)).toBeGreaterThan(0.8);
  });
  it('ist deterministisch', () => {
    const again = synthWaveform(track);
    expect(Array.from(again.overview.data).slice(0, 900)).toEqual(Array.from(wf.overview.data).slice(0, 900));
  });
});

describe('MockCore', () => {
  it('Position läuft mit dem Tempo-Regler (DECK-06)', () => {
    const core = new MockCore({ manualClock: true });
    core.loadNow('a', MOCK_TRACKS[0].id);
    core.apply({ type: 'tempo', deck: 'a', offset: 0.02 });
    core.apply({ type: 'play', deck: 'a', on: true });
    run(core, 10);
    const s = last(core);
    expect(s.decks[0].pos).toBeCloseTo(10.2, 1);
    expect(s.decks[0].bpm).toBeCloseTo(134 * 1.02, 6);
    expect(s.master).toBe('a');
    expect(s.clock_bpm).toBeCloseTo(134 * 1.02, 6);
    expect(s.decks[0].phase).toBeCloseTo(0, 6);
  });
  it('Beat-Sync gleicht Tempo und Phase an und hält sie (DECK-08)', () => {
    const core = new MockCore({ manualClock: true });
    core.loadNow('a', MOCK_TRACKS[0].id);
    core.loadNow('b', MOCK_TRACKS[1].id);
    core.apply({ type: 'play', deck: 'a', on: true });
    run(core, 3.21);
    core.apply({ type: 'seek', deck: 'b', pos: 1.337 });
    core.apply({ type: 'play', deck: 'b', on: true });
    run(core, 1);
    expect(Math.abs(last(core).decks[1].bpm - 136)).toBeLessThan(0.01);
    core.apply({ type: 'sync', deck: 'b', mode: 'beat' });
    run(core, 20);
    const s = last(core);
    expect(s.decks[1].sync).toBe('beat');
    expect(s.decks[1].bpm).toBeCloseTo(134, 1);
    expect(s.decks[1].tempo_offset).toBeCloseTo(134 / 136 - 1, 9);
    expect(Math.abs(s.decks[1].phase)).toBeLessThan(0.005);
    expect(s.decks[1].is_master).toBe(false);
  });
  it('Anschieben verschiebt die Phase, Sync „Tempo“ lässt sie stehen', () => {
    const core = new MockCore({ manualClock: true, scene: 'synced' });
    core.apply({ type: 'sync', deck: 'b', mode: 'tempo' });
    core.apply({ type: 'bend', deck: 'b', amount: 0.04 });
    run(core, 2);
    core.apply({ type: 'bend', deck: 'b', amount: 0 });
    const moved = last(core).decks[1].phase;
    expect(moved).toBeGreaterThan(0.1);
    run(core, 5);
    expect(last(core).decks[1].phase).toBeCloseTo(moved, 3);
  });
  it('Master wechselt ohne Temposprung, wenn das alte Master-Deck stoppt (DECK-09)', () => {
    const core = new MockCore({ manualClock: true, scene: 'synced' });
    run(core, 1);
    core.apply({ type: 'play', deck: 'a', on: false });
    run(core, 1);
    const s = last(core);
    expect(s.master).toBe('b');
    expect(s.clock_bpm).toBeCloseTo(134, 6);
    expect(s.decks[1].bpm).toBeCloseTo(134, 6);
  });
  it('Pegel folgen der Wellenform; Kill, Fader und Limiter wirken', () => {
    const core = new MockCore({ manualClock: true, scene: 'synced' });
    let peak = 0;
    for (let i = 0; i < 120; i++) {
      core.step(1 / 60);
      peak = Math.max(peak, last(core).decks[0].peak[0]);
    }
    expect(peak).toBeGreaterThan(0.5);
    expect(last(core).mixer.master_peak[0]).toBeLessThanOrEqual(0.967);
    // Fader von B ist unten: der Kanal zeigt Pegel (vor dem Fader), trägt aber nichts zur Summe bei.
    core.apply({ type: 'fader', deck: 'a', value: 0 });
    core.step(1 / 60);
    expect(last(core).mixer.master_peak[0]).toBe(0);
    core.apply({ type: 'eq', deck: 'a', band: 'low', db: null });
    core.step(1 / 60);
    expect(last(core).decks[0].eq_db[0]).toBeNull();
  });
  it('Übergang-Szene ist reproduzierbar: Bass von B gekillt, Fader halb, Phase 0', () => {
    const a = new MockCore({ scene: 'transition', frozen: true }).snapshot();
    const b = new MockCore({ scene: 'transition', frozen: true }).snapshot();
    expect(a).toEqual(b);
    expect(a.decks[1].eq_db[0]).toBeNull();
    expect(a.decks[1].fader).toBe(0.5);
    expect(a.decks[0].playing && a.decks[1].playing).toBe(true);
    expect(Math.abs(a.decks[1].phase)).toBeLessThan(1e-6);
    expect(a.master).toBe('a');
  });
  it('eingefrorener Kern nimmt Befehle an und meldet sofort', () => {
    const core = new MockCore({ scene: 'loaded', frozen: true });
    const seen: Snapshot[] = [];
    core.subscribe((s) => seen.push(s));
    core.send({ type: 'crossfader', pos: -0.5 });
    expect(seen.length).toBe(2);
    expect(seen[1].mixer.crossfader).toBe(-0.5);
    expect(seen[1].decks[0].pos).toBe(seen[0].decks[0].pos);
  });
});
