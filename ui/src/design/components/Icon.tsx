// Eigene, schlichte Zeichen aus geraden Kanten – passend zur gefasten Formensprache.
import type { CSSProperties } from 'react';

export type IconName =
  | 'play'
  | 'pause'
  | 'headphones'
  | 'gear'
  | 'rec'
  | 'search'
  | 'slower'
  | 'faster'
  | 'up'
  | 'down'
  | 'minus'
  | 'plus'
  | 'match'
  | 'ok'
  | 'clash'
  | 'warn'
  | 'tostart'
  | 'expand'
  | 'collapse'
  | 'drop';

const PATHS: Record<IconName, string> = {
  play: 'M5 3 L19 12 L5 21 Z',
  pause: 'M5 3 H10 V21 H5 Z M14 3 H19 V21 H14 Z',
  headphones: 'M4 21 V11 L8 4 H16 L20 11 V21 H15 V13 H18 V11.6 L14.8 6 H9.2 L6 11.6 V13 H9 V21 Z',
  gear: 'M10 2 H14 L14.6 5 L17.4 3.6 L20.4 6.6 L19 9.4 L22 10 V14 L19 14.6 L20.4 17.4 L17.4 20.4 L14.6 19 L14 22 H10 L9.4 19 L6.6 20.4 L3.6 17.4 L5 14.6 L2 14 V10 L5 9.4 L3.6 6.6 L6.6 3.6 L9.4 5 Z M12 8.5 L8.5 12 L12 15.5 L15.5 12 Z',
  rec: 'M8 3 H16 L21 8 V16 L16 21 H8 L3 16 V8 Z',
  search: 'M4 4 H14 V14 H4 Z M6.4 6.4 V11.6 H11.6 V6.4 Z M13 14.8 L14.8 13 L21 19.2 L19.2 21 Z',
  slower: 'M11 4 V20 L2 12 Z M22 4 V20 L13 12 Z',
  faster: 'M13 4 V20 L22 12 Z M2 4 V20 L11 12 Z',
  up: 'M12 6 L20 18 H4 Z',
  down: 'M12 18 L20 6 H4 Z',
  minus: 'M4 10 H20 V14 H4 Z',
  plus: 'M10 4 H14 V10 H20 V14 H14 V20 H10 V14 H4 V10 H10 Z',
  // Verträglichkeit: passt = volle Raute, geht = halbe Raute, reibt = Kreuz
  match: 'M12 2 L22 12 L12 22 L2 12 Z',
  ok: 'M12 2 L22 12 L12 22 L2 12 Z M12 6.2 V17.8 L17.8 12 Z',
  clash: 'M5 2.5 L12 9.5 L19 2.5 L21.5 5 L14.5 12 L21.5 19 L19 21.5 L12 14.5 L5 21.5 L2.5 19 L9.5 12 L2.5 5 Z',
  warn: 'M12 2 L23 21 H1 Z M10.8 9 H13.2 V15 H10.8 Z M10.8 16.6 H13.2 V19 H10.8 Z',
  // Sprung zum Anfang: Balken und Dreieck nach links
  tostart: 'M4 4 H8 V20 H4 Z M21 4 V20 L9 12 Z',
  // Liste aufziehen / zurücksetzen
  expand: 'M3 3 H21 V6 H3 Z M12 9 L20 19 H4 Z',
  collapse: 'M3 18 H21 V21 H3 Z M12 15 L20 5 H4 Z',
  // Ablegen: Pfeil in eine Schale
  drop: 'M10 2 H14 V10 H18 L12 16.5 L6 10 H10 Z M3 15 H6 V19 H18 V15 H21 V22 H3 Z',
};

export function Icon({ name, size = '1em', style }: { name: IconName; size?: string; style?: CSSProperties }) {
  return (
    <svg viewBox="0 0 24 24" width={size} height={size} aria-hidden="true" style={{ display: 'block', flex: 'none', ...style }}>
      <path d={PATHS[name]} fill="currentColor" fillRule="evenodd" />
    </svg>
  );
}
