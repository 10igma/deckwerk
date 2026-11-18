// Deck (8.2): Kopf mit Titel und Tonart, Zahlen, Übersicht, Gleichlauf-Block (Phase, Anschieben, Tempo), Transport.
import { useRef } from 'react';
import * as act from '../../app/actions';
import { getDeckAssets } from '../../app/assets';
import { useTrackDrop } from '../../app/dnd';
import { useFrame, useSnap } from '../../app/frame';
import { shortcutKeys } from '../../app/keymap';
import { getUi, useUi } from '../../app/store';
import { deckIndex, type DeckId, type Snapshot } from '../../core/types';
import { DeckPlate, Fader, Icon, KeyBtn, KeyChip, LiveText, PhaseMeter, Readout, useTip, type FaderTick } from '../../design/components';
import { deckColorVar } from '../../design/tokens';
import { useLang, useT, type TFn } from '../../i18n';
import { tempoOffsetToPos, tempoPosToOffset } from '../../lib/curves';
import { decimalSep, formatBpm, formatNumber, formatPercent, formatPhase, type Lang } from '../../lib/format';
import { barBeat } from '../../lib/grid';
import { camelotString, compatClass, harmonicCompat, type CompatClass } from '../../lib/key';
import { formatRemaining, formatTime } from '../../lib/time';
import { Overview } from './Overview';

function tempoTicks(range: number): FaderTick[] {
  // Bis ±10 % ein Strich je Prozent, darüber acht Teile je Seite.
  const step = range <= 0.1 + 1e-9 ? 0.01 : range / 8;
  const n = Math.round(range / step);
  const out: FaderTick[] = [];
  for (let i = -n; i <= n; i++) out.push({ at: 0.5 + (i / n) * 0.5, major: i === 0 || Math.abs(i) === n || (n % 2 === 0 && Math.abs(i) === n / 2) });
  return out;
}

function barText(deck: DeckId, s: Snapshot): string {
  const d = s.decks[deckIndex(deck)];
  const a = getDeckAssets(deck);
  if (!d.loaded || !a) return '--.-';
  const bb = barBeat(a.analysis.grid, d.beat);
  return `${bb.bar}.${bb.beat}`;
}

/** Schlag im Takt als vier Lamellen; die Eins ist breiter. */
function BeatDots({ deck }: { deck: DeckId }) {
  const el = useRef<HTMLDivElement | null>(null);
  const last = useRef('');
  useFrame((s) => {
    const d = s.decks[deckIndex(deck)];
    const a = getDeckAssets(deck);
    const v = d.loaded && a ? String(barBeat(a.analysis.grid, d.beat).beat) : '0';
    if (v !== last.current && el.current) {
      last.current = v;
      el.current.dataset.beat = v;
    }
  });
  return (
    <div className="deck__beats" ref={el} aria-hidden="true">
      <span />
      <span />
      <span />
      <span />
    </div>
  );
}

const COMPAT_ICON: Record<CompatClass, 'match' | 'ok' | 'clash'> = { match: 'match', ok: 'ok', clash: 'clash' };

/** Verträglichkeit der Tonart zum anderen Deck als Symbol mit Kurzhilfe (VIS-06). */
function Compat({ deck, other, t }: { deck: DeckId; other: DeckId; t: TFn }) {
  const mine = useUi((s) => s.deckTracks[deck]?.key ?? null);
  const theirs = useUi((s) => s.deckTracks[other]?.key ?? null);
  const cls = mine !== null && theirs !== null ? compatClass(harmonicCompat(theirs, mine)) : null;
  const tip = useTip(
    cls && mine !== null && theirs !== null
      ? { name: t(`compat.${cls}`), value: `${other.toUpperCase()} ${camelotString(theirs)} → ${deck.toUpperCase()} ${camelotString(mine)}` }
      : undefined,
  );
  if (!cls) return null;
  return (
    <span className="deck__compat" data-compat={cls} {...tip} tabIndex={0} role="img" aria-label={t(`compat.${cls}`)}>
      <Icon name={COMPAT_ICON[cls]} size="0.95rem" />
      <span className="dw-label">{t(`compat.${cls}.short`)}</span>
    </span>
  );
}

