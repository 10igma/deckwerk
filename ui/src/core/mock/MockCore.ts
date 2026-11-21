// Simulierter Kern: verhält sich gegenüber der Oberfläche wie die Rust-Engine,
// erzeugt aber keinen Ton. Positionen laufen, Sync gleicht an, Pegel folgen der Wellenform.
import type { CoreApi, Unsubscribe } from '../api';
import {
  DECK_IDS,
  VALUES_PER_POINT,
  bandIndex,
  deckIndex,
  type AudioDevice,
  type Command,
  type DeckId,
  type DeckState,
  type EngineStatus,
  type ImportResult,
  type LibraryStatus,
  type LoadedTracks,
  type Snapshot,
  type SyncMode,
  type TrackAnalysis,
  type TrackId,
  type TrackInfo,
  type Waveform,
  type XfAssign,
} from '../types';
import { channelOpen } from '../../lib/audible';
import { crossfaderGains, faderGain, filterSetting } from '../../lib/curves';
import { dbToLin, fromWire, toWire } from '../../lib/db';
import { beatAt, secAt } from '../../lib/grid';
import { hash01 } from './prng';
import { MOCK_SAMPLE_RATE, MOCK_TRACKS, trackAnalysis, trackInfo, type MockTrack } from './tracks';
import { synthWaveform } from './waveform';
import { applyScene, type SceneName } from './scenes';

export interface MockDeck {
  track: MockTrack | null;
  wave: Waveform | null;
  pos: number;
  playing: boolean;
  /** Stellung des Tempo-Reglers als Faktor. */
  manualOffset: number;
  bend: number;
  sync: SyncMode;
  gainDb: number;
  eqDb: [number, number, number];
  filter: number;
  fader: number;
  cue: boolean;
  xf: XfAssign;
  /** Sekunden seit dem Start, für die Wahl des Masters. */
  playTime: number;
  /** Zuletzt wirksame Abspielrate. */
  rate: number;
  peak: [number, number];
}

export type MockLibrary = 'full' | 'empty' | 'analyzing';

interface LibraryEntry {
  track: MockTrack;
  analyzed: boolean;
  missing: boolean;
}

export interface MockOptions {
  /** Ausgangszustand. */
  scene?: SceneName;
  /** Bibliothek: voll analysiert (Standard), leer oder mit Tracks in der Analyse. */
  library?: MockLibrary;
  /** Kein Audiogerät: Die Engine läuft stumm, der Status trägt die Meldung. */
  noDevice?: boolean;
  /** Dauer des Ladens in Millisekunden (echtes Dekodieren dauert bis ca. 2 s). */
  loadDelayMs?: number;
  /** Zeit steht: Es läuft kein Zeitgeber, Befehle wirken trotzdem. */
  frozen?: boolean;
  /** Eigener Takt statt setInterval (Tests). */
  manualClock?: boolean;
}

const LIMITER_CEILING = Math.pow(10, -0.3 / 20);
const TICK_MS = 1000 / 60;
const WAVE_CACHE_SIZE = 6;
/** Simulierte Analyse: Sekunden je Track. */
const ANALYSIS_SECONDS = 0.8;
const NO_DEVICE_TEXT = 'Kein Ausgabegerät gefunden (ALSA: Standardgerät nicht verfügbar).';

/** Geräte des simulierten Kerns: Lautsprecher, ein Interface mit vier Ausgängen, ein Gerät, das sich nicht öffnen lässt. */
export const MOCK_DEVICES: AudioDevice[] = [
  { host: 'Mock', name: 'Lautsprecher (eingebaut)', is_default: true, max_channels: 2, sample_rates: [44100, 48000] },
  { host: 'Mock', name: 'Interface 4 Ausgänge', is_default: false, max_channels: 4, sample_rates: [44100, 48000, 96000] },
  { host: 'Mock', name: 'Gerät belegt', is_default: false, max_channels: 2, sample_rates: [48000] },
];

function newDeck(id: DeckId): MockDeck {
  return {
    track: null,
    wave: null,
    pos: 0,
    playing: false,
    manualOffset: 0,
    bend: 0,
    sync: 'off',
    gainDb: 0,
    eqDb: [0, 0, 0],
    filter: 0,
    fader: 1,
    cue: false,
    xf: id === 'a' || id === 'c' ? 'left' : 'right',
    playTime: 0,
    rate: 1,
    peak: [0, 0],
  };
}

