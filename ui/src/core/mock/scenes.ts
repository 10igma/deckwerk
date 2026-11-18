// Feste Ausgangszustände für Screenshots und Vorführung (URL-Parameter `scene`).
import { secAt } from '../../lib/grid';
import type { MockCore } from './MockCore';
import { MOCK_TRACKS } from './tracks';

export const SCENE_NAMES = ['empty', 'loaded', 'synced', 'transition'] as const;
export type SceneName = (typeof SCENE_NAMES)[number];

export const isSceneName = (v: string | null): v is SceneName =>
  v !== null && (SCENE_NAMES as readonly string[]).includes(v);

// Kaltwalzwerk (134 BPM, 8A) auf A, Schicht III (136 BPM, 9A) auf B.
const TRACK_A = MOCK_TRACKS[0].id;
const TRACK_B = MOCK_TRACKS[1].id;

/** Deck auf einen Beat setzen (plus Bruchteil), Position aus dem Raster. */
function placeAtBeat(core: MockCore, deck: 0 | 1, beat: number): void {
  const d = core.decks[deck];
  if (d.track) d.pos = secAt(d.track.grid, beat);
}

export function applyScene(core: MockCore, scene: SceneName): void {
  if (scene === 'empty') return;

  core.loadNow('a', TRACK_A);
  core.loadNow('b', TRACK_B);
  if (scene === 'loaded') {
    // Beide Decks stehen auf dem ersten Beat.
    placeAtBeat(core, 0, 0);
    placeAtBeat(core, 1, 0);
    core.decks[1].cue = true;
    return;
  }

  const [a, b] = core.decks;
  // A läuft als Master im Hauptteil, B folgt im Beat-Sync.
  const late = scene === 'transition';
  placeAtBeat(core, 0, (late ? 176 : 56) * 4 + 1.62);
  placeAtBeat(core, 1, (late ? 30 : 6) * 4 + 1.62);
  a.playing = true;
  a.playTime = 300;
  b.playing = true;
  b.playTime = 40;
  b.sync = 'beat';
  core.masterMode = 'auto';
  core.pickMaster();
  core.time = 311.4;

  if (scene === 'synced') {
    // B wird vorgehört, Fader noch unten.
    b.fader = 0;
    b.cue = true;
  } else {
    // Übergang: Bass von B gekillt, Fader halb offen.
    b.eqDb[0] = -Infinity;
    b.fader = 0.5;
    b.cue = true;
    a.eqDb[2] = -3;
  }
  core.step(0);
}
