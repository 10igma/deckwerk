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
  it('Start mit Vorlauf: Das Deck erreicht seine Stelle nach den genannten Beats, auch von vor dem Anfang (VIS-13)', () => {
    const core = new MockCore({ manualClock: true });
    core.loadNow('a', MOCK_TRACKS[0].id);
    core.loadNow('b', MOCK_TRACKS[0].id);
    core.apply({ type: 'play', deck: 'a', on: true });
    core.apply({ type: 'seek', deck: 'b', pos: 2 });
    run(core, 1);
    // 16 Beats bei 134 BPM sind gut 7 s: Der Start liegt vor dem Anfang des Tracks.
    const lead = (16 * 60) / 134;
    core.apply({ type: 'play_in', deck: 'b', beats: 16 });
    expect(last(core).decks[1].playing).toBe(true);
    expect(last(core).decks[1].pos).toBeCloseTo(2 - lead, 6);
    run(core, lead);
    expect(Math.abs(last(core).decks[1].pos - 2)).toBeLessThan(0.02);
    expect(last(core).master).toBe('a');
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
  it('Auto-Master folgt dem hörbaren Deck (DECK-09): Fader von A zu → B übernimmt ohne Temposprung', () => {
    const core = new MockCore({ manualClock: true, scene: 'transition' });
    run(core, 0.5);
    expect(last(core).master).toBe('a');
    core.apply({ type: 'fader', deck: 'a', value: 0 });
    run(core, 0.1);
    const s = last(core);
    expect(s.master).toBe('b');
    expect(s.clock_bpm).toBeCloseTo(134, 6);
    // Läuft kein Deck hörbar, bleibt der Master, wo er ist.
    core.apply({ type: 'fader', deck: 'b', value: 0 });
    run(core, 0.1);
    expect(last(core).master).toBe('b');
  });
  it('Kanalfader wirkt quadratisch wie im Kern: halber Weg = ein Viertel', () => {
    const level = (fader: number): number => {
      const core = new MockCore({ scene: 'synced', frozen: true });
      core.apply({ type: 'limiter', on: false });
      core.apply({ type: 'fader', deck: 'a', value: fader });
      core.computeLevels();
      return core.snapshot().mixer.master_peak[0];
    };
    expect(level(0.5) / level(1)).toBeCloseTo(0.25, 6);
  });
  it('Bibliothek „analyzing": unanalysierte Tracks ohne Tempo und Tonart, Analyse läuft ab', async () => {
    const core = new MockCore({ manualClock: true, library: 'analyzing' });
    const states: number[] = [];
    core.subscribeLibrary((s) => states.push(s.pending_analysis));
    let tracks = await core.listTracks();
    const waiting = tracks.filter((t) => !t.analyzed && !t.missing);
    expect(waiting.length).toBe(12);
    expect(waiting.every((t) => t.bpm === 0 && t.key === null)).toBe(true);
    expect(tracks.filter((t) => t.missing).length).toBe(1);
    expect(states[0]).toBe(12);
    run(core, 2);
    expect(states[states.length - 1]).toBe(10);
    tracks = await core.listTracks();
    expect(tracks.filter((t) => !t.analyzed && !t.missing).length).toBe(10);
  });
  it('leere Bibliothek, Import bringt Tracks ohne Analyse und erhöht den Stand', async () => {
    const core = new MockCore({ manualClock: true, library: 'empty' });
    const revs: number[] = [];
    core.subscribeLibrary((s) => revs.push(s.rev));
    expect((await core.listTracks()).length).toBe(0);
    const r = await core.importPaths(['/musik/techno']);
    expect(r.added).toBe(6);
    expect(revs.length).toBe(2);
    expect(revs[1]).toBeGreaterThan(revs[0]);
    const tracks = await core.listTracks();
    expect(tracks.length).toBe(6);
    expect(tracks.every((t) => !t.analyzed)).toBe(true);
  });
  it('Laden: fehlende Datei wird mit Klartext abgelehnt, Ladezeit wird eingehalten, Analyse beim Laden nachgeholt', async () => {
    const core = new MockCore({ manualClock: true, library: 'analyzing', loadDelayMs: 30 });
    const tracks = await core.listTracks();
    const missing = tracks.find((t) => t.missing)!;
    await expect(core.loadTrack('a', missing.id)).rejects.toMatch(/Datei nicht gefunden/);
    const pending = tracks.find((t) => !t.analyzed && !t.missing)!;
    const t0 = Date.now();
    const p = core.loadTrack('b', pending.id);
    expect(core.snapshot().decks[1].loaded).toBe(false);
    await p;
    expect(Date.now() - t0).toBeGreaterThanOrEqual(25);
    expect(core.snapshot().decks[1].loaded).toBe(true);
    expect((await core.listTracks()).find((t) => t.id === pending.id)!.analyzed).toBe(true);
  });
  it('ohne Audiogerät trägt der Status die Meldung', () => {
    expect(new MockCore({ manualClock: true }).status().has_device).toBe(true);
    const st = new MockCore({ manualClock: true, noDevice: true }).status();
    expect(st.has_device).toBe(false);
    expect(st.device_error).toMatch(/Ausgabegerät/);
  });
  it('Bibliothek aktualisieren bringt neue Tracks ohne Analyse; die Start-Einstellung bleibt gemerkt (BIB-19)', async () => {
    const core = new MockCore({ manualClock: true, library: 'empty' });
    const before = (await core.listTracks()).length;
    const r = await core.refreshLibrary();
    expect(r.added).toBe(3);
    expect((await core.listTracks()).length).toBe(before + 3);
    expect((await core.getSettings()).refresh_on_start).toBe(true);
    await core.setRefreshOnStart(false);
    expect((await core.getSettings()).refresh_on_start).toBe(false);
  });
  it('Ausgabegerät: Wechsel meldet den Status, Ablehnung lässt die Wahl stehen (ENG-01)', async () => {
    const core = new MockCore({ manualClock: true });
    const chosen: (string | null)[] = [];
    core.subscribeStatus((s) => chosen.push(s.device));
    const devices = await core.listAudioDevices();
    expect(devices.filter((d) => d.is_default).length).toBe(1);
    expect(devices.some((d) => d.max_channels >= 4)).toBe(true);

    await core.selectAudioDevice('Interface 4 Ausgänge');
    expect(chosen).toEqual([null, 'Interface 4 Ausgänge']);
    await expect(core.selectAudioDevice('Gerät belegt')).rejects.toMatch(/Audio-Schnittstelle/);
    await expect(core.selectAudioDevice('gibt es nicht')).rejects.toMatch(/nicht gefunden/);
    expect(core.status().device).toBe('Interface 4 Ausgänge');
    expect(chosen.length).toBe(2);
    await core.selectAudioDevice(null);
    expect(core.status().device).toBe(null);

    const silent = new MockCore({ manualClock: true, noDevice: true });
    expect(await silent.listAudioDevices()).toEqual([]);
    await expect(silent.selectAudioDevice(null)).rejects.toMatch(/kein Ausgabegerät/);
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

describe('Sync im Stand (DECK-08)', () => {
  it('nichts läuft: Tempo des anderen Decks sofort, im Modus „Beat“ dieselbe Stelle im Beat', () => {
    const core = new MockCore({ scene: 'loaded', manualClock: true });
    for (const deck of ['a', 'b'] as const) core.send({ type: 'play', deck, on: false });
    core.send({ type: 'sync', deck: 'b', mode: 'off' });
    core.send({ type: 'tempo', deck: 'a', offset: 0.02 });
    core.send({ type: 'seek', deck: 'a', pos: 20.13 });
    core.send({ type: 'seek', deck: 'b', pos: 41 });
    core.step(1 / 60);
    const before = last(core);
    core.send({ type: 'sync', deck: 'b', mode: 'tempo' });
    core.step(1 / 60);
    expect(last(core).decks[1].bpm).toBeCloseTo(before.decks[0].bpm, 6);
    expect(last(core).decks[1].pos).toBe(before.decks[1].pos);
    core.send({ type: 'sync', deck: 'b', mode: 'beat' });
    core.step(1 / 60);
    const s = last(core);
    const frac = (b: number) => b - Math.floor(b);
    expect(frac(s.decks[1].beat)).toBeCloseTo(frac(s.decks[0].beat), 6);
    expect(Math.abs(s.decks[1].beat - before.decks[1].beat)).toBeLessThanOrEqual(0.5 + 1e-9);
    expect(s.decks[0].pos).toBe(before.decks[0].pos);
  });
});

describe('Sprung im Takt (DECK-20)', () => {
  it('ein laufendes Deck behält seine Phase, ein stehendes springt genau', () => {
    const core = new MockCore({ manualClock: true });
    core.loadNow('a', MOCK_TRACKS[0].id);
    core.apply({ type: 'play', deck: 'a', on: true });
    run(core, 3.21);
    const before = last(core).decks[0];
    const beat = 60 / 134;
    // Ziel mitten zwischen zwei Beats: Gelandet wird ganze Beats weiter, höchstens einen halben neben dem Ziel.
    core.apply({ type: 'seek_snap', deck: 'a', pos: 100.123 });
    const after = last(core).decks[0];
    expect(Math.abs(after.pos - 100.123)).toBeLessThanOrEqual(beat / 2 + 1e-9);
    const beats = (after.pos - before.pos) / beat;
    expect(Math.abs(beats - Math.round(beats))).toBeLessThan(1e-6);
    core.apply({ type: 'play', deck: 'a', on: false });
    core.apply({ type: 'seek_snap', deck: 'a', pos: 50.123 });
    expect(last(core).decks[0].pos).toBeCloseTo(50.123, 9);
  });
});

describe('Mitschnitt (REC-01)', () => {
  it('Zeitstempel für den Dateinamen in Ortszeit, zweistellig', async () => {
    const { recordingStamp } = await import('../../app/actions');
    expect(recordingStamp(new Date(2026, 10, 22, 13, 5, 9))).toBe('2026-11-22 13-05-09');
    expect(recordingStamp(new Date(2027, 0, 3, 0, 0, 0))).toBe('2027-01-03 00-00-00');
  });

  it('simulierter Kern: starten, Stand, beenden; nie zwei zugleich', async () => {
    const core = new MockCore({ manualClock: true });
    expect(await core.recordingStatus()).toBe(null);
    await expect(core.stopRecording()).rejects.toContain('keine Aufnahme');
    const started = await core.startRecording('2026-11-22 13-05-09');
    expect(started.active).toBe(true);
    expect(started.path.endsWith('Deckwerk 2026-11-22 13-05-09.wav')).toBe(true);
    await expect(core.startRecording('x')).rejects.toContain('läuft schon');
    run(core, 3);
    expect((await core.recordingStatus())?.seconds).toBeCloseTo(3, 1);
    const done = await core.stopRecording();
    expect(done.active).toBe(false);
    expect(done.seconds).toBeCloseTo(3, 1);
    expect(await core.recordingStatus()).toBe(null);
  });
});

describe('Eigene Übergänge (TRANS-04)', () => {
  it('simulierter Kern: speichern, ersetzen, löschen; eingebaute Namen sind gesperrt', async () => {
    const core = new MockCore({ manualClock: true });
    const [template] = await core.listTransitions(16);
    expect(await core.listSavedTransitions()).toEqual([]);
    expect((await core.saveTransition({ ...template, name: ' Mein Tausch ' })).map((t) => t.name)).toEqual(['Mein Tausch']);
    expect((await core.saveTransition({ ...template, name: 'Mein Tausch', bars: 32 }))[0].bars).toBe(32);
    await expect(core.saveTransition({ ...template, name: 'fade' })).rejects.toContain('eingebauten Vorlage');
    await expect(core.saveTransition({ ...template, name: '  ' })).rejects.toContain('Namen');
    expect(await core.deleteTransition('Mein Tausch')).toEqual([]);
  });
});
