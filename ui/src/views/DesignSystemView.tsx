// Seite /#/designsystem: alle Tokens und Komponenten in allen Zuständen.
import { useEffect, useRef, useState, type CSSProperties, type ReactNode } from 'react';
import { useCore } from '../app/context';
import { useUi } from '../app/store';
import { DECK_IDS, emptyDeckState, type DeckId } from '../core/types';
import {
  Cell,
  Crossfader,
  CutSide,
  DeckPlate,
  Fader,
  Icon,
  KeyBtn,
  KeyChip,
  Knob,
  ListRow,
  Meter,
  PhaseMeter,
  Readout,
  TipBox,
  ValueField,
  type IconName,
} from '../design/components';
import { cssVar } from '../design/tokens';
import { useLang, useT, type MsgKey } from '../i18n';
import { contrast } from '../lib/color';
import { EQ_CURVE, faderFromDb, faderToDb } from '../lib/curves';
import { FADER_MARKS_DB } from './mix/Mixer';
import { formatDb, formatNumber, type Lang } from '../lib/format';
import { fromCamelot } from '../lib/key';
import { parseInRange } from '../lib/parse';
import { readStageColors } from './mix/stageColors';
import { Canvas2DWaveRenderer } from './mix/wave/canvas2d';
import type { WaveRenderer } from './mix/wave/renderer';
import { WebGLWaveRenderer } from './mix/wave/webgl';
import { buildScene } from './mix/wave/scene';
import './designsystem.css';

const COLOR_GROUPS: { title: MsgKey; tokens: string[] }[] = [
  { title: 'ds.surfaces', tokens: ['--bg-0', '--bg-1', '--bg-2', '--bg-3', '--line-1', '--line-2'] },
  { title: 'ds.text', tokens: ['--text-1', '--text-2', '--text-3', '--ink'] },
  { title: 'ds.decks', tokens: ['--deck-a', '--deck-b', '--deck-c', '--deck-d'] },
  { title: 'ds.bands', tokens: ['--band-low', '--band-mid', '--band-high'] },
  { title: 'ds.status', tokens: ['--status-ok', '--status-warn', '--status-danger', '--status-rec'] },
];
const FONT_SIZES = ['--fs-1', '--fs-2', '--fs-3', '--fs-4', '--fs-5', '--fs-6', '--fs-7'];
const SPACES = ['--sp-1', '--sp-2', '--sp-3', '--sp-4', '--sp-5', '--sp-6'];
const SIZES = ['--ctl-h-sm', '--ctl-h-md', '--ctl-h-lg', '--knob-sm', '--knob-md', '--knob-lg', '--fader-cap', '--row-h', '--header-h'];
const ICONS: IconName[] = ['play', 'pause', 'tostart', 'headphones', 'gear', 'rec', 'search', 'slower', 'faster', 'up', 'down', 'minus', 'plus', 'expand', 'collapse', 'drop', 'match', 'ok', 'clash', 'warn'];

/** Token-Wert mit Dezimalkomma im Deutschen und führender Null (0,25 rem statt .25rem). */
function localize(value: string, lang: Lang): string {
  const v = value.replace(/(^|[^\d])\.(\d)/g, '$10.$2').replace(/(\d)(rem|px|ms)\b/g, '$1 $2');
  return lang === 'de' ? v.replace(/(\d)\.(\d)/g, '$1,$2') : v;
}
const ROW_COLS = '9rem minmax(8rem, 24rem) minmax(6rem, 18rem) 4.6rem 10.5rem 3.6rem minmax(0, 1fr)';

function Section({ title, note, children }: { title: string; note?: string; children: ReactNode }) {
  return (
    <section className="ds__section">
      <h2 className="ds__h2">{title}</h2>
      {note && <p className="ds__note">{note}</p>}
      {children}
    </section>
  );
}

