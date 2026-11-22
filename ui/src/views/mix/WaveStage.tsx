// Parallele Wellenformen (8.2, VIS-02–04): Deck A oben, Deck B unten, dazwischen die Beat-Schiene.
// Beide Bahnen wachsen von der Schiene weg – liegen die Beats übereinander, ist das Bild spiegelgleich.
import { useEffect, useRef, useState } from 'react';
import { getDeckAssets, type DeckAssets } from '../../app/assets';
import { bend, cancelTransition, chooseTransition, focusDeck, loadSavedTransitions, openEditor, peekAfter, peekBy, peekReset, peekStep, send, startTransition, TRANSITION_BARS, TRANSITION_LEAD, transitionRoles } from '../../app/actions';
import { useTrackDrop } from '../../app/dnd';
import { forcedRenderer, webglPresent } from '../../app/env';
import { latestSnapshot, useFrame, useSnap } from '../../app/frame';
import { getUi, setUi, useUi } from '../../app/store';
import { deckIndex, type DeckId } from '../../core/types';
import { DeckPlate, Icon, KeyBtn, LiveText, useTip } from '../../design/components';
import { cssVar, remPx } from '../../design/tokens';
import { useLang, useT } from '../../i18n';
import { beatAt, secAt } from '../../lib/grid';
import type { BeatGrid } from '../../core/types';
import { shortcutKeys } from '../../app/keymap';
import { clamp } from '../../lib/curves';
import { readStageColors } from './stageColors';
import { Canvas2DWaveRenderer } from './wave/canvas2d';
import type { WaveRenderer } from './wave/renderer';
import { buildScene, secondsPerPx, FALLBACK_BPM, type LaneInput, type StageColors } from './wave/scene';
import { WebGLWaveRenderer } from './wave/webgl';

export const ZOOM_STEPS = [1, 2, 4, 8, 16, 32, 64, 128];
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

/** Vorausblick (VIS-13): Tasten zum Verschieben der Ansicht und, solange verschoben, der Weg zurück. */
function PeekControls() {
  const t = useT();
  const lang = useLang();
  const peek = useUi((s) => s.peekBeats);
  const bpm = useSnap((s) => s.clock_bpm);
  const sec = (Math.abs(peek) * 60) / (bpm > 0 ? bpm : FALLBACK_BPM);
  const amount = `${peek > 0 ? '+' : '−'}${t(Math.abs(peek) === 1 ? 'stage.beat' : 'stage.beats', { n: Math.abs(peek) })} · ${Math.floor(sec / 60)}:${String(Math.floor(sec % 60)).padStart(2, '0')}`;
  return (
    <div className="stage__peek" data-active={peek !== 0 ? 'true' : undefined} onPointerDown={(e) => e.stopPropagation()}>
      <KeyBtn size="sm" onPress={() => peekStep(-1, false)} ariaLabel={t('stage.peekBack')} tip={{ name: t('stage.peekBack'), value: t('stage.peekHint'), keys: shortcutKeys('stage.peek_back', undefined, lang) }}>
        <Icon name="slower" size="0.8em" />
      </KeyBtn>
      {peek !== 0 ? (
        <KeyBtn size="sm" mode="toggle" active accent="var(--status-warn)" onPress={peekReset} ariaLabel={t('stage.peekReset')} tip={{ name: t('stage.peekReset'), keys: shortcutKeys('stage.peek_reset', undefined, lang) }} sub={amount}>
          {t('stage.peekOn')}
        </KeyBtn>
      ) : (
        <span className="stage__zoomval dw-num">{t('stage.peekOff')}</span>
      )}
      <KeyBtn size="sm" onPress={() => peekStep(1, false)} ariaLabel={t('stage.peekFwd')} tip={{ name: t('stage.peekFwd'), value: t('stage.peekHint'), keys: shortcutKeys('stage.peek_fwd', undefined, lang) }}>
        <Icon name="faster" size="0.8em" />
      </KeyBtn>
    </div>
  );
}

/** Was ein Klick in die Wellenform trifft. */
export type StageTarget =
  /** Eine Stelle auf der gemeinsamen Zeitachse der laufenden Decks: so viele Clock-Beats ab jetzt. */
  | { kind: 'time'; beats: number }
  /** Eine Stelle in einem stehenden Deck: Position in Sekunden, auf den nächsten Beat gerückt. */
  | { kind: 'deck'; deck: DeckId; pos: number };

