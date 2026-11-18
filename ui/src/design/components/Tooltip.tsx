// Kurzhilfe (9.3): Name, aktueller Wert und Tastenkürzel eines Bedienelements.
import { useEffect, useLayoutEffect, useRef, useSyncExternalStore } from 'react';

export interface TipContent {
  name: string;
  value?: string;
  /** Tastenkürzel als Kappen, z. B. ['Shift', '←']. */
  keys?: string[];
}

interface TipState {
  owner: object;
  rect: DOMRect;
  content: TipContent;
}

let tip: TipState | null = null;
let timer: ReturnType<typeof setTimeout> | null = null;
const listeners = new Set<() => void>();
const SHOW_DELAY = 450;

function set(next: TipState | null): void {
  tip = next;
  for (const l of listeners) l();
}

function cancel(): void {
  if (timer) clearTimeout(timer);
  timer = null;
}

/**
 * Verbindet ein Bedienelement mit der Kurzhilfe. `immediate` zeigt sie sofort
 * (während ein Regler gezogen wird), sonst nach kurzer Verzögerung.
 */
export interface TipHandlers {
  onPointerEnter?: (e: React.PointerEvent) => void;
  onPointerLeave?: () => void;
  onFocus?: (e: React.FocusEvent) => void;
  onBlur?: () => void;
}

export function useTip(content: TipContent | undefined, immediate = false): TipHandlers {
  const owner = useRef({}).current;
  const el = useRef<Element | null>(null);
  const latest = useRef(content);
  latest.current = content;

  const show = (target: Element, delay: number) => {
    el.current = target;
    cancel();
    const open = () => {
      if (latest.current && el.current) set({ owner, rect: el.current.getBoundingClientRect(), content: latest.current });
    };
    if (delay <= 0) open();
    else timer = setTimeout(open, delay);
  };
  const hide = () => {
    cancel();
    if (tip?.owner === owner) set(null);
  };

  // Inhalt nachführen, solange die Hilfe zu diesem Element offen ist.
  useEffect(() => {
    if (tip?.owner === owner && content && el.current) {
      set({ owner, rect: el.current.getBoundingClientRect(), content });
    }
  });

  useEffect(() => {
    if (immediate && el.current && content) show(el.current, 0);
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [immediate]);

  useEffect(() => hide, []); // eslint-disable-line react-hooks/exhaustive-deps

  if (!content) return {};
  return {
    onPointerEnter: (e: React.PointerEvent) => show(e.currentTarget, SHOW_DELAY),
    onPointerLeave: () => {
      if (!immediate) hide();
    },
    onFocus: (e: React.FocusEvent) => {
      if (e.currentTarget.matches(':focus-visible')) show(e.currentTarget, 0);
    },
    onBlur: hide,
  };
}

/** Darstellung der Kurzhilfe (auch statisch für die Designsystem-Seite). */
export function TipBox({ content, isStatic, style, boxRef }: { content: TipContent; isStatic?: boolean; style?: React.CSSProperties; boxRef?: React.Ref<HTMLDivElement> }) {
  return (
    <div className="dw-tip" role="tooltip" data-static={isStatic ? 'true' : undefined} style={style} ref={boxRef}>
      <span className="dw-tip__name">{content.name}</span>
      {content.value !== undefined && <span className="dw-tip__value">{content.value}</span>}
      {content.keys && content.keys.length > 0 && (
        <span className="dw-tip__keys">
          {content.keys.map((k, i) => (
            <kbd className="dw-kbd" key={i}>
              {k}
            </kbd>
          ))}
        </span>
      )}
    </div>
  );
}

/** Einmal in der Hülle einhängen. */
export function TipLayer() {
  const state = useSyncExternalStore(
    (l) => {
      listeners.add(l);
      return () => listeners.delete(l);
    },
    () => tip,
  );
  const box = useRef<HTMLDivElement | null>(null);

  // Über dem Element zentrieren; am Rand einrücken, bei Platzmangel darunter.
  useLayoutEffect(() => {
    const node = box.current;
    if (!node || !state) return;
    const w = node.offsetWidth;
    const h = node.offsetHeight;
    const gap = 6;
    let x = state.rect.left + state.rect.width / 2 - w / 2;
    x = Math.max(4, Math.min(window.innerWidth - w - 4, x));
    let y = state.rect.top - h - gap;
    if (y < 4) y = state.rect.bottom + gap;
    node.style.left = `${Math.round(x)}px`;
    node.style.top = `${Math.round(y)}px`;
  });

  useEffect(() => {
    const close = () => {
      cancel();
      set(null);
    };
    window.addEventListener('blur', close);
    window.addEventListener('hashchange', close);
    return () => {
      window.removeEventListener('blur', close);
      window.removeEventListener('hashchange', close);
    };
  }, []);

  if (!state) return null;
  return <TipBox content={state.content} boxRef={box} style={{ left: -9999, top: -9999 }} />;
}
