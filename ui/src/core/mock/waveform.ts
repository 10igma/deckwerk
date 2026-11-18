// Synthetische Wellenform-Daten in drei Bändern. Es wird kein Audio erzeugt, sondern direkt
// die Hüllkurve je Band: Kick-Raster, Bass, Claps, Hi-Hats, Flächen, Riser und Rolls.
import type { Waveform, WaveformLevel } from '../types';
import { VALUES_PER_POINT } from '../types';
import { hash01, makeRng } from './prng';
import { MOCK_SAMPLE_RATE, type MockTrack, type SectionKind } from './tracks';

const LEVEL_SPP = [64, 256, 1024, 4096];
const OVERVIEW_POINTS = 2000;

interface BarParams {
  kick: number;
  bass: number;
  clap: number;
  hats: number;
  openHat: number;
  ride: number;
  synth: number;
  pad: number;
  /** Fortschritt des Risers 0…1 am Taktanfang und -ende. */
  riser0: number;
  riser1: number;
  /** Kick-Wiederholungen je Beat (Roll), 0 = keiner. */
  roll: number;
  /** Beats seit dem letzten Drop (für das Crash-Becken), sonst groß. */
  sinceDrop: number;
}

function barParams(track: MockTrack): BarParams[] {
  const out: BarParams[] = [];
  let dropBar = -999;
  for (const sec of track.sections) {
    if (sec.kind === 'drop') dropBar = sec.startBar;
    for (let i = 0; i < sec.bars; i++) {
      const rel = i / sec.bars;
      const p: BarParams = {
        kick: 0, bass: 0, clap: 0, hats: 0, openHat: 0, ride: 0, synth: 0, pad: 0,
        riser0: 0, riser1: 0, roll: 0, sinceDrop: (sec.startBar + i - dropBar) * 4,
      };
      const k: SectionKind = sec.kind;
      if (k === 'intro') {
        p.kick = 1; p.hats = i >= 8 ? 0.7 : 0.35; p.openHat = i >= 8 ? 0.5 : 0;
      } else if (k === 'groove') {
        p.kick = 1; p.bass = 0.8; p.hats = 0.8; p.openHat = 0.7; p.clap = i >= 8 ? 0.8 : 0; p.synth = 0.25;
      } else if (k === 'main') {
        p.kick = 1; p.bass = 1; p.hats = 1; p.openHat = 0.9; p.clap = 1; p.synth = 0.6 + 0.3 * rel; p.ride = rel > 0.5 ? 0.7 : 0;
      } else if (k === 'break') {
        p.pad = 0.6 + 0.4 * rel; p.synth = 0.5; p.hats = rel > 0.5 ? 0.3 : 0; p.bass = 0;
      } else if (k === 'build') {
        p.pad = 1; p.synth = 0.8; p.hats = 0.6; p.clap = 0.6;
        p.riser0 = i / sec.bars; p.riser1 = (i + 1) / sec.bars;
        // Roll in den letzten vier Takten: 1/4 → 1/8 → 1/16, letzter halber Takt ohne Kick.
        const left = sec.bars - i;
        p.roll = left <= 1 ? 4 : left <= 2 ? 2 : left <= 4 ? 1 : 0;
      } else if (k === 'drop') {
        p.kick = 1; p.bass = 1; p.hats = 1; p.openHat = 1; p.clap = 1; p.synth = 0.9; p.ride = 0.9; p.pad = 0.2;
      } else {
        // Outro: Elemente fallen weg, zuletzt nur Kick und Hi-Hat.
        p.kick = 1; p.bass = rel < 0.5 ? 0.7 : 0; p.hats = 0.7; p.openHat = rel < 0.75 ? 0.6 : 0; p.clap = rel < 0.25 ? 0.7 : 0;
      }
      out.push(p);
    }
  }
  return out;
}

const softClip = (v: number): number => (v <= 0.8 ? v : 0.8 + 0.2 * Math.tanh((v - 0.8) / 0.2));

