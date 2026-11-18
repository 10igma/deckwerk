import { describe, expect, it } from 'vitest';
import { contrast, oklchToHex } from './color';
import {
  EQ_CURVE,
  GAIN_CURVE,
  TEMPO_RANGES,
  VOLUME_CURVE,
  bpmWithOffset,
  crossfaderGains,
  faderFromDb,
  faderGain,
  faderToDb,
  filterSetting,
  knobAngle,
  nextTempoRange,
  tempoOffsetToPos,
  tempoPosToOffset,
  tempoRangeFor,
} from './curves';
import { dbToLin, fromWire, linToDb, toWire } from './db';
import { formatBpm, formatDb, formatHz, formatPercent, formatPhase } from './format';
import { barBeat, beatAt, constantGrid, isDownbeat, secAt, snapSeek } from './grid';
import { camelot, camelotString, compatClass, fromCamelot, harmonicCompat, keyName, makeKey, openKeyString } from './key';
import { channelOpen } from './audible';
import { parseInRange, parseNumber } from './parse';
import { formatDuration, formatRemaining, formatTime } from './time';

describe('Zeitformat', () => {
  it('zeigt Minuten, Sekunden und Zehntel', () => {
    expect(formatTime(0)).toBe('0:00.0');
    expect(formatTime(7.26)).toBe('0:07.2');
    expect(formatTime(187.44)).toBe('3:07.4');
    expect(formatTime(59.99)).toBe('0:59.9');
    expect(formatTime(60)).toBe('1:00.0');
    expect(formatTime(3723.5)).toBe('62:03.5');
  });
  it('nutzt das Dezimalkomma im Deutschen', () => {
    expect(formatTime(187.44, ',')).toBe('3:07,4');
  });
  it('schreibt negative Zeiten mit echtem Minuszeichen', () => {
    expect(formatTime(-12.3)).toBe('−0:12.3');
  });
  it('Restzeit ist nie positiv und endet bei null', () => {
    expect(formatRemaining(100, 402.6)).toBe('−5:02.6');
    expect(formatRemaining(500, 402.6)).toBe('−0:00.0');
  });
  it('Länge für Listen wird auf Sekunden gerundet', () => {
    expect(formatDuration(402.6)).toBe('6:43');
    expect(formatDuration(59.4)).toBe('0:59');
    expect(formatDuration(NaN)).toBe('-:--');
  });
  it('behält die Breite bei laufender Zeit (keine springenden Ziffern)', () => {
    const lens = new Set([61.0, 61.1, 65.55, 119.99].map((s) => formatTime(s).length));
    expect(lens.size).toBe(1);
  });
});

describe('Zahlenformat', () => {
  it('BPM mit zwei Nachkommastellen, Komma oder Punkt', () => {
    expect(formatBpm(134, 'de')).toBe('134,00');
    expect(formatBpm(137.5, 'en')).toBe('137.50');
    expect(formatBpm(0, 'de')).toBe('---,--');
  });
  it('Tempo-Abweichung mit Vorzeichen', () => {
    expect(formatPercent(0.0075, 'de')).toBe('+0,75 %');
    expect(formatPercent(-0.0147, 'en')).toBe('−1.47 %');
    expect(formatPercent(0, 'de')).toBe('±0,00 %');
    expect(formatPercent(-0.000001, 'de')).toBe('±0,00 %');
  });
  it('Pegel und Frequenzen', () => {
    expect(formatDb(-Infinity, 'de')).toBe('−∞ dB');
    expect(formatDb(-3.24, 'de')).toBe('−3,2 dB');
    expect(formatDb(6, 'en')).toBe('+6.0 dB');
    expect(formatHz(240.4, 'de')).toBe('240 Hz');
    expect(formatHz(1234, 'de')).toBe('1,2 kHz');
    expect(formatHz(20000, 'en')).toBe('20 kHz');
    expect(formatPhase(-0.031, 'de')).toBe('−0,03');
  });
});

