// Zahleneingabe an Reglern (BED-05): Text → Zahl. Komma oder Punkt, echtes oder einfaches Minus,
// Einheit dahinter erlaubt; „−∞", „-inf" und „aus" ergeben −Infinity bzw. werden vom Aufrufer gedeutet.

const MINUS = /[−–—]/g;

/** Liest eine Zahl aus einer Eingabe. Liefert null, wenn nichts Lesbares dasteht. */
export function parseNumber(input: string): number | null {
  const text = input.trim().toLowerCase().replace(MINUS, '-').replace(/\s+/g, '');
  if (text === '') return null;
  if (/^-(∞|inf(inity)?)/.test(text)) return -Infinity;
  const m = /^([+-]?)(\d+(?:[.,]\d*)?|[.,]\d+)/.exec(text.replace('±', ''));
  if (!m) return null;
  const value = parseFloat(m[2].replace(',', '.'));
  if (!Number.isFinite(value)) return null;
  return m[1] === '-' ? -value : value;
}

/** Wie `parseNumber`, begrenzt auf einen Bereich; −Infinity bleibt erhalten, wenn `allowKill`. */
export function parseInRange(input: string, min: number, max: number, allowKill = false): number | null {
  const v = parseNumber(input);
  if (v === null) return null;
  if (v === -Infinity) return allowKill ? v : min;
  return Math.min(max, Math.max(min, v));
}