function Sample({ label, children, wide }: { label: string; children: ReactNode; wide?: boolean }) {
  return (
    <div className="ds__sample" data-wide={wide ? 'true' : undefined}>
      <div className="ds__sampleBody">{children}</div>
      <span className="ds__caption">{label}</span>
    </div>
  );
}

function Swatch({ token }: { token: string }) {
  // Werte erst nach dem Einhängen lesen (Design kann gewechselt haben).
  const theme = useUi((s) => s.theme);
  const [hex, setHex] = useState('');
  const [bg, setBg] = useState('');
  useEffect(() => {
    setHex(cssVar(token));
    setBg(cssVar('--bg-1'));
  }, [token, theme]);
  const ratio = hex && bg ? contrast(hex, bg) : 0;
  const lang = useLang();
  return (
    <div className="ds__swatch">
      <span className="ds__chip" style={{ background: `var(${token})` }} />
      <span className="ds__token dw-num">{token}</span>
      <span className="ds__hex dw-num">{hex}</span>
      <span className="ds__ratio dw-num">{ratio ? `${formatNumber(ratio, 1, lang)} : 1` : ''}</span>
    </div>
  );
}

function TokenLength({ token, kind }: { token: string; kind: 'bar' | 'box' }) {
  const [value, setValue] = useState('');
  const lang = useLang();
  useEffect(() => setValue(cssVar(token)), [token]);
  const style: CSSProperties = kind === 'bar' ? { width: `var(${token})`, height: '0.9rem' } : { width: `var(${token})`, height: `var(${token})` };
  return (
    <div className="ds__len">
      <span className="ds__lenbox" style={style} />
      <span className="ds__token dw-num">{token}</span>
      <span className="ds__hex dw-num">{localize(value, lang)}</span>
    </div>
  );
}

/** Wellenform-Probe: zwei Bahnen wie auf der Bühne, unten mit gekilltem Tiefband. */
function WaveSample() {
  const core = useCore();
  const theme = useUi((s) => s.theme);
  const first = useUi((s) => s.tracks[0]?.id ?? null);
  const canvas = useRef<HTMLCanvasElement | null>(null);
  useEffect(() => {
    const c = canvas.current;
    if (!c || !first) return;
    let alive = true;
    void Promise.all([core.getWaveform(first), core.getAnalysis(first)]).then(([waveform, analysis]) => {
      if (!alive) return;
      const dpr = window.devicePixelRatio || 1;
      const rect = c.getBoundingClientRect();
      c.width = Math.round(rect.width * dpr);
      c.height = Math.round(rect.height * dpr);
      // Derselbe Weg wie in der Ansicht „Mixen": WebGL 2, sonst Canvas 2D.
      let r: WaveRenderer;
      try {
        r = new WebGLWaveRenderer(c);
      } catch {
        r = new Canvas2DWaveRenderer(c);
      }
      r.setTrack('a', waveform);
      r.setTrack('b', waveform);
      const colors = readStageColors();
      const lane = (deck: DeckId, kill: boolean) => {
        const state = emptyDeckState();
        state.loaded = true;
        state.playing = true;
        state.pos = analysis.grid.anchors[0].pos + (60 / analysis.bpm) * (52 * 4 + 1.6 + (kill ? 0.12 : 0));
        state.duration = analysis.duration;
        state.bpm = analysis.bpm;
        state.track_bpm = analysis.bpm;
        if (kill) state.eq_db = [null, 0, 0];
        return { deck, state, waveform, grid: analysis.grid };
      };
      const rem = parseFloat(getComputedStyle(document.documentElement).fontSize);
      r.draw(
        buildScene({ width: c.width, height: c.height, dpr, railH: rem * 2 * dpr, unit: (rem / 14) * dpr, zoomBeats: 8, clockBpm: analysis.bpm, lanes: [lane('a', false), lane('b', true)], colors }),
        colors,
      );
      // Das Bild bleibt stehen; die Zeichenfläche behält ihren Kontext bis zum Abbau der Seite.
    });
    return () => {
      alive = false;
    };
  }, [core, first, theme]);
  return <canvas ref={canvas} className="ds__wave" />;
}

