// Mixer (8.2, MIX-01–09): je Kanal Gain und Filter, EQ mit Kill, Vorhören, Pegel, Fader;
// in der Mitte Master mit Limiter und Kopfhörer; unten der Crossfader.
import * as act from '../../app/actions';
import { useSnap } from '../../app/frame';
import { shortcutKeys } from '../../app/keymap';
import { bandIndex, deckIndex, type Band, type DeckId } from '../../core/types';
import { Crossfader, CutSide, Fader, Icon, KeyBtn, Knob, Meter, ValueField, useTip, type FaderTick } from '../../design/components';
import { deckColorVar } from '../../design/tokens';
import { useLang, useT } from '../../i18n';
import { EQ_CURVE, GAIN_CURVE, VOLUME_CURVE, bipolarToPos, faderFromDb, faderToDb, filterSetting, posToBipolar } from '../../lib/curves';
import { fromWire, toWire } from '../../lib/db';
import { formatDb, formatNumber, type Lang } from '../../lib/format';
import { parseInRange, parseNumber } from '../../lib/parse';

const BAND_ORDER: Band[] = ['high', 'mid', 'low'];
const BAND_COLOR: Record<Band, string> = { low: 'var(--band-low)', mid: 'var(--band-mid)', high: 'var(--band-high)' };
// Kanalfader: quadratische Kurve des Kerns – Striche bei 0, −6, −12 … dB, betont alle 12 dB.
export const FADER_MARKS_DB = [0, -6, -12, -18, -24, -36, -48];
const FADER_TICKS: FaderTick[] = [...FADER_MARKS_DB.map((db) => ({ at: faderFromDb(db), major: db % 12 === 0 })), { at: 0, major: true }];

/** Kurzform der Filterstellung: „TP 1,2k", „HP 240", „aus" (Zahlen unter 1000 sind Hz). */
export function filterShort(pos: number, lang: Lang, off: string, lp: string, hp: string): string {
  const f = filterSetting(pos);
  if (f.type === 'off') return off;
  const hz = f.hz >= 1000 ? `${formatNumber(f.hz / 1000, f.hz >= 10000 ? 0 : 1, lang)}k` : String(Math.round(f.hz));
  return `${f.type === 'lowpass' ? lp : hp} ${hz}`;
}

/** Wert ohne Einheit für die engen Spalten des Mixers; die Einheit dB steht in Kurzhilfe und Eingabe. */
const shortDb = (db: number, lang: Lang): string => (db === -Infinity ? '−∞' : formatNumber(db, 1, lang, true));

/** Pegelanzeige mit Kurzhilfe (die Zeichenfläche selbst nimmt keine Zeigerereignisse). */
function MeterBox({ className, name, children }: { className: string; name: string; children: React.ReactNode }) {
  const tip = useTip({ name });
  return (
    <div className={className} {...tip}>
      {children}
    </div>
  );
}

function EqRow({ deck, band }: { deck: DeckId; band: Band }) {
  const t = useT();
  const lang = useLang();
  const i = deckIndex(deck);
  const b = bandIndex(band);
  const db = fromWire(useSnap((s) => s.decks[i].eq_db[b]));
  const killed = db === -Infinity;
  const name = t(`eq.${band}`);
  return (
    <div className="strip__row strip__row--eq">
      <Knob
        size="sm"
        bipolar
        value={EQ_CURVE.toPos(db)}
        defaultValue={EQ_CURVE.zeroPos}
        killZone={EQ_CURVE.killZone}
        accent={BAND_COLOR[band]}
        onChange={(p) => act.setEq(deck, band, EQ_CURVE.toDb(p))}
        tip={{ name: `${name} · ${deck.toUpperCase()}`, value: formatDb(db, lang) }}
      />
      <div className="strip__meta">
        <span className="dw-label strip__band" data-band={band}>
          {t(`eq.${band}.short`)}
        </span>
        <ValueField
          text={shortDb(db, lang)}
          ariaLabel={`${name} ${deck.toUpperCase()} (dB)`}
          tip={{ name: t('mixer.enterDb'), value: formatDb(db, lang) }}
          onCommit={(text) => {
            const v = parseInRange(text, -36, 6, true);
            if (v === null) return false;
            act.setEq(deck, band, v);
            return true;
          }}
        />
      </div>
      <KeyBtn
        mode="toggle"
        size="sm"
        block
        active={killed}
        accent={BAND_COLOR[band]}
        onPress={() => act.toggleKill(deck, band)}
        tip={{ name: t('eq.kill', { band: name }), value: killed ? t('state.on') : t('state.off'), keys: band === 'low' ? shortcutKeys('deck.kill_low', deck, lang) : [] }}
      >
        {t('eq.killShort')}
      </KeyBtn>
    </div>
  );
}

