import { cssNumber, cssRgb } from '../../design/tokens';
import type { StageColors } from './wave/scene';

/** Farben der Wellenform-Bühne aus den Tokens lesen. */
export function readStageColors(): StageColors {
  return {
    bg: cssRgb('--bg-0'),
    rail: cssRgb('--bg-1'),
    text: cssRgb('--text-1'),
    low: cssRgb('--band-low'),
    mid: cssRgb('--band-mid'),
    high: cssRgb('--band-high'),
    decks: { a: cssRgb('--deck-a'), b: cssRgb('--deck-b'), c: cssRgb('--deck-c'), d: cssRgb('--deck-d') },
    playedDim: cssNumber('--played-dim'),
  };
}
