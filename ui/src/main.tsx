// Reihenfolge der Stile: erst das Designsystem, dann die Ansichten.
import './design/fonts.css';
import './design/tokens.css';
import './design/base.css';
import './design/components/components.css';
import { StrictMode } from 'react';
import { createRoot } from 'react-dom/client';
import { App } from './app/App';
import { bindCore, hydrate, importPaths } from './app/actions';
import { CoreContext } from './app/context';
import { freezeClock } from './app/env';
import { attachCore } from './app/frame';
import { installKeyboard } from './app/keyboard';
import { installScale } from './app/scale';
import { initUi, showNotice } from './app/store';
import { createCore } from './core';

// Einstieg: Kern wählen (heute der simulierte), Oberfläche anschließen, rendern.
const boot = createCore(window.location.search);
if (boot.frozen) freezeClock(new Date(2026, 10, 18, 23, 41, 0));
initUi(window.location.search);
installScale();
bindCore(boot.core);
attachCore(boot.core);
installKeyboard();
void hydrate();

// Im Browser (simulierter Kern): Dateien, die auf das Fenster gezogen werden, importieren.
// In der Desktop-Hülle nimmt Tauri das Ablegen selbst entgegen und importiert (docs/kern-schnittstelle.md).
if (boot.kind === 'mock') {
  window.addEventListener('dragover', (e) => {
    if (e.dataTransfer?.types.includes('Files')) e.preventDefault();
  });
  window.addEventListener('drop', (e) => {
    const files = e.dataTransfer?.files;
    if (!files || files.length === 0) return;
    e.preventDefault();
    void importPaths(Array.from(files, (f) => f.name));
  });
}
document.documentElement.dataset.core = boot.kind;

// Unerwartete Fehler der Oberfläche sichtbar machen (in der Hülle gibt es keine Konsole).
window.addEventListener('error', (e) => showNotice(`${e.message} (${(e.filename || '').split('/').pop()}:${e.lineno})`, 'error'));
window.addEventListener('unhandledrejection', (e) => showNotice(String(e.reason instanceof Error ? e.reason.message : e.reason), 'error'));

createRoot(document.getElementById('root')!).render(
  <StrictMode>
    <CoreContext.Provider value={boot.core}>
      <App />
    </CoreContext.Provider>
  </StrictMode>,
);
