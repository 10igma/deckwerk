import { describe, expect, it } from 'vitest';
import { decodeWaveform, encodeWaveform, WaveformFormatError } from './dwwf';
import { TauriCore, type Invoke, type Schedule } from './TauriCore';
import { emptyEngineStatus, emptySnapshot, type LibraryStatus, type Poll, type Snapshot } from './types';

/** Handgebautes DWWF: Kopf, Übersicht mit 2 Punkten, eine Stufe mit 1 Punkt (64 Samples je Punkt). */
function sample(): Uint8Array {
  const bytes: number[] = [];
  const u32 = (v: number) => bytes.push(v & 255, (v >>> 8) & 255, (v >>> 16) & 255, (v >>> 24) & 255);
  bytes.push(0x44, 0x57, 0x57, 0x46); // "DWWF"
  u32(1); // Format-Version
  u32(48000); // Abtastrate
  u32(128); // Frames, untere 32 Bit
  u32(0); // Frames, obere 32 Bit
  u32(2); // Anzahl Stufen
  // Übersicht: 64 Samples je Punkt, 2 Punkte
  u32(64);
  u32(2);
  //          Tief            Mitte          Hoch      (min, max, rms)
  bytes.push(129, 127, 255, 193, 64, 128, 0, 0, 0); // −1 +1 1 | −63/127 +64/127 128/255 | 0 0 0
  bytes.push(0, 0, 0, 255, 1, 51, 129, 127, 26); //     0 0 0 | −1/127 +1/127 0,2 | −1 +1 26/255
  // Pyramide, erste Stufe: 64 Samples je Punkt, 1 Punkt
  u32(64);
  u32(1);
  bytes.push(246, 10, 5, 236, 20, 10, 226, 30, 15);
  return Uint8Array.from(bytes);
}

describe('DWWF-Dekodierung', () => {
  it('liest Kopf, Übersicht und Stufen in typisierte Felder', () => {
    const wf = decodeWaveform(sample());
    expect(wf.sample_rate).toBe(48000);
    expect(wf.frames).toBe(128);
    expect(wf.overview.samples_per_point).toBe(64);
    expect(wf.overview.data).toBeInstanceOf(Float32Array);
    expect(wf.overview.data.length).toBe(18);
    expect(wf.levels.length).toBe(1);
    expect(wf.levels[0].samples_per_point).toBe(64);
    expect(wf.levels[0].data.length).toBe(9);
  });
  it('min/max vorzeichenbehaftet ÷ 127, rms vorzeichenlos ÷ 255', () => {
    const d = decodeWaveform(sample()).overview.data;
    expect(d[0]).toBeCloseTo(-1, 6);
    expect(d[1]).toBeCloseTo(1, 6);
    expect(d[2]).toBeCloseTo(1, 6);
    expect(d[3]).toBeCloseTo(-63 / 127, 6);
    expect(d[4]).toBeCloseTo(64 / 127, 6);
    expect(d[5]).toBeCloseTo(128 / 255, 6);
    expect(d[9 + 3]).toBeCloseTo(-1 / 127, 6);
    expect(d[9 + 5]).toBeCloseTo(0.2, 6);
    expect(d[9 + 6]).toBeCloseTo(-1, 6);
    expect(d[9 + 8]).toBeCloseTo(26 / 255, 6);
    const l = decodeWaveform(sample()).levels[0].data;
    expect(Array.from(l).map((v) => +v.toFixed(4))).toEqual([-10 / 127, 10 / 127, 5 / 255, -20 / 127, 20 / 127, 10 / 255, -30 / 127, 30 / 127, 15 / 255].map((v) => +v.toFixed(4)));
  });
  it('Byte 128 ist −128/127, wie in dw-core', () => {
    const src = sample();
    src[src.length - 9] = 128;
    expect(decodeWaveform(src).levels[0].data[0]).toBeCloseTo(-128 / 127, 6);
  });
  it('nimmt ArrayBuffer und Ausschnitte eines größeren Puffers', () => {
    const src = sample();
    expect(decodeWaveform(src.buffer.slice(0) as ArrayBuffer).frames).toBe(128);
    const padded = new Uint8Array(src.length + 7);
    padded.set(src, 7);
    expect(decodeWaveform(padded.subarray(7)).levels[0].data.length).toBe(9);
  });
  it('Frames über 32 Bit', () => {
    const src = sample();
    src[16] = 2; // obere 32 Bit = 2
    expect(decodeWaveform(src).frames).toBe(2 * 4294967296 + 128);
  });
  it('weist Fremdes, Abgeschnittenes und andere Versionen ab', () => {
    const src = sample();
    expect(() => decodeWaveform(new TextEncoder().encode('kein Wellenform-Format, sondern Text'))).toThrow(WaveformFormatError);
    expect(() => decodeWaveform(src.subarray(0, 10))).toThrow(WaveformFormatError);
    expect(() => decodeWaveform(src.subarray(0, src.length - 3))).toThrow(WaveformFormatError);
    const v2 = Uint8Array.from(src);
    v2[4] = 2;
    expect(() => decodeWaveform(v2)).toThrow(/Version 2/);
  });
  it('Schreiben und Lesen ergeben dieselben Bytes', () => {
    const src = sample();
    expect(Array.from(encodeWaveform(decodeWaveform(src)))).toEqual(Array.from(src));
  });
});

