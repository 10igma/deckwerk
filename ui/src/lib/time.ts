// Zeitformate für Decks und Bibliothek.

const MINUS = '−';

/** Position als m:ss.d (Zehntel), z. B. 3:07.4. Negative Werte mit echtem Minuszeichen. */
export function formatTime(seconds: number, decimalSep = '.'): string {
  if (!Number.isFinite(seconds)) return `-:--${decimalSep}-`;
  const neg = seconds < 0;
  // Auf Zehntel abschneiden, damit die Anzeige nie vorauseilt.
  const tenthsTotal = Math.floor(Math.abs(seconds) * 10 + 1e-6);
  const m = Math.floor(tenthsTotal / 600);
  const s = Math.floor((tenthsTotal % 600) / 10);
  const d = tenthsTotal % 10;
  return `${neg ? MINUS : ''}${m}:${String(s).padStart(2, '0')}${decimalSep}${d}`;
}

/** Restzeit: immer mit Minuszeichen, nie positiv. */
export function formatRemaining(pos: number, duration: number, decimalSep = '.'): string {
  const rest = Math.max(0, duration - pos);
  return MINUS + formatTime(rest, decimalSep);
}

/** Länge als m:ss (gerundet), für Listen. */
export function formatDuration(seconds: number): string {
  if (!Number.isFinite(seconds) || seconds < 0) return '-:--';
  const total = Math.round(seconds);
  return `${Math.floor(total / 60)}:${String(total % 60).padStart(2, '0')}`;
}