const wrapPhase = (x: number): number => x - Math.round(x);

export class MockCore implements CoreApi {
  readonly decks: MockDeck[] = DECK_IDS.map(newDeck);
  clockBpm = 0;
  clockBeats = 0;
  /** 'auto' = das am längsten laufende Deck; null = freie Clock. */
  masterMode: DeckId | null | 'auto' = 'auto';
  master: DeckId | null = null;
  crossfader = 0;
  masterDb = 0;
  limiterOn = true;
  limiterGrDb = 0;
  masterPeak: [number, number] = [0, 0];
  cueMix = 0.5;
  cueDb = 0;
  frames = 0;
  /** Simulierte Zeit in Sekunden. */
  time = 0;

  private listeners = new Set<(s: Snapshot) => void>();
  private statusListeners = new Set<(s: EngineStatus) => void>();
  private libraryListeners = new Set<(s: LibraryStatus) => void>();
  private library: LibraryEntry[];
  private libraryRev = 1;
  private analysisClock = 0;
  private readonly noDevice: boolean;
  /** Gewähltes Ausgabegerät; null = Standardgerät des Systems. */
  private device: string | null = null;
  private readonly loadDelayMs: number;
  private waves = new Map<TrackId, Waveform>();
  private timer: ReturnType<typeof setInterval> | null = null;
  private lastTick = 0;
  private statusTick = 0;
  private readonly frozen: boolean;

  constructor(options: MockOptions = {}) {
    this.frozen = options.frozen ?? false;
    this.noDevice = options.noDevice ?? false;
    this.loadDelayMs = options.loadDelayMs ?? 0;
    const lib = options.library ?? 'full';
    this.library =
      lib === 'empty'
        ? []
        : MOCK_TRACKS.map((track, i) => ({
            track,
            // „analyzing": jeder dritte Track wartet noch, einer fehlt auf der Platte.
            analyzed: lib === 'full' || i % 3 !== 2,
            missing: lib === 'analyzing' && i === 9,
          }));
    if (options.scene) applyScene(this, options.scene);
    this.computeLevels();
    if (!this.frozen && !options.manualClock && typeof setInterval === 'function') {
      this.lastTick = now();
      this.timer = setInterval(() => {
        const t = now();
        // Nach einer Pause (Tab im Hintergrund) nicht in einem Sprung aufholen.
        const dt = Math.min(0.1, (t - this.lastTick) / 1000);
        this.lastTick = t;
        this.step(dt);
      }, TICK_MS);
    }
  }

  dispose(): void {
    if (this.timer !== null) clearInterval(this.timer);
    this.timer = null;
  }

  // --- CoreApi -------------------------------------------------------------

  send(cmd: Command): void {
    this.apply(cmd);
    if (this.frozen) {
      this.computeLevels();
      this.emit();
    }
  }

  subscribe(listener: (s: Snapshot) => void): Unsubscribe {
    this.listeners.add(listener);
    listener(this.snapshot());
    return () => this.listeners.delete(listener);
  }

  subscribeStatus(listener: (s: EngineStatus) => void): Unsubscribe {
    this.statusListeners.add(listener);
    listener(this.status());
    return () => this.statusListeners.delete(listener);
  }

  subscribeLibrary(listener: (s: LibraryStatus) => void): Unsubscribe {
    this.libraryListeners.add(listener);
    listener(this.libraryStatus());
    return () => this.libraryListeners.delete(listener);
  }

  listTracks(): Promise<TrackInfo[]> {
    const rows = this.library.map((e): TrackInfo => {
      const info = trackInfo(e.track);
      // Ohne Analyse sind Tempo und Tonart unbekannt (wie in der Hülle: bpm = 0, key = null).
      return { ...info, bpm: e.analyzed ? info.bpm : 0, key: e.analyzed ? info.key : null, analyzed: e.analyzed, missing: e.missing };
    });
    rows.sort((a, b) => a.title.localeCompare(b.title, 'de'));
    return Promise.resolve(rows);
  }

  /** Simulierter Import: Jeder Pfad bringt bis zu sechs noch nicht vorhandene Tracks, zunächst ohne Analyse. */
  importPaths(paths: string[]): Promise<ImportResult> {
    let added = 0;
    for (const _path of paths) {
      void _path;
      for (const track of MOCK_TRACKS) {
        if (added >= paths.length * 6) break;
        if (this.library.some((e) => e.track.id === track.id)) continue;
        this.library.push({ track, analyzed: false, missing: false });
        added += 1;
      }
    }
    if (added > 0) this.libraryChanged();
    return Promise.resolve({ added, duplicates: 0, relinked: 0, failed: [] });
  }

