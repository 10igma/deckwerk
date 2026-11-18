// Zeichner in WebGL 2 (VIS-02): je Bahn ein Fragment-Shader, der die drei Bänder gestapelt
// aus einer Textur liest; Linien und Marken als instanzierte Rechtecke.
import type { DeckId, Waveform } from '../../../core/types';
import type { WaveRenderer } from './renderer';
import { BAND_SHARE, packLevel, type StageColors, type StageScene } from './scene';

const TEX_WIDTH = 2048;

const LANE_VS = `#version 300 es
in vec2 a_pos;
void main() { gl_Position = vec4(a_pos, 0.0, 1.0); }`;

// Wichtig: Ganzzahlen ausdrücklich in voller Genauigkeit. Ohne Angabe gilt im Fragment-Shader
// „mediump int" (mindestens 16 Bit) – Punktindizes über 32 767 laufen dann auf manchen Treibern
// über (so gesehen in WebKitGTK), und die Bahn bleibt leer.
const LANE_FS = `#version 300 es
precision highp float;
precision highp int;
precision highp sampler2D;
uniform sampler2D u_tex;
uniform int u_texW;
uniform int u_points;
uniform float u_idxCenter;
uniform float u_idxPerPx;
uniform vec2 u_origin;
uniform vec2 u_size;
uniform float u_dir;
uniform vec3 u_colLow;
uniform vec3 u_colMid;
uniform vec3 u_colHigh;
uniform vec3 u_bg;
uniform vec3 u_gain;
uniform vec3 u_share;
uniform float u_playedDim;
uniform float u_dpr;
out vec4 outColor;

vec3 point(int i) {
  if (i < 0 || i >= u_points) return vec3(0.0);
  return texelFetch(u_tex, ivec2(i % u_texW, i / u_texW), 0).rgb;
}

vec3 ampAt(float px) {
  float c = u_idxCenter + (px - u_size.x * 0.5) * u_idxPerPx;
  if (u_idxPerPx < 1.0) {
    float f = c - 0.5;
    float fl = floor(f);
    return mix(point(int(fl)), point(int(fl) + 1), f - fl);
  }
  int i0 = int(floor(c - u_idxPerPx * 0.5));
  int i1 = int(floor(c + u_idxPerPx * 0.5));
  vec3 m = vec3(0.0);
  for (int k = 0; k < 6; k++) {
    int i = i0 + k;
    if (i > i1) break;
    m = max(m, point(i));
  }
  return m;
}

// Sichtbarkeit eines Bands: Kill = Schraffur, sonst Helligkeit nach EQ-Wert.
float vis(float gain, float hatch) {
  return gain < 0.02 ? mix(0.07, 0.34, hatch) : mix(0.42, 1.0, gain);
}

void main() {
  vec2 p = gl_FragCoord.xy - u_origin;
  float d = u_dir > 0.0 ? p.y : u_size.y - p.y;
  vec3 a = ampAt(p.x) * u_share * u_size.y * 0.97;
  float t1 = a.x;
  float t2 = t1 + a.y;
  float t3 = t2 + a.z;
  float c1 = clamp(t1 - d + 0.5, 0.0, 1.0);
  float c2 = clamp(t2 - d + 0.5, 0.0, 1.0);
  float c3 = clamp(t3 - d + 0.5, 0.0, 1.0);
  float hatch = step(0.5, fract((gl_FragCoord.x + gl_FragCoord.y) / (6.0 * u_dpr)));
  vec3 col = u_bg;
  col = mix(col, mix(u_bg, u_colHigh, vis(u_gain.z, hatch)), c3);
  col = mix(col, mix(u_bg, u_colMid, vis(u_gain.y, hatch)), c2);
  col = mix(col, mix(u_bg, u_colLow, vis(u_gain.x, hatch)), c1);
  if (p.x < u_size.x * 0.5) col = mix(u_bg, col, u_playedDim);
  outColor = vec4(col, 1.0);
}`;

const RECT_VS = `#version 300 es
in vec2 a_corner;
in vec4 a_rect;
in vec4 a_color;
uniform vec2 u_canvas;
out vec4 v_color;
void main() {
  vec2 p = a_rect.xy + a_corner * a_rect.zw;
  gl_Position = vec4(p.x / u_canvas.x * 2.0 - 1.0, 1.0 - p.y / u_canvas.y * 2.0, 0.0, 1.0);
  v_color = a_color;
}`;

