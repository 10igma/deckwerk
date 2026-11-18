// Taste. Modi (BED-04): Auslösen, Umschalten (mit Lämpchen), Halten.
// Zustände: normal, aktiv, gehalten, deaktiviert.
import { useRef, useState, type CSSProperties, type ReactNode } from 'react';
import { useTip, type TipContent } from './Tooltip';

export interface KeyBtnProps {
  mode?: 'trigger' | 'toggle' | 'hold';
  /** Eingerastet bzw. Funktion aktiv. */
  active?: boolean;
  disabled?: boolean;
  size?: 'sm' | 'md' | 'lg';
  /** Farbe im aktiven Zustand (Token), Standard: Schriftfarbe. */
  accent?: string;
  /** Fase oben links (Standard) oder oben rechts (gespiegelte Seite). */
  cut?: 'tl' | 'tr';
  block?: boolean;
  /** Zweite, kleinere Zeile rechts vom Text (z. B. Modus). */
  sub?: ReactNode;
  tip?: TipContent;
  /** Auslösen bzw. Umschalten. */
  onPress?: () => void;
  /** Nur Modus „Halten“. */
  onHoldStart?: () => void;
  onHoldEnd?: () => void;
  /** Zustand „gehalten“ erzwingen (Designsystem-Seite, Tastatur). */
  forceHeld?: boolean;
  ariaLabel?: string;
  className?: string;
  style?: CSSProperties;
  children?: ReactNode;
}

export function KeyBtn(props: KeyBtnProps) {
  const { mode = 'trigger', active = false, disabled = false, size = 'md', accent, cut = 'tl', block, sub, tip } = props;
  const [held, setHeld] = useState(false);
  const holding = useRef(false);
  const tipProps = useTip(tip);

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

  const holdHandlers =
    mode === 'hold'
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
      data-cut={cut}
      data-block={block ? '' : undefined}
      data-active={active ? 'true' : undefined}
      data-held={held || props.forceHeld ? 'true' : undefined}
      disabled={disabled}
      aria-pressed={mode === 'toggle' ? active : undefined}
      aria-label={props.ariaLabel}
      style={accent ? ({ '--key-accent': accent, ...props.style } as CSSProperties) : props.style}
      onClick={mode === 'hold' ? undefined : props.onPress}
      // Tasten nehmen der Tastatur-Steuerung nicht den Fokus weg.
      onMouseDown={(e) => e.preventDefault()}
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