  loadTrack(deck: DeckId, trackId: TrackId): Promise<void> {
    const entry = this.library.find((e) => e.track.id === trackId);
    if (!entry) return Promise.reject(`Track ${trackId} ist nicht in der Bibliothek.`);
    if (entry.missing) return Promise.reject(`/musik/${entry.track.title}.flac: Datei nicht gefunden.`);
    const finish = () => {
      // Wie in der Hülle: Ein noch nicht analysierter Track wird beim Laden analysiert.
      if (!entry.analyzed) {
        entry.analyzed = true;
        this.libraryChanged();
      }
      this.loadNow(deck, trackId);
      if (this.frozen) {
        this.computeLevels();
        this.emit();
      }
    };
    if (this.loadDelayMs <= 0) {
      finish();
      return Promise.resolve();
    }
    return new Promise((resolve) => {
      setTimeout(() => {
        finish();
        resolve();
      }, this.loadDelayMs);
    });
  }

  private libraryStatus(): LibraryStatus {
    return { rev: this.libraryRev, pending_analysis: this.library.filter((e) => !e.analyzed && !e.missing).length };
  }

  private libraryChanged(): void {
    this.libraryRev += 1;
    const st = this.libraryStatus();
    this.libraryListeners.forEach((l) => l(st));
  }

  /** Hintergrund-Analyse: alle 0,8 s simulierter Zeit wird ein Track fertig. */
  private stepAnalysis(dt: number): void {
    const next = this.library.find((e) => !e.analyzed && !e.missing);
    if (!next) {
      this.analysisClock = 0;
      return;
    }
    this.analysisClock += dt;
    if (this.analysisClock >= ANALYSIS_SECONDS) {
      this.analysisClock = 0;
      next.analyzed = true;
      this.libraryChanged();
    }
  }

  getLoadedTracks(): Promise<LoadedTracks> {
    const out = { a: null, b: null, c: null, d: null } as LoadedTracks;
    for (const id of DECK_IDS) out[id] = this.decks[deckIndex(id)].track?.id ?? null;
    return Promise.resolve(out);
  }

  getWaveform(trackId: TrackId): Promise<Waveform> {
    const t = MOCK_TRACKS.find((x) => x.id === trackId);
    if (!t) return Promise.reject(new Error(`unknown track ${trackId}`));
    return Promise.resolve(this.waveFor(t));
  }

  getAnalysis(trackId: TrackId): Promise<TrackAnalysis> {
    const t = MOCK_TRACKS.find((x) => x.id === trackId);
    if (!t) return Promise.reject(new Error(`unknown track ${trackId}`));
    return Promise.resolve(trackAnalysis(t));
  }

  listAudioDevices(): Promise<AudioDevice[]> {
    return Promise.resolve(this.noDevice ? [] : MOCK_DEVICES.map((d) => ({ ...d })));
  }

  /** Wie der Kern: Unbekanntes oder nicht zu öffnendes Gerät wird abgelehnt, die bisherige Wahl bleibt. */
  selectAudioDevice(name: string | null): Promise<void> {
    if (name !== null) {
      const known = !this.noDevice && MOCK_DEVICES.some((d) => d.name === name);
      if (!known) return Promise.reject(`Gerät „${name}“ nicht gefunden`);
      if (name === 'Gerät belegt') return Promise.reject('Audio-Schnittstelle meldet: Gerät wird von einer anderen Anwendung benutzt');
    } else if (this.noDevice) {
      return Promise.reject('kein Ausgabegerät gefunden');
    }
    this.device = name;
    const st = this.status();
    this.statusListeners.forEach((l) => l(st));
    return Promise.resolve();
  }

  // --- Simulation ----------------------------------------------------------

  /** Track sofort laden (auch für Szenen). */
  loadNow(deck: DeckId, trackId: TrackId): void {
    const t = MOCK_TRACKS.find((x) => x.id === trackId);
    if (!t) throw new Error(`unknown track ${trackId}`);
    const d = this.decks[deckIndex(deck)];
    d.track = t;
    d.wave = this.waveFor(t);
    d.pos = 0;
    d.playing = false;
    d.playTime = 0;
    d.bend = 0;
    d.rate = 1;
    if (d.sync === 'off') d.manualOffset = 0;
  }

