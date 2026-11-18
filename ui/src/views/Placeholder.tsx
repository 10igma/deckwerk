// Platzhalter für Ansichten späterer Stufen: ein Satz, was dort kommt.
import { useT, type MsgKey } from '../i18n';
import type { ViewId } from '../app/router';

const TEXT: Record<Exclude<ViewId, 'mixen'>, { title: MsgKey; body: MsgKey; stage: number }> = {
  bibliothek: { title: 'nav.library', body: 'placeholder.library', stage: 3 },
  vorbereiten: { title: 'nav.prepare', body: 'placeholder.prepare', stage: 3 },
  automix: { title: 'nav.automix', body: 'placeholder.automix', stage: 5 },
};

export function Placeholder({ view }: { view: Exclude<ViewId, 'mixen'> }) {
  const t = useT();
  const x = TEXT[view];
  return (
    <div className="placeholder">
      <div className="placeholder__card">
        <span className="dw-label placeholder__stage">{t('placeholder.stage', { stage: x.stage })}</span>
        <h1 className="placeholder__title">{t(x.title)}</h1>
        <p className="placeholder__body">{t(x.body)}</p>
      </div>
    </div>
  );
}
