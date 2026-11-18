// Deck (8.2): Kennung außen, Vergleichswerte (Tonart, Verträglichkeit, BPM) innen am Mixer,
// darunter Übersicht, Gleichlauf-Block (Phase über Tempo) und Transport in fester Reihenfolge.
import { useRef } from 'react';
import * as act from '../../app/actions';
import { getDeckAssets } from '../../app/assets';
import { useTrackDrop } from '../../app/dnd';
import { useFrame, useSnap } from '../../app/frame';
import { shortcutKeys } from '../../app/keymap';
import { getUi, useUi } from '../../app/store';
import { deckIndex, type DeckId, type Snapshot } from '../../core/types';
import { CutSide, DeckPlate, Fader, Icon, KeyBtn, KeyChip, LiveText, PhaseMeter, ValueField, useTip, type FaderTick } from '../../design/components';
import { deckColorVar } from '../../design/tokens';
import { useLang, useT, type TFn } from '../../i18n';
import { tempoOffsetToPos, tempoPosToOffset } from '../../lib/curves';
import { decimalSep, formatBpm, formatNumber, formatPercent, formatPhase, type Lang } from '../../lib/format';
import { barBeat } from '../../lib/grid';
import { camelotString, compatClass, harmonicCompat, type CompatClass } from '../../lib/key';
import { parseNumber } from '../../lib/parse';
import { formatTime } from '../../lib/time';
import { Overview } from './Overview';

function tempoTicks(range: number): FaderTick[] {
  // Bis ±10 % ein Strich je Prozent, darüber acht Teile je Seite.
  const step = range <= 0.1 + 1e-9 ? 0.01 : range / 8;
  const n = Math.round(range / step);
  const out: FaderTick[] = [];
  for (let i = -n; i <= n; i++) out.push({ at: 0.5 + (i / n) * 0.5, major: i === 0 || Math.abs(i) === n || (n % 2 === 0 && Math.abs(i) === n / 2) });
  return out;
}

const deckBarBeat = (deck: DeckId, s: Snapshot) => {
  const d = s.decks[deckIndex(deck)];
  const a = getDeckAssets(deck);
  return d.loaded && a ? barBeat(a.analysis.grid, d.beat) : null;
};

/** Phase gilt nur, wenn eine Clock läuft und das Deck spielt oder sie vorgibt. */
const phaseOf = (deck: DeckId, s: Snapshot): number | null => {
  const d = s.decks[deckIndex(deck)];
  return d.loaded && s.clock_bpm > 0 && (d.playing || d.is_master) ? d.phase : null;
};

/** Restzeit in Sekunden, auf Zehntel abgeschnitten. */
const restTenths = (deck: DeckId, s: Snapshot): number => {
  const d = s.decks[deckIndex(deck)];
  return Math.floor(Math.max(0, d.duration - d.pos) * 10 + 1e-6);
};