const RECT_FS = `#version 300 es
precision mediump float;
in vec4 v_color;
out vec4 outColor;
void main() { outColor = v_color; }`;

interface LevelTex {
  tex: WebGLTexture;
  points: number;
}

function compile(gl: WebGL2RenderingContext, type: number, src: string): WebGLShader {
  const sh = gl.createShader(type);
  if (!sh) throw new Error('Shader konnte nicht angelegt werden');
  gl.shaderSource(sh, src);
  gl.compileShader(sh);
  if (!gl.getShaderParameter(sh, gl.COMPILE_STATUS)) {
    const log = gl.getShaderInfoLog(sh);
    gl.deleteShader(sh);
    throw new Error(`Shader-Fehler: ${log}`);
  }
  return sh;
}

function link(gl: WebGL2RenderingContext, vs: string, fs: string): WebGLProgram {
  const prog = gl.createProgram();
  if (!prog) throw new Error('Programm konnte nicht angelegt werden');
  gl.attachShader(prog, compile(gl, gl.VERTEX_SHADER, vs));
  gl.attachShader(prog, compile(gl, gl.FRAGMENT_SHADER, fs));
  gl.linkProgram(prog);
  if (!gl.getProgramParameter(prog, gl.LINK_STATUS)) throw new Error(`Link-Fehler: ${gl.getProgramInfoLog(prog)}`);
  return prog;
}

export class WebGLWaveRenderer implements WaveRenderer {
  readonly kind = 'webgl2' as const;
  readonly info: string;
  private gl: WebGL2RenderingContext;
  private laneProg: WebGLProgram;
  private rectProg: WebGLProgram;
  private laneVao: WebGLVertexArrayObject;
  private rectVao: WebGLVertexArrayObject;
  private rectBuf: WebGLBuffer;
  private rectData = new Float32Array(8 * 256);
  private laneU: Record<string, WebGLUniformLocation | null> = {};
  private rectCanvasU: WebGLUniformLocation | null;
  private tracks = new Map<DeckId, LevelTex[]>();

  /** Nur bei `present = 'blit'`: eigene, unsichtbare Zeichenfläche für WebGL und der 2D-Kontext der sichtbaren. */
  private glCanvas: HTMLCanvasElement | null = null;
  private out: CanvasRenderingContext2D | null = null;

