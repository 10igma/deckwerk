// Mixer (8.2, MIX-01–09): je Kanal Gain, EQ mit Kill, Filter, Vorhören, Pegel, Fader;
// in der Mitte Master mit Limiter und Kopfhörer; unten der Crossfader.
import * as act from '../../app/actions';
import { useSnap } from '../../app/frame';
import { shortcutKeys } from '../../app/keymap';
import { bandIndex, deckIndex, type Band, type DeckId } from '../../core/types';
import { Crossfader, DeckPlate, Fader, Icon, KeyBtn, Knob, LiveText, Meter, type FaderTick } from '../../design/components';
import { deckColorVar } from '../../design/tokens';
import { useLang, useT } from '../../i18n';
import { EQ_CURVE, GAIN_CURVE, VOLUME_CURVE, bipolarToPos, faderToDb, filterSetting, posToBipolar } from '../../lib/curves';
import { fromWire, toWire } from '../../lib/db';
import { formatDb, formatHz, formatNumber, type Lang } from '../../lib/format';

const BAND_ORDER: Band[] = ['high', 'mid', 'low'];
const BAND_COLOR: Record<Band, string> = { low: 'var(--band-low)', mid: 'var(--band-mid)', high: 'var(--band-high)' };
// Kanalfader: linear in dB, Striche alle 6 dB, betont alle 12 dB.
const FADER_TICKS: FaderTick[] = Array.from({ length: 11 }, (_, i) => ({ at: 1 - i / 10, major: i % 2 === 0 }));

const shortDb = (db: number, lang: Lang): string => (db === -Infinity ? '−∞' : formatNumber(db, 1, lang, true));

function EqRow({ deck, band }: { deck: DeckId; band: Band }) {
  const t = useT();
  const lang = useLang();
  const i = deckIndex(deck);
  const b = bandIndex(band);
  const db = fromWire(useSnap((s) => s.decks[i].eq_db[b]));
  const killed = db === -Infinity;
  const name = t(`eq.${band}`);
  return (
    <div className="strip__row">
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
        <div className="strip__line">
          <span className="dw-label strip__band" data-band={band}>
            {t(`eq.${band}.short`)}
          </span>
          <span className="strip__val dw-num">{shortDb(db, lang)}</span>
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
  const filterText = f.type === 'off' ? t('filter.off') : `${t(f.type === 'lowpass' ? 'filter.lp' : 'filter.hp')} ${formatHz(f.hz, lang)}`;

  return (
    <div className="strip" data-side={side} aria-label={t('mixer.channel', { deck: name })} role="group">
      <div className="strip__knobs">
        <div className="strip__row">
          <Knob
            size="sm"
            bipolar
            value={GAIN_CURVE.toPos(gain)}
            defaultValue={GAIN_CURVE.zeroPos}
            killZone={GAIN_CURVE.killZone}
            onChange={(p) => act.send({ type: 'gain', deck, db: toWire(GAIN_CURVE.toDb(p)) })}
            tip={{ name: `${t('mixer.gain')} · ${name}`, value: formatDb(gain, lang) }}
          />
          <div className="strip__meta">
            <span className="dw-label">{t('mixer.gain')}</span>
            <span className="strip__val dw-num">{shortDb(gain, lang)} dB</span>
          </div>
        </div>
        {BAND_ORDER.map((band) => (
          <EqRow key={band} deck={deck} band={band} />
        ))}
        <div className="strip__row">
          <Knob
            size="sm"
            bipolar
            value={bipolarToPos(filter)}
            defaultValue={0.5}
            onChange={(p) => act.setFilter(deck, posToBipolar(p))}
            tip={{
              name: `${t('mixer.filter')} · ${name}`,
              value: filterText,
              keys: [...shortcutKeys('deck.filter_close', deck, lang), '/', ...shortcutKeys('deck.filter_open', deck, lang).slice(-1)],
            }}
          />
          <div className="strip__meta">
            <span className="dw-label">{t('mixer.filter')}</span>
            <span className="strip__val dw-num">{filterText}</span>
          </div>
        </div>
      </div>

      <div className="strip__level">
        <div className="strip__levelhead">
          <DeckPlate deck={deck} size="sm" cut={side === 'right' ? 'tr' : 'tl'} />
        </div>
        <div className="strip__fadbox">
          <div className="strip__meter">
            <Meter channels={2} read={(s) => s.decks[i].peak} ariaLabel={t('mixer.level', { deck: name })} />
          </div>
          <Fader
            value={fader}
            defaultValue={1}
            ticks={FADER_TICKS}
            accent={deckColorVar(deck)}
            onChange={(v) => act.send({ type: 'fader', deck, value: v })}
            tip={{ name: `${t('mixer.fader')} · ${name}`, value: formatDb(faderToDb(fader), lang) }}
          />
        </div>
        <KeyBtn
          mode="toggle"
          size="sm"
          block
          active={cue}
          accent={deckColorVar(deck)}
          cut={side === 'right' ? 'tr' : 'tl'}
          onPress={() => act.toggleHeadphone(deck)}
          tip={{ name: t('mixer.cue', { deck: name }), value: cue ? t('state.on') : t('state.off'), keys: shortcutKeys('deck.headphone', deck, lang) }}
        >
          <Icon name="headphones" size="1em" />
          {t('mixer.cueShort')}
        </KeyBtn>
      </div>
    </div>
  );
}

function MasterSection() {
  const t = useT();
  const lang = useLang();
  const masterDb = fromWire(useSnap((s) => s.mixer.master_db));
  const limiter = useSnap((s) => s.mixer.limiter_on);
  const cueMix = useSnap((s) => s.mixer.cue_mix);
  const cueDb = fromWire(useSnap((s) => s.mixer.cue_db));
  return (
    <div className="master" role="group" aria-label={t('mixer.master')}>
      <span className="dw-label master__title">{t('mixer.master')}</span>
      <div className="master__body">
        <div className="master__meters">
          <div className="master__meter">
            <Meter channels={2} read={(s) => s.mixer.master_peak} ariaLabel={t('mixer.masterLevel')} />
          </div>
          <div className="master__gr">
            <Meter kind="gr" read={(s) => [s.mixer.limiter_gr_db]} ariaLabel={t('mixer.reduction')} />
          </div>
        </div>
        <div className="master__ctl">
          <Knob
            size="md"
            value={VOLUME_CURVE.toPos(masterDb)}
            defaultValue={VOLUME_CURVE.zeroPos}
            onChange={(p) => act.send({ type: 'master_volume', db: toWire(VOLUME_CURVE.toDb(p)) })}
            label={t('mixer.volume')}
            valueText={formatDb(masterDb, lang)}
            tip={{ name: t('mixer.masterVolume'), value: formatDb(masterDb, lang) }}
          />
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
          <div className="master__grval">
            <span className="dw-label">{t('mixer.reductionShort')}</span>
            <LiveText className="dw-num" read={(s) => `${formatNumber(Math.min(0, s.mixer.limiter_gr_db), 1, lang)} dB`} />
          </div>
        </div>
      </div>
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
          label={t('mixer.volume')}
          valueText={shortDb(cueDb, lang)}
          tip={{ name: t('mixer.phonesVolume'), value: formatDb(cueDb, lang) }}
        />
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
            value: Math.abs(xf) < 0.005 ? t('mixer.center') : `${xf < 0 ? 'A' : 'B'} ${Math.round(Math.abs(xf) * 100)} %`,
            keys: [...shortcutKeys('xf.left', undefined, lang), ...shortcutKeys('xf.right', undefined, lang)],
          }}
        />
      </div>
    </section>
  );
}
