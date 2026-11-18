// Reihenfolge der Stile: erst das Designsystem, dann die Ansichten.
import './design/fonts.css';
import './design/tokens.css';
import './design/base.css';
import './design/components/components.css';
import { StrictMode } from 'react';
import { createRoot } from 'react-dom/client';
import { App } from './app/App';
import { bindCore, hydrate } from './app/actions';
import { CoreContext } from './app/context';
import { freezeClock } from './app/env';
import { attachCore } from './app/frame';
import { installKeyboard } from './app/keyboard';
import { installScale } from './app/scale';
import { initUi } from './app/store';
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

createRoot(document.getElementById('root')!).render(
  <StrictMode>
    <CoreContext.Provider value={boot.core}>
      <App />
    </CoreContext.Provider>
  </StrictMode>,
);