/** Feinste Stufe (64 Samples je Punkt) berechnen. */
function synthFinest(track: MockTrack): Float32Array {
  const sr = MOCK_SAMPLE_RATE;
  const spp = LEVEL_SPP[0];
  const frames = Math.floor(track.duration * sr);
  const points = Math.ceil(frames / spp);
  const data = new Float32Array(points * VALUES_PER_POINT);
  const bars = barParams(track);
  const rnd = makeRng(track.seed ^ 0xabcdef);
  // Muster je Track: Bass auf 16teln, Synth-Figur über zwei Takte.
  const bassPat: number[] = [];
  for (let i = 0; i < 16; i++) bassPat.push(i % 4 === 0 ? 0 : i % 4 === 2 ? 1 : rnd() < 0.55 ? 0.75 : 0);
  const synthPat: number[] = [];
  for (let i = 0; i < 32; i++) synthPat.push(rnd() < 0.36 ? 0.5 + 0.5 * rnd() : 0);
  const hatAccent: number[] = [];
  for (let i = 0; i < 16; i++) hatAccent.push(i % 4 === 2 ? 0 : 0.5 + 0.5 * rnd());
  const beatsPerSec = track.bpm / 60;
  const totalBeats = track.totalBars * 4;

  for (let i = 0; i < points; i++) {
    const t = (i * spp + spp / 2) / sr;
    const beat = (t - track.firstBeat) * beatsPerSec;
    let low = 0;
    let mid = 0;
    let high = 0;
    if (beat >= 0 && beat < totalBeats) {
      const barIdx = Math.floor(beat / 4);
      const p = bars[Math.min(barIdx, bars.length - 1)];
      const beatInBar = beat - barIdx * 4;
      const fb = beat - Math.floor(beat);
      const step16 = Math.floor(beatInBar * 4);
      const f16 = beatInBar * 4 - step16;

      // Kick: harter Anschlag, Ausklang über knapp einen halben Beat.
      let kickEnv = 0;
      if (p.kick > 0) {
        kickEnv = p.kick * Math.exp(-fb * 5.2) * Math.min(1, fb / 0.012 + 0.35);
      } else if (p.roll > 0) {
        const lastHalf = p.roll === 4 && beatInBar >= 2;
        if (!lastHalf) {
          const fr = beat * p.roll - Math.floor(beat * p.roll);
          kickEnv = (0.55 + 0.1 * p.roll) * Math.exp(-fr * 3.2);
        }
      }
      // Bass: 16tel-Figur, von der Kick weggedrückt.
      const duck = p.kick > 0 ? 1 - 0.85 * Math.exp(-fb * 5) : 1;
      const bassEnv = p.bass * bassPat[step16] * Math.exp(-f16 * 1.4) * duck * 0.62;
      low = Math.max(kickEnv * 0.96, bassEnv) + 0.25 * Math.min(kickEnv, bassEnv);
      low += p.pad * 0.06;

      // Mitten: Kick-Klick, Clap auf 2 und 4, Synth-Figur, Fläche.
      const click = kickEnv > 0 ? 0.34 * Math.exp(-fb * 34) : 0;
      const beatNo = Math.floor(beatInBar);
      const clap = p.clap > 0 && beatNo % 2 === 1 ? 0.6 * p.clap * Math.exp(-fb * 8.5) : 0;
      const sStep = (barIdx % 2) * 16 + step16;
      const synth = p.synth * synthPat[sStep] * Math.exp(-f16 * 2.0) * 0.8;
      const pad = p.pad * 0.5 * (0.8 + 0.2 * Math.sin((beat / 8) * Math.PI * 2));
      const riser = p.riser0 + (p.riser1 - p.riser0) * (beatInBar / 4);
      mid = click + clap + synth + pad + riser * riser * 0.3;

      // Höhen: geschlossene Hi-Hat auf 16teln, offene auf dem Offbeat, Ride, Riser, Crash.
      const closed = p.hats * 0.34 * hatAccent[step16] * Math.exp(-f16 * 6);
      const open = p.openHat > 0 && fb >= 0.5 ? 0.66 * p.openHat * Math.exp(-(fb - 0.5) * 6) : 0;
      const ride = p.ride * 0.36 * Math.exp(-(fb < 0.5 ? fb : fb - 0.5) * 4.5);
      const crash = p.sinceDrop >= 0 && p.sinceDrop < 8 ? 0.62 * Math.exp(-(p.sinceDrop + beatInBar) * 0.55) : 0;
      high = closed + open + ride + riser * riser * 0.6 + crash;

      // Lautstärke-Einblendung am Anfang des Intros vermeiden wir bewusst: Techno beginnt hart.
      const tex = 0.9 + 0.1 * hash01(i, track.seed);
      low = softClip(low * (0.95 + 0.05 * hash01(i, 11)));
      mid = softClip(mid * tex);
      high = softClip(high * (0.82 + 0.18 * hash01(i, 23)));
    } else {
      // Stille vor dem ersten Beat und nach dem letzten Takt: nur Grundrauschen.
      high = 0.004;
    }
    const o = i * VALUES_PER_POINT;
    const amps = [low, mid, high];
    for (let b = 0; b < 3; b++) {
      const a = amps[b];
      data[o + b * 3] = -a * (0.9 + 0.1 * hash01(i, 31 + b));
      data[o + b * 3 + 1] = a;
      data[o + b * 3 + 2] = a * (b === 0 ? 0.62 : 0.5);
    }
  }
  return data;
}

