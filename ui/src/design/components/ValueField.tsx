// Zahleneingabe (BED-05): Der angezeigte Wert eines Reglers ist anklickbar und wird zum Eingabefeld.
// Enter übernimmt, Esc verwirft; Komma und Punkt gelten beide.
import { useEffect, useRef, useState, type CSSProperties } from 'react';
import { useTip, type TipContent } from './Tooltip';

export interface ValueFieldProps {
  /** Angezeigter Wert samt Einheit, z. B. „−3,2 dB". */
  text: string;
  /** Vorbelegung des Eingabefelds (ohne Einheit); Standard: `text`. */
  editText?: string;
  /** Eingabe übernehmen. Liefert false, wenn der Text nicht lesbar ist – das Feld bleibt dann offen. */
  onCommit: (input: string) => boolean;
  ariaLabel: string;
  disabled?: boolean;
  tip?: TipContent;
  className?: string;
  style?: CSSProperties;
  /** Zustand „Eingabe offen" erzwingen (Designsystem-Seite). */
  forceEditing?: boolean;
}

export function ValueField({ text, editText, onCommit, ariaLabel, disabled, tip, className, style, forceEditing }: ValueFieldProps) {
  const [editing, setEditing] = useState(false);
  const [draft, setDraft] = useState('');
  const [bad, setBad] = useState(false);
  const input = useRef<HTMLInputElement | null>(null);
  const tipProps = useTip(editing ? undefined : tip);

  useEffect(() => {
    if (editing) {
      input.current?.focus();
      input.current?.select();
    }
  }, [editing]);

  if (editing || forceEditing) {
    return (
      <input
        ref={input}
        className={`dw-value dw-value--edit${className ? ' ' + className : ''}`}
        style={style}
        data-bad={bad ? 'true' : undefined}
        value={forceEditing && !editing ? (editText ?? text) : draft}
        readOnly={forceEditing && !editing}
        aria-label={ariaLabel}
        inputMode="decimal"
        spellCheck={false}
        autoComplete="off"
        onChange={(e) => {
          setDraft(e.target.value);
          setBad(false);
        }}
        onKeyDown={(e) => {
          if (e.key === 'Enter') {
            if (onCommit(draft)) setEditing(false);
            else setBad(true);
          } else if (e.key === 'Escape') {
            setEditing(false);
          }
          e.stopPropagation();
        }}
        onBlur={() => setEditing(false)}
      />
    );
  }
  return (
    <button
      type="button"
      className={`dw-value${className ? ' ' + className : ''}`}
      style={style}
      disabled={disabled}
      aria-label={`${ariaLabel}: ${text}`}
      onClick={() => {
        setDraft(editText ?? text);
        setBad(false);
        setEditing(true);
      }}
      {...tipProps}
    >
      {text}
    </button>
  );
}