  /**
   * Wirft, wenn kein WebGL-2-Kontext entsteht – der Aufrufer nimmt dann Canvas 2D.
   * `present`: 'direct' zeichnet in die sichtbare Fläche. 'blit' rechnet in einer unsichtbaren
   * WebGL-Fläche und kopiert jedes Bild in die sichtbare 2D-Fläche – nötig, wo die Web-Ansicht
   * WebGL-Flächen nicht auf den Schirm bringt (WebKitGTK ohne beschleunigte Darstellung).
   */
  constructor(canvas: HTMLCanvasElement, present: 'direct' | 'blit' = 'direct') {
    let target = canvas;
    if (present === 'blit') {
      this.out = canvas.getContext('2d', { alpha: false });
      if (!this.out) throw new Error('Canvas 2D nicht verfügbar');
      this.glCanvas = document.createElement('canvas');
      target = this.glCanvas;
    }
    const gl = target.getContext('webgl2', { alpha: false, antialias: false, depth: false, stencil: false, powerPreference: 'high-performance' });
    if (!gl) throw new Error('WebGL 2 nicht verfügbar');
    this.gl = gl;
    const dbg = gl.getExtension('WEBGL_debug_renderer_info');
    this.info = String(dbg ? gl.getParameter(dbg.UNMASKED_RENDERER_WEBGL) : gl.getParameter(gl.RENDERER)) + (present === 'blit' ? ' · über Canvas 2D gezeigt' : '');
    this.laneProg = link(gl, LANE_VS, LANE_FS);
    this.rectProg = link(gl, RECT_VS, RECT_FS);
    for (const name of ['u_tex', 'u_texW', 'u_points', 'u_idxCenter', 'u_idxPerPx', 'u_origin', 'u_size', 'u_dir', 'u_colLow', 'u_colMid', 'u_colHigh', 'u_bg', 'u_gain', 'u_share', 'u_playedDim', 'u_dpr']) {
      this.laneU[name] = gl.getUniformLocation(this.laneProg, name);
    }
    this.rectCanvasU = gl.getUniformLocation(this.rectProg, 'u_canvas');

    // Bahn: ein Rechteck über den ganzen Ausschnitt (Viewport begrenzt es auf die Bahn).
    this.laneVao = gl.createVertexArray()!;
    gl.bindVertexArray(this.laneVao);
    const quad = gl.createBuffer();
    gl.bindBuffer(gl.ARRAY_BUFFER, quad);
    gl.bufferData(gl.ARRAY_BUFFER, new Float32Array([-1, -1, 1, -1, -1, 1, 1, 1]), gl.STATIC_DRAW);
    const posLoc = gl.getAttribLocation(this.laneProg, 'a_pos');
    gl.enableVertexAttribArray(posLoc);
    gl.vertexAttribPointer(posLoc, 2, gl.FLOAT, false, 0, 0);

    // Rechtecke: Einheitsquadrat, je Instanz Lage und Farbe.
    this.rectVao = gl.createVertexArray()!;
    gl.bindVertexArray(this.rectVao);
    const corner = gl.createBuffer();
    gl.bindBuffer(gl.ARRAY_BUFFER, corner);
    gl.bufferData(gl.ARRAY_BUFFER, new Float32Array([0, 0, 1, 0, 0, 1, 1, 1]), gl.STATIC_DRAW);
    const cornerLoc = gl.getAttribLocation(this.rectProg, 'a_corner');
    gl.enableVertexAttribArray(cornerLoc);
    gl.vertexAttribPointer(cornerLoc, 2, gl.FLOAT, false, 0, 0);
    this.rectBuf = gl.createBuffer()!;
    gl.bindBuffer(gl.ARRAY_BUFFER, this.rectBuf);
    gl.bufferData(gl.ARRAY_BUFFER, this.rectData.byteLength, gl.DYNAMIC_DRAW);
    const rectLoc = gl.getAttribLocation(this.rectProg, 'a_rect');
    const colorLoc = gl.getAttribLocation(this.rectProg, 'a_color');
    gl.enableVertexAttribArray(rectLoc);
    gl.vertexAttribPointer(rectLoc, 4, gl.FLOAT, false, 32, 0);
    gl.vertexAttribDivisor(rectLoc, 1);
    gl.enableVertexAttribArray(colorLoc);
    gl.vertexAttribPointer(colorLoc, 4, gl.FLOAT, false, 32, 16);
    gl.vertexAttribDivisor(colorLoc, 1);
    gl.bindVertexArray(null);
  }

  setTrack(deck: DeckId, waveform: Waveform | null): void {
    const gl = this.gl;
    for (const old of this.tracks.get(deck) ?? []) gl.deleteTexture(old.tex);
    this.tracks.delete(deck);
    if (!waveform) return;
    const levels: LevelTex[] = waveform.levels.map((lv) => {
      const packed = packLevel(lv.data, TEX_WIDTH);
      const tex = gl.createTexture()!;
      gl.bindTexture(gl.TEXTURE_2D, tex);
      gl.texImage2D(gl.TEXTURE_2D, 0, gl.RGBA8, packed.width, packed.height, 0, gl.RGBA, gl.UNSIGNED_BYTE, packed.bytes);
      gl.texParameteri(gl.TEXTURE_2D, gl.TEXTURE_MIN_FILTER, gl.NEAREST);
      gl.texParameteri(gl.TEXTURE_2D, gl.TEXTURE_MAG_FILTER, gl.NEAREST);
      gl.texParameteri(gl.TEXTURE_2D, gl.TEXTURE_WRAP_S, gl.CLAMP_TO_EDGE);
      gl.texParameteri(gl.TEXTURE_2D, gl.TEXTURE_WRAP_T, gl.CLAMP_TO_EDGE);
      return { tex, points: packed.points };
    });
    this.tracks.set(deck, levels);
  }

