import { describe, expect, it } from 'vitest';
import de from '../i18n/de.json';
import en from '../i18n/en.json';
import { translate } from '../i18n';
import { DEFAULT_KEYMAP, bindingKeys, findBinding, findConflicts, resolveKey, shortcutKeys, type KeyLike } from './keymap';
import { parseRoute } from './router';
import { webglPresent } from './env';
import { remForViewport } from './scale';

const key = (code: string, mods: Partial<KeyLike> = {}): KeyLike => ({ code, shiftKey: false, ctrlKey: false, metaKey: false, altKey: false, ...mods });
const hit = (code: string, mods: Partial<KeyLike> = {}) => {
  const r = resolveKey(key(code, mods));
  return r ? { action: r.binding.action, deck: r.binding.deck, fine: r.fine } : null;
};

describe('Tastenbelegung (8.7)', () => {
  it('Deck A: Q W E · A S · D F · T · G', () => {
    expect(hit('KeyQ')).toEqual({ action: 'deck.play', deck: 'a', fine: false });
    expect(hit('KeyW')).toEqual({ action: 'deck.cue', deck: 'a', fine: false });
    expect(hit('KeyE')).toEqual({ action: 'deck.sync', deck: 'a', fine: false });
    expect(hit('KeyA')?.action).toBe('deck.bend_down');
    expect(hit('KeyS')?.action).toBe('deck.bend_up');
    expect(hit('KeyD')?.action).toBe('deck.tempo_down');
    expect(hit('KeyF')?.action).toBe('deck.tempo_up');
    expect(hit('KeyT')).toEqual({ action: 'deck.headphone', deck: 'a', fine: false });
    expect(hit('KeyG')).toEqual({ action: 'deck.kill_low', deck: 'a', fine: false });
  });
  it('Deck B: U I O · J K · L ; · Y · H', () => {
    expect(hit('KeyU')).toEqual({ action: 'deck.play', deck: 'b', fine: false });
    expect(hit('KeyI')).toEqual({ action: 'deck.cue', deck: 'b', fine: false });
    expect(hit('KeyO')).toEqual({ action: 'deck.sync', deck: 'b', fine: false });
    expect(hit('KeyJ')?.action).toBe('deck.bend_down');
    expect(hit('KeyK')?.action).toBe('deck.bend_up');
    expect(hit('KeyL')?.action).toBe('deck.tempo_down');
    expect(hit('Semicolon')).toEqual({ action: 'deck.tempo_up', deck: 'b', fine: false });
    expect(hit('KeyY')).toEqual({ action: 'deck.headphone', deck: 'b', fine: false });
    expect(hit('KeyH')).toEqual({ action: 'deck.kill_low', deck: 'b', fine: false });
  });
  it('Shift macht Tempo-Schritte fein', () => {
    expect(hit('KeyD', { shiftKey: true })).toEqual({ action: 'deck.tempo_down', deck: 'a', fine: true });
    expect(hit('Semicolon', { shiftKey: true })).toEqual({ action: 'deck.tempo_up', deck: 'b', fine: true });
  });
  it('Strg/Cmd + A/S und J/K steuern die Filter', () => {
    expect(hit('KeyA', { ctrlKey: true })).toEqual({ action: 'deck.filter_close', deck: 'a', fine: false });
    expect(hit('KeyS', { metaKey: true })).toEqual({ action: 'deck.filter_open', deck: 'a', fine: false });
    expect(hit('KeyJ', { ctrlKey: true })).toEqual({ action: 'deck.filter_close', deck: 'b', fine: false });
    expect(hit('KeyK', { ctrlKey: true })).toEqual({ action: 'deck.filter_open', deck: 'b', fine: false });
  });
  it('Kanalfader auf Alt + Tempo-Tasten (Strg + F bleibt die Suche), fein mit Shift', () => {
    expect(hit('KeyD', { altKey: true })).toEqual({ action: 'deck.fader_up', deck: 'a', fine: false });
    expect(hit('KeyF', { altKey: true })).toEqual({ action: 'deck.fader_down', deck: 'a', fine: false });
    expect(hit('KeyL', { altKey: true })).toEqual({ action: 'deck.fader_up', deck: 'b', fine: false });
    expect(hit('Semicolon', { altKey: true, shiftKey: true })).toEqual({ action: 'deck.fader_down', deck: 'b', fine: true });
    // Die in 8.7 genannte Kombination Strg + F ist dort doppelt belegt; sie bleibt die Suche.
    expect(hit('KeyF', { ctrlKey: true })?.action).toBe('search.focus');
    expect(hit('KeyD', { ctrlKey: true })).toBeNull();
  });
  it('Leertaste spielt das Deck mit Fokus, Esc löst den Regler-Fokus', () => {
    expect(hit('Space')).toEqual({ action: 'deck.play_focus', deck: undefined, fine: false });
    expect(hit('Escape')?.action).toBe('focus.release');
    expect(shortcutKeys('deck.play_focus', undefined, 'de')).toEqual(['␣']);
  });
  it('Pfeile: Crossfader, Liste, Laden mit Shift, ganz mit Alt', () => {
    expect(hit('ArrowLeft')?.action).toBe('xf.left');
    expect(hit('ArrowRight')?.action).toBe('xf.right');
    expect(hit('ArrowLeft', { shiftKey: true })).toEqual({ action: 'deck.load', deck: 'a', fine: false });
    expect(hit('ArrowRight', { shiftKey: true })).toEqual({ action: 'deck.load', deck: 'b', fine: false });
    expect(hit('ArrowLeft', { altKey: true })?.action).toBe('xf.full_left');
    expect(hit('ArrowRight', { altKey: true })?.action).toBe('xf.full_right');
    expect(hit('ArrowUp')?.action).toBe('list.up');
    expect(hit('ArrowDown')?.action).toBe('list.down');
  });
  it('Ansichten auf F1–F4, Suche auf Strg/Cmd + F, Snap auf F10', () => {
    expect(hit('F1')?.action).toBe('view.mixen');
    expect(hit('F4')?.action).toBe('view.automix');
    expect(hit('KeyF', { ctrlKey: true })?.action).toBe('search.focus');
    expect(hit('KeyF', { metaKey: true })?.action).toBe('search.focus');
    expect(hit('F10')?.action).toBe('global.snap');
  });
  it('Funktionen späterer Stufen sind belegt, lösen aber noch nichts aus', () => {
    for (const code of ['Digit1', 'KeyC', 'KeyZ', 'KeyV', 'KeyR', 'Comma', 'Slash', 'KeyP', 'F9']) expect(hit(code)).toBeNull();
    expect(resolveKey(key('Digit1'), DEFAULT_KEYMAP, 2)?.binding).toMatchObject({ action: 'deck.hotcue', deck: 'a', arg: 1 });
    expect(resolveKey(key('Digit0'), DEFAULT_KEYMAP, 2)?.binding).toMatchObject({ action: 'deck.hotcue', deck: 'b', arg: 4 });
    expect(resolveKey(key('KeyP'), DEFAULT_KEYMAP, 2)?.binding).toMatchObject({ action: 'deck.flux', deck: 'b' });
  });
  it('unbelegte Tasten und falsche Umschalter ergeben nichts', () => {
    expect(hit('KeyQ', { shiftKey: true })).toBeNull();
    expect(hit('KeyQ', { ctrlKey: true })).toBeNull();
    expect(hit('Backquote')).toBeNull();
  });
  it('keine Kombination ist doppelt belegt', () => {
    expect(findConflicts()).toEqual([]);
  });
  it('Halten-Tasten: Cue und Anschieben/Abbremsen', () => {
    const holds = DEFAULT_KEYMAP.filter((b) => b.mode === 'hold').map((b) => b.action);
    expect(new Set(holds)).toEqual(new Set(['deck.cue', 'deck.bend_down', 'deck.bend_up']));
  });
  it('Kürzel-Beschriftung für die Kurzhilfe', () => {
    expect(shortcutKeys('deck.play', 'a', 'de')).toEqual(['Q']);
    expect(shortcutKeys('deck.load', 'b', 'de')).toEqual(['Shift', '→']);
    expect(shortcutKeys('deck.filter_close', 'a', 'de')).toEqual(['Strg', 'A']);
    expect(shortcutKeys('deck.filter_close', 'a', 'en')).toEqual(['Ctrl', 'A']);
    expect(bindingKeys(findBinding('deck.tempo_up', 'b')!, 'de')).toEqual([';']);
  });
});

