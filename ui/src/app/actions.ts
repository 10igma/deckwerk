// Bedienhandlungen: Maus und Tastatur rufen dieselben Funktionen auf, die daraus Befehle
// an den Kern machen (7.4). Zusammengesetzte Abläufe (Cue-Taste, Kill, Laden) und der
// Live-Schutz (9.3) liegen hier.
import { errorText, type CoreApi } from '../core/api';
import { DECK_IDS, bandIndex, deckIndex, type AppSettings, type AudioDevice, type Band, type Command, type DeckId, type DeckState, type Snapshot, type TrackId, type TrackInfo } from '../core/types';
import { clamp, nextTempoRange, TEMPO_RANGES, TEMPO_RESOLUTION, tempoRangeFor } from '../lib/curves';
import { fromWire, toWire } from '../lib/db';
import { beatAt, secAt, snapSeek } from '../lib/grid';
import { translate, type MsgKey, type Params } from '../i18n';
import { clearDeckAssets, getDeckAssets, loadDeckAssets } from './assets';
import { latestSnapshot } from './frame';
import { ConfirmGuard, CONFIRM_MS, HOLD_MS, isAudible, loadVerdict, seekVerdict } from './guard';
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
const say = (key: MsgKey, params?: Params): string => translate(getUi().lang, key, params);
const audible = (deck: DeckId): boolean => isAudible(deckState(deck), latestSnapshot().mixer.crossfader);

/** Deck mit Fokus merken: das zuletzt bediente (Leertaste, 8.7). */
export function focusDeck(deck: DeckId): void {
  if (getUi().focusDeck !== deck) setUi({ focusDeck: deck });
}

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
export const FADER_STEP = 0.05;

// --- Live-Schutz (9.3) --------------------------------------------------------

const guard = new ConfirmGuard();
const holdTimers = new Map<string, ReturnType<typeof setTimeout>>();
let armTimer: ReturnType<typeof setTimeout> | null = null;
const now = (): number => (typeof performance !== 'undefined' ? performance.now() : Date.now());

function disarm(id: string): void {
  guard.clear(id);
  if (getUi().armedId === id) setUi({ armedId: null });
}

/**
 * Geschützte Aktion, Taste gedrückt: Der erste Druck zeigt den Hinweis und wartet;
 * ausgeführt wird nach 400 ms Halten oder beim zweiten Druck innerhalb 1 s.
 */
function guardedDown(id: string, run: () => void, notice: string): void {
  if (!getUi().liveGuard) {
    run();
    return;
  }
  if (guard.press(id, now()) === 'run') {
    guardedUp(id);
    disarm(id);
    run();
    return;
  }
  setUi({ armedId: id });
  showNotice(notice);
  if (armTimer) clearTimeout(armTimer);
  armTimer = setTimeout(() => {
    if (getUi().armedId === id) setUi({ armedId: null });
  }, CONFIRM_MS);
  const old = holdTimers.get(id);
  if (old) clearTimeout(old);
  holdTimers.set(
    id,
    setTimeout(() => {
      holdTimers.delete(id);
      disarm(id);
      run();
    }, HOLD_MS),
  );
}

/** Geschützte Aktion, Taste losgelassen: Halten abbrechen; der zweite Druck bleibt möglich. */
function guardedUp(id: string): void {
  const t = holdTimers.get(id);
  if (t) clearTimeout(t);
  holdTimers.delete(id);
}

// --- Transport ---------------------------------------------------------------

/** Läuft ein anderes geladenes Deck? Dann bleibt ein stehendes Deck im Vorausblick an seiner Stelle. */
function otherDeckPlaying(deck: DeckId): boolean {
  return DECK_IDS.some((id) => id !== deck && deckState(id).loaded && deckState(id).playing);
}

/** Play-Taste gedrückt. Ein hörbar laufendes Deck stoppt nur geschützt. */
export function playDown(deck: DeckId): void {
  const d = deckState(deck);
  if (!d.loaded) return;
  focusDeck(deck);
  if (getUi().cueHeld[deck]) {
    // Cue halten + Play: läuft nach dem Loslassen der Cue-Taste weiter (CDJ-Gewohnheit).
    setDeckUi('cueHeld', deck, false);
    return;
  }
  if (!d.playing) {
    // Vorausblick (VIS-13): Läuft ein anderes Deck, steht dieses im Bild an der Stelle, die es dann
    // spielen soll. Der Start rechnet den Vorlauf zurück, damit das Bild so eintrifft.
    const peek = getUi().peekBeats;
    if (peek !== 0 && otherDeckPlaying(deck)) send({ type: 'play_in', deck, beats: peek });
    else send({ type: 'play', deck, on: true });
    return;
  }
  const stop = () => send({ type: 'play', deck, on: false });
  if (audible(deck)) guardedDown(`stop.${deck}`, stop, say('guard.stop', { deck: deckName(deck) }));
  else stop();
}

