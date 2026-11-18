// Hash-Router: #/mixen, #/bibliothek, #/vorbereiten, #/automix, #/designsystem.
import { useSyncExternalStore } from 'react';

export const VIEWS = ['mixen', 'bibliothek', 'vorbereiten', 'automix'] as const;
export type ViewId = (typeof VIEWS)[number];
export type RouteId = ViewId | 'designsystem';

export function parseRoute(hash: string): RouteId {
  const name = hash.replace(/^#\/?/, '').split(/[/?]/)[0];
  if (name === 'designsystem') return 'designsystem';
  return (VIEWS as readonly string[]).includes(name) ? (name as ViewId) : 'mixen';
}

function subscribe(l: () => void): () => void {
  window.addEventListener('hashchange', l);
  return () => window.removeEventListener('hashchange', l);
}

export const useRoute = (): RouteId => useSyncExternalStore(subscribe, () => parseRoute(window.location.hash));

export function navigate(route: RouteId): void {
  window.location.hash = `#/${route}`;
}
