// „Hörbar" nach Spezifikation 9.3: Kanalfader offen und Crossfader dem Kanal nicht ganz abgewandt.
import type { XfAssign } from '../core/types';

export interface ChannelLike {
  fader: number;
  xf_assign: XfAssign;
}

const EPS = 1e-4;

/** Trägt der Kanal bei dieser Mixer-Stellung zur Summe bei? */
export function channelOpen(ch: ChannelLike, crossfader: number): boolean {
  if (ch.fader <= EPS) return false;
  if (ch.xf_assign === 'left') return crossfader < 1 - EPS;
  if (ch.xf_assign === 'right') return crossfader > -1 + EPS;
  return true;
}
