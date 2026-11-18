// Umgebung der Oberfläche: Uhrzeit (für Screenshots feststellbar) und URL-Schalter.

let frozenTime: Date | null = null;

/** Uhr anhalten (deterministischer Modus). */
export function freezeClock(at: Date): void {
  frozenTime = at;
}

export const isClockFrozen = (): boolean => frozenTime !== null;
export const nowDate = (): Date => frozenTime ?? new Date();

/** Zeichner per URL erzwingen: ?renderer=2d (Canvas-Rückfall prüfen). */
export function forcedRenderer(search: string): 'canvas2d' | null {
  return new URLSearchParams(search).get('renderer') === '2d' ? 'canvas2d' : null;
}