  private waveFor(t: MockTrack): Waveform {
    let w = this.waves.get(t.id);
    if (!w) {
      w = synthWaveform(t);
      this.waves.set(t.id, w);
      // Älteste Einträge verwerfen, solange sie auf keinem Deck liegen.
      for (const key of this.waves.keys()) {
        if (this.waves.size <= WAVE_CACHE_SIZE) break;
        if (key !== t.id && !this.decks.some((d) => d.track?.id === key)) this.waves.delete(key);
      }
    }
    return w;
  }

  apply(cmd: Command): void {
    const d = 'deck' in cmd && cmd.deck ? this.decks[deckIndex(cmd.deck)] : null;
    switch (cmd.type) {
      case 'play':
        if (!d || !d.track) break;
        if (cmd.on && !d.playing) {
          if (d.pos >= d.track.duration - 0.05) d.pos = 0;
          d.playing = true;
          d.playTime = 0;
          this.pickMaster();
          if (d.sync === 'beat' && this.master !== cmd.deck) this.alignPhase(d);
        } else if (!cmd.on) {
          d.playing = false;
          d.bend = 0;
          this.pickMaster();
        }
        break;
      case 'seek':
        if (d && d.track) d.pos = Math.min(Math.max(0, cmd.pos), d.track.duration);
        break;
      case 'tempo':
        if (d) d.manualOffset = Math.min(1, Math.max(-1, cmd.offset));
        break;
      case 'bend':
        if (d) d.bend = Math.min(0.5, Math.max(-0.5, cmd.amount));
        break;
      case 'sync':
        if (!d) break;
        if (cmd.mode === 'off' && d.sync !== 'off' && d.track) {
          // Regler übernimmt das zuletzt geführte Tempo, damit nichts springt.
          d.manualOffset = this.effectiveOffset(d, cmd.deck);
        }
        d.sync = cmd.mode;
        if (cmd.mode === 'beat' && d.track && this.master !== cmd.deck && this.clockBpm > 0) this.alignPhase(d);
        break;
      case 'set_master':
        this.masterMode = cmd.deck;
        this.pickMaster();
        break;
      case 'clock_tempo':
        if (this.master === null) this.clockBpm = Math.min(300, Math.max(20, cmd.bpm));
        break;
      case 'gain':
        if (d) d.gainDb = Math.min(12, fromWire(cmd.db));
        break;
      case 'eq':
        if (d) d.eqDb[bandIndex(cmd.band)] = Math.min(6, fromWire(cmd.db));
        break;
      case 'filter':
        if (d) d.filter = Math.min(1, Math.max(-1, cmd.pos));
        break;
      case 'fader':
        if (d) d.fader = Math.min(1, Math.max(0, cmd.value));
        break;
      case 'cue':
        if (d) d.cue = cmd.on;
        break;
      case 'xf_assign':
        if (d) d.xf = cmd.assign;
        break;
      case 'crossfader':
        this.crossfader = Math.min(1, Math.max(-1, cmd.pos));
        break;
      case 'master_volume':
        this.masterDb = Math.min(6, fromWire(cmd.db));
        break;
      case 'limiter':
        this.limiterOn = cmd.on;
        break;
      case 'cue_mix':
        this.cueMix = Math.min(1, Math.max(0, cmd.value));
        break;
      case 'cue_volume':
        this.cueDb = Math.min(6, fromWire(cmd.db));
        break;
    }
  }

  /** Master bestimmen (DECK-09). Ein Wechsel übernimmt das laufende Clock-Tempo, wenn das neue Deck synchron lief. */
  pickMaster(): void {
    let next: DeckId | null;
    if (this.masterMode === null) {
      next = null;
    } else if (this.masterMode !== 'auto') {
      next = this.masterMode;
    } else {
      // Automatisch: das am längsten laufende hörbare Deck (DECK-09); läuft keines hörbar,
      // das am längsten laufende.
      const audible = (d: MockDeck): boolean => d.playing && channelOpen({ fader: d.fader, xf_assign: d.xf }, this.crossfader);
      const cur = this.master ? this.decks[deckIndex(this.master)] : null;
      const anyAudible = this.decks.some(audible);
      if (cur && cur.playing && (audible(cur) || !anyAudible)) {
        next = this.master;
      } else {
        next = null;
        let best = -1;
        for (const id of DECK_IDS) {
          const d = this.decks[deckIndex(id)];
          if (!d.playing || (anyAudible && !audible(d))) continue;
          if (d.playTime > best) {
            best = d.playTime;
            next = id;
          }
        }
      }
    }
    if (next !== this.master) {
      const d = next ? this.decks[deckIndex(next)] : null;
      if (d && d.track) {
        if (d.sync !== 'off' && this.clockBpm > 0) {
          d.manualOffset = this.clockBpm / d.track.bpm - 1;
        } else {
          this.clockBpm = d.track.bpm * (1 + d.manualOffset);
        }
        this.clockBeats = Math.round(this.clockBeats) + (this.beatOf(d) - Math.floor(this.beatOf(d)));
      }
      this.master = next;
    }
  }

