// Bibliothek in der Ansicht „Mixen": Suche, sortierbare Spalten, Laden auf Deck A/B
// per Lade-Taste, Shift + Pfeil oder Drag-and-drop; Doppelklick lädt nur in ein leeres Deck (9.3).
import { useEffect, useMemo, useRef, useState } from 'react';
import { loadDown, loadToFreeDeck, loadUp, refreshLibrary, setVisibleTrackIds, toggleLibraryBig } from '../../app/actions';
import { beginTrackDrag, useDraggedTrack } from '../../app/dnd';
import { shortcutKeys } from '../../app/keymap';
import { setUi, useUi, type SortKey } from '../../app/store';
import type { DeckId, TrackInfo } from '../../core/types';
import { Cell, Icon, KeyBtn, KeyChip, ListRow, useTip } from '../../design/components';
import { useLang, useT, type MsgKey } from '../../i18n';
import { useSnap } from '../../app/frame';
import { formatBpm, type Lang } from '../../lib/format';
import { camelotSortValue, camelotString, harmonicCompat } from '../../lib/key';
import { formatDuration } from '../../lib/time';

// Titel und Künstler sind begrenzt, BPM und Tonart schließen direkt an; der Rest bleibt für spätere Spalten frei.
const COLUMNS = '5.4rem minmax(8rem, 24rem) minmax(6rem, 18rem) 4.6rem 10.5rem 3.6rem minmax(0, 1fr)';
const HEADS: { key: SortKey; label: MsgKey; num?: boolean }[] = [
  { key: 'title', label: 'lib.title' },
  { key: 'artist', label: 'lib.artist' },
  { key: 'bpm', label: 'lib.bpm', num: true },
  { key: 'key', label: 'lib.key' },
  { key: 'duration', label: 'lib.duration', num: true },
];

const fold = (s: string): string => s.normalize('NFD').replace(/[̀-ͯ]/g, '').toLowerCase();

/** Suche über Titel, Künstler, BPM und Tonart; mehrere Wörter müssen alle vorkommen. */
export function filterTracks(tracks: TrackInfo[], query: string): TrackInfo[] {
  const words = fold(query).split(/\s+/).filter(Boolean);
  if (words.length === 0) return tracks;
  return tracks.filter((tr) => {
    const hay = fold(`${tr.title} ${tr.artist} ${tr.bpm > 0 ? tr.bpm : ''} ${tr.key === null ? '' : camelotString(tr.key)}`);
    return words.every((w) => hay.includes(w));
  });
}

/** Größter Tempo-Abstand, der noch als passend gilt (wie in den Übergängen, 5.10: Δ ≤ 6 %). */
export const MATCH_TEMPO = 0.06;
/** Kleinste harmonische Verträglichkeit K (5.11), die noch als passend gilt: bis „diagonal" und „+2". */
export const MATCH_KEY = 0.6;

/**
 * Filter „passt zum laufenden Track" (BIB-11, vereinfacht): Tempo höchstens `MATCH_TEMPO` neben dem
 * Bezugs-Track und Tonart mit K ≥ `MATCH_KEY` für den Wechsel vom Bezugs-Track aus. Verglichen werden
 * die analysierten Werte der Tracks, nicht das gerade gespielte Tempo. Der Bezugs-Track selbst und
 * Tracks ohne Tempo oder Tonart fallen heraus.
 */
export function matchTracks(tracks: TrackInfo[], ref: Pick<TrackInfo, 'id' | 'bpm' | 'key'> | null): TrackInfo[] {
  if (!ref || !(ref.bpm > 0) || ref.key === null) return tracks;
  const refKey = ref.key;
  return tracks.filter((tr) => tr.id !== ref.id && tr.bpm > 0 && tr.key !== null && Math.abs(tr.bpm / ref.bpm - 1) <= MATCH_TEMPO + 1e-9 && harmonicCompat(refKey, tr.key) >= MATCH_KEY);
}

