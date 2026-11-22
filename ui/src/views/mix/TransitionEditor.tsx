// Editor für Übergänge (TRANS-04): je Regler eine Spur mit den Kurven des gehenden und des kommenden
// Decks. Punkte setzen (Klick), ziehen, löschen (Doppelklick); waagerecht rasten sie auf Beats.
import { useEffect, useLayoutEffect, useRef, useState } from 'react';
import { closeEditor, deleteSavedTransition, previewEditor, saveEditor, stopPreview } from '../../app/actions';
import { useFrame, useSnap } from '../../app/frame';
import { setUi, useUi } from '../../app/store';
import type { Transition, TransitionPoint } from '../../core/types';
import { KeyBtn } from '../../design/components';
import { useT, type MsgKey } from '../../i18n';
import { PARAMS, addPoint, bassClashes, findLane, fromHeight, movePoint, removePoint, rescale, setCurve, snapBeat, timeRange, toHeight, valueAt, valueText, type Param, type Side } from '../../lib/transition';

const SIDES: Side[] = ['out', 'in'];
const LENGTHS = [4, 8, 16, 32, 64];
const PARAM_LABEL: Record<Param, MsgKey> = { fader: 'ed.fader', eq_low: 'eq.low', eq_mid: 'eq.mid', eq_high: 'eq.high', filter: 'ed.filter' };
const CURVES: NonNullable<TransitionPoint['curve']>[] = ['linear', 'smooth', 'step'];
/** Radius eines Punkts und Fangweite der Maus in Pixeln. */
const DOT = 5;
const GRAB = 9;
/** Rand der Spur oben und unten in Pixeln, damit Punkte am Anschlag ganz sichtbar bleiben. */
const PAD = 7;

type Sel = { side: Side; param: Param; index: number };

/** Breite und Höhe eines Elements in Pixeln, mitgeführt bei Größenänderungen. */
function useSize<T extends HTMLElement>(): [React.RefObject<T | null>, { w: number; h: number }] {
  const ref = useRef<T | null>(null);
  const [size, setSize] = useState({ w: 0, h: 0 });
  useLayoutEffect(() => {
    const el = ref.current;
    if (!el) return;
    const read = () => setSize({ w: el.clientWidth, h: el.clientHeight });
    read();
    const ro = new ResizeObserver(read);
    ro.observe(el);
    return () => ro.disconnect();
  }, []);
  return [ref, size];
}