  private beatOf(d: MockDeck): number {
    return d.track ? beatAt(d.track.grid, d.pos) : 0;
  }

  private phaseOf(d: MockDeck): number {
    return wrapPhase(this.beatOf(d) - this.clockBeats);
  }

  /** Phasen-Sprung auf die Master-Clock (Sync-Taste). */
  alignPhase(d: MockDeck): void {
    if (!d.track) return;
    const beat = this.beatOf(d) - this.phaseOf(d);
    d.pos = Math.max(0, secAt(d.track.grid, beat < 0 ? beat + Math.ceil(-beat) : beat));
  }

  private effectiveOffset(d: MockDeck, id: DeckId): number {
    if (!d.track) return 0;
    if (d.sync !== 'off' && this.master !== id && this.clockBpm > 0) return this.clockBpm / d.track.bpm - 1;
    return d.manualOffset;
  }

  /** Simulation um `dt` Sekunden weiterführen und einen Schnappschuss senden. */
  step(dt: number): void {
    this.time += dt;
    this.frames += Math.round(dt * MOCK_SAMPLE_RATE);
    for (const d of this.decks) if (d.playing) d.playTime += dt;
    this.pickMaster();

    // Phasenfehler am Anfang des Schritts messen – bevor der Master weiterläuft.
    const phaseErr = this.decks.map((d) => (d.track && this.clockBpm > 0 ? this.phaseOf(d) : 0));

    const m = this.master ? this.decks[deckIndex(this.master)] : null;
    if (m && m.track) {
      // Der Master bestimmt die Clock.
      this.clockBpm = m.track.bpm * (1 + m.manualOffset);
      m.rate = (1 + m.manualOffset) * (1 + m.bend);
      if (m.playing) {
        const before = this.beatOf(m);
        this.advance(m, dt);
        this.clockBeats += this.beatOf(m) - before;
      }
    } else if (this.clockBpm > 0) {
      this.clockBeats += (dt * this.clockBpm) / 60;
    }

    for (const id of DECK_IDS) {
      const d = this.decks[deckIndex(id)];
      if (d === m || !d.track) continue;
      let rate = 1 + this.effectiveOffset(d, id);
      if (d.sync === 'beat' && d.playing && this.clockBpm > 0 && d.bend === 0) {
        // Kleine Abweichungen unhörbar ausregeln: höchstens ±1 % Ratenänderung (7.6).
        const err = phaseErr[deckIndex(id)];
        if (Math.abs(err) < 0.125) rate *= 1 - Math.min(0.01, Math.max(-0.01, err * 0.5));
      }
      d.rate = rate * (1 + d.bend);
      if (d.playing) this.advance(d, dt);
    }

    this.computeLevels();
    this.emit();
    this.stepAnalysis(dt);
    this.statusTick += dt;
    if (this.statusTick >= 0.25) {
      this.statusTick = 0;
      const st = this.status();
      this.statusListeners.forEach((l) => l(st));
    }
  }

  private advance(d: MockDeck, dt: number): void {
    if (!d.track) return;
    d.pos += dt * d.rate;
    if (d.pos >= d.track.duration) {
      d.pos = d.track.duration;
      d.playing = false;
    } else if (d.pos < 0) {
      d.pos = 0;
    }
  }