export function DesignSystemView() {
  const t = useT();
  const lang = useLang();
  const [knob, setKnob] = useState(0.72);
  const [eq, setEq] = useState(0.32);
  const [fader, setFader] = useState(0.7);
  const [tempo, setTempo] = useState(0.58);
  const [xf, setXf] = useState(-0.35);
  const [toggle, setToggle] = useState(true);
  const [entry, setEntry] = useState(-3.2);
  const eqDb = EQ_CURVE.toDb(eq);

  return (
    <div className="ds">
      <header className="ds__head">
        <div>
          <h1 className="ds__h1">{t('ds.title')}</h1>
          <p className="ds__lead">{t('ds.lead')}</p>
        </div>
        <a className="ds__back dw-label" href="#/mixen">
          ← {t('ds.back')}
        </a>
      </header>

      <Section title={t('ds.colors')} note={t('ds.contrastOn', { bg: 'bg-1' })}>
        <div className="ds__groups">
          {COLOR_GROUPS.map((g) => (
            <div key={g.title} className="ds__group">
              <h3 className="dw-label">{t(g.title)}</h3>
              {g.tokens.map((tk) => (
                <Swatch key={tk} token={tk} />
              ))}
            </div>
          ))}
        </div>
      </Section>

      <Section title={t('ds.keys')} note={t('ds.keysNote')}>
        <div className="ds__keys">
          {(['B', 'A'] as const).map((letter) => (
            <div key={letter} className="ds__keyrow">
              {Array.from({ length: 12 }, (_, n) => (
                <KeyChip key={n} musicalKey={fromCamelot(n + 1, letter)} lang={lang} size="lg" showName />
              ))}
            </div>
          ))}
          <div className="ds__keyrow">
            <KeyChip musicalKey={null} lang={lang} size="lg" />
          </div>
        </div>
      </Section>

      <Section title={t('ds.type')}>
        <div className="ds__type">
          {FONT_SIZES.map((fs) => (
            <div key={fs} className="ds__typerow">
              <span className="ds__token dw-num">{fs}</span>
              <span style={{ font: `500 var(${fs}) / 1.2 var(--font-text)` }}>Kaltwalzwerk · Schicht III</span>
              <span style={{ font: `600 var(${fs}) / 1.2 var(--font-label)`, letterSpacing: 'var(--track-label)', textTransform: 'uppercase' }}>Tempo · Phase</span>
              <span className="dw-num" style={{ fontSize: `var(${fs})`, fontWeight: 500, lineHeight: 1.2 }}>
                134{lang === 'de' ? ',' : '.'}00 −6:11{lang === 'de' ? ',' : '.'}3
              </span>
            </div>
          ))}
          <div className="ds__typerow ds__typerow--legend">
            <span />
            <span className="dw-label">{t('ds.typeText')} · Barlow Semi Condensed</span>
            <span className="dw-label">{t('ds.typeLabel')} · Barlow Condensed</span>
            <span className="dw-label">{t('ds.typeNum')} · Martian Mono</span>
          </div>
        </div>
      </Section>

      <Section title={t('ds.spacing')}>
        <div className="ds__groups">
          <div className="ds__group">
            {SPACES.map((s) => (
              <TokenLength key={s} token={s} kind="bar" />
            ))}
          </div>
          <div className="ds__group">
            {SIZES.map((s) => (
              <TokenLength key={s} token={s} kind="bar" />
            ))}
          </div>
          <div className="ds__group">
            <TokenLength token="--cut-1" kind="box" />
            <TokenLength token="--cut-2" kind="box" />
            <div className="ds__shapes">
              <span className="ds__shape" style={{ '--c': 'var(--cut-1)' } as CSSProperties} />
              <span className="ds__shape" style={{ '--c': 'var(--cut-2)' } as CSSProperties} />
              <span className="ds__shape ds__shape--hatch" style={{ '--c': 'var(--cut-2)' } as CSSProperties} />
            </div>
          </div>
        </div>
      </Section>

      <h2 className="ds__h2 ds__h2--major">{t('ds.components')}</h2>

      <Section title={t('ds.key')}>
        <div className="ds__matrix">
          <span />
          <span className="dw-label">{t('ds.stateNormal')}</span>
          <span className="dw-label">{t('ds.stateHover')}</span>
          <span className="dw-label">{t('ds.stateHeld')}</span>
          <span className="dw-label">{t('ds.stateActive')}</span>
          <span className="dw-label">{t('ds.stateActiveHeld')}</span>
          <span className="dw-label">{t('ds.stateFocus')}</span>
          <span className="dw-label">{t('ds.stateDisabled')}</span>

          <span className="dw-label">{t('ds.keyTrigger')}</span>
          <KeyBtn>Beat</KeyBtn>
          <KeyBtn forceState="hover">Beat</KeyBtn>
          <KeyBtn forceHeld>Beat</KeyBtn>
          <span className="ds__na">–</span>
          <span className="ds__na">–</span>
          <KeyBtn forceState="focus">Beat</KeyBtn>
          <KeyBtn disabled>Beat</KeyBtn>

          <span className="dw-label">{t('ds.keyToggle')}</span>
          <KeyBtn mode="toggle">Sync</KeyBtn>
          <KeyBtn mode="toggle" forceState="hover">
            Sync
          </KeyBtn>
          <KeyBtn mode="toggle" forceHeld>
            Sync
          </KeyBtn>
          <KeyBtn mode="toggle" active accent="var(--deck-a)">
            Sync
          </KeyBtn>
          <KeyBtn mode="toggle" active forceHeld accent="var(--deck-a)">
            Sync
          </KeyBtn>
          <KeyBtn mode="toggle" forceState="focus">
            Sync
          </KeyBtn>
          <KeyBtn mode="toggle" disabled>
            Sync
          </KeyBtn>

          <span className="dw-label">{t('ds.keyOutline')}</span>
          <KeyBtn mode="toggle" tone="outline">
            <Icon name="play" size="0.95em" /> Play
          </KeyBtn>
          <KeyBtn mode="toggle" tone="outline" forceState="hover">
            <Icon name="play" size="0.95em" /> Play
          </KeyBtn>
          <KeyBtn mode="toggle" tone="outline" forceHeld>
            <Icon name="play" size="0.95em" /> Play
          </KeyBtn>
          <KeyBtn mode="toggle" tone="outline" active accent="var(--deck-a)">
            <Icon name="play" size="0.95em" /> Play
          </KeyBtn>
          <KeyBtn mode="toggle" tone="outline" active forceHeld accent="var(--deck-a)">
            <Icon name="play" size="0.95em" /> Play
          </KeyBtn>
          <KeyBtn mode="toggle" tone="outline" forceState="focus">
            <Icon name="play" size="0.95em" /> Play
          </KeyBtn>
          <KeyBtn mode="toggle" tone="outline" disabled>
            <Icon name="play" size="0.95em" /> Play
          </KeyBtn>

          <span className="dw-label">{t('ds.keyHold')}</span>
          <KeyBtn mode="hold">Cue</KeyBtn>
          <KeyBtn mode="hold" forceState="hover">
            Cue
          </KeyBtn>
          <KeyBtn mode="hold" forceHeld>
            Cue
          </KeyBtn>
          <span className="ds__na">–</span>
          <span className="ds__na">–</span>
          <KeyBtn mode="hold" forceState="focus">
            Cue
          </KeyBtn>
          <KeyBtn mode="hold" disabled>
            Cue
          </KeyBtn>
        </div>
        <p className="ds__note">{t('ds.cutRule')}</p>
        <div className="ds__rowflex">
          <Sample label={`${t('ds.sizes')}: sm · md · lg`}>
            <KeyBtn size="sm">Kill</KeyBtn>
            <KeyBtn size="md">Beat</KeyBtn>
            <KeyBtn size="lg">
              <Icon name="play" /> Play
            </KeyBtn>
          </Sample>
          <Sample label="A · B · C · D">
            {DECK_IDS.map((d) => (
              <CutSide key={d} value={d === 'b' || d === 'd' ? 'tr' : 'tl'}>
                <KeyBtn mode="toggle" active accent={`var(--deck-${d})`}>
                  Sync
                </KeyBtn>
              </CutSide>
            ))}
          </Sample>
          <Sample label={t('ds.stateArmed')}>
            <KeyBtn mode="toggle" tone="outline" active armed accent="var(--deck-a)">
              <Icon name="play" size="0.95em" /> Play
            </KeyBtn>
            <KeyBtn mode="hold" armed>
              Cue
            </KeyBtn>
          </Sample>
          <Sample label={t('eq.killShort')}>
            <KeyBtn mode="toggle" size="sm" active accent="var(--band-low)">
              Kill
            </KeyBtn>
            <KeyBtn mode="toggle" size="sm" active accent="var(--band-mid)">
              Kill
            </KeyBtn>
            <KeyBtn mode="toggle" size="sm" active accent="var(--band-high)">
              Kill
            </KeyBtn>
          </Sample>
          <Sample label={`${t('ds.keyToggle')} (live)`}>
            <KeyBtn mode="toggle" active={toggle} onPress={() => setToggle(!toggle)} tip={{ name: 'Snap', value: toggle ? t('state.on') : t('state.off'), keys: ['F10'] }}>
              Snap
            </KeyBtn>
          </Sample>
        </div>
      </Section>

      <Section title={t('ds.knob')} note={t('ds.knobNote')}>
        <div className="ds__rowflex">
          <Sample label={`${t('ds.unipolar')} · sm`}>
            <Knob size="sm" value={knob} defaultValue={0.75} onChange={setKnob} label={t('mixer.volume')} valueText={`${Math.round(knob * 100)} %`} />
          </Sample>
          <Sample label={`${t('ds.unipolar')} · md`}>
            <Knob size="md" value={knob} defaultValue={0.75} onChange={setKnob} label={t('mixer.volume')} valueText={`${Math.round(knob * 100)} %`} tip={{ name: t('mixer.masterVolume'), value: `${Math.round(knob * 100)} %` }} />
          </Sample>
          <Sample label={`${t('ds.unipolar')} · lg`}>
            <Knob size="lg" value={knob} defaultValue={0.75} onChange={setKnob} label={t('mixer.volume')} valueText={`${Math.round(knob * 100)} %`} />
          </Sample>
          <Sample label={`${t('ds.bipolar')} · ${t('eq.low.short')}`}>
            <Knob size="md" bipolar value={eq} defaultValue={0.5} killZone={0.05} accent="var(--band-low)" onChange={setEq} label={t('eq.low.short')} valueText={formatDb(eqDb, lang)} />
          </Sample>
          <Sample label={`${t('ds.bipolar')} · ${t('eq.mid.short')}`}>
            <Knob size="md" bipolar value={0.68} defaultValue={0.5} killZone={0.05} accent="var(--band-mid)" onChange={() => {}} label={t('eq.mid.short')} valueText={formatDb(EQ_CURVE.toDb(0.68), lang)} />
          </Sample>
          <Sample label={`${t('eq.killShort')} (−∞)`}>
            <Knob size="md" bipolar value={0} defaultValue={0.5} killZone={0.05} accent="var(--band-high)" onChange={() => {}} label={t('eq.high.short')} valueText={formatDb(-Infinity, lang)} />
          </Sample>
          <Sample label={t('ds.stateHover')}>
            <Knob size="md" value={0.55} defaultValue={0.5} onChange={() => {}} forceState="hover" label={t('mixer.gain')} valueText={formatNumber(1.2, 1, lang, true)} />
          </Sample>
          <Sample label={t('ds.stateHeld')}>
            <Knob size="md" value={0.4} defaultValue={0.5} onChange={() => {}} forceHeld label={t('mixer.gain')} valueText={formatNumber(-3.2, 1, lang, true)} />
          </Sample>
          <Sample label={t('ds.stateFocus')}>
            <Knob size="md" value={0.5} defaultValue={0.5} onChange={() => {}} forceState="focus" label={t('mixer.gain')} valueText={formatNumber(0, 1, lang, true)} />
          </Sample>
          <Sample label={t('ds.stateDisabled')}>
            <Knob size="md" value={0.6} defaultValue={0.5} onChange={() => {}} disabled label={t('mixer.filter')} valueText={t('filter.off')} />
          </Sample>
        </div>
      </Section>

      <Section title={t('ds.fader')} note={t('ds.faderNote')}>
        <div className="ds__rowflex">
          <Sample label={t('ds.stateNormal')}>
            <div className="ds__vfader">
              <Fader value={fader} onChange={setFader} accent="var(--deck-a)" ticks={FADER_MARKS_DB.map((db) => ({ at: faderFromDb(db), major: db % 12 === 0 }))} tip={{ name: t('mixer.fader'), value: formatDb(faderToDb(fader), lang) }} />
            </div>
          </Sample>
          <Sample label={t('ds.stateHeld')}>
            <div className="ds__vfader">
              <Fader value={0.45} onChange={() => {}} accent="var(--deck-b)" forceHeld />
            </div>
          </Sample>
          <Sample label={t('ds.stateDisabled')}>
            <div className="ds__vfader">
              <Fader value={0.3} onChange={() => {}} disabled />
            </div>
          </Sample>
          <Sample label={`${t('deck.tempo')} · ${t('ds.bipolar')}`} wide>
            <Fader orientation="h" value={tempo} defaultValue={0.5} fillFrom={0.5} onChange={setTempo} accent="var(--deck-a)" ticks={Array.from({ length: 17 }, (_, i) => ({ at: i / 16, major: i % 4 === 0 }))} />
          </Sample>
          <Sample label={`${t('deck.tempo')} · ${t('ds.guided')}`} wide>
            <Fader orientation="h" value={0.41} defaultValue={0.5} fillFrom={0.5} onChange={() => {}} accent="var(--deck-b)" guided ticks={Array.from({ length: 17 }, (_, i) => ({ at: i / 16, major: i % 4 === 0 }))} />
          </Sample>
          <Sample label={t('ds.crossfader')} wide>
            <div style={{ width: '100%' }}>
              <Crossfader value={xf} onChange={setXf} left="a" right="b" tip={{ name: t('mixer.crossfader'), value: `${Math.round(xf * 100)} %`, keys: ['←', '→'] }} />
            </div>
          </Sample>
        </div>
      </Section>

      <Section title={t('ds.meter')} note={t('ds.meterNote')}>
        <div className="ds__rowflex">
          {(
            [
              ['ds.meterQuiet', [0.04]],
              ['ds.meterNominal', [0.3]],
              ['ds.meterHot', [0.8]],
              ['ds.meterClip', [1.3]],
              ['ds.meterStereo', [0.5, 0.42]],
            ] as [MsgKey, number[]][]
          ).map(([label, v]) => (
            <Sample key={label} label={t(label)}>
              <div className="ds__meter" style={{ width: v.length > 1 ? '1.3rem' : '0.7rem' }}>
                <Meter channels={v.length} read={() => v} />
              </div>
            </Sample>
          ))}
          <Sample label={`${t('ds.meterGr')} −4 dB`}>
            <div className="ds__meter" style={{ width: '0.5rem' }}>
              <Meter kind="gr" read={() => [-4]} />
            </div>
          </Sample>
        </div>
      </Section>

      <Section title={t('ds.readout')}>
        <div className="ds__rowflex">
          <Sample label="lg">
            <Readout label={t('deck.bpm')} size="lg">
              {formatNumber(134, 2, lang)}
            </Readout>
          </Sample>
          <Sample label="lg · rechts">
            <Readout label={t('deck.remaining')} size="lg" align="right">
              −1:26{lang === 'de' ? ',' : '.'}7
            </Readout>
          </Sample>
          <Sample label="md · muted">
            <Readout label={t('deck.bar')} size="md" tone="muted">
              177.2
            </Readout>
          </Sample>
          <Sample label="sm">
            <Readout label={t('mixer.gain')} unit="dB">
              {formatNumber(-3.2, 1, lang, true)}
            </Readout>
          </Sample>
        </div>
        <h3 className="dw-label ds__sub">{t('ds.valueField')}</h3>
        <p className="ds__note">{t('ds.valueFieldNote')}</p>
        <div className="ds__rowflex">
          <Sample label={t('ds.stateNormal')}>
            <ValueField text={formatNumber(entry, 1, lang, true)} ariaLabel={t('mixer.gain')} onCommit={(text) => {
              const v = parseInRange(text, -48, 12);
              if (v === null) return false;
              setEntry(v);
              return true;
            }} />
          </Sample>
          <Sample label={t('ds.valueEditing')}>
            <span style={{ width: '4.5rem', display: 'block' }}>
              <ValueField text={formatNumber(-3.2, 1, lang, true)} ariaLabel={t('mixer.gain')} onCommit={() => true} forceEditing />
            </span>
          </Sample>
        </div>
      </Section>

      <Section title={t('ds.row')}>
        <div className="ds__list" role="table">
          <ListRow columns={ROW_COLS} head>
            <span className="dw-label">{t('lib.load')}</span>
            <span className="dw-label">{t('lib.title')}</span>
            <span className="dw-label">{t('lib.artist')}</span>
            <span className="dw-label" style={{ textAlign: 'right' }}>
              {t('lib.bpm')}
            </span>
            <span className="dw-label">{t('lib.key')}</span>
            <span className="dw-label" style={{ textAlign: 'right' }}>
              {t('lib.duration')}
            </span>
            <span />
          </ListRow>
          {(
            [
              ['Kaltwalzwerk', 'Mara Volt', 134, fromCamelot(8, 'A'), '6:42', {}, t('ds.stateNormal'), 'ok'],
              ['Schicht III', 'Hektor Blei', 136, fromCamelot(9, 'A'), '7:05', { selected: true, zebra: true }, t('ds.rowSelected'), 'ok'],
              ['Null Uhr Vier', 'Okular', 128, fromCamelot(7, 'B'), '6:01', { dragging: true }, t('ds.rowDragging'), 'ok'],
              ['Vakuum', 'Ilse Draht', 131, null, '6:22', { zebra: true }, t('ds.rowNoKey'), 'ok'],
              ['Rohrpost', 'Kontor 9', 0, null, '7:03', {}, t('lib.pending'), 'pending'],
              ['Gleis 13', 'Jonte Maar', 127, fromCamelot(11, 'B'), '6:04', { dim: true, zebra: true }, t('lib.missing'), 'missing'],
            ] as [string, string, number, number | null, string, { selected?: boolean; dragging?: boolean; dim?: boolean; zebra?: boolean }, string, string][]
          ).map(([title, artist, bpm, key, len, state, label, kind]) => (
            <ListRow key={title} columns={ROW_COLS} {...state}>
              <span className="dw-label dw-truncate">{label}</span>
              <Cell strong>{title}</Cell>
              <Cell>{artist}</Cell>
              <Cell num>{bpm > 0 ? formatNumber(bpm, 1, lang) : '–'}</Cell>
              <span>
                {kind === 'pending' ? (
                  <span className="lib__state" data-state="pending">
                    <span className="lib__spinner" aria-hidden="true" />
                    {t('lib.pending')}
                  </span>
                ) : kind === 'missing' ? (
                  <span className="lib__state" data-state="missing">
                    <Icon name="warn" size="0.9em" />
                    {t('lib.missing')}
                  </span>
                ) : (
                  <KeyChip musicalKey={key} lang={lang} showName />
                )}
              </span>
              <Cell num>{len}</Cell>
              <span />
            </ListRow>
          ))}
        </div>
      </Section>

      <Section title={t('ds.tip')} note={t('ds.tipNote')}>
        <div className="ds__rowflex">
          <TipBox isStatic content={{ name: t('deck.play'), keys: ['Q'] }} />
          <TipBox isStatic content={{ name: `${t('eq.low')} · A`, value: formatDb(-3.2, lang) }} />
          <TipBox isStatic content={{ name: t('lib.loadTo', { deck: 'B' }), keys: ['Shift', '→'] }} />
          <TipBox isStatic content={{ name: t('mixer.crossfader'), value: 'A 35 %', keys: ['←', '→'] }} />
        </div>
        <h3 className="dw-label ds__sub">{t('ds.notice')}</h3>
        <div className="dw-notice ds__notice">
          <Icon name="warn" size="1.2em" />
          {t('guard.stop', { deck: 'A' })}
        </div>
        <div className="dw-notice ds__notice" data-tone="error">
          <Icon name="warn" size="1.2em" />
          {t('notice.missing', { title: 'Gleis 13' })}
        </div>
      </Section>

      <Section title={t('ds.plates')}>
        <div className="ds__rowflex">
          <Sample label="lg">
            {DECK_IDS.map((d) => (
              <DeckPlate key={d} deck={d} size="lg" cut={d === 'b' || d === 'd' ? 'tr' : 'tl'} />
            ))}
          </Sample>
          <Sample label="md">
            {DECK_IDS.map((d) => (
              <DeckPlate key={d} deck={d} />
            ))}
          </Sample>
          <Sample label="sm">
            {DECK_IDS.map((d) => (
              <DeckPlate key={d} deck={d} size="sm" />
            ))}
          </Sample>
          <Sample label={`${t('compat.match.short')} · ${t('compat.ok.short')} · ${t('compat.clash.short')}`}>
            <span className="deck__compat" data-compat="match">
              <Icon name="match" size="1rem" />
            </span>
            <span className="deck__compat" data-compat="ok">
              <Icon name="ok" size="1rem" />
            </span>
            <span className="deck__compat" data-compat="clash">
              <Icon name="clash" size="1rem" />
            </span>
          </Sample>
        </div>
      </Section>

      <Section title={t('ds.phase')}>
        <div className="ds__phases">
          {(
            [
              ['ds.phaseLocked', 0],
              ['ds.phaseEarly', -0.18],
              ['ds.phaseLate', 0.31],
              ['ds.phaseOff', null],
            ] as [MsgKey, number | null][]
          ).map(([label, v]) => (
            <div key={label} className="ds__phase">
              <PhaseMeter read={() => v} />
              <span className="ds__caption">
                {t(label)}
                {v !== null ? ` · ${formatNumber(v, 2, lang, true)}` : ''}
              </span>
            </div>
          ))}
        </div>
      </Section>

      <Section title={t('ds.waveform')} note={t('ds.waveformNote')}>
        <WaveSample />
        <div className="ds__wavelegend">
          <span className="ds__caption">↑ {t('ds.waveNormal')}</span>
          <span className="ds__caption">↓ {t('ds.waveKill')}</span>
        </div>
      </Section>

      <Section title={t('ds.icons')}>
        <div className="ds__icons">
          {ICONS.map((n) => (
            <span key={n} className="ds__icon">
              <Icon name={n} size="1.4rem" />
              <span className="ds__caption">{n}</span>
            </span>
          ))}
        </div>
      </Section>
    </div>
  );
}
