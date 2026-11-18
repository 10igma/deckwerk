// Fader: senkrecht (Kanal) oder waagrecht (Crossfader, Tempo). Gezogen wird relativ –
// ein Klick neben die Kappe lässt den Wert nicht springen.
import { useRef, type CSSProperties } from 'react';
import { useDragValue } from './useDragValue';
import { useTip, type TipContent, type TipSide } from './Tooltip';
import { CutSide } from './KeyBtn';
import { DeckPlate } from './Marks';
import type { DeckId } from '../../core/types';

export interface FaderTick {
  /** Reglerweg 0 … 1. */
  at: number;
  major?: boolean;
}

export interface FaderProps {
  /** Reglerweg 0 … 1. */
  value: number;
  onChange: (v: number) => void;
  defaultValue?: number;
  orientation?: 'v' | 'h';
  /** Füllung ab diesem Reglerweg (z. B. 0,5 für bipolar); ohne Angabe ab 0. */
  fillFrom?: number;
  ticks?: FaderTick[];
  accent?: string;
  disabled?: boolean;
  /** Wert wird von außen geführt (z. B. Sync): Kappe hohl. */
  guided?: boolean;
  tip?: TipContent;
  /** Lage der Kurzhilfe; senkrechte Fader zeigen sie seitlich, damit sie beim Ziehen nichts verdeckt. */
  tipSide?: TipSide;
  ariaLabel?: string;
  wheelStep?: number;
  forceHeld?: boolean;
  onHeldChange?: (held: boolean) => void;
  style?: CSSProperties;
}

export function Fader(props: FaderProps) {
  const { value, orientation = 'v', fillFrom = 0, ticks, disabled } = props;
  const slot = useRef<HTMLDivElement | null>(null);
  const vertical = orientation === 'v';
  const drag = useDragValue<HTMLDivElement>({
    value,
    onChange: props.onChange,
    defaultValue: props.defaultValue,
    disabled,
    axis: vertical ? 'y' : 'x',
    travel: () => {
      const r = slot.current?.getBoundingClientRect();
      return r ? (vertical ? r.height : r.width) : 200;
    },
    wheelStep: props.wheelStep,
    onHeldChange: props.onHeldChange,
  });
  const tipProps = useTip(props.tip, drag.held, props.tipSide ?? 'top');
  const v = Math.min(1, Math.max(0, value));
  const lo = Math.min(v, fillFrom);
  const hi = Math.max(v, fillFrom);
  const pct = (x: number) => `${(x * 100).toFixed(3)}%`;
  const along = (x: number) => `calc((100% - var(--cap)) * ${x.toFixed(5)})`;
  const tickPos = (x: number) => `calc(var(--cap) / 2 + (100% - var(--cap)) * ${x.toFixed(5)})`;

  return (
    <div
      className="dw-fader"
      ref={drag.ref}
      data-orient={orientation}
      data-held={drag.held || props.forceHeld ? 'true' : undefined}
      data-disabled={disabled ? 'true' : undefined}
      data-guided={props.guided ? 'true' : undefined}
      role="slider"
      tabIndex={disabled ? -1 : 0}
      aria-orientation={vertical ? 'vertical' : 'horizontal'}
      aria-label={props.ariaLabel ?? props.tip?.name}
      aria-valuemin={0}
      aria-valuemax={100}
      aria-valuenow={Math.round(v * 100)}
      aria-valuetext={props.tip?.value}
      aria-disabled={disabled || undefined}
      style={props.accent ? ({ '--fader-accent': props.accent, ...props.style } as CSSProperties) : props.style}
      {...tipProps}
      {...drag.handlers}
    >
      {ticks?.map((t, i) => (
        <span
          key={i}
          className="dw-fader__tick"
          data-major={t.major ? 'true' : undefined}
          style={vertical ? { bottom: tickPos(t.at) } : { left: tickPos(t.at) }}
        />
      ))}
      <div className="dw-fader__slot" ref={slot}>
        <div
          className="dw-fader__fill"
          style={vertical ? { bottom: pct(lo), top: pct(1 - hi) } : { left: pct(lo), right: pct(1 - hi) }}
        />
      </div>
      <div className="dw-fader__cap" style={vertical ? { bottom: along(v) } : { left: along(v) }} />
    </div>
  );
}

export interface CrossfaderProps {
  /** −1 (links) … +1 (rechts). */
  value: number;
  onChange: (v: number) => void;
  left: DeckId;
  right: DeckId;
  tip?: TipContent;
  disabled?: boolean;
  forceHeld?: boolean;
}

const XF_TICKS: FaderTick[] = [0, 0.25, 0.5, 0.75, 1].map((at) => ({ at, major: at === 0.5 }));

/** Crossfader: waagrechter Fader zwischen zwei Deck-Schildern, Mitte markiert. */
export function Crossfader(props: CrossfaderProps) {
  return (
    <div className="dw-xfader">
      <DeckPlate deck={props.left} size="sm" />
      <Fader
        orientation="h"
        value={(props.value + 1) / 2}
        onChange={(p) => {
          const v = p * 2 - 1;
          // Leichte Rast in der Mitte.
          props.onChange(Math.abs(v) < 0.015 ? 0 : v);
        }}
        defaultValue={0.5}
        fillFrom={0.5}
        ticks={XF_TICKS}
        tip={props.tip}
        disabled={props.disabled}
        forceHeld={props.forceHeld}
      />
      <CutSide value="tr">
        <DeckPlate deck={props.right} size="sm" />
      </CutSide>
    </div>
  );
}