/**
 * Rechnet einen Klick in die Wellenform um. `x` und `width` in denselben Pixeln; die Bildmitte ist
 * die Abspielstelle plus Vorausblick. Ein laufendes Deck liegt auf der Zeitachse der Clock; ein
 * stehendes bleibt im Vorausblick an seiner Stelle, solange ein anderes läuft (VIS-13).
 */
export function stageTarget(args: {
  x: number;
  width: number;
  zoomBeats: number;
  peekBeats: number;
  clockBpm: number;
  deck: DeckId;
  state: { playing: boolean; pos: number; tempo_offset: number; duration: number };
  anyPlaying: boolean;
  grid: BeatGrid | null;
}): StageTarget {
  const offset = ((args.x - args.width / 2) / Math.max(1, args.width)) * args.zoomBeats;
  if (args.state.playing) return { kind: 'time', beats: args.peekBeats + offset };
  // Stehendes Deck: Echtzeit-Abstand zur Bildmitte mal Abspielrate ergibt die Strecke im Track.
  const shown = args.anyPlaying ? offset : args.peekBeats + offset;
  const raw = args.state.pos + ((shown * 60) / (args.clockBpm > 0 ? args.clockBpm : FALLBACK_BPM)) * (1 + args.state.tempo_offset);
  const snapped = args.grid ? secAt(args.grid, Math.round(beatAt(args.grid, raw))) : raw;
  return { kind: 'deck', deck: args.deck, pos: clamp(snapped, 0, args.state.duration) };
}

/** Menü nach einem Klick in die Wellenform: Übergang hier enden oder beginnen lassen, Ansicht oder Deck hierher. */
function StageMenu({ target, left, top, onClose }: { target: StageTarget; left: number; top: number; onClose: () => void }) {
  const t = useT();
  const bars = useUi((s) => s.transitionBars);
  const name = useUi((s) => s.transitionName);
  const running = useUi((s) => s.transition !== null);
  const loadedA = useSnap((s) => s.decks[0].loaded);
  const loadedB = useSnap((s) => s.decks[1].loaded);
  const playingA = useSnap((s) => s.decks[0].playing);
  const playingB = useSnap((s) => s.decks[1].playing);
  const master = useSnap((s) => s.master);
  const bpm = useSnap((s) => s.clock_bpm);
  const roles = transitionRoles({ loaded: loadedA, playing: playingA }, { loaded: loadedB, playing: playingB }, master);
  const choose = (run: () => void) => () => {
    onClose();
    run();
  };
  const length = bars * 4;
  const template = name === 'fade' ? t('trans.fade') : name === 'bass_swap' ? t('trans.bassSwap') : name;
  let body;
  if (target.kind === 'deck') {
    body = (
      <KeyBtn size="sm" onPress={choose(() => send({ type: 'seek', deck: target.deck, pos: target.pos }))}>
        {t('menu.deckHere', { deck: target.deck.toUpperCase() })}
      </KeyBtn>
    );
  } else {
    const beats = target.beats;
    const sec = (Math.abs(beats) * 60) / (bpm > 0 ? bpm : FALLBACK_BPM);
    const dirs = roles ? { from: roles.from.toUpperCase(), to: roles.to.toUpperCase() } : null;
    const endOk = dirs !== null && !running && beats >= length + TRANSITION_LEAD;
    const startOk = dirs !== null && !running && beats >= TRANSITION_LEAD;
    body = (
      <>
        <span className="stage__menuhead dw-num">
          {t('menu.at', { n: Math.round(beats), time: `${beats < 0 ? '−' : ''}${Math.floor(sec / 60)}:${String(Math.floor(sec % 60)).padStart(2, '0')}` })}
          {' · '}
          {template}, {t('trans.bars', { n: bars })}
        </span>
        <KeyBtn size="sm" disabled={!endOk} onPress={choose(() => void startTransition({ type: 'end_in', beats }))} tip={{ name: t('menu.endHere'), value: endOk ? t('trans.endHint') : dirs === null ? t('trans.needDecks') : t('trans.tooLate', { n: length + TRANSITION_LEAD }) }}>
          {dirs ? t('menu.endHereDir', dirs) : t('menu.endHere')}
        </KeyBtn>
        <KeyBtn size="sm" disabled={!startOk} onPress={choose(() => void startTransition({ type: 'end_in', beats: beats + length }))} tip={{ name: t('menu.startHere'), value: startOk ? t('menu.startHint') : dirs === null ? t('trans.needDecks') : t('menu.startTooLate', { n: TRANSITION_LEAD }) }}>
          {dirs ? t('menu.startHereDir', dirs) : t('menu.startHere')}
        </KeyBtn>
        <KeyBtn size="sm" onPress={choose(() => setUi({ peekBeats: peekAfter(0, beats) }))}>
          {t('menu.viewHere')}
        </KeyBtn>
      </>
    );
  }
  return (
    <div className="stage__menu" role="menu" style={{ left, top }} onPointerDown={(e) => e.stopPropagation()}>
      {body}
      <KeyBtn size="sm" onPress={onClose}>
        {t('menu.close')}
      </KeyBtn>
    </div>
  );
}

