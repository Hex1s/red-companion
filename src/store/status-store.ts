import { create } from "zustand";

export type StatusKind = "loading" | "success" | "error" | "info";

export type StatusAction = { label: string; run: () => void };

export type StatusItem = {
  id: string;
  kind: StatusKind;
  text: string;
  action?: StatusAction;
  /** Fading out: still on screen, removed a moment later. */
  closing: boolean;
};

export type StatusInput = {
  /** Same id = the same message: it is updated in place (loading → success), not stacked. */
  id?: string;
  kind: StatusKind;
  text: string;
  action?: StatusAction;
  /** Overrides how long a success / info message stays. Loading and errors never time out. */
  autoHideMs?: number;
};

/** How long a message stays before it fades out; `undefined` = until dismissed. */
const AUTO_HIDE_MS: Record<StatusKind, number | undefined> = {
  loading: undefined,
  success: 2200,
  info: 4500,
  error: undefined,
};
const FADE_MS = 200;

type StatusState = {
  items: StatusItem[];
  show: (input: StatusInput) => string;
  dismiss: (id: string) => void;
};

const timers = new Map<string, number>();
let counter = 0;

function clearTimer(id: string) {
  const timer = timers.get(id);
  if (timer !== undefined) window.clearTimeout(timer);
  timers.delete(id);
}

/** UI state of the status area (bottom right of the work area): loading, saved, errors. */
export const useStatusStore = create<StatusState>((set, get) => ({
  items: [],

  show: ({ id, kind, text, action, autoHideMs }) => {
    const key = id ?? `status:${++counter}`;
    clearTimer(key);
    set((state) => {
      const item: StatusItem = { id: key, kind, text, action, closing: false };
      const exists = state.items.some((i) => i.id === key);
      return {
        items: exists ? state.items.map((i) => (i.id === key ? item : i)) : [...state.items, item],
      };
    });
    const hideAfter = autoHideMs ?? AUTO_HIDE_MS[kind];
    if (hideAfter !== undefined) {
      timers.set(
        key,
        window.setTimeout(() => get().dismiss(key), hideAfter),
      );
    }
    return key;
  },

  dismiss: (id) => {
    clearTimer(id);
    set((state) => ({ items: state.items.map((i) => (i.id === id ? { ...i, closing: true } : i)) }));
    timers.set(
      id,
      window.setTimeout(() => {
        timers.delete(id);
        set((state) => ({ items: state.items.filter((i) => i.id !== id || !i.closing) }));
      }, FADE_MS),
    );
  },
}));

/** Shorthand for code outside React components (hooks, event handlers). */
export const status = {
  show: (input: StatusInput) => useStatusStore.getState().show(input),
  dismiss: (id: string) => useStatusStore.getState().dismiss(id),
  /** Error message that stays until closed. */
  error: (text: string, id?: string) => useStatusStore.getState().show({ id, kind: "error", text }),
};