export function DeckPanel({ deck, side }: { deck: DeckId; side: 'left' | 'right' }) {
  const t = useT();
  const lang: Lang = useLang();
  const i = deckIndex(deck);
  const name = deck.toUpperCase();
  const other: DeckId = deck === 'a' ? 'b' : 'a';
  const color = deckColorVar(deck);
  const track = useUi((s) => s.deckTracks[deck]);
  const range = useUi((s) => s.tempoRange[deck]);
  const cueHeld = useUi((s) => s.cueHeld[deck]);
  const syncPref = useUi((s) => s.syncPref[deck]);
  const loaded = useSnap((s) => s.decks[i].loaded);
  const playing = useSnap((s) => s.decks[i].playing);
  const sync = useSnap((s) => s.decks[i].sync);
  const isMaster = useSnap((s) => s.decks[i].is_master);
  const tempoOffset = useSnap((s) => s.decks[i].tempo_offset);
  const drop = useTrackDrop(deck);
  const guided = sync !== 'off' && !isMaster;
  const keys = (a: Parameters<typeof shortcutKeys>[0]) => shortcutKeys(a, deck, lang);
  const sep = decimalSep(lang);
  const cut = side === 'right' ? 'tr' : 'tl';

  return (
    <section className="deck" data-side={side} data-deck={deck} data-over={drop.over ? 'true' : undefined} aria-label={t('deck.aria', { deck: name })} {...drop.handlers}>
      <header className="deck__head">
        <DeckPlate deck={deck} size="lg" cut={cut} />
        <div className="deck__titles">
          {loaded && track ? (
            <>
              <h2 className="deck__title dw-truncate" title={track.title}>
                {track.title}
              </h2>
              <p className="deck__artist dw-truncate" title={track.artist}>
                {track.artist}
              </p>
            </>
          ) : (
            <>
              <h2 className="deck__title deck__title--empty">{t('deck.empty')}</h2>
              <p className="deck__artist">{t('deck.emptyHint', { key: keys('deck.load').join(' + ') })}</p>
            </>
          )}
        </div>
        <div className="deck__keybox">
          {loaded && track && (
            <>
              <KeyChip musicalKey={track.key} lang={lang} size="lg" showName />
              <Compat deck={deck} other={other} t={t} />
            </>
          )}
        </div>
      </header>

      <div className="deck__numbers">
        <div className="deck__col">
          <Readout label={t('deck.bpm')} size="lg">
            <LiveText read={(s) => formatBpm(s.decks[i].bpm, lang)} />
          </Readout>
          <div className="deck__subline">
            <span className="dw-label">{t('deck.original')}</span>
            <span className="dw-num">{loaded && track ? formatBpm(track.bpm, lang) : formatBpm(0, lang)}</span>
            <LiveText className="dw-num" read={(s) => (s.decks[i].loaded ? formatPercent(s.decks[i].tempo_offset, lang) : `${formatNumber(0, 2, lang, true)} %`)} />
          </div>
        </div>
        <div className="deck__col">
          <Readout label={t('deck.bar')} size="md" tone="muted">
            <LiveText read={(s) => barText(deck, s)} />
          </Readout>
          <BeatDots deck={deck} />
        </div>
        <div className="deck__col" data-align="right">
          <Readout label={t('deck.remaining')} size="lg" align="right">
            <LiveText read={(s) => (s.decks[i].loaded ? formatRemaining(s.decks[i].pos, s.decks[i].duration, sep) : `−-:--${sep}-`)} />
          </Readout>
          <div className="deck__subline">
            <span className="dw-label">{t('deck.elapsed')}</span>
            <LiveText className="dw-num" read={(s) => (s.decks[i].loaded ? formatTime(s.decks[i].pos, sep) : `-:--${sep}-`)} />
          </div>
        </div>
      </div>

      <div className="deck__overview">
        <Overview deck={deck} ariaLabel={t('deck.overview', { deck: name })} />
        {!loaded && <span className="deck__overview-empty dw-label">{t('deck.dropHere', { deck: name })}</span>}
      </div>

      <div className="deck__run">
        <span className="dw-label">{t('deck.phase')}</span>
        <PhaseMeter
          read={(s) => (s.decks[i].loaded && s.clock_bpm > 0 && (s.decks[i].playing || s.decks[i].is_master) ? s.decks[i].phase : null)}
          ariaLabel={t('deck.phase')}
        />
        <LiveText
          className="deck__phaseval dw-num"
          read={(s) => (s.decks[i].loaded && s.clock_bpm > 0 && (s.decks[i].playing || s.decks[i].is_master) ? formatPhase(s.decks[i].phase, lang) : `±0${sep}00`)}
        />

        <span className="dw-label">{t('deck.tempo')}</span>
        <div className="deck__tempo">
          <KeyBtn
            mode="hold"
            size="sm"
            disabled={!loaded}
            onHoldStart={() => act.bend(deck, -1)}
            onHoldEnd={() => act.bend(deck, 0)}
            ariaLabel={t('deck.slower')}
            tip={{ name: t('deck.slower'), value: formatPercent(-act.BEND_AMOUNT, lang, 0), keys: keys('deck.bend_down') }}
          >
            <Icon name="slower" size="0.9em" />
          </KeyBtn>
          <Fader
            orientation="h"
            value={tempoOffsetToPos(tempoOffset, range)}
            onChange={(p) => act.setTempo(deck, tempoPosToOffset(p, range))}
            defaultValue={0.5}
            fillFrom={0.5}
            ticks={tempoTicks(range)}
            accent={color}
            disabled={!loaded}
            guided={guided}
            wheelStep={0.001 / (2 * range)}
            tip={{
              name: guided ? t('deck.tempoGuided') : t('deck.tempo'),
              value: formatPercent(tempoOffset, lang),
              keys: [...keys('deck.tempo_down'), ...keys('deck.tempo_up')],
            }}
          />
          <KeyBtn
            mode="hold"
            size="sm"
            cut="tr"
            disabled={!loaded}
            onHoldStart={() => act.bend(deck, 1)}
            onHoldEnd={() => act.bend(deck, 0)}
            ariaLabel={t('deck.faster')}
            tip={{ name: t('deck.faster'), value: formatPercent(act.BEND_AMOUNT, lang, 0), keys: keys('deck.bend_up') }}
          >
            <Icon name="faster" size="0.9em" />
          </KeyBtn>
        </div>
        <KeyBtn
          size="sm"
          cut="tr"
          onPress={() => act.cycleTempoRange(deck)}
          tip={{ name: t('deck.range'), value: `± ${Math.round(range * 100)} %` }}
          className="deck__range"
        >
          <span className="dw-num">±{Math.round(range * 100)}</span>
        </KeyBtn>
      </div>

      <div className="deck__transport">
        <KeyBtn
          mode="toggle"
          size="lg"
          active={playing && !cueHeld}
          disabled={!loaded}
          accent={color}
          onPress={() => act.togglePlay(deck)}
          tip={{ name: playing ? t('deck.pause') : t('deck.play'), keys: keys('deck.play') }}
          className="deck__play"
        >
          <Icon name={playing && !cueHeld ? 'pause' : 'play'} size="1em" />
          {playing && !cueHeld ? t('deck.pause') : t('deck.play')}
        </KeyBtn>
        <KeyBtn
          mode="hold"
          size="lg"
          disabled={!loaded}
          forceHeld={cueHeld}
          onHoldStart={() => act.cueDown(deck)}
          onHoldEnd={() => act.cueUp(deck)}
          tip={{ name: t('deck.cue'), value: formatTime(getUi().cuePoints[deck], sep), keys: keys('deck.cue') }}
        >
          {t('deck.cueShort')}
        </KeyBtn>
        <KeyBtn
          mode="toggle"
          size="lg"
          active={sync !== 'off'}
          disabled={!loaded}
          accent={color}
          onPress={() => act.toggleSync(deck)}
          tip={{ name: t('deck.sync'), value: t(sync === 'off' ? 'sync.off' : `sync.${sync}`), keys: keys('deck.sync') }}
        >
          {t('deck.syncShort')}
        </KeyBtn>
        <KeyBtn size="lg" disabled={!loaded} onPress={() => act.cycleSyncMode(deck)} tip={{ name: t('deck.syncMode'), value: t(`sync.${syncPref}`) }}>
          {t(`sync.${syncPref}`)}
        </KeyBtn>
        <KeyBtn
          mode="toggle"
          size="lg"
          cut="tr"
          active={isMaster}
          disabled={!loaded}
          accent={color}
          onPress={() => act.setMaster(deck)}
          tip={{ name: t('deck.master'), value: isMaster ? t('deck.masterIs') : t('deck.masterSet') }}
          className="deck__master"
        >
          {t('deck.masterShort')}
        </KeyBtn>
      </div>
    </section>
  );
}
