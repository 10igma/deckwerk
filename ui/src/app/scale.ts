// Skalierung (VIS-10): Die Wurzelgröße wächst mit dem Fenster, aber langsamer als linear.
// So wird bei 4K alles größer, und trotzdem bleibt mehr Platz für Wellenform und Liste.

export const BASE_WIDTH = 1280;
export const BASE_HEIGHT = 720;
export const BASE_REM = 14;

/** Wurzelgröße in Pixeln für eine Fenstergröße. `userScale` ist die Einstellung „Skalierung“. */
export function remForViewport(width: number, height: number, userScale = 1): number {
  const s = Math.max(0.5, Math.min(width / BASE_WIDTH, height / BASE_HEIGHT));
  const rem = BASE_REM * Math.pow(s, 0.75) * userScale;
  return Math.round(Math.min(48, Math.max(10, rem)) * 4) / 4;
}

export function installScale(): () => void {
  const apply = () => {
    document.documentElement.style.fontSize = `${remForViewport(window.innerWidth, window.innerHeight)}px`;
  };
  apply();
  window.addEventListener('resize', apply);
  return () => window.removeEventListener('resize', apply);
}