describe('Regler-Kennlinien', () => {
  it('EQ: Mitte 0 dB, rechts +6 dB, untere 5 % sind Kill (5.14)', () => {
    expect(EQ_CURVE.toDb(0.5)).toBe(0);
    expect(EQ_CURVE.toDb(1)).toBeCloseTo(6);
    expect(EQ_CURVE.toDb(0.75)).toBeCloseTo(3);
    expect(EQ_CURVE.toDb(0.049)).toBe(-Infinity);
    expect(EQ_CURVE.toDb(0)).toBe(-Infinity);
    expect(EQ_CURVE.toDb(0.05)).toBeCloseTo(-36);
  });
  it('EQ fällt nach links stetig und monoton', () => {
    let prev = EQ_CURVE.toDb(0.05);
    for (let p = 0.06; p <= 1.0001; p += 0.01) {
      const db = EQ_CURVE.toDb(p);
      expect(db).toBeGreaterThan(prev);
      expect(db - prev).toBeLessThan(2.2);
      prev = db;
    }
  });
  it('Hin- und Rückrechnung stimmen überein', () => {
    for (const curve of [EQ_CURVE, GAIN_CURVE, VOLUME_CURVE]) {
      for (let p = curve.killZone; p <= 1; p += 0.05) {
        expect(curve.toPos(curve.toDb(p))).toBeCloseTo(p, 6);
      }
      expect(curve.toPos(-Infinity)).toBe(0);
      expect(curve.toDb(curve.zeroPos)).toBe(0);
    }
  });
  it('Gain reicht bis +12 dB, Lautstärke bis +6 dB', () => {
    expect(GAIN_CURVE.toDb(1)).toBeCloseTo(12);
    expect(VOLUME_CURVE.toDb(1)).toBeCloseTo(6);
    expect(VOLUME_CURVE.toDb(0.75)).toBe(0);
  });
  it('Kanalfader: quadratisch wie im Kern – Anzeige 40·log10(Stellung), halber Weg = −12 dB (MIX-05)', () => {
    expect(faderToDb(1)).toBe(0);
    expect(faderToDb(0.5)).toBeCloseTo(-12.04, 2);
    expect(faderToDb(0.25)).toBeCloseTo(-24.08, 2);
    expect(faderToDb(0.1)).toBeCloseTo(-40, 6);
    expect(faderToDb(0)).toBe(-Infinity);
    // Anzeige und Verstärkung des Kerns (Stellung²) stimmen überein.
    for (const v of [0.05, 0.3, 0.5, 0.8, 1]) expect(faderToDb(v)).toBeCloseTo(linToDb(faderGain(v)), 9);
    expect(faderGain(0.5)).toBe(0.25);
  });
  it('Skalenstriche des Kanalfaders: dB → Stellung und zurück', () => {
    expect(faderFromDb(0)).toBe(1);
    expect(faderFromDb(-12)).toBeCloseTo(0.501, 3);
    expect(faderFromDb(-Infinity)).toBe(0);
    for (const db of [-48, -24, -6, 0]) expect(faderToDb(faderFromDb(db))).toBeCloseTo(db, 9);
  });
  it('Filter: Ruhezone, Tiefpass links, Hochpass rechts (5.14)', () => {
    expect(filterSetting(0).type).toBe('off');
    expect(filterSetting(0.019).type).toBe('off');
    expect(filterSetting(-1)).toEqual({ type: 'lowpass', hz: 80 });
    expect(filterSetting(-0.5).hz).toBeCloseTo(Math.sqrt(20000 * 80), 3);
    expect(filterSetting(1).type).toBe('highpass');
    expect(filterSetting(1).hz).toBeCloseTo(8000);
    expect(filterSetting(0.5).hz).toBeCloseTo(400);
  });
  it('Tempo: Bereich, Auflösung 0,01 % und angezeigtes BPM (DECK-06)', () => {
    expect(tempoPosToOffset(0.5, 0.08)).toBe(0);
    expect(tempoPosToOffset(1, 0.08)).toBeCloseTo(0.08);
    expect(tempoPosToOffset(0, 0.08)).toBeCloseTo(-0.08);
    const o = tempoPosToOffset(0.5123, 0.08);
    expect(Math.abs(o / 0.0001 - Math.round(o / 0.0001))).toBeLessThan(1e-6);
    expect(tempoOffsetToPos(0.04, 0.08)).toBeCloseTo(0.75);
    expect(bpmWithOffset(136, -0.0147)).toBeCloseTo(134.0, 1);
    expect(TEMPO_RANGES).toEqual([0.02, 0.04, 0.06, 0.08, 0.1, 0.16, 0.25, 0.5, 1]);
    expect(nextTempoRange(0.08)).toBe(0.1);
    expect(nextTempoRange(1)).toBe(0.02);
    expect(tempoRangeFor(0.09, 0.02)).toBe(0.1);
    expect(tempoRangeFor(0.01, 0.08)).toBe(0.08);
  });
  it('Drehregler überstreicht 270°', () => {
    expect(knobAngle(0)).toBe(-135);
    expect(knobAngle(0.5)).toBe(0);
    expect(knobAngle(1)).toBe(135);
    expect(knobAngle(2)).toBe(135);
  });
  it('Crossfader „weich“ hält die Summenleistung konstant (MIX-06)', () => {
    for (let x = -1; x <= 1; x += 0.1) {
      const [l, r] = crossfaderGains(x);
      expect(l * l + r * r).toBeCloseTo(1, 9);
    }
    expect(crossfaderGains(-1)[0]).toBeCloseTo(1);
    expect(crossfaderGains(-1)[1]).toBeCloseTo(0);
  });
  it('dB-Umrechnung und Übertragungsformat (−∞ = null)', () => {
    expect(dbToLin(-Infinity)).toBe(0);
    expect(dbToLin(6)).toBeCloseTo(1.9953, 3);
    expect(linToDb(0)).toBe(-Infinity);
    expect(toWire(-Infinity)).toBeNull();
    expect(toWire(-3)).toBe(-3);
    expect(fromWire(null)).toBe(-Infinity);
  });
});