  /** Bandpegel an der Abspielposition, nach EQ und Filter. */
  private bandLevels(d: MockDeck): [number, number, number] {
    if (!d.track || !d.wave || !d.playing) return [0, 0, 0];
    const lvl = d.wave.levels[2];
    const points = lvl.data.length / VALUES_PER_POINT;
    const idx = Math.min(points - 1, Math.max(0, Math.floor((d.pos * d.wave.sample_rate) / lvl.samples_per_point)));
    const f = filterSetting(d.filter);
    const centers = [70, 900, 8000];
    const out: [number, number, number] = [0, 0, 0];
    for (let b = 0; b < 3; b++) {
      let a = lvl.data[idx * VALUES_PER_POINT + b * 3 + 1] * dbToLin(d.eqDb[b]);
      if (f.type === 'lowpass') a *= 1 / Math.sqrt(1 + Math.pow(centers[b] / f.hz, 4));
      if (f.type === 'highpass') a *= 1 / Math.sqrt(1 + Math.pow(f.hz / centers[b], 4));
      out[b] = a;
    }
    return out;
  }

  /** Kanal- und Summenpegel aus dem aktuellen Zustand ableiten. */
  computeLevels(): void {
    const [xfL, xfR] = crossfaderGains(this.crossfader);
    let sumL = 0;
    let sumR = 0;
    const q = Math.floor(this.time * 30);
    this.decks.forEach((d, i) => {
      const [lo, mi, hi] = this.bandLevels(d);
      const level = Math.sqrt(lo * lo + 0.7 * mi * mi + 0.5 * hi * hi) * dbToLin(d.gainDb) * 0.92;
      // Leichte Unterschiede zwischen links und rechts.
      const l = level * (0.94 + 0.06 * hash01(q, i * 2 + 1));
      const r = level * (0.94 + 0.06 * hash01(q, i * 2 + 2));
      d.peak = [l, r];
      const xf = d.xf === 'left' ? xfL : d.xf === 'right' ? xfR : 1;
      const g = faderGain(d.fader) * xf;
      sumL += l * g;
      sumR += r * g;
    });
    const vol = dbToLin(this.masterDb);
    let l = sumL * vol * 0.9;
    let r = sumR * vol * 0.9;
    const top = Math.max(l, r);
    if (this.limiterOn && top > LIMITER_CEILING) {
      const g = LIMITER_CEILING / top;
      this.limiterGrDb = 20 * Math.log10(g);
      l *= g;
      r *= g;
    } else {
      this.limiterGrDb = 0;
    }
    this.masterPeak = [l, r];
  }

  status(): EngineStatus {
    const playing = this.decks.filter((d) => d.playing).length;
    const wobble = 0.02 * hash01(Math.floor(this.time * 4), 77);
    return {
      dsp_load: 0.11 + 0.045 * playing + wobble,
      xruns: 0,
      sample_rate: 48000,
      buffer_size: 256,
      has_device: !this.noDevice,
      device_error: this.noDevice ? NO_DEVICE_TEXT : null,
      device: this.device,
    };
  }

  snapshot(): Snapshot {
    const decks = this.decks.map((d, i): DeckState => {
      const id = DECK_IDS[i];
      const loaded = d.track !== null;
      const offset = this.effectiveOffset(d, id);
      return {
        loaded,
        playing: d.playing,
        pos: d.pos,
        duration: d.track?.duration ?? 0,
        beat: this.beatOf(d),
        track_bpm: d.track?.bpm ?? 0,
        bpm: d.track ? d.track.bpm * (d.playing ? d.rate : 1 + offset) : 0,
        tempo_offset: offset,
        sync: d.sync,
        is_master: this.master === id,
        phase: loaded && this.clockBpm > 0 ? this.phaseOf(d) : 0,
        peak: [d.peak[0], d.peak[1]],
        gain_db: toWire(d.gainDb),
        eq_db: [toWire(d.eqDb[0]), toWire(d.eqDb[1]), toWire(d.eqDb[2])],
        filter: d.filter,
        fader: d.fader,
        cue: d.cue,
        xf_assign: d.xf,
      };
    }) as Snapshot['decks'];
    return {
      clock_bpm: this.clockBpm,
      clock_beats: this.clockBeats,
      master: this.master,
      decks,
      mixer: {
        crossfader: this.crossfader,
        master_db: toWire(this.masterDb),
        limiter_on: this.limiterOn,
        limiter_gr_db: this.limiterGrDb,
        master_peak: [this.masterPeak[0], this.masterPeak[1]],
        cue_mix: this.cueMix,
        cue_db: toWire(this.cueDb),
      },
      frames: this.frames,
    };
  }

  private emit(): void {
    const s = this.snapshot();
    this.listeners.forEach((l) => l(s));
  }
}

function now(): number {
  return typeof performance !== 'undefined' ? performance.now() : Date.now();
}
