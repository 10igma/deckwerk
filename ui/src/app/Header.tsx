// Kopfzeile (8.2): Ansichten · Master-Clock · Q/S · Aufnahme · Last · Uhr · Einstellungen.
import { useEffect, useRef, useState } from 'react';
import { listAudioDevices, selectAudioDevice, toggleSnap } from './actions';
import { getDeckAssets } from './assets';
import { nowDate } from './env';
import { useEngineStatus, useFrame, useSnap } from './frame';
import { bindingKeys, findBinding, shortcutKeys } from './keymap';
import { navigate, VIEWS, type RouteId, type ViewId } from './router';
import { setLang, setLiveGuard, setUi, useUi } from './store';
import type { AudioDevice } from '../core/types';
import { DeckPlate, Icon, KeyBtn, LiveText, useTip } from '../design/components';
import { useLang, useT, type MsgKey } from '../i18n';
import { formatBpm } from '../lib/format';
import { barBeat } from '../lib/grid';

const VIEW_LABEL: Record<ViewId, MsgKey> = {
  mixen: 'nav.mix',
  bibliothek: 'nav.library',
  vorbereiten: 'nav.prepare',
  automix: 'nav.automix',
};

function Brand() {
  // Zeichen: zwei Wellenhälften, die sich an der Schiene spiegeln.
  return (
    <div className="hdr__brand" aria-label="Deckwerk">
      <svg viewBox="0 0 28 24" width="1.6rem" height="1.4rem" aria-hidden="true">
        <path d="M2 11 V5 H6 V11 Z M9 11 V1 H13 V11 Z M16 11 V7 H20 V11 Z M23 11 V4 H26 V11 Z" fill="var(--text-1)" />
        <path d="M2 13 V19 H6 V13 Z M9 13 V23 H13 V13 Z M16 13 V17 H20 V13 Z M23 13 V20 H26 V13 Z" fill="var(--text-3)" />
      </svg>
      <span>Deckwerk</span>
    </div>
  );
}

function BeatLamp() {
  const el = useRef<HTMLDivElement | null>(null);
  const last = useRef('');
  useFrame((s) => {
    let v = '0';
    if (s.clock_bpm > 0) {
      const a = s.master ? getDeckAssets(s.master) : null;
      const md = s.master ? s.decks[['a', 'b', 'c', 'd'].indexOf(s.master)] : null;
      v = a && md ? String(barBeat(a.analysis.grid, md.beat).beat) : String((((Math.floor(s.clock_beats) % 4) + 4) % 4) + 1);
      if (md && !md.playing) v = '0';
    }
    if (v !== last.current && el.current) {
      last.current = v;
      el.current.dataset.beat = v;
    }
  });
  return (
    <div className="hdr__beats" ref={el} aria-hidden="true">
      <span />
      <span />
      <span />
      <span />
    </div>
  );
}

function ClockBlock() {
  const t = useT();
  const lang = useLang();
  const master = useSnap((s) => s.master);
  const tip = useTip({ name: t('hdr.clock'), value: master ? t('hdr.clockFrom', { deck: master.toUpperCase() }) : t('hdr.clockFree') });
  return (
    <div className="hdr__clock" {...tip}>
      <span className="dw-label">{t('hdr.clockShort')}</span>
      <LiveText className="hdr__bpm dw-num" read={(s) => formatBpm(s.clock_bpm, lang)} />
      <span className="dw-label">BPM</span>
      {master ? <DeckPlate deck={master} size="sm" /> : <span className="hdr__free dw-label">{t('hdr.free')}</span>}
      <BeatLamp />
    </div>
  );
}

function LoadMeter() {
  const t = useT();
  const st = useEngineStatus();
  const pct = Math.round(st.dsp_load * 100);
  const tip = useTip({
    name: t('hdr.load'),
    value: `${pct} % · ${t('hdr.xruns', { n: st.xruns })} · ${st.sample_rate / 1000} kHz / ${st.buffer_size > 0 ? st.buffer_size : t('hdr.bufferDefault')}`,
  });
  return (
    <div className="hdr__load" data-level={st.dsp_load > 0.7 ? 'high' : st.dsp_load > 0.4 ? 'mid' : 'low'} {...tip}>
      <span className="dw-label">{t('hdr.loadShort')}</span>
      <span className="hdr__loadbar" aria-hidden="true">
        <span style={{ width: `${Math.min(100, pct)}%` }} />
        <i />
      </span>
      <span className="dw-num hdr__loadval">{String(pct).padStart(2, ' ')} %</span>
    </div>
  );
}