export function playUp(deck: DeckId): void {
  guardedUp(`stop.${deck}`);
}

/** Cue-Taste gedrückt (CDJ-Verhalten, DECK-05). */
export function cueDown(deck: DeckId): void {
  const d = deckState(deck);
  const ui = getUi();
  if (!d.loaded || ui.cueHeld[deck]) return;
  focusDeck(deck);
  if (d.playing) {
    // Läuft: zurück zum Cue-Punkt und anhalten – am hörbaren Deck geschützt.
    const back = () => {
      send({ type: 'play', deck, on: false });
      send({ type: 'seek', deck, pos: getUi().cuePoints[deck] });
    };
    if (audible(deck)) guardedDown(`cue.${deck}`, back, say('guard.cue', { deck: deckName(deck) }));
    else back();
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
  guardedUp(`cue.${deck}`);
  const ui = getUi();
  if (!ui.cueHeld[deck]) return;
  setDeckUi('cueHeld', deck, false);
  send({ type: 'play', deck, on: false });
  send({ type: 'seek', deck, pos: ui.cuePoints[deck] });
}

/** Sprung zum Anfang (DECK-05). Im laufenden Deck gesperrt. */
export function toStart(deck: DeckId): void {
  const d = deckState(deck);
  if (!d.loaded) return;
  focusDeck(deck);
  if (d.playing) {
    showNotice(say('guard.toStart', { deck: deckName(deck) }));
    return;
  }
  send({ type: 'seek', deck, pos: 0 });
}

export function toggleSync(deck: DeckId): void {
  const d = deckState(deck);
  if (!d.loaded) return;
  focusDeck(deck);
  send({ type: 'sync', deck, mode: d.sync === 'off' ? getUi().syncPref[deck] : 'off' });
}

/** Zwischen „Beat" und „Tempo" wechseln; wirkt sofort, wenn Sync an ist. */
export function cycleSyncMode(deck: DeckId): void {
  const next = getUi().syncPref[deck] === 'beat' ? 'tempo' : 'beat';
  setDeckUi('syncPref', deck, next);
  focusDeck(deck);
  if (deckState(deck).sync !== 'off') send({ type: 'sync', deck, mode: next });
}

export function setMaster(deck: DeckId): void {
  if (!deckState(deck).loaded) return;
  focusDeck(deck);
  send({ type: 'set_master', deck });
}

/** Anschieben (+1), Abbremsen (−1), loslassen (0). */
export function bend(deck: DeckId, dir: -1 | 0 | 1): void {
  if (dir !== 0) {
    if (!deckState(deck).loaded) return;
    focusDeck(deck);
  }
  send({ type: 'bend', deck, amount: dir * BEND_AMOUNT });
}

export function setTempo(deck: DeckId, offset: number): void {
  const rounded = Math.round(clamp(offset, -1, 1) / TEMPO_RESOLUTION) * TEMPO_RESOLUTION;
  focusDeck(deck);
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

/** Bereich, den der Tempo-Regler zeigt: wächst mit, wenn Sync mehr verlangt als eingestellt. */
export function shownTempoRange(range: number, offset: number): number {
  return tempoRangeFor(offset, range);
}

/** Tempo-Bereich weiterschalten; `dir = -1` schaltet zurück. */
export function cycleTempoRange(deck: DeckId, dir: 1 | -1 = 1): void {
  const d = deckState(deck);
  const cur = getUi().tempoRange[deck];
  let next: number;
  if (dir === 1) {
    next = nextTempoRange(cur);
  } else {
    const i = TEMPO_RANGES.findIndex((r) => Math.abs(r - cur) < 1e-9);
    next = TEMPO_RANGES[(i - 1 + TEMPO_RANGES.length) % TEMPO_RANGES.length];
  }
  // Der Bereich darf den eingestellten Wert nicht abschneiden.
  setDeckUi('tempoRange', deck, tempoRangeFor(d.sync === 'off' || d.is_master ? d.tempo_offset : 0, next));
  focusDeck(deck);
}

/**
 * Sprung in der Übersicht (DECK-20). Im laufenden Deck nur mit Snap und phasenerhaltend;
 * ist das Deck hörbar, zusätzlich nur mit Shift (9.3). Liefert, ob gesprungen wurde.
 */
export function seek(deck: DeckId, pos: number, shift = false): boolean {
  const d = deckState(deck);
  if (!d.loaded) return false;
  focusDeck(deck);
  const ui = getUi();
  const verdict = seekVerdict({ playing: d.playing, audible: audible(deck), snap: ui.snap, shift, guard: ui.liveGuard });
  if (verdict === 'need-snap') {
    showNotice(say('guard.seekSnap', { deck: deckName(deck) }));
    return false;
  }
  if (verdict === 'need-shift') {
    showNotice(say('guard.seekShift', { deck: deckName(deck) }));
    return false;
  }
  let target = clamp(pos, 0, d.duration);
  const assets = getDeckAssets(deck);
  if (d.playing && assets) target = clamp(snapSeek(assets.analysis.grid, d.pos, target), 0, d.duration);
  // Laufendes Deck: Die Phase hält der Kern im Augenblick des Sprungs. Das hier berechnete Ziel
  // stammt aus einer Position, die schon einige Millisekunden alt ist, und wäre allein nicht im Takt.
  send({ type: d.playing ? 'seek_snap' : 'seek', deck, pos: target });
  return true;
}

// --- Mixer -------------------------------------------------------------------

export function toggleHeadphone(deck: DeckId): void {
  focusDeck(deck);
  send({ type: 'cue', deck, on: !deckState(deck).cue });
}

export function setGain(deck: DeckId, db: number): void {
  send({ type: 'gain', deck, db: toWire(Math.min(12, db)) });
}

export function setEq(deck: DeckId, band: Band, db: number): void {
  send({ type: 'eq', deck, band, db: toWire(Math.min(6, db)) });
}

/** Kill: Band ganz aus; erneut drücken stellt den vorherigen Wert wieder her. */
export function toggleKill(deck: DeckId, band: Band): void {
  const i = bandIndex(band);
  const cur = fromWire(deckState(deck).eq_db[i]);
  const before = getUi().eqBeforeKill[deck];
  focusDeck(deck);
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
  focusDeck(deck);
  send({ type: 'filter', deck, pos: Math.abs(next) < 0.02 ? 0 : next });
}

export function setFader(deck: DeckId, value: number): void {
  send({ type: 'fader', deck, value: clamp(value, 0, 1) });
}

/** Kanalfader schrittweise: +1 = auf, −1 = zu; fein = ein Fünftel. */
export function faderStep(deck: DeckId, dir: -1 | 1, fine: boolean): void {
  const cur = stepBase(`fader.${deck}`, deckState(deck).fader);
  const next = clamp(Math.round((cur + dir * (fine ? FADER_STEP / 5 : FADER_STEP)) * 100) / 100, 0, 1);
  stepSent(`fader.${deck}`, next);
  focusDeck(deck);
  setFader(deck, next);
}

export function crossfaderStep(dir: -1 | 1, full: boolean): void {
  const cur = stepBase('crossfader', latestSnapshot().mixer.crossfader);
  const next = clamp(full ? dir : Math.round((cur + dir * CROSSFADER_STEP) * 100) / 100, -1, 1);
  stepSent('crossfader', next);
  send({ type: 'crossfader', pos: next });
}

// --- Laden ---------------------------------------------------------------------

function findTrack(trackId: TrackId): TrackInfo | undefined {
  return getUi().tracks.find((t) => t.id === trackId);
}

/** Lädt ohne weitere Prüfung; zeigt den Ladezustand und Fehler der Hülle im Klartext. */
async function doLoad(deck: DeckId, track: TrackInfo): Promise<boolean> {
  if (!core || getUi().loading[deck]) return false;
  setDeckUi('loading', deck, track);
  try {
    await core.loadTrack(deck, track.id);
  } catch (err) {
    setDeckUi('loading', deck, null);
    showNotice(say('notice.loadFailed', { title: track.title, reason: errorText(err) }), 'error');
    return false;
  }
  setDeckUi('deckTracks', deck, findTrack(track.id) ?? track);
  setDeckUi('cueHeld', deck, false);
  try {
    const assets = await loadDeckAssets(core, deck, track.id);
    if (assets) {
      // Cue-Punkt und Position auf den ersten Beat.
      const first = Math.max(0, secAt(assets.analysis.grid, 0));
      setDeckUi('cuePoints', deck, first);
      send({ type: 'seek', deck, pos: first });
    }
  } catch (err) {
    showNotice(say('notice.waveFailed', { title: track.title, reason: errorText(err) }), 'error');
  }
  setDeckUi('loading', deck, null);
  focusDeck(deck);
  return true;
}

function loadable(trackId: TrackId): TrackInfo | null {
  const track = findTrack(trackId);
  if (!track) return null;
  if (track.missing) {
    showNotice(say('notice.missing', { title: track.title }), 'error');
    return null;
  }
  // Unvollständige Datei: spielbar, aber kürzer als angegeben – beim Laden darauf hinweisen (BIB-20).
  if (track.incomplete) showNotice(say('notice.incomplete', { title: track.title }), 'error');
  return track;
}

/**
 * Gezieltes Laden, Taste gedrückt (Lade-Taste, Shift + Pfeil). Ein hörbares Deck wird nur
 * nach Halten oder zweitem Druck ersetzt; ein unhörbares ohne Hürde (9.3).
 */
export function loadDown(deck: DeckId, trackId: TrackId): void {
  const track = loadable(trackId);
  if (!track) return;
  const verdict = loadVerdict({ loaded: deckState(deck).loaded, audible: audible(deck), targeted: true, guard: getUi().liveGuard });
  if (verdict === 'guard') guardedDown(`load.${deck}`, () => void doLoad(deck, track), say('guard.load', { deck: deckName(deck) }));
  else void doLoad(deck, track);
}

export function loadUp(deck: DeckId): void {
  guardedUp(`load.${deck}`);
}

/** Ablegen auf ein Deck: gezielt, aber ohne zweiten Druck – ein hörbares Deck bleibt gesperrt. */
export function loadByDrop(deck: DeckId, trackId: TrackId): void {
  const track = loadable(trackId);
  if (!track) return;
  if (getUi().liveGuard && audible(deck)) {
    showNotice(say('guard.drop', { deck: deckName(deck) }));
    return;
  }
  void doLoad(deck, track);
}

export function loadSelectedDown(deck: DeckId): void {
  const id = getUi().selectedId;
  if (id) loadDown(deck, id);
  else showNotice(say('notice.nothingSelected'));
}

/** Doppelklick in der Liste: lädt nur in ein leeres Deck (9.3). */
export function loadToFreeDeck(trackId: TrackId, decks: DeckId[] = ['a', 'b']): void {
  const track = loadable(trackId);
  if (!track) return;
  const ui = getUi();
  const snap = latestSnapshot();
  const empty = decks.find((d) => !snap.decks[deckIndex(d)].loaded && !ui.loading[d]);
  if (empty) void doLoad(empty, track);
  else showNotice(say('notice.noneFree'));
}

// --- Bibliothek ------------------------------------------------------------------

let lastRev = -1;

/** Liste vom Kern holen und geladene Decks an die neuen Angaben anpassen (BPM, Tonart nach der Analyse). */
export async function refreshTracks(): Promise<void> {
  if (!core) return;
  const tracks = await core.listTracks();
  const ui = getUi();
  const deckTracks = { ...ui.deckTracks };
  for (const deck of DECK_IDS) {
    const cur = deckTracks[deck];
    if (cur) deckTracks[deck] = tracks.find((t) => t.id === cur.id) ?? cur;
  }
  setUi({ tracks, deckTracks, tracksReady: true });
}

/** Beim Start: Bibliothek beobachten und bereits geladene Decks vom Kern übernehmen. */
export async function hydrate(): Promise<void> {
  if (!core) return;
  const c = core;
  c.subscribeLibrary((s) => {
    if (s.pending_analysis !== getUi().pendingAnalysis) setUi({ pendingAnalysis: s.pending_analysis });
    if (s.rev !== lastRev) {
      lastRev = s.rev;
      void refreshTracks().catch((err) => showNotice(say('notice.libraryFailed', { reason: errorText(err) }), 'error'));
    }
  });
  try {
    await refreshTracks();
    const loaded = await c.getLoadedTracks();
    for (const deck of DECK_IDS) {
      const id = loaded[deck];
      const track = id ? findTrack(id) ?? null : null;
      setDeckUi('deckTracks', deck, track);
      if (id && track) {
        const assets = await loadDeckAssets(c, deck, id);
        if (assets) setDeckUi('cuePoints', deck, Math.max(0, secAt(assets.analysis.grid, 0)));
      } else {
        clearDeckAssets(deck);
      }
    }
  } catch (err) {
    showNotice(say('notice.libraryFailed', { reason: errorText(err) }), 'error');
  }
}

/** Import (im Browser: Dateien auf das Fenster gezogen; in der Hülle importiert diese selbst). */
export async function importPaths(paths: string[]): Promise<void> {
  if (!core || paths.length === 0) return;
  try {
    const r = await core.importPaths(paths);
    showNotice(say('notice.imported', { added: r.added, failed: r.failed.length }));
  } catch (err) {
    showNotice(say('notice.importFailed', { reason: errorText(err) }), 'error');
  }
}

/** Bibliothek aktualisieren: Musikordner nach neuen Dateien durchsuchen (BIB-19). */
export async function refreshLibrary(): Promise<void> {
  if (!core) return;
  try {
    const r = await core.refreshLibrary();
    if (r.folders === 0 && r.unreachable === 0) showNotice(say('notice.refreshNoFolder'));
    else if (r.unreachable > 0) showNotice(say('notice.refreshUnreachable', { added: r.added, n: r.unreachable }), 'error');
    else if (r.added === 0 && r.relinked === 0) showNotice(say('notice.refreshNothing', { missing: r.missing }));
    else showNotice(say('notice.refreshed', { added: r.added, relinked: r.relinked, missing: r.missing }));
  } catch (err) {
    showNotice(say('notice.refreshFailed', { reason: errorText(err) }), 'error');
  }
}

/** Einstellungen der Hülle; ohne Kern oder bei einem Fehler null. */
export async function getAppSettings(): Promise<AppSettings | null> {
  if (!core) return null;
  try {
    return await core.getSettings();
  } catch {
    return null;
  }
}

/** Ergebnis: Fehlertext oder null. */
export async function setRefreshOnStart(on: boolean): Promise<string | null> {
  if (!core) return null;
  try {
    await core.setRefreshOnStart(on);
    return null;
  } catch (err) {
    return errorText(err);
  }
}

// --- Audio-Ausgabe ---------------------------------------------------------------

/** Ausgabegeräte für die Einstellungen; ohne Kern oder bei einem Fehler eine leere Liste. */
export async function listAudioDevices(): Promise<AudioDevice[]> {
  if (!core) return [];
  try {
    return await core.listAudioDevices();
  } catch {
    return [];
  }
}

/** Wechselt das Ausgabegerät (null = Standardgerät des Systems). Ergebnis: Fehlertext oder null. */
export async function selectAudioDevice(name: string | null): Promise<string | null> {
  if (!core) return null;
  try {
    await core.selectAudioDevice(name);
    return null;
  } catch (err) {
    return errorText(err);
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

// --- Vorausblick (VIS-13) --------------------------------------------------------

/** Grenzen des Vorausblicks in Beats: ein Stück zurück, weit nach vorn. */
export const PEEK_MIN = -64;
export const PEEK_MAX = 4096;
/** Schritt der Tasten in Beats: vier Takte, mit Shift ein Takt. */
export const PEEK_STEP = 16;
export const PEEK_STEP_FINE = 4;

/** Neuer Vorausblick nach einem Schritt: ganze Beats, auf die Grenzen beschränkt. */
export function peekAfter(current: number, beats: number): number {
  return clamp(Math.round(current + beats), PEEK_MIN, PEEK_MAX);
}

/** Ansicht der Wellenformen um `beats` verschieben; die Wiedergabe läuft unverändert weiter. */
export function peekBy(beats: number): void {
  setUi((s) => ({ peekBeats: peekAfter(s.peekBeats, beats) }));
}

/** Schritt der Tasten: vier Takte vor oder zurück, fein ein Takt. */
export function peekStep(dir: -1 | 1, fine: boolean): void {
  peekBy(dir * (fine ? PEEK_STEP_FINE : PEEK_STEP));
}

/** Zurück zur Abspielstelle. */
export function peekReset(): void {
  if (getUi().peekBeats !== 0) setUi({ peekBeats: 0 });
}

export function toggleSnap(): void {
  setUi((s) => ({ snap: !s.snap }));
}

export function toggleLibraryBig(): void {
  setUi((s) => ({ libraryBig: !s.libraryBig }));
}
