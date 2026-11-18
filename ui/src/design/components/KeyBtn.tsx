// Taste. Modi (BED-04): Auslösen, Umschalten (mit Lämpchen), Halten.
// Zustände: normal, Zeiger darüber, gehalten, aktiv, scharf (Live-Schutz wartet), deaktiviert, Fokus.
import { createContext, useContext, useRef, useState, type CSSProperties, type ReactNode } from 'react';
import { useTip, type TipContent } from './Tooltip';

/**
 * Seite der Fase. Regel: linke Hälfte des Pults (Deck A, Kanal A) und alles Mittige = oben links,
 * rechte Hälfte (Deck B, Kanal B) = oben rechts. Ein umschließender `CutSide` legt die Seite fest.
 */
export type Cut = 'tl' | 'tr';
const CutContext = createContext<Cut>('tl');
export const CutSide = CutContext.Provider;
export const useCut = (): Cut => useContext(CutContext);

export interface KeyBtnProps {
  mode?: 'trigger' | 'toggle' | 'hold';
  /** Eingerastet bzw. Funktion aktiv. */
  active?: boolean;
  disabled?: boolean;
  size?: 'sm' | 'md' | 'lg';
  /** Farbe im aktiven Zustand (Token), Standard: Schriftfarbe. */
  accent?: string;
  /**
   * Darstellung des aktiven Zustands: 'fill' = ganze Fläche in der Akzentfarbe (Standard),
   * 'outline' = kräftiger Umriss, Lämpchen und Zeichen in der Akzentfarbe (große Tasten wie Play).
   */
  tone?: 'fill' | 'outline';
  /** Fase oben links oder rechts; ohne Angabe gilt die Seite des umschließenden Bereichs. */
  cut?: Cut;
  block?: boolean;
  /** Zweite, kleinere Angabe rechts vom Text (z. B. Wert oder Tastenkappe). */
  sub?: ReactNode;
  tip?: TipContent;
  /** Auslösen bzw. Umschalten (Klick). */
  onPress?: () => void;
  /** Drücken und Loslassen getrennt – für „Halten" und für geschützte Aktionen (9.3). */
  onHoldStart?: () => void;
  onHoldEnd?: () => void;
  /** Zustand „gehalten" erzwingen (Designsystem-Seite, Tastatur). */
  forceHeld?: boolean;
  /** Live-Schutz: Die Aktion wartet auf Halten oder den zweiten Druck. */
  armed?: boolean;
  /** Zeiger-darüber und Fokus erzwingen (nur Designsystem-Seite). */
  forceState?: 'hover' | 'focus';
  ariaLabel?: string;
  className?: string;
  style?: CSSProperties;
  onContextMenu?: (e: React.MouseEvent) => void;
  onWheel?: (e: React.WheelEvent) => void;
  children?: ReactNode;
}

export function KeyBtn(props: KeyBtnProps) {
  const { mode = 'trigger', active = false, disabled = false, size = 'md', accent, tone = 'fill', block, sub, tip } = props;
  const sideCut = useCut();
  const cut = props.cut ?? sideCut;
  const [held, setHeld] = useState(false);
  const holding = useRef(false);
  const tipProps = useTip(tip);
  const usesHold = !!props.onHoldStart || mode === 'hold';

  const startHold = () => {
    if (holding.current) return;
    holding.current = true;
    setHeld(true);
    props.onHoldStart?.();
  };
  const endHold = () => {
    if (!holding.current) return;
    holding.current = false;
    setHeld(false);
    props.onHoldEnd?.();
  };

  const holdHandlers = usesHold
    ? {
        onPointerDown: (e: React.PointerEvent<HTMLButtonElement>) => {
          if (e.button !== 0 || disabled) return;
          e.currentTarget.setPointerCapture(e.pointerId);
          startHold();
        },
        onPointerUp: endHold,
        onPointerCancel: endHold,
        onLostPointerCapture: endHold,
        onKeyDown: (e: React.KeyboardEvent) => {
          if ((e.key === ' ' || e.key === 'Enter') && !e.repeat) startHold();
        },
        onKeyUp: (e: React.KeyboardEvent) => {
          if (e.key === ' ' || e.key === 'Enter') endHold();
        },
        onBlur: () => {
          endHold();
          tipProps.onBlur?.();
        },
      }
    : {};

  return (
    <button
      type="button"
      className={`dw-key${props.className ? ' ' + props.className : ''}`}
      data-size={size}
      data-mode={mode}
      data-tone={tone}
      data-cut={cut}
      data-block={block ? '' : undefined}
      data-active={active ? 'true' : undefined}
      data-held={held || props.forceHeld ? 'true' : undefined}
      data-armed={props.armed ? 'true' : undefined}
      data-force={props.forceState}
      disabled={disabled}
      aria-pressed={mode === 'toggle' ? active : undefined}
      aria-label={props.ariaLabel}
      style={accent ? ({ '--key-accent': accent, ...props.style } as CSSProperties) : props.style}
      // Wer Drücken/Loslassen getrennt meldet, löst nicht zusätzlich per Klick aus.
      onClick={usesHold ? undefined : props.onPress}
      // Tasten nehmen der Tastatur-Steuerung nicht den Fokus weg.
      onMouseDown={(e) => e.preventDefault()}
      onContextMenu={props.onContextMenu}
      onWheel={props.onWheel}
      {...tipProps}
      {...holdHandlers}
    >
      {mode === 'toggle' && <span className="dw-key__lamp" aria-hidden="true" />}
      <span className="dw-key__content">
        {props.children}
        {sub !== undefined && <span className="dw-key__sub">{sub}</span>}
      </span>
    </button>
  );
}
