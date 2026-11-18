// Drehregler: sechskantige Kappe mit Zeiger, Wertbogen außen. Bipolar zeigt den Bogen ab der Mitte.
import type { CSSProperties } from 'react';
import { knobAngle, KNOB_SWEEP } from '../../lib/curves';
import { remPx } from '../tokens';
import { useDragValue } from './useDragValue';
import { hideTip, useTip, type TipContent } from './Tooltip';

export interface KnobProps {
  /** Reglerweg 0 … 1. */
  value: number;
  onChange: (v: number) => void;
  /** Reglerweg des Standardwerts (Doppelklick). Ohne Angabe gibt es keinen Doppelklick-Sprung. */
  defaultValue?: number;
  /** Reglerweg der Kerbe (Bezugswert, z. B. 0 dB); Standard: `defaultValue`. */
  detent?: number;
  /** Bogen ab der Mitte statt ab dem linken Anschlag. */
  bipolar?: boolean;
  /** Anteil des Wegs am linken Anschlag, der „Kill“ bedeutet (wird markiert). */
  killZone?: number;
  size?: 'sm' | 'md' | 'lg';
  /** Farbe des Wertbogens (Token). */
  accent?: string;
  label?: string;
  /** Wert als Text unter dem Regler. */
  valueText?: string;
  disabled?: boolean;
  tip?: TipContent;
  ariaLabel?: string;
  forceHeld?: boolean;
  forceState?: 'hover' | 'focus';
}

const R_ARC = 43;
const R_CAP = 31;

function polar(r: number, deg: number): [number, number] {
  const a = (deg * Math.PI) / 180;
  return [50 + r * Math.sin(a), 50 - r * Math.cos(a)];
}

function arc(r: number, from: number, to: number): string {
  const [a, b] = from <= to ? [from, to] : [to, from];
  const [x0, y0] = polar(r, a);
  const [x1, y1] = polar(r, b);
  return `M ${x0.toFixed(2)} ${y0.toFixed(2)} A ${r} ${r} 0 ${b - a > 180 ? 1 : 0} 1 ${x1.toFixed(2)} ${y1.toFixed(2)}`;
}

const HEX = Array.from({ length: 6 }, (_, i) => polar(R_CAP, i * 60).map((v) => v.toFixed(2)).join(',')).join(' ');

export function Knob(props: KnobProps) {
  const { value, bipolar, size = 'md', disabled, defaultValue } = props;
  const drag = useDragValue<HTMLDivElement>({
    value,
    onChange: props.onChange,
    defaultValue,
    disabled,
    axis: 'y',
    travel: () => remPx() * 12,
    // Beim Ziehen keine Kurzhilfe: Der Wert steht neben dem Regler, die Hilfe würde Nachbarn verdecken.
    onHeldChange: (h) => {
      if (h) hideTip();
    },
  });
  const held = drag.held || !!props.forceHeld;
  const tipProps = useTip(drag.held ? undefined : props.tip);
  const angle = knobAngle(value);
  const start = -KNOB_SWEEP / 2;
  const detent = props.detent ?? defaultValue;
  const origin = bipolar ? knobAngle(detent ?? 0.5) : start;
  const showValue = Math.abs(angle - origin) > 0.5;
  const [dx0, dy0] = polar(47, knobAngle(detent ?? 0));
  const [dx1, dy1] = polar(52, knobAngle(detent ?? 0));

  return (
    <div
      className="dw-knob"
      data-size={size}
      data-held={held ? 'true' : undefined}
      data-disabled={disabled ? 'true' : undefined}
      data-force={props.forceState}
      style={props.accent ? ({ '--knob-accent': props.accent } as CSSProperties) : undefined}
    >
      <div
        className="dw-knob__dial"
        ref={drag.ref}
        role="slider"
        tabIndex={disabled ? -1 : 0}
        aria-label={props.ariaLabel ?? props.label ?? props.tip?.name}
        aria-valuemin={0}
        aria-valuemax={100}
        aria-valuenow={Math.round(value * 100)}
        aria-valuetext={props.valueText ?? props.tip?.value}
        aria-disabled={disabled || undefined}
        {...tipProps}
        {...drag.handlers}
      >
        <svg viewBox="0 0 100 100" aria-hidden="true">
          <path className="dw-knob__track" d={arc(R_ARC, start, -start)} />
          {props.killZone !== undefined && props.killZone > 0 && (
            <path className="dw-knob__kill" d={arc(R_ARC, start + 1.5, start + KNOB_SWEEP * props.killZone)} />
          )}
          {showValue && <path className="dw-knob__value" d={arc(R_ARC, origin, angle)} />}
          {detent !== undefined && <line className="dw-knob__detent" x1={dx0} y1={dy0} x2={dx1} y2={dy1} />}
          <g transform={`rotate(${angle.toFixed(2)} 50 50)`}>
            <polygon className="dw-knob__cap" points={HEX} />
            <line className="dw-knob__pointer" x1="50" y1="38" x2="50" y2="21" />
          </g>
        </svg>
      </div>
      {props.label && <span className="dw-knob__label">{props.label}</span>}
      {props.valueText !== undefined && <span className="dw-knob__text">{props.valueText}</span>}
    </div>
  );
}