/** Bezugs-Deck für den Filter: der Master, sonst das laufende, sonst das erste geladene Deck. */
export function matchDeck(master: DeckId | null, loaded: Record<'a' | 'b', boolean>, playing: Record<'a' | 'b', boolean>): 'a' | 'b' | null {
  if ((master === 'a' || master === 'b') && loaded[master]) return master;
  if (playing.a && loaded.a) return 'a';
  if (playing.b && loaded.b) return 'b';
  return loaded.a ? 'a' : loaded.b ? 'b' : null;
}

/** Sortieren; Tracks ohne Tempo bzw. Tonart (noch nicht analysiert) stehen immer am Ende. */
export function sortTracks(tracks: TrackInfo[], key: SortKey, dir: 1 | -1, lang: Lang): TrackInfo[] {
  const unknown = (t: TrackInfo): boolean => (key === 'bpm' ? !(t.bpm > 0) : key === 'key' ? t.key === null : false);
  const cmp = (a: TrackInfo, b: TrackInfo): number => {
    if (key === 'title' || key === 'artist') return a[key].localeCompare(b[key], lang) || a.title.localeCompare(b.title, lang);
    if (key === 'key') return camelotSortValue(a.key) - camelotSortValue(b.key) || a.bpm - b.bpm;
    return a[key] - b[key] || a.title.localeCompare(b.title, lang);
  };
  return [...tracks].sort((a, b) => {
    const ua = unknown(a);
    const ub = unknown(b);
    if (ua !== ub) return ua ? 1 : -1;
    return cmp(a, b) * dir;
  });
}

function LoadBtn({ deck, track, loaded }: { deck: DeckId; track: TrackInfo; loaded: boolean }) {
  const t = useT();
  const lang = useLang();
  const name = deck.toUpperCase();
  const tip = useTip({
    name: track.missing ? t('lib.missing') : loaded ? t('lib.loadedOn', { deck: name }) : t('lib.loadTo', { deck: name }),
    keys: track.missing ? [] : shortcutKeys('deck.load', deck, lang),
  });
  return (
    <button
      type="button"
      className="lib__load"
      data-deck={deck}
      data-loaded={loaded ? 'true' : undefined}
      disabled={track.missing}
      aria-label={t('lib.loadTo', { deck: name })}
      // Nicht in der Tab-Kette: Die Zeile ist per ↑/↓ erreichbar, geladen wird mit Shift + ←/→.
      tabIndex={-1}
      onMouseDown={(e) => e.preventDefault()}
      // Drücken und Loslassen getrennt: Ein hörbares Deck wird nur nach Halten oder zweitem Druck ersetzt.
      onPointerDown={(e) => {
        if (e.button !== 0) return;
        e.stopPropagation();
        e.currentTarget.setPointerCapture(e.pointerId);
        loadDown(deck, track.id);
      }}
      onPointerUp={() => loadUp(deck)}
      onPointerCancel={() => loadUp(deck)}
      onClick={(e) => e.stopPropagation()}
      onDoubleClick={(e) => e.stopPropagation()}
      {...tip}
    >
      <span>{name}</span>
    </button>
  );
}

/** Musikordner nach neuen Dateien durchsuchen; während des Laufs gesperrt. */
function RefreshButton() {
  const t = useT();
  const [busy, setBusy] = useState(false);
  const run = () => {
    if (busy) return;
    setBusy(true);
    void refreshLibrary().finally(() => setBusy(false));
  };
  return (
    <KeyBtn size="sm" disabled={busy} onPress={run} tip={{ name: t('lib.refresh'), value: t('lib.refreshHint') }} ariaLabel={t('lib.refresh')}>
      {busy ? <span className="lib__spinner" aria-hidden="true" /> : null}
      {t('lib.refreshShort')}
    </KeyBtn>
  );
}

