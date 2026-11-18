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

/**
 * Wie WebGL auf den Schirm kommt. WebKitGTK (Tauri unter Linux) zeigt WebGL-Flächen ohne
 * beschleunigte Darstellung nicht an – dort wird das fertige Bild in eine 2D-Fläche kopiert.
 * Erzwingen per URL: ?renderer=blit.
 */
export function webglPresent(search: string, userAgent: string): 'direct' | 'blit' {
  if (new URLSearchParams(search).get('renderer') === 'blit') return 'blit';
  const webkitGtk = /Linux/.test(userAgent) && /AppleWebKit/.test(userAgent) && !/Chrome|Chromium|Android/.test(userAgent);
  return webkitGtk ? 'blit' : 'direct';
}