const TRANSITION_NAMES = ['bass_swap', 'fade'] as const;

/** Stand eines Übergangs für die Anzeige: vor dem Start die Beats bis T0, danach der laufende Takt. */
export function transitionProgress(clockBeats: number, t0: number, bars: number): { phase: 'waiting'; beats: number } | { phase: 'running'; bar: number } | { phase: 'done' } {
  const t = clockBeats - t0;
  if (t < 0) return { phase: 'waiting', beats: Math.ceil(-t - 1e-9) };
  if (t >= bars * 4) return { phase: 'done' };
  return { phase: 'running', bar: Math.floor(t / 4) + 1 };
}

/**
 * Zone des Übergangs für die Wellenform, in Clock-Beats ab jetzt: der eingeplante Übergang, sonst –
 * bei verschobenem Vorausblick – die Vorschau eines Übergangs, der an der Linie in der Bildmitte endet.
 */
export function transitionZone(
  run: { t0_beat: number; bars: number } | null,
  clockBeats: number,
  peekBeats: number,
  bars: number,
  possible: boolean,
): { startBeats: number; endBeats: number; preview: boolean } | null {
  if (run) return { startBeats: run.t0_beat - clockBeats, endBeats: run.t0_beat - clockBeats + run.bars * 4, preview: false };
  if (peekBeats > 0 && possible) return { startBeats: peekBeats - bars * 4, endBeats: peekBeats, preview: true };
  return null;
}

/**
 * Übergang auf Knopfdruck (TRANS-05): Vorlage und Länge wählen, starten. Der Kern beginnt an der
 * nächsten Grenze im Vier-Takt-Raster und fährt die Regler; hier steht, wie weit er ist.
 */
