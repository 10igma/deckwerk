// Pegelanzeige aus Lamellen (MIX-07): Spitze mit Rücklauf, Spitzenhalter, Übersteuerung mit Halten.
// Zeichnet in Canvas direkt aus dem Snapshot, ohne React-Rendering.
import { useEffect, useRef } from 'react';
import type { Snapshot } from '../../core/types';
import { useFrame } from '../../app/frame';
import { linToDb } from '../../lib/db';
import { cssVar } from '../tokens';

export const METER_MIN_DB = -36;
export const METER_MAX_DB = 6;
export const METER_WARN_DB = -9;
/** Rücklauf in dB je Sekunde. */
export const METER_FALL = 30;
export const PEAK_HOLD_MS = 1200;
export const CLIP_HOLD_MS = 2500;

/** Pegel in dB → Anteil der Anzeige 0 … 1. */
export function meterPos(db: number, min = METER_MIN_DB, max = METER_MAX_DB): number {
  if (!(db > min)) return 0;
  return Math.min(1, (db - min) / (max - min));
}

export interface MeterProps {
  /** Spitzenpegel je Kanal (linear), bzw. bei kind='gr' die Pegelreduktion in dB (≤ 0). */
  read: (s: Snapshot) => number[];
  channels?: number;
  /** 'level' = Pegel von unten, 'gr' = Pegelreduktion von oben. */
  kind?: 'level' | 'gr';
  /** Bereich der Reduktions-Anzeige in dB. */
  grRange?: number;
  ariaLabel?: string;
}

interface ChannelState {
  shown: number;
  hold: number;
  holdUntil: number;
  clipUntil: number;
}

export function Meter({ read, channels = 1, kind = 'level', grRange = 12, ariaLabel }: MeterProps) {
  const canvas = useRef<HTMLCanvasElement | null>(null);
  const state = useRef<ChannelState[]>([]);
  const size = useRef({ w: 0, h: 0, dpr: 1 });
  const lastKey = useRef('');

  useEffect(() => {
    const c = canvas.current;
    if (!c) return;
    const ro = new ResizeObserver(() => {
      const dpr = window.devicePixelRatio || 1;
      const r = c.getBoundingClientRect();
      size.current = { w: Math.max(1, Math.round(r.width * dpr)), h: Math.max(1, Math.round(r.height * dpr)), dpr };
      c.width = size.current.w;
      c.height = size.current.h;
      lastKey.current = '';
    });
    ro.observe(c);
    return () => ro.disconnect();
  }, []);

  useFrame((snap, time, dt) => {
    const c = canvas.current;
    const { w, h, dpr } = size.current;
    if (!c || w < 2 || h < 2) return;
    const values = read(snap);
    while (state.current.length < channels) state.current.push({ shown: -Infinity, hold: -Infinity, holdUntil: 0, clipUntil: 0 });

    const slat = Math.max(2, Math.round(3 * dpr));
    const gap = Math.max(1, Math.round(dpr));
    const clipH = kind === 'level' ? Math.round(5 * dpr) + gap : 0;
    const count = Math.max(4, Math.floor((h - clipH + gap) / (slat + gap)));
    const colGap = channels > 1 ? Math.max(1, Math.round(2 * dpr)) : 0;
    const colW = Math.floor((w - colGap * (channels - 1)) / channels);

    // Ballistik: sofort hoch, gleichmäßig zurück.
    let key = `${cssVar('--bg-0')}`;
    const lit: number[] = [];
    const holds: number[] = [];
    const clips: boolean[] = [];
    for (let ch = 0; ch < channels; ch++) {
      const st = state.current[ch];
      const raw = values[ch] ?? 0;
      if (kind === 'gr') {
        const red = Math.min(grRange, Math.max(0, -raw));
        st.shown = Math.max(red, (Number.isFinite(st.shown) ? st.shown : 0) - (12 * dt) / 1000);
        lit.push(Math.round((st.shown / grRange) * count));
        holds.push(-1);
        clips.push(false);
      } else {
        const db = linToDb(raw);
        st.shown = Math.max(db, st.shown - (METER_FALL * dt) / 1000);
        if (st.shown < METER_MIN_DB - 1) st.shown = -Infinity;
        if (db >= st.hold) {
          st.hold = db;
          st.holdUntil = time + PEAK_HOLD_MS;
        } else if (time > st.holdUntil) {
          st.hold = st.shown;
        }
        if (raw > 1.0005) st.clipUntil = time + CLIP_HOLD_MS;
        lit.push(Math.round(meterPos(st.shown) * count));
        holds.push(Math.round(meterPos(st.hold) * count) - 1);
        clips.push(time < st.clipUntil);
      }
      key += `|${lit[ch]},${holds[ch]},${clips[ch] ? 1 : 0}`;
    }
    if (key === lastKey.current) return;
    lastKey.current = key;

    const ctx = c.getContext('2d');
    if (!ctx) return;
    const off = cssVar('--bg-3');
    const base = cssVar('--text-2');
    const warn = cssVar('--status-warn');
    const danger = cssVar('--status-danger');
    const peak = cssVar('--text-1');
    ctx.clearRect(0, 0, w, h);
    const warnFrom = Math.round(meterPos(METER_WARN_DB) * count);
    const zeroAt = Math.round(meterPos(0) * count);
    for (let ch = 0; ch < channels; ch++) {
      const x = ch * (colW + colGap);
      if (kind === 'level') {
        // Übersteuerung: eigener Block oben, bleibt nach einer Spitze stehen.
        ctx.fillStyle = clips[ch] ? danger : off;
        ctx.fillRect(x, 0, colW, clipH - gap);
      }
      for (let i = 0; i < count; i++) {
        const on = i < lit[ch];
        let color = off;
        if (kind === 'gr') {
          color = on ? warn : off;
          ctx.fillStyle = color;
          ctx.fillRect(x, i * (slat + gap), colW, slat);
          continue;
        }
        if (on) color = i >= zeroAt ? danger : i >= warnFrom ? warn : base;
        if (i === holds[ch] && holds[ch] >= 0) color = i >= zeroAt ? danger : peak;
        ctx.fillStyle = color;
        const y = h - (i + 1) * (slat + gap) + gap;
        ctx.fillRect(x, y, colW, slat);
      }
    }
  });

  return <canvas className="dw-meter" ref={canvas} role="img" aria-label={ariaLabel} />;
}