/** Dauerhafter Warnhinweis, solange kein Audiogerät läuft; die Meldung des Systems steht in der Kurzhilfe. */
function DeviceWarning() {
  const t = useT();
  const st = useEngineStatus();
  const tip = useTip({ name: t('hdr.noDevice'), value: st.device_error ?? t('hdr.noDeviceHint') });
  if (st.has_device) return null;
  return (
    <span className="hdr__warn" role="status" tabIndex={0} {...tip}>
      <Icon name="warn" size="1em" />
      {t('hdr.noDeviceShort')}
    </span>
  );
}

function ViewTab({ view, route, hint }: { view: ViewId; route: RouteId; hint: string }) {
  const t = useT();
  const tip = useTip({ name: t('nav.open', { view: t(VIEW_LABEL[view]) }), keys: hint ? [hint] : [] });
  return (
    <a className="hdr__tab" href={`#/${view}`} aria-current={route === view ? 'page' : undefined} draggable={false} {...tip}>
      {t(VIEW_LABEL[view])}
      <kbd>{hint}</kbd>
    </a>
  );
}

function WallClock() {
  const t = useT();
  const tip = useTip({ name: t('hdr.time') });
  const [now, setNow] = useState(nowDate);
  useEffect(() => {
    const id = setInterval(() => setNow(nowDate()), 5000);
    return () => clearInterval(id);
  }, []);
  const text = `${String(now.getHours()).padStart(2, '0')}:${String(now.getMinutes()).padStart(2, '0')}`;
  return (
    <span className="hdr__time dw-num" {...tip}>
      {text}
    </span>
  );
}

/** Wahl des Ausgabegeräts (ENG-01). Der Wechsel wirkt sofort; geladene Decks bleiben erhalten. */
function AudioOutput() {
  const t = useT();
  const st = useEngineStatus();
  const [devices, setDevices] = useState<AudioDevice[]>([]);
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState<string | null>(null);
  useEffect(() => {
    let alive = true;
    void listAudioDevices().then((list) => {
      if (alive) setDevices(list);
    });
    return () => {
      alive = false;
    };
  }, []);
  const choose = (name: string | null) => {
    if (busy || name === st.device) return;
    setBusy(true);
    setError(null);
    void selectAudioDevice(name).then((failure) => {
      setError(failure);
      setBusy(false);
    });
  };
  const systemDefault = devices.find((d) => d.is_default);
  return (
    <div className="settings__block">
      <span className="dw-label">{t('settings.audio')}</span>
      <div className="settings__devices" role="group" aria-label={t('settings.audio')}>
        <KeyBtn
          mode="toggle"
          size="sm"
          block
          className="settings__device"
          active={st.device === null}
          disabled={busy}
          onPress={() => choose(null)}
          sub={systemDefault ? systemDefault.name : undefined}
          tip={{ name: t('settings.audioDefault'), value: systemDefault?.name }}
        >
          {t('settings.audioDefault')}
        </KeyBtn>
        {devices.map((d) => (
          <KeyBtn
            key={`${d.host}/${d.name}`}
            mode="toggle"
            size="sm"
            block
            className="settings__device"
            active={st.device === d.name}
            disabled={busy}
            onPress={() => choose(d.name)}
            sub={t('settings.audioChannels', { n: d.max_channels })}
            tip={{ name: d.name, value: `${d.host} · ${t('settings.audioChannels', { n: d.max_channels })}` }}
          >
            {d.name}
          </KeyBtn>
        ))}
      </div>
      {error !== null && (
        <p className="settings__note" data-tone="error" role="alert">
          {t('settings.audioFailed', { reason: error })}
        </p>
      )}
      <p className="settings__note">{t('settings.audioNote')}</p>
    </div>
  );
}

