// Wahl des Kerns. Heute: MockCore. Später entscheidet hier die Tauri-Hülle.
import type { CoreApi } from './api';
import { MockCore } from './mock/MockCore';
import { isSceneName } from './mock/scenes';

export type { CoreApi } from './api';
export * from './types';

export interface CoreBoot {
  core: CoreApi;
  /** Zeit steht (Screenshots): Auch die Uhr der Oberfläche bleibt fest. */
  frozen: boolean;
}

/**
 * URL-Parameter des simulierten Kerns:
 *   ?scene=empty|loaded|synced|transition   Ausgangszustand
 *   ?freeze=1                               fester Zeitpunkt, nichts läuft weiter
 */
export function createCore(search: string): CoreBoot {
  const params = new URLSearchParams(search);
  const scene = params.get('scene');
  const frozen = params.get('freeze') === '1';
  const core = new MockCore({ scene: isSceneName(scene) ? scene : undefined, frozen });
  return { core, frozen };
}