function ChannelStrip({ deck, side }: { deck: DeckId; side: 'left' | 'right' }) {
  const t = useT();
  const lang = useLang();
  const i = deckIndex(deck);
  const name = deck.toUpperCase();
  const gain = fromWire(useSnap((s) => s.decks[i].gain_db));
  const filter = useSnap((s) => s.decks[i].filter);
  const fader = useSnap((s) => s.decks[i].fader);
  const cue = useSnap((s) => s.decks[i].cue);
  const f = filterSetting(filter);
  const filterLong = f.type === 'off' ? t('filter.off') : `${t(f.type === 'lowpass' ? 'filter.lowpass' : 'filter.highpass')} ${filterShort(filter, lang, '', '', '').trim()}`;
  const filterText = filterShort(filter, lang, t('filter.off'), t('filter.lp'), t('filter.hp'));

  return (
    <CutSide value={side === 'right' ? 'tr' : 'tl'}>
      <div className="strip" data-side={side} aria-label={t('mixer.channel', { deck: name })} role="group" onPointerDownCapture={() => act.focusDeck(deck)}>
        <div className="strip__knobs">
          <div className="strip__row strip__row--pair">
            <Knob
              size="sm"
              bipolar
              value={GAIN_CURVE.toPos(gain)}
              defaultValue={GAIN_CURVE.zeroPos}
              killZone={GAIN_CURVE.killZone}
              onChange={(p) => act.setGain(deck, GAIN_CURVE.toDb(p))}
              tip={{ name: `${t('mixer.gain')} · ${name}`, value: formatDb(gain, lang) }}
            />
            <div className="strip__meta">
              <span className="dw-label">{t('mixer.gain')}</span>
              <ValueField
                text={shortDb(gain, lang)}
                ariaLabel={`${t('mixer.gain')} ${name} (dB)`}
                tip={{ name: t('mixer.enterDb'), value: formatDb(gain, lang) }}
                onCommit={(text) => {
                  const v = parseInRange(text, -48, 12, true);
                  if (v === null) return false;
                  act.setGain(deck, v);
                  return true;
                }}
              />
            </div>
            <Knob
              size="sm"
              bipolar
              value={bipolarToPos(filter)}
              defaultValue={0.5}
              onChange={(p) => act.setFilter(deck, posToBipolar(p))}
              tip={{
                name: `${t('mixer.filter')} · ${name}`,
                value: filterLong,
                keys: [...shortcutKeys('deck.filter_close', deck, lang), '/', ...shortcutKeys('deck.filter_open', deck, lang).slice(-1)],
              }}
            />
            <div className="strip__meta">
              <span className="dw-label">{t('mixer.filter')}</span>
              <ValueField
                text={filterText}
                editText={formatNumber(filter * 100, 0, lang, true)}
                ariaLabel={`${t('mixer.filter')} ${name}`}
                tip={{ name: t('mixer.filterEnter') }}
                onCommit={(text) => {
                  // Eingabe in Prozent des Reglerwegs: −100 (Tiefpass zu) … +100 (Hochpass zu).
                  const v = parseNumber(text);
                  if (v === null || v === -Infinity) return false;
                  act.setFilter(deck, Math.max(-1, Math.min(1, v / 100)));
                  return true;
                }}
              />
            </div>
          </div>
          {BAND_ORDER.map((band) => (
            <EqRow key={band} deck={deck} band={band} />
          ))}
        </div>

        <div className="strip__level">
          <div className="strip__fadbox">
            <MeterBox className="strip__meter" name={t('mixer.level', { deck: name })}>
              <Meter channels={2} read={(s) => s.decks[i].peak} ariaLabel={t('mixer.level', { deck: name })} />
            </MeterBox>
            {/* Kein Doppelklick-Sprung: Ein geschlossener Kanal öffnet sich nie mit einem Griff (9.3). */}
            <Fader
              value={fader}
              ticks={FADER_TICKS}
              accent={deckColorVar(deck)}
              onChange={(v) => act.setFader(deck, v)}
              tip={{ name: `${t('mixer.fader')} · ${name}`, value: formatDb(faderToDb(fader), lang), keys: [...shortcutKeys('deck.fader_up', deck, lang), '/', ...shortcutKeys('deck.fader_down', deck, lang).slice(-1)] }}
            />
          </div>
          <KeyBtn
            mode="toggle"
            size="sm"
            block
            active={cue}
            accent={deckColorVar(deck)}
            onPress={() => act.toggleHeadphone(deck)}
            tip={{ name: t('mixer.cue', { deck: name }), value: cue ? t('state.on') : t('state.off'), keys: shortcutKeys('deck.headphone', deck, lang) }}
          >
            <Icon name="headphones" size="1em" />
            <DeckLetter deck={deck} />
          </KeyBtn>
        </div>
      </div>
    </CutSide>
  );
}

/** Deck-Buchstabe als Text (in Tasten, wo ein gefülltes Schild zu laut wäre). */
function DeckLetter({ deck }: { deck: DeckId }) {
  return <span className="strip__letter">{deck.toUpperCase()}</span>;
}

