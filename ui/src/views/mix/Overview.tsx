// Übersichts-Wellenform (VIS-01): ganzer Track, Position, Cue-Punkt; Klick springt (DECK-20).
import { useEffect, useRef } from 'react';
import { seek } from '../../app/actions';
import { getDeckAssets, type DeckAssets } from '../../app/assets';
import { latestSnapshot, useFrame } from '../../app/frame';
import { getUi } from '../../app/store';
import { deckIndex, VALUES_PER_POINT, type DeckId } from '../../core/types';
import { useTip } from '../../design/components';
import { cssNumber, cssVar, deckTokenName } from '../../design/tokens';
import { useT } from '../../i18n';
import { BAND_SHARE } from './wave/scene';

interface Props {
  deck: DeckId;
  ariaLabel: string;
}

/** Statisches Bild der Übersicht in ein eigenes Canvas zeichnen. */
function renderStatic(assets: DeckAssets, w: number, h: number): HTMLCanvasElement {
  const off = document.createElement('canvas');
  off.width = w;
  off.height = h;
  const ctx = off.getContext('2d');
  if (!ctx) return off;
  const data = assets.waveform.overview.data;
  const points = Math.floor(data.length / VALUES_PER_POINT);
  const colors = [cssVar('--band-low'), cssVar('--band-mid'), cssVar('--band-high')];
  const amp = [0, 0, 0];
  // Jede Spalte fasst mindestens einige Punkte zusammen, sonst flimmert das Kick-Raster als Kamm.
  const span = Math.max(points / w, 6);
  for (let x = 0; x < w; x++) {
    const mid = ((x + 0.5) / w) * points;
    const i0 = Math.max(0, Math.floor(mid - span / 2));
    const i1 = Math.max(i0 + 1, Math.ceil(mid + span / 2));
    amp[0] = amp[1] = amp[2] = 0;
    for (let i = i0; i < i1 && i < points; i++) {
      for (let b = 0; b < 3; b++) {
        // RMS-nah zeichnen, damit Abschnitte (Breakdown, Drop) klar hervortreten.
        const o = i * VALUES_PER_POINT + b * 3;
        const v = 0.45 * Math.max(-data[o], data[o + 1]) + 0.9 * data[o + 2];
        if (v > amp[b]) amp[b] = v;
      }
    }
    let from = 0;
    for (let b = 0; b < 3; b++) {
      const hgt = Math.min(h - from, amp[b] * BAND_SHARE[b] * h * 1.02);
      if (hgt > 0.4) {
        ctx.fillStyle = colors[b];
        ctx.fillRect(x, h - from - hgt, 1, hgt);
      }
      from += hgt;
    }
  }
  return off;
}

export function Overview({ deck, ariaLabel }: Props) {
  const canvas = useRef<HTMLCanvasElement | null>(null);
  const size = useRef({ w: 0, h: 0, dpr: 1 });
  const cache = useRef<{ assets: DeckAssets; w: number; h: number; theme: string; image: HTMLCanvasElement } | null>(null);
  const lastKey = useRef('');
  const dragging = useRef(false);
  const t = useT();
  const tip = useTip({ name: t('deck.overviewTip'), value: t('deck.overviewTipValue'), keys: ['Shift'] });

  useEffect(() => {
    const c = canvas.current;
    if (!c) return;
    const ro = new ResizeObserver(() => {
      const dpr = window.devicePixelRatio || 1;
      const r = c.getBoundingClientRect();
      size.current = { w: Math.max(2, Math.round(r.width * dpr)), h: Math.max(2, Math.round(r.height * dpr)), dpr };
      lastKey.current = '';
    });
    ro.observe(c);
    return () => ro.disconnect();
  }, []);

  useFrame((snap) => {
    const c = canvas.current;
    const { w, h, dpr } = size.current;
    if (!c || w < 4) return;
    const d = snap.decks[deckIndex(deck)];
    const assets = d.loaded ? getDeckAssets(deck) : null;
    const theme = document.documentElement.dataset.theme ?? '';
    const cue = getUi().cuePoints[deck];
    const posPx = d.duration > 0 ? Math.round((d.pos / d.duration) * w) : 0;
    const key = `${w}x${h}|${assets ? assets.trackId : '-'}|${posPx}|${cue.toFixed(2)}|${theme}`;
    if (key === lastKey.current) return;
    lastKey.current = key;
    if (c.width !== w || c.height !== h) {
      c.width = w;
      c.height = h;
    }
    const ctx = c.getContext('2d');
    if (!ctx) return;
    ctx.clearRect(0, 0, w, h);
    if (!assets || d.duration <= 0) return;
    const cached = cache.current;
    if (!cached || cached.assets !== assets || cached.w !== w || cached.h !== h || cached.theme !== theme) {
      cache.current = { assets, w, h, theme, image: renderStatic(assets, w, h) };
    }
    ctx.drawImage(cache.current!.image, 0, 0);
    // Gespielter Teil abgedunkelt.
    ctx.globalAlpha = 1 - cssNumber('--played-dim');
    ctx.fillStyle = cssVar('--bg-1');
    ctx.fillRect(0, 0, posPx, h);
    ctx.globalAlpha = 1;
    // Cue-Punkt: kleine Marke am oberen Rand.
    const cuePx = Math.round((cue / d.duration) * w);
    const m = Math.round(4 * dpr);
    ctx.fillStyle = cssVar('--text-1');
    ctx.beginPath();
    ctx.moveTo(cuePx - m, 0);
    ctx.lineTo(cuePx + m, 0);
    ctx.lineTo(cuePx, m * 1.4);
    ctx.closePath();
    ctx.fill();
    // Position in der Deck-Farbe.
    const lw = Math.max(2, Math.round(2 * dpr));
    ctx.fillStyle = cssVar('--bg-0');
    ctx.fillRect(posPx - lw / 2 - dpr, 0, lw + 2 * dpr, h);
    ctx.fillStyle = cssVar(deckTokenName(deck));
    ctx.fillRect(posPx - lw / 2, 0, lw, h);
  });

  const seekTo = (e: React.PointerEvent<HTMLCanvasElement>) => {
    const d = latestSnapshot().decks[deckIndex(deck)];
    if (!d.loaded) return;
    const r = e.currentTarget.getBoundingClientRect();
    return seek(deck, ((e.clientX - r.left) / r.width) * d.duration, e.shiftKey);
  };

  return (
    <canvas
      ref={canvas}
      className="overview"
      role="img"
      aria-label={ariaLabel}
      {...tip}
      onPointerDown={(e) => {
        if (e.button !== 0) return;
        e.currentTarget.setPointerCapture(e.pointerId);
        dragging.current = seekTo(e) ?? false;
      }}
      onPointerMove={(e) => {
        // Im laufenden Deck nur der Klick, kein Schleifen.
        if (dragging.current && !latestSnapshot().decks[deckIndex(deck)].playing) seekTo(e);
      }}
      onPointerUp={() => (dragging.current = false)}
      onPointerCancel={() => (dragging.current = false)}
    />
  );
}
