// Bedienhandlungen: Maus und Tastatur rufen dieselben Funktionen auf, die daraus Befehle
// an den Kern machen (7.4). Zusammengesetzte Abläufe (Cue-Taste, Kill, Laden) liegen hier.
import type { CoreApi } from '../core/api';
import { DECK_IDS, bandIndex, deckIndex, type Band, type Command, type DeckId, type DeckState, type Snapshot, type TrackId } from '../core/types';
import { clamp, nextTempoRange, TEMPO_RESOLUTION, tempoRangeFor } from '../lib/curves';
import { fromWire, toWire } from '../lib/db';
import { beatAt, secAt, snapSeek } from '../lib/grid';
import { translate } from '../i18n';
import { clearDeckAssets, getDeckAssets, loadDeckAssets } from './assets';
import { latestSnapshot } from './frame';
import { getUi, setDeckUi, setUi, showNotice } from './store';

let core: CoreApi | null = null;

export function bindCore(c: CoreApi): void {
  core = c;
}

export function send(cmd: Command): void {
  core?.send(cmd);
}

const deckState = (deck: DeckId): DeckState => latestSnapshot().decks[deckIndex(deck)];
const deckName = (deck: DeckId): string => deck.toUpperCase();

// Schrittweise Befehle (Tasten) können schneller kommen als der nächste Snapshot.
// Bis der Kern antwortet, zählt deshalb der zuletzt gesendete Wert.
const pending = new Map<string, { snap: Snapshot; value: number }>();

function stepBase(key: string, fromSnapshot: number): number {
  const p = pending.get(key);
  return p && p.snap === latestSnapshot() ? p.value : fromSnapshot;
}

function stepSent(key: string, value: number): void {
  pending.set(key, { snap: latestSnapshot(), value });
}

/** Stärke von Anschieben/Abbremsen (DECK-07, Standard ±4 %). */
export const BEND_AMOUNT = 0.04;
export const CROSSFADER_STEP = 0.05;
export const FILTER_STEP = 0.05;

// --- Transport ---------------------------------------------------------------

export function togglePlay(deck: DeckId): void {
  const d = deckState(deck);
  if (!d.loaded) return;
  send({ type: 'play', deck, on: !d.playing });
}

/** Cue-Taste gedrückt (CDJ-Verhalten, DECK-05). */
export function cueDown(deck: DeckId): void {
  const d = deckState(deck);
  const ui = getUi();
  if (!d.loaded || ui.cueHeld[deck]) return;
  if (d.playing) {
    // Läuft: zurück zum Cue-Punkt und anhalten.
    send({ type: 'play', deck, on: false });
    send({ type: 'seek', deck, pos: ui.cuePoints[deck] });
    return;
  }
  // Steht: Cue-Punkt hierher legen (mit Snap auf den nächsten Beat) und vorspielen, solange gehalten.
  let pos = d.pos;
  const assets = getDeckAssets(deck);
  if (Math.abs(pos - ui.cuePoints[deck]) > 0.005) {
    if (ui.snap && assets) pos = Math.max(0, secAt(assets.analysis.grid, Math.round(beatAt(assets.analysis.grid, pos))));
    setDeckUi('cuePoints', deck, pos);
    send({ type: 'seek', deck, pos });
  }
  setDeckUi('cueHeld', deck, true);
  send({ type: 'play', deck, on: true });
}

/** Cue-Taste losgelassen: anhalten und zurück zum Cue-Punkt. */
export function cueUp(deck: DeckId): void {
  const ui = getUi();
  if (!ui.cueHeld[deck]) return;
  setDeckUi('cueHeld', deck, false);
  send({ type: 'play', deck, on: false });
  send({ type: 'seek', deck, pos: ui.cuePoints[deck] });
}

export function toggleSync(deck: DeckId): void {
  const d = deckState(deck);
  send({ type: 'sync', deck, mode: d.sync === 'off' ? getUi().syncPref[deck] : 'off' });
}