function SortHead({ head, sortKey, sortDir, onSort }: { head: (typeof HEADS)[number]; sortKey: SortKey; sortDir: 1 | -1; onSort: (k: SortKey) => void }) {
  const t = useT();
  const active = head.key === sortKey;
  const tip = useTip({ name: t('lib.sortBy', { column: t(head.label) }), value: active ? t(sortDir === 1 ? 'lib.sortAsc' : 'lib.sortDesc') : undefined });
  return (
    <button
      type="button"
      role="columnheader"
      className="lib__sort dw-label"
      data-num={head.num ? 'true' : undefined}
      data-active={active ? 'true' : undefined}
      aria-sort={active ? (sortDir === 1 ? 'ascending' : 'descending') : 'none'}
      onMouseDown={(e) => e.preventDefault()}
      onClick={() => onSort(head.key)}
      {...tip}
    >
      {t(head.label)}
      <Icon name={active && sortDir === -1 ? 'down' : 'up'} size="0.6rem" style={{ visibility: active ? 'visible' : 'hidden' }} />
    </button>
  );
}

export function LibraryPanel() {
  const t = useT();
  const lang = useLang();
  const tracks = useUi((s) => s.tracks);
  const ready = useUi((s) => s.tracksReady);
  const search = useUi((s) => s.search);
  const sortKey = useUi((s) => s.sortKey);
  const sortDir = useUi((s) => s.sortDir);
  const selectedId = useUi((s) => s.selectedId);
  const pending = useUi((s) => s.pendingAnalysis);
  const big = useUi((s) => s.libraryBig);
  const onA = useUi((s) => s.deckTracks.a?.id ?? null);
  const onB = useUi((s) => s.deckTracks.b?.id ?? null);
  const matchOn = useUi((s) => s.matchFilter);
  const trackA = useUi((s) => s.deckTracks.a);
  const trackB = useUi((s) => s.deckTracks.b);
  const master = useSnap((s) => s.master);
  const playingA = useSnap((s) => s.decks[0].playing);
  const playingB = useSnap((s) => s.decks[1].playing);
  const refDeck = matchDeck(master, { a: trackA !== null, b: trackB !== null }, { a: playingA, b: playingB });
  const ref = refDeck === 'a' ? trackA : refDeck === 'b' ? trackB : null;
  const refUsable = ref !== null && ref.bpm > 0 && ref.key !== null;
  const dragId = useDraggedTrack();
  const body = useRef<HTMLDivElement | null>(null);

  const rows = useMemo(
    () => sortTracks(filterTracks(matchOn && refUsable ? matchTracks(tracks, ref) : tracks, search), sortKey, sortDir, lang),
    [tracks, search, sortKey, sortDir, lang, matchOn, refUsable, ref],
  );

  useEffect(() => {
    setVisibleTrackIds(rows.map((r) => r.id));
  }, [rows]);

  // Auswahl sichtbar halten, wenn sie per Tastatur wandert.
  useEffect(() => {
    if (!selectedId) return;
    body.current?.querySelector<HTMLElement>(`[data-track="${selectedId}"]`)?.scrollIntoView({ block: 'nearest' });
  }, [selectedId]);

  const sortBy = (key: SortKey) => setUi(key === sortKey ? { sortDir: sortDir === 1 ? -1 : 1 } : { sortKey: key, sortDir: 1 });
  const empty = ready && tracks.length === 0;

  return (
    <section className="lib" aria-label={t('lib.aria')}>
      <div className="lib__bar">
        <span className="dw-label lib__name">{t('lib.heading')}</span>
        <label className="lib__search">
          <Icon name="search" size="0.95rem" />
          <input
            data-search-input
            type="search"
            value={search}
            placeholder={t('lib.search')}
            aria-label={t('lib.search')}
            spellCheck={false}
            autoComplete="off"
            disabled={empty}
            onChange={(e) => setUi({ search: e.target.value })}
          />
          <span className="lib__searchkeys">
            {shortcutKeys('search.focus', undefined, lang).map((k) => (
              <kbd className="dw-kbd" key={k}>
                {k}
              </kbd>
            ))}
          </span>
        </label>
        <span className="lib__count dw-num">{t('lib.count', { shown: rows.length, total: tracks.length })}</span>
        {pending > 0 && (
          <span className="lib__analysis" role="status">
            <span className="lib__spinner" aria-hidden="true" />
            {t(pending === 1 ? 'lib.analysisOne' : 'lib.analysis', { n: pending })}
          </span>
        )}
        <span className="lib__hint">{t('lib.hint')}</span>
        <KeyBtn
          mode="toggle"
          size="sm"
          active={matchOn && refUsable}
          disabled={!refUsable}
          onPress={() => setUi({ matchFilter: !matchOn })}
          tip={{
            name: t('lib.match'),
            value:
              refUsable && ref && ref.key !== null && refDeck
                ? t('lib.matchHint', { deck: refDeck.toUpperCase(), bpm: formatBpm(ref.bpm, lang, 1), pct: Math.round(MATCH_TEMPO * 100), key: camelotString(ref.key) })
                : t('lib.matchNoRef'),
          }}
          ariaLabel={t('lib.match')}
        >
          {t('lib.matchShort')}
        </KeyBtn>
        <RefreshButton />
        <KeyBtn
          mode="toggle"
          size="sm"
          active={big}
          onPress={toggleLibraryBig}
          tip={{ name: big ? t('lib.shrink') : t('lib.grow'), value: t('lib.growHint') }}
          ariaLabel={big ? t('lib.shrink') : t('lib.grow')}
        >
          <Icon name={big ? 'collapse' : 'expand'} size="0.9em" />
          {t('lib.growShort')}
        </KeyBtn>
      </div>

      {empty ? (
        <div className="lib__empty">
          <Icon name="drop" size="2.6rem" />
          <div>
            <h3 className="lib__emptytitle">{t('lib.emptyTitle')}</h3>
            <p className="lib__emptytext">{t('lib.emptyText')}</p>
          </div>
        </div>
      ) : (
        <div className="lib__table" role="table" aria-rowcount={rows.length + 1} ref={body}>
          <ListRow columns={COLUMNS} head>
            <span className="dw-label" role="columnheader">
              {t('lib.load')}
            </span>
            {HEADS.map((h) => (
              <SortHead key={h.key} head={h} sortKey={sortKey} sortDir={sortDir} onSort={sortBy} />
            ))}
            <span />
          </ListRow>
          {rows.map((tr, n) => (
            <ListRow
              key={tr.id}
              columns={COLUMNS}
              selected={tr.id === selectedId}
              dragging={tr.id === dragId}
              dim={tr.missing}
              zebra={n % 2 === 1}
              data-track={tr.id}
              onPointerDown={(e) => {
                if (!tr.missing) beginTrackDrag(e, tr.id, tr.artist ? `${tr.artist} – ${tr.title}` : tr.title);
              }}
              onClick={() => setUi({ selectedId: tr.id })}
              onDoubleClick={() => loadToFreeDeck(tr.id)}
            >
              <span className="lib__loads" role="cell">
                <LoadBtn deck="a" track={tr} loaded={onA === tr.id} />
                <LoadBtn deck="b" track={tr} loaded={onB === tr.id} />
              </span>
              <Cell strong>{tr.title}</Cell>
              <Cell>{tr.artist}</Cell>
              <Cell num>{tr.bpm > 0 ? formatBpm(tr.bpm, lang, 1) : '–'}</Cell>
              <span role="cell" className="lib__keycell">
                {tr.missing ? (
                  <span className="lib__state" data-state="missing">
                    <Icon name="warn" size="0.9em" />
                    {t('lib.missing')}
                  </span>
                ) : tr.incomplete ? (
                  <span className="lib__state" data-state="incomplete" title={t('lib.incompleteHint')}>
                    <Icon name="warn" size="0.9em" />
                    {t('lib.incomplete')}
                  </span>
                ) : !tr.analyzed ? (
                  <span className="lib__state" data-state="pending">
                    <span className="lib__spinner" aria-hidden="true" />
                    {t('lib.pending')}
                  </span>
                ) : (
                  <KeyChip musicalKey={tr.key} lang={lang} showName />
                )}
              </span>
              <Cell num>{tr.duration > 0 ? formatDuration(tr.duration) : '–'}</Cell>
              <span />
            </ListRow>
          ))}
          {rows.length === 0 && <div className="lib__none">{matchOn && refUsable && search.trim() === '' ? t('lib.noneMatch') : t('lib.none', { query: search })}</div>}
        </div>
      )}
    </section>
  );
}