function SettingsPanel() {
  const t = useT();
  const lang = useLang();
  const theme = useUi((s) => s.theme);
  const guard = useUi((s) => s.liveGuard);
  const renderer = useUi((s) => s.rendererInfo);
  return (
    <div className="settings" role="dialog" aria-label={t('hdr.settings')}>
      <AudioOutput />
      <div className="settings__row">
        <span className="dw-label">{t('settings.guard')}</span>
        <div className="settings__opts">
          <KeyBtn mode="toggle" size="sm" active={guard} onPress={() => setLiveGuard(true)}>
            {t('state.on')}
          </KeyBtn>
          <KeyBtn mode="toggle" size="sm" active={!guard} onPress={() => setLiveGuard(false)}>
            {t('state.off')}
          </KeyBtn>
        </div>
      </div>
      <p className="settings__note">{t('settings.guardNote')}</p>
      <div className="settings__row">
        <span className="dw-label">{t('settings.language')}</span>
        <div className="settings__opts">
          <KeyBtn mode="toggle" size="sm" active={lang === 'de'} onPress={() => setLang('de')}>
            Deutsch
          </KeyBtn>
          <KeyBtn mode="toggle" size="sm" active={lang === 'en'} onPress={() => setLang('en')}>
            English
          </KeyBtn>
        </div>
      </div>
      <div className="settings__row">
        <span className="dw-label">{t('settings.theme')}</span>
        <div className="settings__opts">
          <KeyBtn mode="toggle" size="sm" active={theme === 'dark'} onPress={() => setUi({ theme: 'dark' })}>
            {t('settings.themeDark')}
          </KeyBtn>
          <KeyBtn mode="toggle" size="sm" active={theme === 'contrast'} onPress={() => setUi({ theme: 'contrast' })}>
            {t('settings.themeContrast')}
          </KeyBtn>
        </div>
      </div>
      <div className="settings__row">
        <span className="dw-label">{t('settings.design')}</span>
        <div className="settings__opts">
          <KeyBtn
            size="sm"
            onPress={() => {
              setUi({ settingsOpen: false });
              navigate('designsystem');
            }}
          >
            {t('settings.designOpen')}
          </KeyBtn>
        </div>
      </div>
      <div className="settings__row">
        <span className="dw-label">{t('settings.renderer')}</span>
        <span className="settings__value">{renderer || '–'}</span>
      </div>
      <p className="settings__note">{t('settings.note')}</p>
    </div>
  );
}

export function Header({ route }: { route: RouteId }) {
  const t = useT();
  const lang = useLang();
  const snap = useUi((s) => s.snap);
  const open = useUi((s) => s.settingsOpen);
  const fkey = (v: ViewId) => {
    const b = findBinding(`view.${v}`);
    return b ? bindingKeys(b, lang).join('+') : '';
  };

  return (
    <header className="hdr">
      <div className="hdr__left">
        <Brand />
        <nav className="hdr__nav" aria-label={t('nav.aria')}>
          {VIEWS.map((v) => (
            <ViewTab key={v} view={v} route={route} hint={fkey(v)} />
          ))}
        </nav>
      </div>

      <ClockBlock />

      <div className="hdr__right">
        <DeviceWarning />
        <KeyBtn mode="toggle" size="sm" disabled tip={{ name: t('hdr.quantize'), value: t('hdr.later', { stage: 2 }), keys: ['F9'] }} ariaLabel={t('hdr.quantize')}>
          <span className="dw-num">Q</span>
        </KeyBtn>
        <KeyBtn mode="toggle" size="sm" active={snap} onPress={toggleSnap} tip={{ name: t('hdr.snap'), value: snap ? t('state.on') : t('state.off'), keys: shortcutKeys('global.snap', undefined, lang) }} ariaLabel={t('hdr.snap')}>
          <span className="dw-num">S</span>
        </KeyBtn>
        <KeyBtn mode="toggle" size="sm" disabled tip={{ name: t('hdr.record'), value: t('hdr.later', { stage: 2 }), keys: [lang === 'de' ? 'Strg' : 'Ctrl', 'R'] }}>
          <Icon name="rec" size="0.85em" />
          {t('hdr.recordShort')}
        </KeyBtn>
        <LoadMeter />
        <WallClock />
        <div className="hdr__gear">
          <KeyBtn mode="toggle" size="sm" active={open} onPress={() => setUi({ settingsOpen: !open })} ariaLabel={t('hdr.settings')} tip={{ name: t('hdr.settings') }}>
            <Icon name="gear" size="1.05em" />
          </KeyBtn>
          {open && <SettingsPanel />}
        </div>
      </div>
    </header>
  );
}