/** Zwischen „Beat“ und „Tempo“ wechseln; wirkt sofort, wenn Sync an ist. */
export function cycleSyncMode(deck: DeckId): void {
  const next = getUi().syncPref[deck] === 'beat' ? 'tempo' : 'beat';
  setDeckUi('syncPref', deck, next);
  if (deckState(deck).sync !== 'off') send({ type: 'sync', deck, mode: next });
}

export function setMaster(deck: DeckId): void {
  send({ type: 'set_master', deck });
}

/** Anschieben (+1), Abbremsen (−1), loslassen (0). */
export function bend(deck: DeckId, dir: -1 | 0 | 1): void {
  send({ type: 'bend', deck, amount: dir * BEND_AMOUNT });
}

export function setTempo(deck: DeckId, offset: number): void {
  const rounded = Math.round(clamp(offset, -1, 1) / TEMPO_RESOLUTION) * TEMPO_RESOLUTION;
  send({ type: 'tempo', deck, offset: rounded });
}

/** Tempo schrittweise: 0,1 %, fein 0,01 %. */
export function tempoStep(deck: DeckId, dir: -1 | 1, fine: boolean): void {
  const d = deckState(deck);
  if (!d.loaded) return;
  const range = getUi().tempoRange[deck];
  const next = clamp(stepBase(`tempo.${deck}`, d.tempo_offset) + dir * (fine ? 0.0001 : 0.001), -range, range);
  stepSent(`tempo.${deck}`, next);
  setTempo(deck, next);
}

export function cycleTempoRange(deck: DeckId): void {
  const d = deckState(deck);
  let next = nextTempoRange(getUi().tempoRange[deck]);
  // Der Bereich darf den eingestellten Wert nicht abschneiden.
  next = tempoRangeFor(d.tempo_offset, next);
  setDeckUi('tempoRange', deck, next);
}

/** Sprung in der Übersicht. Im laufenden Deck mit Snap phasenerhaltend (DECK-20). */
export function seek(deck: DeckId, pos: number): void {
  const d = deckState(deck);
  if (!d.loaded) return;
  let target = clamp(pos, 0, d.duration);
  const assets = getDeckAssets(deck);
  if (d.playing && getUi().snap && assets) {
    target = clamp(snapSeek(assets.analysis.grid, d.pos, target), 0, d.duration);
  }
  send({ type: 'seek', deck, pos: target });
}

// --- Mixer -------------------------------------------------------------------

export function toggleHeadphone(deck: DeckId): void {
  send({ type: 'cue', deck, on: !deckState(deck).cue });
}

export function setEq(deck: DeckId, band: Band, db: number): void {
  send({ type: 'eq', deck, band, db: toWire(db) });
}

/** Kill: Band ganz aus; erneut drücken stellt den vorherigen Wert wieder her. */
export function toggleKill(deck: DeckId, band: Band): void {
  const i = bandIndex(band);
  const cur = fromWire(deckState(deck).eq_db[i]);
  const before = getUi().eqBeforeKill[deck];
  if (cur === -Infinity) {
    setEq(deck, band, before[i]);
  } else {
    const next: [number, number, number] = [before[0], before[1], before[2]];
    next[i] = cur;
    setDeckUi('eqBeforeKill', deck, next);
    setEq(deck, band, -Infinity);
  }
}

export function setFilter(deck: DeckId, pos: number): void {
  // In der Ruhezone rastet der Regler auf exakt 0 (Bypass).
  send({ type: 'filter', deck, pos: Math.abs(pos) < 0.02 ? 0 : clamp(pos, -1, 1) });
}

/** Filter schrittweise: −1 = zu (Tiefpass), +1 = auf (Hochpass). */
export function filterStep(deck: DeckId, dir: -1 | 1): void {
  const cur = stepBase(`filter.${deck}`, deckState(deck).filter);
  const next = clamp(Math.round((cur + dir * FILTER_STEP) * 100) / 100, -1, 1);
  stepSent(`filter.${deck}`, next);
  send({ type: 'filter', deck, pos: Math.abs(next) < 0.02 ? 0 : next });
}