describe('Sprachdateien (BED-07)', () => {
  it('Deutsch und Englisch haben dieselben Schlüssel', () => {
    expect(Object.keys(en).sort()).toEqual(Object.keys(de).sort());
  });
  it('kein Text ist leer, Platzhalter stimmen überein', () => {
    const slots = (s: string) => (s.match(/\{\w+\}/g) ?? []).sort().join();
    for (const k of Object.keys(de) as (keyof typeof de)[]) {
      expect(de[k].length, k).toBeGreaterThan(0);
      expect((en as Record<string, string>)[k].length, k).toBeGreaterThan(0);
      expect(slots((en as Record<string, string>)[k]), k).toBe(slots(de[k]));
    }
  });
  it('Platzhalter werden ersetzt', () => {
    expect(translate('de', 'guard.seekShift', { deck: 'A' })).toBe('Deck A ist hörbar – Sprung nur mit Shift + Klick.');
    expect(translate('en', 'stage.beats', { n: 8 })).toBe('8 beats');
  });
});

describe('Hülle', () => {
  it('WebGL wird unter WebKitGTK über eine 2D-Fläche gezeigt, sonst direkt', () => {
    const chrome = 'Mozilla/5.0 (X11; Linux x86_64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/140.0.0.0 Safari/537.36';
    const webkitGtk = 'Mozilla/5.0 (X11; Linux x86_64) AppleWebKit/605.1.15 (KHTML, like Gecko) Version/17.4 Safari/605.1.15';
    const mac = 'Mozilla/5.0 (Macintosh; Intel Mac OS X 10_15_7) AppleWebKit/605.1.15 (KHTML, like Gecko)';
    const edge = 'Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/140.0.0.0 Safari/537.36 Edg/140.0.0.0';
    expect(webglPresent('', chrome)).toBe('direct');
    expect(webglPresent('', webkitGtk)).toBe('blit');
    expect(webglPresent('', mac)).toBe('direct');
    expect(webglPresent('', edge)).toBe('direct');
    expect(webglPresent('?renderer=blit', chrome)).toBe('blit');
  });
  it('Routen aus dem Hash', () => {
    expect(parseRoute('')).toBe('mixen');
    expect(parseRoute('#/designsystem')).toBe('designsystem');
    expect(parseRoute('#/automix')).toBe('automix');
    expect(parseRoute('#/gibtsnicht')).toBe('mixen');
  });
  it('Skalierung wächst mit dem Fenster, aber langsamer (VIS-10)', () => {
    expect(remForViewport(1280, 720)).toBe(14);
    const sizes = [remForViewport(1920, 1080), remForViewport(2560, 1440), remForViewport(3840, 2160)];
    expect(sizes[0]).toBeGreaterThan(14);
    expect(sizes[1]).toBeGreaterThan(sizes[0]);
    expect(sizes[2]).toBeGreaterThan(sizes[1]);
    expect(sizes[2]).toBeLessThan(14 * 3);
    // Das schmalere Maß bestimmt: ein sehr breites Fenster macht nichts größer.
    expect(remForViewport(3840, 720)).toBe(14);
  });
});