function MasterSection() {
  const t = useT();
  const lang = useLang();
  const masterDb = fromWire(useSnap((s) => s.mixer.master_db));
  const limiter = useSnap((s) => s.mixer.limiter_on);
  const cueMix = useSnap((s) => s.mixer.cue_mix);
  const cueDb = fromWire(useSnap((s) => s.mixer.cue_db));
  const volumeCommit = (send: (db: number) => void) => (text: string) => {
    const v = parseInRange(text, -60, 6, true);
    if (v === null) return false;
    send(v);
    return true;
  };
  return (
    <div className="master" role="group" aria-label={t('mixer.master')}>
      <div className="master__meters">
        <MeterBox className="master__meter" name={t('mixer.masterLevel')}>
          <Meter channels={2} read={(s) => s.mixer.master_peak} ariaLabel={t('mixer.masterLevel')} />
        </MeterBox>
        <MeterBox className="master__gr" name={t('mixer.reduction')}>
          <Meter kind="gr" read={(s) => [s.mixer.limiter_gr_db]} ariaLabel={t('mixer.reduction')} />
        </MeterBox>
      </div>
      <div className="master__ctl">
        <div className="strip__row strip__row--single">
          {/* Kerbe bei 0 dB, aber kein Doppelklick-Sprung (9.3). */}
          <Knob
            size="sm"
            value={VOLUME_CURVE.toPos(masterDb)}
            detent={VOLUME_CURVE.zeroPos}
            onChange={(p) => act.send({ type: 'master_volume', db: toWire(VOLUME_CURVE.toDb(p)) })}
            tip={{ name: t('mixer.masterVolume'), value: formatDb(masterDb, lang) }}
          />
          <div className="strip__meta">
            <span className="dw-label">{t('mixer.master')}</span>
            <ValueField
              text={shortDb(masterDb, lang)}
              ariaLabel={`${t('mixer.masterVolume')} (dB)`}
              tip={{ name: t('mixer.enterDb'), value: formatDb(masterDb, lang) }}
              onCommit={volumeCommit((db) => act.send({ type: 'master_volume', db: toWire(db) }))}
            />
          </div>
        </div>
        <KeyBtn
          mode="toggle"
          size="sm"
          block
          active={limiter}
          onPress={() => act.send({ type: 'limiter', on: !limiter })}
          tip={{ name: t('mixer.limiter'), value: limiter ? t('state.on') : t('state.off') }}
        >
          {t('mixer.limiterShort')}
        </KeyBtn>
        <span className="dw-label master__title">
          <Icon name="headphones" size="1em" style={{ display: 'inline-block', verticalAlign: '-0.12em', marginRight: '0.35em' }} />
          {t('mixer.phones')}
        </span>
        <div className="master__phones">
          <Knob
            size="sm"
            bipolar
            value={cueMix}
            defaultValue={0.5}
            onChange={(p) => act.send({ type: 'cue_mix', value: p })}
            label={t('mixer.cueMix')}
            valueText={cueMix < 0.02 ? 'CUE' : cueMix > 0.98 ? 'MST' : `${Math.round(cueMix * 100)} %`}
            tip={{ name: t('mixer.cueMixLong'), value: `${Math.round((1 - cueMix) * 100)} % Cue · ${Math.round(cueMix * 100)} % Master` }}
          />
          <Knob
            size="sm"
            value={VOLUME_CURVE.toPos(cueDb)}
            defaultValue={VOLUME_CURVE.zeroPos}
            onChange={(p) => act.send({ type: 'cue_volume', db: toWire(VOLUME_CURVE.toDb(p)) })}
            label={t('mixer.phonesLevel')}
            valueText={shortDb(cueDb, lang)}
            tip={{ name: t('mixer.phonesVolume'), value: formatDb(cueDb, lang) }}
          />
        </div>
      </div>
    </div>
  );
}

export function Mixer() {
  const t = useT();
  const lang = useLang();
  const xf = useSnap((s) => s.mixer.crossfader);
  return (
    <section className="mixer" aria-label={t('mixer.aria')}>
      <div className="mixer__strips">
        <ChannelStrip deck="a" side="left" />
        <MasterSection />
        <ChannelStrip deck="b" side="right" />
      </div>
      <div className="mixer__xf">
        <Crossfader
          value={xf}
          left="a"
          right="b"
          onChange={(v) => act.send({ type: 'crossfader', pos: v })}
          tip={{
            name: t('mixer.crossfader'),
            value: `${Math.abs(xf) < 0.005 ? t('mixer.center') : `${xf < 0 ? 'A' : 'B'} ${Math.round(Math.abs(xf) * 100)} %`} · ${t('mixer.xfFull')}`,
            keys: [...shortcutKeys('xf.left', undefined, lang), ...shortcutKeys('xf.right', undefined, lang)],
          }}
        />
      </div>
    </section>
  );
}