function TransitionBar() {
  const t = useT();
  const run = useUi((s) => s.transition);
  const name = useUi((s) => s.transitionName);
  const bars = useUi((s) => s.transitionBars);
  const loadedA = useSnap((s) => s.decks[0].loaded);
  const loadedB = useSnap((s) => s.decks[1].loaded);
  const playingA = useSnap((s) => s.decks[0].playing);
  const playingB = useSnap((s) => s.decks[1].playing);
  const saved = useUi((s) => s.savedTransitions);
  const own = saved.some((s) => s.name === name);
  useEffect(() => {
    void loadSavedTransitions();
  }, []);
  const over = useSnap((s) => run !== null && s.clock_beats > run.end_beat);
  useEffect(() => {
    if (over) setUi({ transition: null });
  }, [over]);
  const master = useSnap((s) => s.master);
  const peek = useUi((s) => s.peekBeats);
  const roles = transitionRoles({ loaded: loadedA, playing: playingA }, { loaded: loadedB, playing: playingB }, master);
  // Mit Vorausblick endet der Übergang an der Linie in der Bildmitte; dafür muss sie weit genug voraus liegen.
  const tooLate = peek > 0 && peek < bars * 4 + TRANSITION_LEAD;
  const stop = (e: { stopPropagation: () => void }) => e.stopPropagation();

  if (run) {
    const label = t('trans.fromTo', { from: run.from.toUpperCase(), to: run.to.toUpperCase() });
    return (
      <div className="stage__trans" data-active="true" onPointerDown={stop}>
        <span className="stage__transstate dw-num" role="status">
          {label}
          {' · '}
          <LiveText
            read={(s) => {
              const p = transitionProgress(s.clock_beats, run.t0_beat, run.bars);
              if (p.phase === 'waiting') return t(p.beats === 1 ? 'trans.inBeat' : 'trans.inBeats', { n: p.beats });
              if (p.phase === 'running') return t('trans.bar', { n: p.bar, of: run.bars });
              return t('trans.finishing');
            }}
          />
        </span>
        <KeyBtn size="sm" onPress={() => void cancelTransition()} tip={{ name: t('trans.cancel'), value: t('trans.cancelHint') }} ariaLabel={t('trans.cancel')}>
          {t('trans.cancelShort')}
        </KeyBtn>
      </div>
    );
  }
  return (
    <div className="stage__trans" onPointerDown={stop}>
      <select className="stage__select" value={name} aria-label={t('trans.template')} onChange={(e) => chooseTransition(e.target.value)}>
        {TRANSITION_NAMES.map((n) => (
          <option key={n} value={n}>
            {t(n === 'bass_swap' ? 'trans.bassSwap' : 'trans.fade')}
          </option>
        ))}
        {saved.map((s) => (
          <option key={s.name} value={s.name}>
            {s.name}
          </option>
        ))}
      </select>
      {/* Ein eigener Übergang bringt seine Länge mit. */}
      <select className="stage__select" value={bars} disabled={own} aria-label={t('trans.length')} onChange={(e) => setUi({ transitionBars: Number(e.target.value) })}>
        {[...new Set<number>([...TRANSITION_BARS, bars])].sort((a, b) => a - b).map((b) => (
          <option key={b} value={b}>
            {t('trans.bars', { n: b })}
          </option>
        ))}
      </select>
      <KeyBtn
        size="sm"
        disabled={!roles || tooLate}
        onPress={() => void startTransition()}
        tip={{
          name: t('trans.start'),
          value: !roles ? t('trans.needDecks') : tooLate ? t('trans.tooLate', { n: bars * 4 + TRANSITION_LEAD }) : peek > 0 ? t('trans.endHint') : t('trans.startHint'),
        }}
        ariaLabel={t('trans.start')}
      >
        {roles ? t(peek > 0 ? 'trans.fromToEnd' : 'trans.fromTo', { from: roles.from.toUpperCase(), to: roles.to.toUpperCase() }) : t('trans.startShort')}
      </KeyBtn>
      <KeyBtn size="sm" onPress={() => void openEditor()} tip={{ name: t('ed.open'), value: t('ed.openHint') }} ariaLabel={t('ed.open')}>
        {t('ed.openShort')}
      </KeyBtn>
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
  const drag = useRef<{ deck: DeckId; x: number; y: number; pos: number; mode: 'bend' | 'seek' | 'pan'; moved: boolean; secPerPx: number; pxPerBeat: number; peek: number } | null>(null);
  const [menu, setMenu] = useState<{ target: StageTarget; left: number; top: number } | null>(null);
  useEffect(() => {
    if (!menu) return;
    const onKey = (e: KeyboardEvent) => {
      if (e.key === 'Escape') setMenu(null);
    };
    window.addEventListener('keydown', onKey);
    return () => window.removeEventListener('keydown', onKey);
  }, [menu]);

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

  // Mausrad zoomt; mit Shift oder waagerecht verschiebt es die Ansicht (Vorausblick, VIS-13).
  // Nicht-passiv, damit nichts rollt.
  useEffect(() => {
    const el = wrap.current;
    if (!el) return;
    const onWheel = (e: WheelEvent) => {
      e.preventDefault();
      const sideways = Math.abs(e.deltaX) > Math.abs(e.deltaY);
      if (e.shiftKey || sideways) {
        // Manche Systeme melden Shift + Rad als waagerechte Bewegung.
        const delta = sideways ? e.deltaX : e.deltaY;
        // Schritt: ein Viertel des sichtbaren Ausschnitts, mindestens ein Beat.
        if (delta !== 0) peekBy(Math.sign(delta) * Math.max(1, getUi().zoomBeats / 4));
        return;
      }
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
    const zone = transitionZone(
      ui.transition,
      snap.clock_beats,
      ui.peekBeats,
      ui.transitionBars,
      transitionRoles(snap.decks[0], snap.decks[1], snap.master) !== null,
    );
    let key = `${w}x${h}|${ui.zoomBeats}|${ui.peekBeats}|${snap.clock_bpm.toFixed(3)}|${zone ? `${zone.startBeats.toFixed(3)}:${zone.endBeats.toFixed(3)}:${zone.preview}` : ''}`;
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
      peekBeats: ui.peekBeats,
      clockBpm: snap.clock_bpm,
      lanes: [laneInput(LANES[0]), laneInput(LANES[1])],
      colors: colors.current.value,
      zone,
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

  // Ziehen in der Wellenform (DECK-07, VIS-13): Ein stehendes Deck wird verschoben. Ein laufendes
  // wird angeschoben oder gebremst – außer die Ansicht ist schon verschoben oder Shift ist gedrückt:
  // Dann zieht man nur die Ansicht (Vorausblick), die Wiedergabe bleibt unberührt. Ein Klick ohne
  // Ziehen öffnet das Menü für diese Stelle.
  const onPointerDown = (e: React.PointerEvent<HTMLDivElement>) => {
    if (menu) setMenu(null);
    if (e.button !== 0 || !wrap.current) return;
    const rect = wrap.current.getBoundingClientRect();
    const deck = e.clientY - rect.top < rect.height / 2 ? LANES[0] : LANES[1];
    const snap = latestSnapshot();
    const d = snap.decks[deckIndex(deck)];
    if (!d.loaded) return;
    focusDeck(deck);
    const ui = getUi();
    const refBpm = snap.clock_bpm > 0 ? snap.clock_bpm : d.bpm || FALLBACK_BPM;
    e.currentTarget.setPointerCapture(e.pointerId);
    drag.current = {
      deck,
      x: e.clientX,
      y: e.clientY,
      pos: d.pos,
      mode: !d.playing ? 'seek' : e.shiftKey || ui.peekBeats !== 0 ? 'pan' : 'bend',
      moved: false,
      secPerPx: secondsPerPx(rect.width, ui.zoomBeats, refBpm) * (1 + d.tempo_offset),
      pxPerBeat: rect.width / ui.zoomBeats,
      peek: ui.peekBeats,
    };
  };
  const onPointerMove = (e: React.PointerEvent<HTMLDivElement>) => {
    const g = drag.current;
    if (!g) return;
    const dx = e.clientX - g.x;
    // Erst ab ein paar Pixeln gilt es als Ziehen; darunter bleibt es ein Klick.
    if (!g.moved && Math.abs(dx) < 4) return;
    g.moved = true;
    if (g.mode === 'bend') {
      // Nach links ziehen = Track vorziehen = schneller.
      send({ type: 'bend', deck: g.deck, amount: clamp(-dx / (remPx() * 70), -0.12, 0.12) });
    } else if (g.mode === 'pan') {
      // Nach links ziehen = weiter nach vorn sehen.
      setUi({ peekBeats: peekAfter(g.peek, -dx / g.pxPerBeat) });
    } else {
      send({ type: 'seek', deck: g.deck, pos: Math.max(0, g.pos - dx * g.secPerPx) });
    }
  };
  const onPointerUp = () => {
    const g = drag.current;
    if (!g) return;
    drag.current = null;
    if (g.mode === 'bend' && g.moved) bend(g.deck, 0);
    if (g.moved || !wrap.current) return;
    // Klick: Menü für diese Stelle.
    const rect = wrap.current.getBoundingClientRect();
    const snap = latestSnapshot();
    const ui = getUi();
    const d = snap.decks[deckIndex(g.deck)];
    const target = stageTarget({
      x: g.x - rect.left,
      width: rect.width,
      zoomBeats: ui.zoomBeats,
      peekBeats: ui.peekBeats,
      clockBpm: snap.clock_bpm,
      deck: g.deck,
      state: d,
      anyPlaying: LANES.some((l) => snap.decks[deckIndex(l)].loaded && snap.decks[deckIndex(l)].playing),
      grid: getDeckAssets(g.deck)?.analysis.grid ?? null,
    });
    // Im Bild halten: nach links und oben ausweichen, wenn rechts oder unten kein Platz ist.
    const left = Math.min(g.x - rect.left, rect.width - remPx() * 17);
    const top = Math.min(g.y - rect.top, rect.height - remPx() * 11);
    setMenu({ target, left: Math.max(0, left), top: Math.max(0, top) });
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
      {menu && <StageMenu target={menu.target} left={menu.left} top={menu.top} onClose={() => setMenu(null)} />}
      <div className="stage__bottom">
        <TransitionBar />
        <PeekControls />
      </div>
    </div>
  );
}
