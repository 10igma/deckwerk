// Bibliothek in der Ansicht „Mixen“: Suche, sortierbare Spalten, Laden auf Deck A/B
// per Doppelklick, Taste oder Drag-and-drop (DECK-04).
import { useEffect, useMemo, useRef, useState } from 'react';
import { loadToFreeDeck, loadTrack, setVisibleTrackIds } from '../../app/actions';
import { startTrackDrag } from '../../app/dnd';
import { shortcutKeys } from '../../app/keymap';
import { setUi, useUi, type SortKey } from '../../app/store';
import type { DeckId, TrackInfo } from '../../core/types';
import { Cell, Icon, KeyChip, ListRow, useTip } from '../../design/components';
import { useLang, useT, type MsgKey } from '../../i18n';
import { formatBpm, type Lang } from '../../lib/format';
import { camelotSortValue, camelotString } from '../../lib/key';
import { formatDuration } from '../../lib/time';

const COLUMNS = '3.9rem minmax(8rem, 2.4fr) minmax(6rem, 1.5fr) 5rem 9rem 3.6rem';
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
    const hay = fold(`${tr.title} ${tr.artist} ${tr.bpm} ${tr.key === null ? '' : camelotString(tr.key)}`);
    return words.every((w) => hay.includes(w));
  });
}

export function sortTracks(tracks: TrackInfo[], key: SortKey, dir: 1 | -1, lang: Lang): TrackInfo[] {
  const cmp = (a: TrackInfo, b: TrackInfo): number => {
    if (key === 'title' || key === 'artist') return a[key].localeCompare(b[key], lang) || a.title.localeCompare(b.title, lang);
    if (key === 'key') return camelotSortValue(a.key) - camelotSortValue(b.key) || a.bpm - b.bpm;
    return a[key] - b[key] || a.title.localeCompare(b.title, lang);
  };
  return [...tracks].sort((a, b) => cmp(a, b) * dir);
}

function LoadBtn({ deck, track, loaded }: { deck: DeckId; track: TrackInfo; loaded: boolean }) {
  const t = useT();
  const lang = useLang();
  const tip = useTip({ name: loaded ? t('lib.loadedOn', { deck: deck.toUpperCase() }) : t('lib.loadTo', { deck: deck.toUpperCase() }), keys: shortcutKeys('deck.load', deck, lang) });
  return (
    <button
      type="button"
      className="lib__load"
      data-deck={deck}
      data-loaded={loaded ? 'true' : undefined}
      aria-label={t('lib.loadTo', { deck: deck.toUpperCase() })}
      onMouseDown={(e) => e.preventDefault()}
      onClick={(e) => {
        e.stopPropagation();
        void loadTrack(deck, track.id);
      }}
      onDoubleClick={(e) => e.stopPropagation()}
      {...tip}
    >
      {deck.toUpperCase()}
    </button>
  );
}

export function LibraryPanel() {
  const t = useT();
  const lang = useLang();
  const tracks = useUi((s) => s.tracks);
  const search = useUi((s) => s.search);
  const sortKey = useUi((s) => s.sortKey);
  const sortDir = useUi((s) => s.sortDir);
  const selectedId = useUi((s) => s.selectedId);
  const onA = useUi((s) => s.deckTracks.a?.id ?? null);
  const onB = useUi((s) => s.deckTracks.b?.id ?? null);
  const [dragId, setDragId] = useState<string | null>(null);
  const body = useRef<HTMLDivElement | null>(null);

  const rows = useMemo(() => sortTracks(filterTracks(tracks, search), sortKey, sortDir, lang), [tracks, search, sortKey, sortDir, lang]);

  useEffect(() => {
    setVisibleTrackIds(rows.map((r) => r.id));
  }, [rows]);

  // Auswahl sichtbar halten, wenn sie per Tastatur wandert.
  useEffect(() => {
    if (!selectedId) return;
    body.current?.querySelector<HTMLElement>(`[data-track="${selectedId}"]`)?.scrollIntoView({ block: 'nearest' });
  }, [selectedId]);

  const sortBy = (key: SortKey) => setUi(key === sortKey ? { sortDir: sortDir === 1 ? -1 : 1 } : { sortKey: key, sortDir: 1 });

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
        <span className="lib__hint">{t('lib.hint')}</span>
      </div>

      <div className="lib__table" role="table" aria-rowcount={rows.length + 1}>
        <ListRow columns={COLUMNS} head>
          <span className="dw-label" role="columnheader">
            {t('lib.load')}
          </span>
          {HEADS.map((h) => (
            <button
              key={h.key}
              type="button"
              role="columnheader"
              className="lib__sort dw-label"
              data-num={h.num ? 'true' : undefined}
              data-active={h.key === sortKey ? 'true' : undefined}
              aria-sort={h.key === sortKey ? (sortDir === 1 ? 'ascending' : 'descending') : 'none'}
              onMouseDown={(e) => e.preventDefault()}
              onClick={() => sortBy(h.key)}
            >
              {t(h.label)}
              <Icon name={h.key === sortKey && sortDir === -1 ? 'down' : 'up'} size="0.6rem" style={{ visibility: h.key === sortKey ? 'visible' : 'hidden' }} />
            </button>
          ))}
        </ListRow>
        <div className="lib__body" ref={body}>
          {rows.map((tr) => (
            <ListRow
              key={tr.id}
              columns={COLUMNS}
              selected={tr.id === selectedId}
              dragging={tr.id === dragId}
              data-track={tr.id}
              draggable
              onDragStart={(e) => {
                startTrackDrag(e, tr.id);
                setDragId(tr.id);
              }}
              onDragEnd={() => setDragId(null)}
              onClick={() => setUi({ selectedId: tr.id })}
              onDoubleClick={() => loadToFreeDeck(tr.id)}
            >
              <span className="lib__loads" role="cell">
                <LoadBtn deck="a" track={tr} loaded={onA === tr.id} />
                <LoadBtn deck="b" track={tr} loaded={onB === tr.id} />
              </span>
              <Cell strong>{tr.title}</Cell>
              <Cell>{tr.artist}</Cell>
              <Cell num>{formatBpm(tr.bpm, lang, 1)}</Cell>
              <span role="cell" className="lib__keycell">
                <KeyChip musicalKey={tr.key} lang={lang} showName />
              </span>
              <Cell num>{formatDuration(tr.duration)}</Cell>
            </ListRow>
          ))}
          {rows.length === 0 && <div className="lib__none">{t('lib.none', { query: search })}</div>}
        </div>
      </div>
    </section>
  );
}
