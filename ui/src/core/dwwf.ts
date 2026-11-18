// Wellenform im Binärformat „DWWF" (docs/kern-schnittstelle.md, `Waveform::to_bytes` in dw-core).
// Little Endian. Kopf: "DWWF", Version u32 (= 1), Abtastrate u32, Frames u64, Anzahl Stufen u32.
// Je Stufe: Samples je Punkt u32, Anzahl Punkte u32, dann 9 Bytes je Punkt.
// Die erste Stufe ist die Übersicht, danach folgt die Pyramide.
import { VALUES_PER_POINT, type Waveform, type WaveformLevel } from './types';

export const DWWF_MAGIC = 'DWWF';
export const DWWF_VERSION = 1;
const HEADER_BYTES = 24;

export class WaveformFormatError extends Error {}

function decodeLevel(view: DataView, bytes: Uint8Array, pos: number): { level: WaveformLevel; next: number } {
  if (pos + 8 > bytes.length) throw new WaveformFormatError('Wellenform: Stufenkopf abgeschnitten');
  const samplesPerPoint = view.getUint32(pos, true);
  const points = view.getUint32(pos + 4, true);
  const len = points * VALUES_PER_POINT;
  const start = pos + 8;
  if (start + len > bytes.length) throw new WaveformFormatError('Wellenform: Daten abgeschnitten');
  const data = new Float32Array(len);
  for (let i = 0; i < len; i++) {
    const b = bytes[start + i];
    // min und max sind vorzeichenbehaftet (÷ 127), rms vorzeichenlos (÷ 255).
    data[i] = i % 3 === 2 ? b / 255 : (b > 127 ? b - 256 : b) / 127;
  }
  return { level: { samples_per_point: samplesPerPoint, data }, next: start + len };
}

/** DWWF-Bytes in typisierte Felder dekodieren. Wirft `WaveformFormatError` bei fremden oder abgeschnittenen Daten. */
export function decodeWaveform(input: ArrayBuffer | Uint8Array): Waveform {
  const bytes = input instanceof Uint8Array ? input : new Uint8Array(input);
  if (bytes.length < HEADER_BYTES) throw new WaveformFormatError('Wellenform: Kopf fehlt');
  const magic = String.fromCharCode(bytes[0], bytes[1], bytes[2], bytes[3]);
  if (magic !== DWWF_MAGIC) throw new WaveformFormatError('Wellenform: kein DWWF-Format');
  const view = new DataView(bytes.buffer, bytes.byteOffset, bytes.byteLength);
  const version = view.getUint32(4, true);
  if (version !== DWWF_VERSION) throw new WaveformFormatError(`Wellenform: unbekannte Format-Version ${version}`);
  const sampleRate = view.getUint32(8, true);
  // u64 als Zahl: Frames bleiben weit unter 2^53.
  const frames = view.getUint32(12, true) + view.getUint32(16, true) * 4294967296;
  const count = view.getUint32(20, true);
  if (count < 1) throw new WaveformFormatError('Wellenform: keine Stufen');
  let pos = HEADER_BYTES;
  const all: WaveformLevel[] = [];
  for (let i = 0; i < count; i++) {
    const r = decodeLevel(view, bytes, pos);
    all.push(r.level);
    pos = r.next;
  }
  return { sample_rate: sampleRate, frames, overview: all[0], levels: all.slice(1) };
}

/** Gegenstück für Tests und den simulierten Kern: Wellenform in DWWF-Bytes schreiben. */
export function encodeWaveform(wf: Waveform): Uint8Array {
  const all = [wf.overview, ...wf.levels];
  const size = HEADER_BYTES + all.reduce((n, l) => n + 8 + l.data.length, 0);
  const out = new Uint8Array(size);
  const view = new DataView(out.buffer);
  for (let i = 0; i < 4; i++) out[i] = DWWF_MAGIC.charCodeAt(i);
  view.setUint32(4, DWWF_VERSION, true);
  view.setUint32(8, wf.sample_rate, true);
  view.setUint32(12, wf.frames % 4294967296, true);
  view.setUint32(16, Math.floor(wf.frames / 4294967296), true);
  view.setUint32(20, all.length, true);
  let pos = HEADER_BYTES;
  for (const l of all) {
    view.setUint32(pos, l.samples_per_point, true);
    view.setUint32(pos + 4, Math.floor(l.data.length / VALUES_PER_POINT), true);
    pos += 8;
    for (let i = 0; i < l.data.length; i++) {
      const v = l.data[i];
      if (i % 3 === 2) out[pos + i] = Math.round(Math.min(1, Math.max(0, v)) * 255);
      else out[pos + i] = Math.round(Math.min(1, Math.max(-1, v)) * 127) & 0xff;
    }
    pos += l.data.length;
  }
  return out;
}
