// Parallele Wellenformen (8.2, VIS-02–04): Deck A oben, Deck B unten, dazwischen die Beat-Schiene.
// Beide Bahnen wachsen von der Schiene weg – liegen die Beats übereinander, ist das Bild spiegelgleich.
import { useEffect, useRef, useState } from 'react';
import { getDeckAssets, type DeckAssets } from '../../app/assets';
import { bend, focusDeck, send } from '../../app/actions';
import { useTrackDrop } from '../../app/dnd';
import { forcedRenderer, webglPresent } from '../../app/env';
import { latestSnapshot, useFrame, useSnap } from '../../app/frame';
import { getUi, setUi, useUi } from '../../app/store';
import { deckIndex, type DeckId } from '../../core/types';
import { DeckPlate, Icon, KeyBtn, useTip } from '../../design/components';
import { cssVar, remPx } from '../../design/tokens';
import { useT } from '../../i18n';
import { clamp } from '../../lib/curves';
import { readStageColors } from './stageColors';
import { Canvas2DWaveRenderer } from './wave/canvas2d';
import type { WaveRenderer } from './wave/renderer';
import { buildScene, secondsPerPx, FALLBACK_BPM, type LaneInput, type StageColors } from './wave/scene';
import { WebGLWaveRenderer } from './wave/webgl';

export const ZOOM_STEPS = [1, 2, 4, 8, 16, 32];
const LANES: [DeckId, DeckId] = ['a', 'b'];
/** Höhe der Beat-Schiene in rem (je Deck eine Hälfte). */
export const RAIL_REM = 2;

function stepZoom(dir: -1 | 1): void {
  const cur = ZOOM_STEPS.indexOf(getUi().zoomBeats);
  const next = clamp((cur < 0 ? 3 : cur) + dir, 0, ZOOM_STEPS.length - 1);
  setUi({ zoomBeats: ZOOM_STEPS[next] });
}

function LaneOverlay({ deck, position }: { deck: DeckId; position: 'top' | 'bottom' }) {
  const t = useT();
  const track = useUi((s) => s.deckTracks[deck]);
  const loading = useUi((s) => s.loading[deck]);
  const loaded = useSnap((s) => s.decks[deckIndex(deck)].loaded);
  const drop = useTrackDrop(deck);
  const name = deck.toUpperCase();
  // Kurzhilfe am Schild erklärt, was Ziehen und Mausrad in der Wellenform tun (9.3).
  const tip = useTip({ name: t('stage.tip', { deck: name }), value: t('stage.tipValue'), keys: [t('key.wheel')] });
  return (
    <div className="stage__lane" data-pos={position} data-over={drop.over ? 'true' : undefined} {...drop.handlers}>
      <div className="stage__tag" {...tip}>
        <DeckPlate deck={deck} />
        {loading ? (
          <span className="stage__title dw-truncate">
            {t('deck.loading')} <b>{loading.title}</b>
          </span>
        ) : loaded && track ? (
          <span className="stage__title dw-truncate">
            <b>{track.title}</b>
            {track.artist ? ` · ${track.artist}` : ''}
          </span>
        ) : (
          <span className="stage__title stage__title--empty">{t('stage.empty', { deck: name })}</span>
        )}
      </div>
      {drop.over && <div className="stage__drop dw-label">{t('deck.dropHere', { deck: name })}</div>}
    </div>
  );
}