  draw(scene: StageScene, colors: StageColors): void {
    const gl = this.gl;
    const { width, height } = scene;
    if (this.glCanvas && (this.glCanvas.width !== width || this.glCanvas.height !== height)) {
      this.glCanvas.width = width;
      this.glCanvas.height = height;
    }
    gl.disable(gl.BLEND);
    gl.viewport(0, 0, width, height);
    gl.clearColor(colors.bg[0], colors.bg[1], colors.bg[2], 1);
    gl.clear(gl.COLOR_BUFFER_BIT);

    gl.useProgram(this.laneProg);
    gl.bindVertexArray(this.laneVao);
    const u = this.laneU;
    gl.uniform1i(u.u_tex, 0);
    gl.uniform1i(u.u_texW, TEX_WIDTH);
    gl.uniform3fv(u.u_colLow, colors.low);
    gl.uniform3fv(u.u_colMid, colors.mid);
    gl.uniform3fv(u.u_colHigh, colors.high);
    gl.uniform3fv(u.u_bg, colors.bg);
    gl.uniform3fv(u.u_share, BAND_SHARE);
    gl.uniform1f(u.u_playedDim, colors.playedDim);
    gl.uniform1f(u.u_dpr, scene.dpr);
    gl.activeTexture(gl.TEXTURE0);
    for (const lane of scene.lanes) {
      const tex = this.tracks.get(lane.deck)?.[lane.level];
      if (!lane.loaded || !tex) continue;
      const yBottom = height - lane.y - lane.h;
      gl.viewport(lane.x, yBottom, lane.w, lane.h);
      gl.bindTexture(gl.TEXTURE_2D, tex.tex);
      gl.uniform1i(u.u_points, tex.points);
      gl.uniform1f(u.u_idxCenter, lane.idxCenter);
      gl.uniform1f(u.u_idxPerPx, lane.idxPerPx);
      gl.uniform2f(u.u_origin, lane.x, yBottom);
      gl.uniform2f(u.u_size, lane.w, lane.h);
      gl.uniform1f(u.u_dir, lane.dir);
      gl.uniform3fv(u.u_gain, lane.bandGain);
      gl.drawArrays(gl.TRIANGLE_STRIP, 0, 4);
    }

    this.drawRects(scene);
    // Fertiges Bild in die sichtbare Fläche kopieren.
    if (this.out && this.glCanvas) this.out.drawImage(this.glCanvas, 0, 0);
  }

  /** Linien und Marken als instanzierte Rechtecke. */
  private drawRects(scene: StageScene): void {
    const gl = this.gl;
    const { width, height } = scene;
    const n = scene.rects.length;
    if (n === 0) return;
    if (this.rectData.length < n * 8) {
      this.rectData = new Float32Array(Math.max(n * 8, this.rectData.length * 2));
      gl.bindBuffer(gl.ARRAY_BUFFER, this.rectBuf);
      gl.bufferData(gl.ARRAY_BUFFER, this.rectData.byteLength, gl.DYNAMIC_DRAW);
    }
    for (let i = 0; i < n; i++) {
      const r = scene.rects[i];
      this.rectData.set([r.x, r.y, r.w, r.h, r.color[0], r.color[1], r.color[2], r.color[3]], i * 8);
    }
    gl.viewport(0, 0, width, height);
    gl.enable(gl.BLEND);
    gl.blendFuncSeparate(gl.SRC_ALPHA, gl.ONE_MINUS_SRC_ALPHA, gl.ONE, gl.ONE);
    gl.useProgram(this.rectProg);
    gl.uniform2f(this.rectCanvasU, width, height);
    gl.bindVertexArray(this.rectVao);
    gl.bindBuffer(gl.ARRAY_BUFFER, this.rectBuf);
    gl.bufferSubData(gl.ARRAY_BUFFER, 0, this.rectData.subarray(0, n * 8));
    gl.drawArraysInstanced(gl.TRIANGLE_STRIP, 0, 4, n);
    gl.bindVertexArray(null);
  }

  /**
   * Selbsttest nach dem Zeichnen: Liest einen Punkt der Abspielmarke zurück. Manche Umgebungen
   * liefern einen WebGL-2-Kontext, der nichts zeichnet – dann ist Canvas 2D die bessere Wahl.
   */
  selfTest(scene: StageScene): { ok: boolean; detail: string } {
    const gl = this.gl;
    const px = new Uint8Array(4);
    const x = Math.round(scene.width / 2);
    const y = Math.round(scene.height / 2);
    gl.readPixels(x, y, 1, 1, gl.RGBA, gl.UNSIGNED_BYTE, px);
    const err = gl.getError();
    // Die Abspielmarke ist heller als der Grund (Kalkweiß oder Umriss-Grau).
    const ok = err === gl.NO_ERROR && px[0] > 60;
    return { ok, detail: `rgb ${px[0]},${px[1]},${px[2]} · Fehler 0x${err.toString(16)}` };
  }

  dispose(): void {
    const gl = this.gl;
    for (const levels of this.tracks.values()) for (const l of levels) gl.deleteTexture(l.tex);
    this.tracks.clear();
    gl.deleteProgram(this.laneProg);
    gl.deleteProgram(this.rectProg);
  }
}
