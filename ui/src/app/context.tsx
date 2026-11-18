import { createContext, useContext } from 'react';
import type { CoreApi } from '../core/api';

export const CoreContext = createContext<CoreApi | null>(null);

export function useCore(): CoreApi {
  const core = useContext(CoreContext);
  if (!core) throw new Error('CoreContext fehlt');
  return core;
}