describe('Tonarten', () => {
  it('Camelot- und Open-Key-Tabelle wie in dw-core', () => {
    expect(camelot(makeKey(0, false))).toEqual({ number: 8, letter: 'B' });
    expect(camelot(makeKey(9, true))).toEqual({ number: 8, letter: 'A' });
    expect(camelotString(makeKey(8, true))).toBe('1A');
    expect(camelotString(makeKey(11, false))).toBe('1B');
    expect(camelotString(makeKey(1, true))).toBe('12A');
    expect(camelotString(makeKey(5, true))).toBe('4A');
    expect(openKeyString(makeKey(0, false))).toBe('1d');
    expect(openKeyString(makeKey(2, true))).toBe('12m');
  });
  it('Camelot hin und zurück für alle 24 Tonarten', () => {
    for (let k = 0; k < 24; k++) {
      const c = camelot(k);
      expect(fromCamelot(c.number, c.letter)).toBe(k);
    }
  });
  it('Verträglichkeit nach 5.11', () => {
    const k = (n: number, l: 'A' | 'B') => fromCamelot(n, l)!;
    expect(harmonicCompat(k(8, 'A'), k(8, 'A'))).toBe(1.0);
    expect(harmonicCompat(k(8, 'A'), k(7, 'A'))).toBe(0.9);
    expect(harmonicCompat(k(8, 'A'), k(9, 'A'))).toBe(0.9);
    expect(harmonicCompat(k(8, 'A'), k(8, 'B'))).toBe(0.9);
    expect(harmonicCompat(k(8, 'A'), k(9, 'B'))).toBe(0.6);
    expect(harmonicCompat(k(8, 'B'), k(7, 'A'))).toBe(0.6);
    expect(harmonicCompat(k(8, 'A'), k(10, 'A'))).toBe(0.6);
    expect(harmonicCompat(k(8, 'A'), k(3, 'A'))).toBe(0.5);
    expect(harmonicCompat(k(8, 'A'), k(2, 'A'))).toBe(0.1);
    expect(harmonicCompat(k(12, 'A'), k(1, 'A'))).toBe(0.9);
    expect(compatClass(0.9)).toBe('match');
    expect(compatClass(0.6)).toBe('ok');
    expect(compatClass(0.1)).toBe('clash');
  });
  it('Namen deutsch und englisch', () => {
    expect(keyName(makeKey(9, true), 'de')).toBe('a-Moll');
    expect(keyName(makeKey(11, false), 'de')).toBe('H-Dur');
    expect(keyName(makeKey(11, false), 'en')).toBe('B major');
  });
});