function LaneRow({ draft, param, active, selected, onChange, onSelect, onActive }: {
  draft: Transition;
  param: Param;
  active: Side;
  selected: Sel | null;
  onChange: (t: Transition) => void;
  onSelect: (s: Sel | null) => void;
  onActive: (s: Side) => void;
}) {
  const t = useT();
  const [ref, { w, h }] = useSize<HTMLDivElement>();
  const drag = useRef<Sel | null>(null);
  const [lo, hi] = timeRange(draft);
  const x = (beat: number) => ((beat - lo) / (hi - lo)) * w;
  const y = (value: number | null) => PAD + (1 - toHeight(param, value)) * Math.max(1, h - 2 * PAD);
  const beatAtX = (px: number) => lo + (px / Math.max(1, w)) * (hi - lo);
  const valueAtY = (py: number) => fromHeight(param, 1 - (py - PAD) / Math.max(1, h - 2 * PAD));

  const local = (e: React.PointerEvent) => {
    const rect = ref.current!.getBoundingClientRect();
    return { px: e.clientX - rect.left, py: e.clientY - rect.top };
  };
  /** Punkt unter der Maus: zuerst auf der aktiven Seite suchen. */
  const hit = (px: number, py: number): Sel | null => {
    for (const side of [active, ...SIDES.filter((s) => s !== active)]) {
      const lane = findLane(draft, side, param);
      const index = lane ? lane.points.findIndex((p) => Math.hypot(x(p.beat) - px, y(p.value) - py) <= GRAB) : -1;
      if (index >= 0) return { side, param, index };
    }
    return null;
  };
  const onPointerDown = (e: React.PointerEvent<HTMLDivElement>) => {
    if (e.button !== 0 || w === 0) return;
    const { px, py } = local(e);
    let sel = hit(px, py);
    if (!sel) {
      // Leere Stelle: neuen Punkt auf der aktiven Seite setzen und gleich weiterziehen.
      const beat = snapBeat(draft, beatAtX(px));
      const next = addPoint(draft, active, param, beat, valueAtY(py));
      onChange(next);
      sel = { side: active, param, index: findLane(next, active, param)!.points.findIndex((p) => p.beat === beat) };
    } else if (sel.side !== active) {
      onActive(sel.side);
    }
    onSelect(sel);
    drag.current = sel;
    e.currentTarget.setPointerCapture(e.pointerId);
  };
  const onPointerMove = (e: React.PointerEvent<HTMLDivElement>) => {
    const g = drag.current;
    if (!g) return;
    const { px, py } = local(e);
    // Shift: feines Raster (Sechzehntel).
    const step = e.shiftKey ? 0.25 : 1;
    onChange(movePoint(draft, g.side, g.param, g.index, snapBeat(draft, beatAtX(px), step), valueAtY(py), step));
  };
  const onPointerUp = () => {
    drag.current = null;
  };
  const onDoubleClick = (e: React.MouseEvent<HTMLDivElement>) => {
    const rect = ref.current!.getBoundingClientRect();
    const sel = hit(e.clientX - rect.left, e.clientY - rect.top);
    if (!sel) return;
    onChange(removePoint(draft, sel.side, sel.param, sel.index));
    onSelect(null);
  };

  // Kurven: je Viertel-Beat ein Stützpunkt der Zeichnung, ab dem ersten Punkt der Spur.
  const path = (side: Side): string => {
    const lane = findLane(draft, side, param);
    if (!lane || w === 0) return '';
    let d = '';
    for (let b = lane.points[0].beat; b <= hi + 1e-9; b += 0.25) {
      const v = valueAt(lane, b);
      if (v === undefined) continue;
      d += `${d === '' ? 'M' : 'L'}${x(b).toFixed(1)} ${y(v).toFixed(1)}`;
    }
    return d;
  };
  const bars = [];
  for (let b = Math.ceil(lo / 4) * 4; b <= hi; b += 4) bars.push(b);
  const clashes = param === 'eq_low' ? bassClashes(draft) : [];

  return (
    <div className="ted__row">
      <div className="ted__label">
        <span className="dw-label">{t(PARAM_LABEL[param])}</span>
        <span className="ted__sides">
          {SIDES.map((side) => (
            <button key={side} type="button" className="ted__side" data-side={side} data-active={active === side ? 'true' : undefined} onClick={() => onActive(side)} title={t(side === 'out' ? 'ed.outLong' : 'ed.inLong')}>
              {t(side === 'out' ? 'ed.out' : 'ed.in')}
            </button>
          ))}
        </span>
      </div>
      <div className="ted__lane" ref={ref} onPointerDown={onPointerDown} onPointerMove={onPointerMove} onPointerUp={onPointerUp} onPointerCancel={onPointerUp} onDoubleClick={onDoubleClick}>
        <svg width={w} height={h} aria-hidden="true">
          {clashes.map(([from, to]) => (
            <rect key={from} className="ted__clash" x={x(from)} y={0} width={Math.max(1, x(to) - x(from))} height={h} />
          ))}
          {bars.map((b) => (
            <line key={b} className="ted__grid" data-strong={b % 16 === 0 ? 'true' : undefined} data-zero={b === 0 ? 'true' : undefined} x1={x(b)} x2={x(b)} y1={0} y2={h} />
          ))}
          {SIDES.map((side) => (
            <path key={side} className="ted__curve" data-side={side} data-active={active === side ? 'true' : undefined} d={path(side)} />
          ))}
          {SIDES.map((side) =>
            (findLane(draft, side, param)?.points ?? []).map((p, index) => (
              <circle
                key={`${side}${index}`}
                className="ted__dot"
                data-side={side}
                data-selected={selected && selected.side === side && selected.param === param && selected.index === index ? 'true' : undefined}
                cx={x(p.beat)}
                cy={y(p.value)}
                r={DOT}
              />
            )),
          )}
        </svg>
      </div>
    </div>
  );
}

/**
 * Marke „ab hier wird vorgehört" und Abspielmarke über allen Spuren. Die Abspielmarke läuft in der
 * Bildschleife mit der Clock, nicht über React.
 */
function PreviewMarks({ lo, hi, from }: { lo: number; hi: number; from: number }) {
  const run = useUi((s) => s.preview);
  const head = useRef<HTMLDivElement | null>(null);
  useFrame((s) => {
    const el = head.current;
    if (!el) return;
    const beat = run ? s.clock_beats - run.t0_beat : NaN;
    const show = beat >= lo && beat <= hi && (s.decks[0].playing || s.decks[1].playing);
    el.style.visibility = show ? 'visible' : 'hidden';
    if (show) el.style.left = `${((beat - lo) / (hi - lo)) * 100}%`;
  });
  return (
    <div className="ted__over" aria-hidden="true">
      <div className="ted__from" style={{ left: `${((from - lo) / (hi - lo)) * 100}%` }} />
      <div className="ted__head" ref={head} />
    </div>
  );
}