/** Gröbere Stufe: je `factor` Punkte zusammenfassen (min, max, RMS). */
export function reduceLevel(src: Float32Array, factor: number): Float32Array {
  const srcPoints = src.length / VALUES_PER_POINT;
  const points = Math.ceil(srcPoints / factor);
  const out = new Float32Array(points * VALUES_PER_POINT);
  for (let i = 0; i < points; i++) {
    const from = i * factor;
    const to = Math.min(srcPoints, from + factor);
    const o = i * VALUES_PER_POINT;
    for (let b = 0; b < 3; b++) {
      let mn = 0;
      let mx = 0;
      let sq = 0;
      for (let j = from; j < to; j++) {
        const s = j * VALUES_PER_POINT + b * 3;
        if (src[s] < mn) mn = src[s];
        if (src[s + 1] > mx) mx = src[s + 1];
        sq += src[s + 2] * src[s + 2];
      }
      out[o + b * 3] = mn;
      out[o + b * 3 + 1] = mx;
      out[o + b * 3 + 2] = Math.sqrt(sq / Math.max(1, to - from));
    }
  }
  return out;
}

function makeOverview(src: Float32Array, srcSpp: number, frames: number): WaveformLevel {
  const srcPoints = src.length / VALUES_PER_POINT;
  const out = new Float32Array(OVERVIEW_POINTS * VALUES_PER_POINT);
  const per = srcPoints / OVERVIEW_POINTS;
  for (let i = 0; i < OVERVIEW_POINTS; i++) {
    const from = Math.floor(i * per);
    const to = Math.max(from + 1, Math.min(srcPoints, Math.floor((i + 1) * per)));
    const o = i * VALUES_PER_POINT;
    for (let b = 0; b < 3; b++) {
      let mn = 0;
      let mx = 0;
      let sq = 0;
      for (let j = from; j < to; j++) {
        const s = j * VALUES_PER_POINT + b * 3;
        if (src[s] < mn) mn = src[s];
        if (src[s + 1] > mx) mx = src[s + 1];
        sq += src[s + 2] * src[s + 2];
      }
      out[o + b * 3] = mn;
      out[o + b * 3 + 1] = mx;
      out[o + b * 3 + 2] = Math.sqrt(sq / (to - from));
    }
  }
  return { samples_per_point: Math.round((srcSpp * srcPoints) / OVERVIEW_POINTS) || Math.round(frames / OVERVIEW_POINTS), data: out };
}

export function synthWaveform(track: MockTrack): Waveform {
  const frames = Math.floor(track.duration * MOCK_SAMPLE_RATE);
  const l0 = synthFinest(track);
  const l1 = reduceLevel(l0, 4);
  const l2 = reduceLevel(l1, 4);
  const l3 = reduceLevel(l2, 4);
  const levels: WaveformLevel[] = [l0, l1, l2, l3].map((data, i) => ({ samples_per_point: LEVEL_SPP[i], data }));
  return { sample_rate: MOCK_SAMPLE_RATE, frames, overview: makeOverview(l2, LEVEL_SPP[2], frames), levels };
}
