// Wahl des Kerns: in der Desktop-Hülle der echte (TauriCore), im Browser der simulierte (MockCore).
import { invoke } from '@tauri-apps/api/core';
import type { CoreApi } from './api';
import { MockCore, type MockLibrary } from './mock/MockCore';
import { isSceneName } from './mock/scenes';
import { TauriCore } from './TauriCore';

export type { CoreApi } from './api';
export * from './types';

export interface CoreBoot {
  core: CoreApi;
  kind: 'tauri' | 'mock';
  /** Zeit steht (Screenshots): Auch die Uhr der Oberfläche bleibt fest. */
  frozen: boolean;
}

/** Läuft die Oberfläche in der Tauri-Hülle? */
export const inTauri = (): boolean => typeof window !== 'undefined' && '__TAURI_INTERNALS__' in window;

/**
 * URL-Parameter des simulierten Kerns (nur im Browser):
 *   ?scene=empty|loaded|synced|transition   Ausgangszustand
 *   ?freeze=1                               fester Zeitpunkt, nichts läuft weiter
 *   ?library=empty|analyzing                leere Bibliothek bzw. Tracks in der Analyse
 *   ?device=none                            kein Audiogerät
 *   ?loaddelay=1200                         Ladezeit eines Tracks in Millisekunden
 */
export function createCore(search: string): CoreBoot {
  if (inTauri()) return { core: new TauriCore(invoke), kind: 'tauri', frozen: false };
  const params = new URLSearchParams(search);
  const scene = params.get('scene');
  const library = params.get('library');
  const frozen = params.get('freeze') === '1';
  const core = new MockCore({
    scene: isSceneName(scene) ? scene : undefined,
    frozen,
    library: library === 'empty' || library === 'analyzing' ? (library as MockLibrary) : 'full',
    noDevice: params.get('device') === 'none',
    loadDelayMs: Number(params.get('loaddelay')) || 0,
  });
  return { core, kind: 'mock', frozen };
}
