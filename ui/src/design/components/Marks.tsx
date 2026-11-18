// Kleine Marken: Deck-Schild, Tonart-Marke, Zahlenanzeige, Listen-Zeile.
import type { CSSProperties, HTMLAttributes, ReactNode } from 'react';
import type { DeckId, Key } from '../../core/types';
import { camelotString, isMinor, keyName } from '../../lib/key';
import type { Lang } from '../../lib/format';
import { keyColorVar } from '../keyColors';
import { useCut, type Cut } from './KeyBtn';

/** Deck-Schild: Buchstabe auf der Deck-Farbe. */
export function DeckPlate({ deck, size, cut, hollow }: { deck: DeckId; size?: 'sm' | 'md' | 'lg'; cut?: Cut; hollow?: boolean }) {
  const side = useCut();
  return (
    <span className="dw-plate" data-deck={deck} data-size={size ?? 'md'} data-cut={cut ?? side} data-hollow={hollow ? 'true' : undefined}>
      {deck.toUpperCase()}
    </span>
  );
}

/**
 * Tonart-Marke: Camelot-Code in Schriftfarbe auf dunklem Grund, die Tonart-Farbe als Balken links.
 * So bleibt die gefüllte Farbfläche den Decks und aktiven Tasten vorbehalten. Dur mit Kerbe, Moll ohne.
 */
export function KeyChip({ musicalKey, lang, size, showName }: { musicalKey: Key | null; lang: Lang; size?: 'md' | 'lg'; showName?: boolean }) {
  const mode = musicalKey === null ? 'none' : isMinor(musicalKey) ? 'minor' : 'major';
  const name = musicalKey === null ? null : keyName(musicalKey, lang);
  return (
    <span
      className="dw-keychip"
      data-mode={mode}
      data-size={size ?? 'md'}
      style={{ '--chip': keyColorVar(musicalKey) } as CSSProperties}
      title={name ?? undefined}
    >
      <span className="dw-keychip__code">{musicalKey === null ? '–' : camelotString(musicalKey)}</span>
      {showName && name && <span className="dw-keychip__name">{name}</span>}
    </span>
  );
}

export interface ReadoutProps {
  label: string;
  size?: 'sm' | 'md' | 'lg';
  align?: 'left' | 'right';
  tone?: 'normal' | 'muted';
  unit?: string;
  children: ReactNode;
}

/** Zahlenanzeige: Beschriftung über dem Wert, Ziffern mit fester Breite. */
export function Readout({ label, size = 'sm', align = 'left', tone = 'normal', unit, children }: ReadoutProps) {
  return (
    <div className="dw-readout" data-size={size} data-align={align} data-tone={tone}>
      <span className="dw-label">{label}</span>
      <span className="dw-readout__value">
        {children}
        {unit && <span className="dw-readout__unit">{unit}</span>}
      </span>
    </div>
  );
}

export interface ListRowProps extends HTMLAttributes<HTMLDivElement> {
  /** Spalten als grid-template-columns. */
  columns: string;
  head?: boolean;
  selected?: boolean;
  dragging?: boolean;
  /** Gedämpft: Eintrag ist nicht nutzbar (z. B. Datei fehlt). */
  dim?: boolean;
  /** Dunklere Wechselzeile. */
  zebra?: boolean;
}

/** Listen-Zeile mit festen Spalten. Zustände: normal, Zeiger darüber, ausgewählt, gezogen. */
export function ListRow({ columns, head, selected, dragging, dim, zebra, children, style, ...rest }: ListRowProps) {
  return (
    <div
      className="dw-row"
      role="row"
      data-head={head ? 'true' : undefined}
      data-selected={selected ? 'true' : undefined}
      data-dragging={dragging ? 'true' : undefined}
      data-dim={dim ? 'true' : undefined}
      data-zebra={zebra ? 'true' : undefined}
      aria-selected={head ? undefined : !!selected}
      style={{ gridTemplateColumns: columns, ...style }}
      {...rest}
    >
      {children}
    </div>
  );
}

export function Cell({ children, num, strong, style }: { children?: ReactNode; num?: boolean; strong?: boolean; style?: CSSProperties }) {
  return (
    <span className="dw-row__cell" role="cell" data-num={num ? 'true' : undefined} data-strong={strong ? 'true' : undefined} style={style}>
      {children}
    </span>
  );
}