/** Hülle zum Testen: `invoke` wird aufgezeichnet, Antworten werden von Hand ausgelöst. */
function harness() {
  const calls: { cmd: string; args?: Record<string, unknown> }[] = [];
  const waiting: { resolve: (v: unknown) => void; reject: (e: unknown) => void; cmd: string }[] = [];
  const invoke = ((cmd: string, args?: Record<string, unknown>) => {
    calls.push({ cmd, args });
    return new Promise((resolve, reject) => waiting.push({ resolve: resolve as (v: unknown) => void, reject, cmd }));
  }) as Invoke;
  let tick: () => void = () => {};
  let stopped = false;
  const schedule: Schedule = (t) => {
    tick = t;
    return () => {
      stopped = true;
    };
  };
  const core = new TauriCore(invoke, schedule);
  const flush = () => new Promise((r) => setTimeout(r, 0));
  return { core, calls, waiting, frame: () => tick(), flush, isStopped: () => stopped };
}

const poll = (over: Partial<Poll> = {}): Poll => ({ snapshot: emptySnapshot(), status: emptyEngineStatus(), library_rev: 1, pending_analysis: 0, ...over });

describe('TauriCore', () => {
  it('ruft core_poll einmal je Bild ab und startet keinen Abruf, solange der vorige läuft', async () => {
    const h = harness();
    h.frame();
    h.frame();
    h.frame();
    expect(h.calls.filter((c) => c.cmd === 'core_poll').length).toBe(1);
    h.waiting[0].resolve(poll());
    await h.flush();
    h.frame();
    expect(h.calls.filter((c) => c.cmd === 'core_poll').length).toBe(2);
    expect(h.core.polls).toBe(2);
  });
  it('liefert Schnappschüsse und meldet Status und Bibliothek nur bei Änderung', async () => {
    const h = harness();
    const snaps: Snapshot[] = [];
    const libs: LibraryStatus[] = [];
    const devices: boolean[] = [];
    h.core.subscribe((s) => snaps.push(s));
    h.core.subscribeLibrary((s) => libs.push(s));
    h.core.subscribeStatus((s) => devices.push(s.has_device));
    expect([snaps.length, libs.length, devices.length]).toEqual([1, 1, 1]); // sofortiger Stand

    const first = emptySnapshot();
    first.clock_bpm = 132;
    h.frame();
    h.waiting[0].resolve(poll({ snapshot: first, library_rev: 3, pending_analysis: 2, status: { ...emptyEngineStatus(), has_device: false, device_error: 'kein Gerät' } }));
    await h.flush();
    expect(snaps[1].clock_bpm).toBe(132);
    expect(libs[1]).toEqual({ rev: 3, pending_analysis: 2 });
    expect(devices).toEqual([true, false]);

    h.frame();
    h.waiting[1].resolve(poll({ library_rev: 3, pending_analysis: 2, status: { ...emptyEngineStatus(), has_device: false, device_error: 'kein Gerät' } }));
    await h.flush();
    expect(snaps.length).toBe(3);
    expect(libs.length).toBe(2);
    expect(devices.length).toBe(2);

    h.frame();
    h.waiting[2].resolve(poll({ library_rev: 3, pending_analysis: 1 }));
    await h.flush();
    expect(libs[2]).toEqual({ rev: 3, pending_analysis: 1 });
    expect(devices).toEqual([true, false, true]);
  });
  it('nach einem Fehler beim Abruf geht es im nächsten Bild weiter', async () => {
    const h = harness();
    const errors: unknown[] = [];
    const orig = console.error;
    console.error = (...a: unknown[]) => errors.push(a);
    try {
      h.frame();
      h.waiting[0].reject('Die Audio-Engine antwortet nicht.');
      await h.flush();
      h.frame();
      expect(h.calls.length).toBe(2);
      h.waiting[1].reject('Die Audio-Engine antwortet nicht.');
      await h.flush();
      expect(errors.length).toBe(1); // derselbe Text wird nur einmal gemeldet
    } finally {
      console.error = orig;
    }
  });
  it('bildet die Befehle der Schnittstelle ab (Namen und Argumente in camelCase)', async () => {
    const h = harness();
    h.core.send({ type: 'eq', deck: 'b', band: 'low', db: null });
    void h.core.listTracks();
    void h.core.importPaths(['/musik']);
    void h.core.loadTrack('a', '17');
    void h.core.getLoadedTracks();
    void h.core.getAnalysis('17');
    void h.core.listAudioDevices();
    void h.core.selectAudioDevice('Interface 4 Ausgänge');
    void h.core.selectAudioDevice(null);
    expect(h.calls).toEqual([
      { cmd: 'core_send', args: { cmd: { type: 'eq', deck: 'b', band: 'low', db: null } } },
      { cmd: 'library_list', args: undefined },
      { cmd: 'library_import', args: { paths: ['/musik'] } },
      { cmd: 'deck_load', args: { deck: 'a', trackId: '17' } },
      { cmd: 'deck_loaded', args: undefined },
      { cmd: 'track_analysis', args: { trackId: '17' } },
      { cmd: 'audio_devices', args: undefined },
      { cmd: 'audio_select', args: { device: 'Interface 4 Ausgänge' } },
      { cmd: 'audio_select', args: { device: null } },
    ]);
  });
  it('dekodiert die Wellenform aus ArrayBuffer, Uint8Array oder Zahlenfeld', async () => {
    for (const shape of ['buffer', 'bytes', 'array'] as const) {
      const h = harness();
      const p = h.core.getWaveform('5');
      const src = sample();
      h.waiting[0].resolve(shape === 'buffer' ? src.buffer : shape === 'bytes' ? src : Array.from(src));
      const wf = await p;
      expect(h.calls[0]).toEqual({ cmd: 'track_waveform', args: { trackId: '5' } });
      expect(wf.sample_rate).toBe(48000);
      expect(wf.levels[0].data.length).toBe(9);
    }
  });
  it('gibt Ablehnungen der Hülle beim Laden weiter', async () => {
    const h = harness();
    const p = h.core.loadTrack('b', '9');
    h.waiting[0].reject('/musik/a.flac: Datei nicht gefunden');
    await expect(p).rejects.toBe('/musik/a.flac: Datei nicht gefunden');
  });
  it('meldet einen Gerätewechsel als Statusänderung (ENG-01)', async () => {
    const h = harness();
    const chosen: (string | null)[] = [];
    h.core.subscribeStatus((s) => chosen.push(s.device));
    h.frame();
    h.waiting[0].resolve(poll({ status: { ...emptyEngineStatus(), device: 'Interface 4 Ausgänge' } }));
    await h.flush();
    h.frame();
    h.waiting[1].resolve(poll({ status: { ...emptyEngineStatus(), device: 'Interface 4 Ausgänge' } }));
    await h.flush();
    expect(chosen).toEqual([null, 'Interface 4 Ausgänge']);
  });
  it('gibt die Ablehnung eines Geräts weiter', async () => {
    const h = harness();
    const p = h.core.selectAudioDevice('weg');
    h.waiting[0].reject('Gerät „weg“ nicht gefunden');
    await expect(p).rejects.toBe('Gerät „weg“ nicht gefunden');
  });
  it('dispose beendet den Abruf', () => {
    const h = harness();
    h.core.dispose();
    expect(h.isStopped()).toBe(true);
  });
});
