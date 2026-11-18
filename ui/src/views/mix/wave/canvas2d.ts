// Rückfall ohne WebGL: dieselbe Szene spaltenweise in Canvas 2D.
import type { DeckId, Waveform } from '../../../core/types';
import type { WaveRenderer } from './renderer';
import { BAND_SHARE, sampleColumn, type Rgb, type StageColors, type StageScene } from './scene';

const css = (c: Rgb, k = 1): string => `rgb(${Math.round(c[0] * 255 * k)}, ${Math.round(c[1] * 255 * k)}, ${Math.round(c[2] * 255 * k)})`;
const blend = (bg: Rgb, fg: Rgb, t: number): Rgb => [bg[0] + (fg[0] - bg[0]) * t, bg[1] + (fg[1] - bg[1]) * t, bg[2] + (fg[2] - bg[2]) * t];

export class Canvas2DWaveRenderer implements WaveRenderer {
  readonly kind = 'canvas2d' as const;
  readonly info = 'Canvas 2D';
  private ctx: CanvasRenderingContext2D;
  private tracks = new Map<DeckId, Waveform>();
  private col = new Float32Array(3);

  constructor(canvas: HTMLCanvasElement) {
    const ctx = canvas.getContext('2d', { alpha: false });
    if (!ctx) throw new Error('Canvas 2D nicht verfügbar');
    this.ctx = ctx;
  }

  setTrack(deck: DeckId, waveform: Waveform | null): void {
    if (waveform) this.tracks.set(deck, waveform);
    else this.tracks.delete(deck);
  }

  draw(scene: StageScene, colors: StageColors): void {
    const ctx = this.ctx;
    ctx.globalAlpha = 1;
    ctx.fillStyle = css(colors.bg);
    ctx.fillRect(0, 0, scene.width, scene.height);
    const bandColors = [colors.low, colors.mid, colors.high];
    // Schraffur wie im Shader: Streifen unter 135°, Periode 6 Pixel (mal Geräte-Faktor).
    const period = 6 * scene.dpr;
    const shade = (c: Rgb, v: number, side: number): string => css(blend(colors.bg, blend(colors.bg, c, v), side === 0 ? colors.playedDim : 1));

    for (const lane of scene.lanes) {
      const wf = this.tracks.get(lane.deck);
      if (!lane.loaded || !wf) continue;
      const data = wf.levels[lane.level].data;
      const center = lane.w / 2;
      // Farben je Band und Seite der Abspielmarke vorab mischen; gekillte Bänder hell/dunkel für die Schraffur.
      const styles = [0, 1].map((side) =>
        bandColors.map((c, b) => {
          const g = lane.bandGain[b];
          return g < 0.02 ? [shade(c, 0.07, side), shade(c, 0.34, side)] : [shade(c, 0.42 + 0.58 * g, side)];
        }),
      );
      for (let x = 0; x < lane.w; x++) {
        sampleColumn(data, lane.points, lane.idxCenter + (x + 0.5 - center) * lane.idxPerPx, lane.idxPerPx, this.col);
        const side = x < center ? 0 : 1;
        let from = 0;
        for (let b = 0; b < 3; b++) {
          const hgt = this.col[b] * BAND_SHARE[b] * lane.h * 0.97;
          if (hgt > 0.3) {
            const y = lane.dir > 0 ? lane.y + lane.h - from - hgt : lane.y + from;
            const st = styles[side][b];
            ctx.fillStyle = st[0];
            ctx.fillRect(lane.x + x, y, 1, hgt);
            if (st.length > 1) {
              // Helle Streifen: dort, wo (x + y von unten) in der zweiten Hälfte der Periode liegt.
              ctx.fillStyle = st[1];
              const yBottom = scene.height - (y + hgt);
              const base = lane.x + x + yBottom;
              let k = Math.floor(base / period) * period + period / 2 - base;
              for (; k < hgt; k += period) {
                const s0 = Math.max(0, k);
                const s1 = Math.min(hgt, k + period / 2);
                if (s1 > s0) ctx.fillRect(lane.x + x, y + hgt - s1, 1, s1 - s0);
              }
            }
          }
          from += hgt;
        }
      }
    }

    for (const r of scene.rects) {
      ctx.globalAlpha = r.color[3];
      ctx.fillStyle = css([r.color[0], r.color[1], r.color[2]]);
      ctx.fillRect(r.x, r.y, r.w, r.h);
    }
    ctx.globalAlpha = 1;
  }

  dispose(): void {
    this.tracks.clear();
  }
}
