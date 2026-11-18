import { useEffect } from 'react';
import { Header } from './Header';
import { useRoute } from './router';
import { setUi, useUi } from './store';
import { TipLayer } from '../design/components';
import { Icon } from '../design/components';
import { DesignSystemView } from '../views/DesignSystemView';
import { MixView } from '../views/mix/MixView';
import { Placeholder } from '../views/Placeholder';
import './app.css';

function NoticeLayer() {
  const notice = useUi((s) => s.notice);
  if (!notice) return null;
  return (
    <div className="dw-notice" role="status" key={notice.id}>
      <Icon name="warn" size="1.2em" />
      {notice.text}
    </div>
  );
}

export function App() {
  const route = useRoute();
  const lang = useUi((s) => s.lang);
  const theme = useUi((s) => s.theme);
  const settingsOpen = useUi((s) => s.settingsOpen);

  useEffect(() => {
    document.documentElement.lang = lang;
  }, [lang]);
  useEffect(() => {
    if (theme === 'dark') delete document.documentElement.dataset.theme;
    else document.documentElement.dataset.theme = theme;
  }, [theme]);

  // Die Designsystem-Seite rollt als ganze Seite, alle anderen Ansichten füllen genau das Fenster.
  useEffect(() => {
    document.body.dataset.scroll = route === 'designsystem' ? 'true' : 'false';
  }, [route]);

  // Einstellungen schließen: Klick daneben oder Esc.
  useEffect(() => {
    if (!settingsOpen) return;
    const close = (e: Event) => {
      if (e instanceof KeyboardEvent && e.key !== 'Escape') return;
      if (e instanceof MouseEvent && e.target instanceof Element && e.target.closest('.hdr__gear')) return;
      setUi({ settingsOpen: false });
    };
    window.addEventListener('pointerdown', close);
    window.addEventListener('keydown', close);
    return () => {
      window.removeEventListener('pointerdown', close);
      window.removeEventListener('keydown', close);
    };
  }, [settingsOpen]);

  return (
    <div className="app" data-page={route === 'designsystem' ? 'true' : undefined}>
      <Header route={route} />
      <main className="app__main">
        {route === 'mixen' && <MixView />}
        {route === 'designsystem' && <DesignSystemView />}
        {route !== 'mixen' && route !== 'designsystem' && <Placeholder view={route} />}
      </main>
      <NoticeLayer />
      <TipLayer />
    </div>
  );
}