describe('Beatgrid', () => {
  const g = constantGrid(120, 0.25, 1);
  it('rechnet wie dw-core', () => {
    expect(beatAt(g, 0.25)).toBe(0);
    expect(beatAt(g, 1.25)).toBeCloseTo(2);
    expect(secAt(g, 4)).toBeCloseTo(2.25);
    expect(isDownbeat(g, 1)).toBe(true);
    expect(isDownbeat(g, 0)).toBe(false);
    const two = { anchors: [{ pos: 0, bpm: 120 }, { pos: 8, bpm: 150 }], anchor_beats: [0, 16], bar_offset: 0 };
    expect(beatAt(two, 8.4)).toBeCloseTo(17);
    expect(secAt(two, 17)).toBeCloseTo(8.4);
  });
  it('Takt-/Beat-Zähler', () => {
    const g0 = constantGrid(120, 0, 0);
    expect(barBeat(g0, 0)).toEqual({ bar: 1, beat: 1 });
    expect(barBeat(g0, 3.9)).toEqual({ bar: 1, beat: 4 });
    expect(barBeat(g0, 4)).toEqual({ bar: 2, beat: 1 });
    expect(barBeat(g0, 706.62)).toEqual({ bar: 177, beat: 3 });
    expect(barBeat(g, 1)).toEqual({ bar: 1, beat: 1 });
  });
  it('Snap-Sprung erhält die Phase (DECK-20)', () => {
    const g0 = constantGrid(120, 0, 0);
    const target = snapSeek(g0, 10.13, 61.4);
    const delta = beatAt(g0, target) - beatAt(g0, 10.13);
    expect(delta).toBeCloseTo(Math.round(delta), 9);
    expect(Math.abs(target - 61.4)).toBeLessThanOrEqual(0.25);
  });
});

describe('Farbrechnung', () => {
  it('Kontrast nach WCAG', () => {
    expect(contrast('#000000', '#FFFFFF')).toBeCloseTo(21, 5);
    expect(contrast('#777777', '#777777')).toBe(1);
  });
  it('OKLCH bleibt im sRGB-Raum', () => {
    expect(oklchToHex(1, 0, 0)).toBe('#FFFFFF');
    expect(oklchToHex(0, 0, 0)).toBe('#000000');
    expect(oklchToHex(0.7, 0.4, 260)).toMatch(/^#[0-9A-F]{6}$/);
  });
});

describe('Zahleneingabe (BED-05)', () => {
  it('liest Komma und Punkt, Vorzeichen und Einheit', () => {
    expect(parseNumber('3,5')).toBe(3.5);
    expect(parseNumber('3.5')).toBe(3.5);
    expect(parseNumber('\u22123,2 dB')).toBe(-3.2);
    expect(parseNumber('-3,2dB')).toBe(-3.2);
    expect(parseNumber('+0,75 %')).toBe(0.75);
    expect(parseNumber('±0,0')).toBe(0);
    expect(parseNumber(' 12 ')).toBe(12);
    expect(parseNumber(',5')).toBe(0.5);
  });
  it('erkennt −∞ und weist Unlesbares ab', () => {
    expect(parseNumber('\u2212∞')).toBe(-Infinity);
    expect(parseNumber('-inf')).toBe(-Infinity);
    expect(parseNumber('')).toBeNull();
    expect(parseNumber('laut')).toBeNull();
  });
  it('begrenzt auf den Bereich des Reglers', () => {
    expect(parseInRange('20', -36, 6)).toBe(6);
    expect(parseInRange('-99', -36, 6)).toBe(-36);
    expect(parseInRange('-inf', -36, 6, true)).toBe(-Infinity);
    expect(parseInRange('-inf', -36, 6)).toBe(-36);
    expect(parseInRange('x', -36, 6)).toBeNull();
  });
});

describe('Hörbar (9.3)', () => {
  it('Kanalfader offen und Crossfader nicht ganz abgewandt', () => {
    expect(channelOpen({ fader: 1, xf_assign: 'left' }, 0)).toBe(true);
    expect(channelOpen({ fader: 0, xf_assign: 'left' }, 0)).toBe(false);
    expect(channelOpen({ fader: 1, xf_assign: 'left' }, 1)).toBe(false);
    expect(channelOpen({ fader: 1, xf_assign: 'left' }, 0.99)).toBe(true);
    expect(channelOpen({ fader: 1, xf_assign: 'right' }, -1)).toBe(false);
    expect(channelOpen({ fader: 0.01, xf_assign: 'right' }, 1)).toBe(true);
    expect(channelOpen({ fader: 1, xf_assign: 'thru' }, -1)).toBe(true);
  });
});