/** Schlag im Takt als vier Lamellen; die Eins ist breiter. */
function BeatDots({ deck }: { deck: DeckId }) {
  const el = useRef<HTMLDivElement | null>(null);
  const last = useRef('');
  useFrame((s) => {
    const bb = deckBarBeat(deck, s);
    const v = bb ? String(bb.beat) : '0';
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

/** Große Zahl mit kleinerem, gedämpftem Anhang (Zehntel bzw. Schlag): Der unruhige Teil tritt zurück. */
function BigNumber({ main, tail, empty, label }: { main: (s: Snapshot) => string; tail?: (s: Snapshot) => string; empty: boolean; label: string }) {
  if (empty) {
    return (
      <span className="deck__big deck__big--empty dw-num" aria-label={label}>
        –
      </span>
    );
  }
  return (
    <span className="deck__big dw-num" aria-label={label}>
      <LiveText read={main} />
      {tail && <LiveText className="deck__tail" read={tail} />}
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
  const focused = useUi((s) => s.focusDeck === deck);
  const armed = useUi((s) => s.armedId);
  const loadingTrack = useUi((s) => s.loading[deck]);
  const loaded = useSnap((s) => s.decks[i].loaded);
  const playing = useSnap((s) => s.decks[i].playing);
  const sync = useSnap((s) => s.decks[i].sync);
  const isMaster = useSnap((s) => s.decks[i].is_master);
  const tempoOffset = useSnap((s) => s.decks[i].tempo_offset);
  const drop = useTrackDrop(deck);
  const guided = sync !== 'off' && !isMaster;
  const shownRange = act.shownTempoRange(range, tempoOffset);
  const keys = (a: Parameters<typeof shortcutKeys>[0]) => shortcutKeys(a, deck, lang);
  const sep = decimalSep(lang);
  const running = playing && !cueHeld;
  const phaseTip = useTip({ name: t('deck.phaseLong') });
  const dash = '–';

  return (
    <CutSide value={side === 'right' ? 'tr' : 'tl'}>
      <section
        className="deck"
        data-side={side}
        data-deck={deck}
        data-focus={focused ? 'true' : undefined}
        data-over={drop.over ? 'true' : undefined}
        aria-label={t('deck.aria', { deck: name })}
        onPointerDownCapture={() => act.focusDeck(deck)}
        {...drop.handlers}
      >
        <header className="deck__head">
          <DeckPlate deck={deck} size="lg" />
          <div className="deck__titles">
            {loadingTrack ? (
              <>
                <h2 className="deck__title dw-truncate" title={loadingTrack.title}>
                  {loadingTrack.title}
                </h2>
                <p className="deck__artist deck__loading" role="status">
                  <span className="deck__loadbar" aria-hidden="true" />
                  {t('deck.loadingLong')}
                </p>
              </>
            ) : loaded && track ? (
              <>
                <h2 className="deck__title dw-truncate" title={track.title}>
                  {track.title}
                </h2>
                <p className="deck__artist dw-truncate" title={track.artist}>
                  {track.artist}
                </p>
              </>
            ) : (
              <h2 className="deck__title deck__title--empty">{t('deck.empty')}</h2>
            )}
          </div>
          <div className="deck__keybox">
            {loaded && track && !loadingTrack && (
              <>
                <KeyChip musicalKey={track.key} lang={lang} size="lg" showName />
                <Compat deck={deck} other={other} t={t} />
              </>
            )}
          </div>
        </header>

        <div className="deck__numbers">
          <div className="deck__col" data-role="rest">
            <BigNumber
              empty={!loaded}
              label={t('deck.remaining')}
              main={(s) => {
                const r = restTenths(deck, s);
                return `−${Math.floor(r / 600)}:${String(Math.floor((r % 600) / 10)).padStart(2, '0')}`;
              }}
              tail={(s) => `${sep}${restTenths(deck, s) % 10}`}
            />
            <div className="deck__subline">
              <span className="dw-label">{t('deck.remaining')}</span>
              <span className="deck__dot" aria-hidden="true" />
              <span className="dw-label">{t('deck.elapsedShort')}</span>
              {loaded ? <LiveText className="dw-num" read={(s) => formatTime(s.decks[i].pos, sep)} /> : <span className="dw-num">{dash}</span>}
            </div>
          </div>
          <div className="deck__col" data-role="bar">
            <span className="deck__mid dw-num" aria-label={t('deck.bar')}>
              {loaded ? (
                <>
                  <LiveText read={(s) => String(deckBarBeat(deck, s)?.bar ?? dash)} />
                  <LiveText className="deck__tail" read={(s) => `·${deckBarBeat(deck, s)?.beat ?? ''}`} />
                </>
              ) : (
                <span className="deck__big--empty">{dash}</span>
              )}
            </span>
            <BeatDots deck={deck} />
            <div className="deck__subline">
              <span className="dw-label">{t('deck.bar')}</span>
            </div>
          </div>
          <div className="deck__col" data-role="bpm">
            <span className="deck__bpm">
              <BigNumber empty={!loaded} label={t('deck.bpm')} main={(s) => formatBpm(s.decks[i].bpm, lang)} />
              <span className="dw-label">{t('deck.bpm')}</span>
            </span>
            <div className="deck__subline">
              <span className="dw-label">{t('deck.originalShort')}</span>
              <span className="dw-num">{loaded && track && track.bpm > 0 ? formatBpm(track.bpm, lang) : dash}</span>
              {loaded ? (
                <ValueField
                  className="deck__offset"
                  text={formatPercent(tempoOffset, lang)}
                  editText={formatNumber(tempoOffset * 100, 2, lang, true)}
                  ariaLabel={t('deck.tempo')}
                  disabled={guided}
                  tip={{ name: guided ? t('deck.tempoGuided') : t('deck.tempoEnter') }}
                  onCommit={(text) => {
                    const v = parseNumber(text);
                    if (v === null || v === -Infinity) return false;
                    act.setTempo(deck, Math.max(-range, Math.min(range, v / 100)));
                    return true;
                  }}
                />
              ) : null}
            </div>
          </div>
        </div>

        <div className="deck__overview">
          <Overview deck={deck} ariaLabel={t('deck.overview', { deck: name })} />
        </div>

        <div className="deck__run">
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
          <div className="deck__sync">
            <div className="deck__phase" {...phaseTip}>
              <PhaseMeter read={(s) => phaseOf(deck, s)} ariaLabel={t('deck.phase')} />
            </div>
            <Fader
              orientation="h"
              value={tempoOffsetToPos(tempoOffset, shownRange)}
              onChange={(p) => act.setTempo(deck, tempoPosToOffset(p, shownRange))}
              defaultValue={0.5}
              fillFrom={0.5}
              ticks={tempoTicks(shownRange)}
              accent={color}
              disabled={!loaded}
              guided={guided}
              wheelStep={0.001 / (2 * shownRange)}
              tip={{
                name: guided ? t('deck.tempoGuided') : t('deck.tempo'),
                value: formatPercent(tempoOffset, lang),
                keys: [...keys('deck.tempo_down'), ...keys('deck.tempo_up')],
              }}
            />
          </div>
          <KeyBtn
            mode="hold"
            size="sm"
            disabled={!loaded}
            onHoldStart={() => act.bend(deck, 1)}
            onHoldEnd={() => act.bend(deck, 0)}
            ariaLabel={t('deck.faster')}
            tip={{ name: t('deck.faster'), value: formatPercent(act.BEND_AMOUNT, lang, 0), keys: keys('deck.bend_up') }}
          >
            <Icon name="faster" size="0.9em" />
          </KeyBtn>
          <KeyBtn
            size="sm"
            // Klick schaltet weiter; Rechtsklick, Shift-Klick oder Mausrad nach unten schalten zurück.
            onPress={() => act.cycleTempoRange(deck, 1)}
            onContextMenu={(e) => {
              e.preventDefault();
              act.cycleTempoRange(deck, -1);
            }}
            onWheel={(e) => act.cycleTempoRange(deck, e.deltaY > 0 ? -1 : 1)}
            tip={{ name: t('deck.range'), value: `± ${Math.round(shownRange * 100)} % · ${t('deck.rangeHint')}` }}
            className="deck__range"
          >
            <span className="dw-num">±{Math.round(shownRange * 100)}</span>
          </KeyBtn>
          <span className="deck__phaseval">
            <span className="dw-label">{t('deck.phase')}</span>
            {loaded ? (
              <LiveText
                className="dw-num"
                read={(s) => {
                  const p = phaseOf(deck, s);
                  return p === null ? dash : formatPhase(p, lang);
                }}
              />
            ) : (
              <span className="dw-num">{dash}</span>
            )}
          </span>
        </div>

        <div className="deck__transport">
          <KeyBtn
            size="md"
            disabled={!loaded || playing}
            onPress={() => act.toStart(deck)}
            ariaLabel={t('deck.toStart')}
            tip={{ name: t('deck.toStart'), value: playing ? t('deck.toStartLocked') : undefined }}
          >
            <Icon name="tostart" size="0.95em" />
          </KeyBtn>
          <KeyBtn
            mode="hold"
            size="md"
            disabled={!loaded}
            forceHeld={cueHeld}
            armed={armed === `cue.${deck}`}
            onHoldStart={() => act.cueDown(deck)}
            onHoldEnd={() => act.cueUp(deck)}
            tip={{ name: t('deck.cue'), value: `${t('deck.cuePoint')} ${formatTime(getUi().cuePoints[deck], sep)}`, keys: keys('deck.cue') }}
          >
            {t('deck.cueShort')}
          </KeyBtn>
          <KeyBtn
            mode="toggle"
            tone="outline"
            size="md"
            active={running}
            disabled={!loaded}
            accent={color}
            armed={armed === `stop.${deck}`}
            onHoldStart={() => act.playDown(deck)}
            onHoldEnd={() => act.playUp(deck)}
            tip={{
              name: running ? t('deck.playStop') : t('deck.playStart'),
              value: focused ? t('deck.focusHint') : undefined,
              keys: focused ? [...keys('deck.play'), '␣'] : keys('deck.play'),
            }}
            className="deck__play"
          >
            <Icon name="play" size="0.95em" />
            {t('deck.play')}
          </KeyBtn>
          <span className="deck__fuge" aria-hidden="true" />
          <div className="deck__syncgroup">
            <KeyBtn
              mode="toggle"
              size="md"
              active={sync !== 'off'}
              disabled={!loaded}
              accent={color}
              onPress={() => act.toggleSync(deck)}
              tip={{ name: t('deck.sync'), value: t(sync === 'off' ? 'sync.off' : `sync.${sync}`), keys: keys('deck.sync') }}
            >
              {t('deck.syncShort')}
            </KeyBtn>
            <KeyBtn size="md" disabled={!loaded} onPress={() => act.cycleSyncMode(deck)} tip={{ name: t('deck.syncMode'), value: t(`sync.${syncPref}`) }} className="deck__mode">
              {t(`sync.${syncPref}`)}
            </KeyBtn>
          </div>
          <KeyBtn
            mode="toggle"
            size="md"
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
    </CutSide>
  );
}

/** Deck als eine Zeile (Bibliothek aufgezogen, 8.2): Kennung, Titel, Übersicht, BPM, Tonart, Restzeit. */
export function MiniDeck({ deck, side }: { deck: DeckId; side: 'left' | 'right' }) {
  const t = useT();
  const lang = useLang();
  const i = deckIndex(deck);
  const name = deck.toUpperCase();
  const track = useUi((s) => s.deckTracks[deck]);
  const focused = useUi((s) => s.focusDeck === deck);
  const loaded = useSnap((s) => s.decks[i].loaded);
  const playing = useSnap((s) => s.decks[i].playing);
  const drop = useTrackDrop(deck);
  const sep = decimalSep(lang);
  return (
    <CutSide value={side === 'right' ? 'tr' : 'tl'}>
      <section
        className="minideck"
        data-deck={deck}
        data-side={side}
        data-focus={focused ? 'true' : undefined}
        data-over={drop.over ? 'true' : undefined}
        data-playing={playing ? 'true' : undefined}
        aria-label={t('deck.aria', { deck: name })}
        onPointerDownCapture={() => act.focusDeck(deck)}
        {...drop.handlers}
      >
        <DeckPlate deck={deck} />
        <span className="minideck__title dw-truncate">{loaded && track ? track.title : t('deck.empty')}</span>
        <div className="minideck__overview">
          <Overview deck={deck} ariaLabel={t('deck.overview', { deck: name })} />
        </div>
        <span className="minideck__num dw-num">{loaded ? <LiveText read={(s) => formatBpm(s.decks[i].bpm, lang)} /> : '–'}</span>
        <KeyChip musicalKey={loaded && track ? track.key : null} lang={lang} />
        <span className="minideck__num minideck__rest dw-num">
          {loaded ? (
            <LiveText
              read={(s) => {
                const r = restTenths(deck, s);
                return `−${Math.floor(r / 600)}:${String(Math.floor((r % 600) / 10)).padStart(2, '0')}${sep}${r % 10}`;
              }}
            />
          ) : (
            '–'
          )}
        </span>
      </section>
    </CutSide>
  );
}