export function WaveStage() {
  const t = useT();
  const zoom = useUi((s) => s.zoomBeats);
  const [mode, setMode] = useState<'webgl2' | 'canvas2d'>(() => forcedRenderer(window.location.search) ?? 'webgl2');
  const wrap = useRef<HTMLDivElement | null>(null);
  const canvas = useRef<HTMLCanvasElement | null>(null);
  const renderer = useRef<WaveRenderer | null>(null);
  const size = useRef({ w: 0, h: 0, dpr: 1 });
  const shown = useRef<Record<string, DeckAssets | null>>({});
  const colors = useRef<{ theme: string; value: StageColors } | null>(null);
  const lastKey = useRef('');
  const tested = useRef(false);
  const drag = useRef<{ deck: DeckId; x: number; pos: number; playing: boolean; secPerPx: number } | null>(null);

  // Zeichner anlegen: WebGL 2, sonst Canvas 2D.
  useEffect(() => {
    const c = canvas.current;
    if (!c) return;
    let r: WaveRenderer;
    try {
      r = mode === 'webgl2' ? new WebGLWaveRenderer(c, webglPresent(window.location.search, navigator.userAgent)) : new Canvas2DWaveRenderer(c);
    } catch (err) {
      if (mode === 'webgl2') {
        console.warn('WebGL 2 nicht nutzbar, Rückfall auf Canvas 2D:', err);
        setMode('canvas2d');
      }
      return;
    }
    renderer.current = r;
    setUi({ rendererInfo: r.kind === 'webgl2' ? `WebGL 2 · ${r.info}` : r.info });
    shown.current = {};
    lastKey.current = '';
    const lost = (e: Event) => {
      e.preventDefault();
      setMode('canvas2d');
    };
    c.addEventListener('webglcontextlost', lost);
    return () => {
      c.removeEventListener('webglcontextlost', lost);
      r.dispose();
      renderer.current = null;
    };
  }, [mode]);

  // Größe der Zeichenfläche folgt dem Element (HiDPI-fähig).
  useEffect(() => {
    const el = wrap.current;
    if (!el) return;
    const ro = new ResizeObserver(() => {
      const dpr = window.devicePixelRatio || 1;
      const rect = el.getBoundingClientRect();
      size.current = { w: Math.max(2, Math.round(rect.width * dpr)), h: Math.max(2, Math.round(rect.height * dpr)), dpr };
      lastKey.current = '';
    });
    ro.observe(el);
    return () => ro.disconnect();
  }, []);

  // Mausrad zoomt (nicht-passiv, damit nichts rollt).
  useEffect(() => {
    const el = wrap.current;
    if (!el) return;
    const onWheel = (e: WheelEvent) => {
      e.preventDefault();
      if (e.deltaY !== 0) stepZoom(e.deltaY > 0 ? 1 : -1);
    };
    el.addEventListener('wheel', onWheel, { passive: false });
    return () => el.removeEventListener('wheel', onWheel);
  }, []);

  useFrame((snap) => {
    const r = renderer.current;
    const c = canvas.current;
    const { w, h, dpr } = size.current;
    if (!r || !c || w < 4 || h < 4) return;
    if (c.width !== w || c.height !== h) {
      c.width = w;
      c.height = h;
    }
    const theme = document.documentElement.dataset.theme ?? '';
    if (!colors.current || colors.current.theme !== theme || cssVar('--bg-0') === '') {
      colors.current = { theme, value: readStageColors() };
      lastKey.current = '';
    }
    // Neue Tracks an den Zeichner geben.
    for (const deck of LANES) {
      const a = getDeckAssets(deck);
      const loaded = snap.decks[deckIndex(deck)].loaded;
      const want = loaded ? a : null;
      if (shown.current[deck] !== want) {
        shown.current[deck] = want;
        r.setTrack(deck, want ? want.waveform : null);
        lastKey.current = '';
      }
    }
    const ui = getUi();
    let key = `${w}x${h}|${ui.zoomBeats}|${snap.clock_bpm.toFixed(3)}`;
    for (const deck of LANES) {
      const d = snap.decks[deckIndex(deck)];
      key += `|${d.loaded ? 1 : 0}${d.playing ? 1 : 0}:${d.pos.toFixed(4)}:${d.tempo_offset.toFixed(5)}:${d.eq_db.join(',')}`;
    }
    if (key === lastKey.current) return;
    lastKey.current = key;
    const laneInput = (deck: DeckId): LaneInput => {
      const a = shown.current[deck] ?? null;
      return { deck, state: snap.decks[deckIndex(deck)], waveform: a?.waveform ?? null, grid: a?.analysis.grid ?? null };
    };
    const scene = buildScene({
      width: w,
      height: h,
      dpr,
      railH: remPx() * RAIL_REM * dpr,
      unit: (remPx() / 14) * dpr,
      zoomBeats: ui.zoomBeats,
      clockBpm: snap.clock_bpm,
      lanes: [laneInput(LANES[0]), laneInput(LANES[1])],
      colors: colors.current.value,
    });
    r.draw(scene, colors.current.value);
    // Einmal nach dem ersten Bild prüfen, ob WebGL wirklich zeichnet; sonst auf Canvas 2D wechseln.
    if (r instanceof WebGLWaveRenderer && !tested.current) {
      tested.current = true;
      const test = r.selfTest(scene);
      setUi({ rendererInfo: test.ok ? `WebGL 2 · ${r.info}` : `WebGL 2 zeichnet nicht (${test.detail}) – Canvas 2D` });
      if (!test.ok) {
        console.warn('WebGL 2 zeichnet nicht, Rückfall auf Canvas 2D:', test.detail);
        setMode('canvas2d');
      }
    }
  });

  // Ziehen in der Wellenform: laufendes Deck anschieben/abbremsen, stehendes Deck verschieben (DECK-07).
  const onPointerDown = (e: React.PointerEvent<HTMLDivElement>) => {
    if (e.button !== 0 || !wrap.current) return;
    const rect = wrap.current.getBoundingClientRect();
    const deck = e.clientY - rect.top < rect.height / 2 ? LANES[0] : LANES[1];
    const snap = latestSnapshot();
    const d = snap.decks[deckIndex(deck)];
    if (!d.loaded) return;
    focusDeck(deck);
    const refBpm = snap.clock_bpm > 0 ? snap.clock_bpm : d.bpm || FALLBACK_BPM;
    e.currentTarget.setPointerCapture(e.pointerId);
    drag.current = {
      deck,
      x: e.clientX,
      pos: d.pos,
      playing: d.playing,
      secPerPx: secondsPerPx(rect.width, getUi().zoomBeats, refBpm) * (1 + d.tempo_offset),
    };
  };
  const onPointerMove = (e: React.PointerEvent<HTMLDivElement>) => {
    const g = drag.current;
    if (!g) return;
    const dx = e.clientX - g.x;
    if (g.playing) {
      // Nach links ziehen = Track vorziehen = schneller.
      send({ type: 'bend', deck: g.deck, amount: clamp(-dx / (remPx() * 70), -0.12, 0.12) });
    } else {
      send({ type: 'seek', deck: g.deck, pos: Math.max(0, g.pos - dx * g.secPerPx) });
    }
  };
  const onPointerUp = () => {
    const g = drag.current;
    if (!g) return;
    drag.current = null;
    if (g.playing) bend(g.deck, 0);
  };

  return (
    <div
      className="stage"
      ref={wrap}
      data-renderer={mode}
      onPointerDown={onPointerDown}
      onPointerMove={onPointerMove}
      onPointerUp={onPointerUp}
      onPointerCancel={onPointerUp}
    >
      <canvas key={mode} ref={canvas} className="stage__canvas" role="img" aria-label={t('stage.aria')} />
      <LaneOverlay deck={LANES[0]} position="top" />
      <LaneOverlay deck={LANES[1]} position="bottom" />
      <div className="stage__zoom" onPointerDown={(e) => e.stopPropagation()}>
        <KeyBtn size="sm" onPress={() => stepZoom(-1)} disabled={zoom <= ZOOM_STEPS[0]} ariaLabel={t('stage.zoomIn')} tip={{ name: t('stage.zoomIn'), keys: [t('key.wheel')] }}>
          <Icon name="plus" size="0.8em" />
        </KeyBtn>
        <span className="stage__zoomval dw-num">{t(zoom === 1 ? 'stage.beat' : 'stage.beats', { n: zoom })}</span>
        <KeyBtn size="sm" onPress={() => stepZoom(1)} disabled={zoom >= ZOOM_STEPS[ZOOM_STEPS.length - 1]} ariaLabel={t('stage.zoomOut')} tip={{ name: t('stage.zoomOut'), keys: [t('key.wheel')] }}>
          <Icon name="minus" size="0.8em" />
        </KeyBtn>
      </div>
    </div>
  );
}