/** Editor für den Übergang im Entwurf (`ui.editor`). */
export function TransitionEditor() {
  const t = useT();
  const editor = useUi((s) => s.editor);
  const [selected, setSelected] = useState<Sel | null>(null);
  const [active, setActive] = useState<Record<Param, Side>>({ fader: 'in', eq_low: 'in', eq_mid: 'in', eq_high: 'in', filter: 'out' });
  const draft = editor?.draft ?? null;
  const preview = useUi((s) => s.preview);
  const previewFrom = useUi((s) => s.previewFrom);
  // Zu Ende gespielt: Die Hülle hat aufgeräumt, die Taste springt zurück.
  const over = useSnap((s) => preview !== null && s.clock_beats > preview.end_beat);
  useEffect(() => {
    if (over) setUi({ preview: null });
  }, [over]);
  const change = (next: Transition) => setUi({ editor: editor ? { ...editor, draft: next } : null });

  // Entf löscht den gewählten Punkt, Esc hebt die Auswahl auf.
  useEffect(() => {
    if (!draft) return;
    const onKey = (e: KeyboardEvent) => {
      if ((e.target as HTMLElement | null)?.tagName === 'INPUT') return;
      if ((e.key === 'Delete' || e.key === 'Backspace') && selected) {
        e.preventDefault();
        change(removePoint(draft, selected.side, selected.param, selected.index));
        setSelected(null);
      }
    };
    window.addEventListener('keydown', onKey);
    return () => window.removeEventListener('keydown', onKey);
  });
  if (!editor || !draft) return null;

  const point = selected ? findLane(draft, selected.side, selected.param)?.points[selected.index] : undefined;
  const [lo, hi] = timeRange(draft);
  const clashes = bassClashes(draft);
  const from = Math.min(previewFrom, (draft.bars - 1) * 4);
  const ticks = [];
  for (let b = 0; b <= hi; b += 16) ticks.push(b);

  return (
    <section className="ted" aria-label={t('ed.title')}>
      <div className="ted__bar">
        <span className="dw-label">{t('ed.title')}</span>
        <input className="ted__name" value={draft.name} maxLength={60} spellCheck={false} aria-label={t('ed.name')} onChange={(e) => change({ ...draft, name: e.target.value })} />
        <select className="stage__select" value={draft.bars} aria-label={t('trans.length')} onChange={(e) => { change(rescale(draft, Number(e.target.value))); setSelected(null); }}>
          {[...new Set([...LENGTHS, draft.bars])].sort((a, b) => a - b).map((b) => (
            <option key={b} value={b}>
              {t('trans.bars', { n: b })}
            </option>
          ))}
        </select>
        <span className="ted__point dw-num" role="status">
          {selected && point
            ? `${t(selected.side === 'out' ? 'ed.out' : 'ed.in')} · ${t(PARAM_LABEL[selected.param])} · ${t('ed.at', { bar: Math.floor(point.beat / 4) + 1, beat: (((point.beat % 4) + 4) % 4) + 1 })} · ${valueText(selected.param, point.value, t('eq.killShort'))}`
            : clashes.length > 0
              ? t('ed.clash', { n: clashes.length })
              : t('ed.hint')}
        </span>
        {selected && point && selected.index > 0 && (
          <span className="ted__curves">
            {CURVES.map((c) => (
              <KeyBtn key={c} size="sm" mode="toggle" active={(point.curve ?? 'linear') === c} onPress={() => change(setCurve(draft, selected.side, selected.param, selected.index, c))}>
                {t(c === 'linear' ? 'ed.linear' : c === 'smooth' ? 'ed.smooth' : 'ed.step')}
              </KeyBtn>
            ))}
          </span>
        )}
        <span className="ted__spacer" />
        <select className="stage__select" value={from} aria-label={t('ed.previewFrom')} onChange={(e) => setUi({ previewFrom: Number(e.target.value) })}>
          {Array.from({ length: draft.bars }, (_, i) => (
            <option key={i} value={i * 4}>
              {t('ed.fromBar', { n: i + 1 })}
            </option>
          ))}
        </select>
        <KeyBtn size="sm" mode="toggle" active={preview !== null} onPress={() => void (preview ? stopPreview() : previewEditor())} tip={{ name: t('ed.preview'), value: t('ed.previewHint') }}>
          {t(preview ? 'ed.previewStop' : 'ed.preview')}
        </KeyBtn>
        {editor.original !== null && (
          <KeyBtn size="sm" onPress={() => void deleteSavedTransition(editor.original!)}>
            {t('ed.delete')}
          </KeyBtn>
        )}
        <KeyBtn size="sm" onPress={() => void saveEditor()} tip={{ name: t('ed.save'), value: t('ed.saveHint') }}>
          {t('ed.save')}
        </KeyBtn>
        <KeyBtn size="sm" onPress={closeEditor}>
          {t('ed.close')}
        </KeyBtn>
      </div>
      <div className="ted__body" data-clash={clashes.length > 0 ? 'true' : undefined}>
        <div className="ted__row ted__row--ruler">
          <div className="ted__label" />
          <div className="ted__ruler dw-num">
            {ticks.map((b) => (
              <span key={b} style={{ left: `${((b - lo) / (hi - lo)) * 100}%` }}>
                {b / 4 + 1}
              </span>
            ))}
          </div>
        </div>
        <PreviewMarks lo={lo} hi={hi} from={from} />
        {PARAMS.map((param) => (
          <LaneRow key={param} draft={draft} param={param} active={active[param]} selected={selected} onChange={change} onSelect={setSelected} onActive={(side) => setActive((a) => ({ ...a, [param]: side }))} />
        ))}
      </div>
    </section>
  );
}