export function crossfaderStep(dir: -1 | 1, full: boolean): void {
  const cur = stepBase('crossfader', latestSnapshot().mixer.crossfader);
  const next = clamp(full ? dir : Math.round((cur + dir * CROSSFADER_STEP) * 100) / 100, -1, 1);
  stepSent('crossfader', next);
  send({ type: 'crossfader', pos: next });
}

// --- Laden ---------------------------------------------------------------------

/** Track auf ein Deck laden. Ein laufendes Deck ist geschützt (DECK-04). */
export async function loadTrack(deck: DeckId, trackId: TrackId): Promise<boolean> {
  if (!core) return false;
  const ui = getUi();
  const track = ui.tracks.find((t) => t.id === trackId);
  if (!track) return false;
  if (deckState(deck).playing) {
    showNotice(translate(ui.lang, 'notice.loadProtected', { deck: deckName(deck) }));
    return false;
  }
  await core.loadTrack(deck, trackId);
  setDeckUi('deckTracks', deck, track);
  setDeckUi('cueHeld', deck, false);
  const assets = await loadDeckAssets(core, deck, trackId);
  if (assets) {
    // Cue-Punkt und Position auf den ersten Beat.
    const first = Math.max(0, secAt(assets.analysis.grid, 0));
    setDeckUi('cuePoints', deck, first);
    send({ type: 'seek', deck, pos: first });
  }
  return true;
}

export function loadSelected(deck: DeckId): void {
  const id = getUi().selectedId;
  if (id) void loadTrack(deck, id);
  else showNotice(translate(getUi().lang, 'notice.nothingSelected'));
}

/** Doppelklick: auf das erste freie Deck, sonst auf das erste stehende. */
export function loadToFreeDeck(trackId: TrackId, decks: DeckId[] = ['a', 'b']): void {
  const snap = latestSnapshot();
  const empty = decks.find((d) => !snap.decks[deckIndex(d)].loaded);
  const stopped = decks.find((d) => !snap.decks[deckIndex(d)].playing);
  const target = empty ?? stopped;
  if (target) void loadTrack(target, trackId);
  else showNotice(translate(getUi().lang, 'notice.allPlaying'));
}

/** Beim Start: Bibliothek und bereits geladene Decks vom Kern übernehmen. */
export async function hydrate(): Promise<void> {
  if (!core) return;
  const [tracks, loaded] = await Promise.all([core.listTracks(), core.getLoadedTracks()]);
  setUi({ tracks });
  for (const deck of DECK_IDS) {
    const id = loaded[deck];
    const track = id ? tracks.find((t) => t.id === id) ?? null : null;
    setDeckUi('deckTracks', deck, track);
    if (id && track) {
      const assets = await loadDeckAssets(core, deck, id);
      if (assets) setDeckUi('cuePoints', deck, Math.max(0, secAt(assets.analysis.grid, 0)));
    } else {
      clearDeckAssets(deck);
    }
  }
}

// --- Liste -----------------------------------------------------------------------

let visibleIds: TrackId[] = [];

/** Die Liste meldet ihre sichtbare Reihenfolge, damit ↑/↓ ihr folgen. */
export function setVisibleTrackIds(ids: TrackId[]): void {
  visibleIds = ids;
}

export function moveSelection(dir: -1 | 1): void {
  if (visibleIds.length === 0) return;
  const cur = getUi().selectedId;
  const i = cur ? visibleIds.indexOf(cur) : -1;
  const next = i < 0 ? (dir > 0 ? 0 : visibleIds.length - 1) : clamp(i + dir, 0, visibleIds.length - 1);
  setUi({ selectedId: visibleIds[next] });
}

export function toggleSnap(): void {
  setUi((s) => ({ snap: !s.snap }));
}

