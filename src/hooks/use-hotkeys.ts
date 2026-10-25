import { useEffect, useRef } from "react";
import { HOTKEYS, HOTKEY_IDS, matches, modalOpen, type HotkeyId } from "@/lib/hotkeys";

export type HotkeyHandlers = Partial<Record<HotkeyId, () => void>>;

/**
 * Listens for the shortcuts of `lib/hotkeys` and calls the matching handler. A shortcut with no
 * handler is left to the browser. The handlers may change on every render; the listener is bound once.
 */
export function useHotkeys(handlers: HotkeyHandlers) {
  const latest = useRef(handlers);
  latest.current = handlers;

  useEffect(() => {
    const onKey = (e: KeyboardEvent) => {
      if (e.repeat) return;
      for (const id of HOTKEY_IDS) {
        const hotkey = HOTKEYS[id];
        const handler = latest.current[id];
        if (!handler || !matches(e, hotkey)) continue;
        e.preventDefault();
        if (!hotkey.game || !modalOpen()) handler();
        return;
      }
    };
    window.addEventListener("keydown", onKey);
    return () => window.removeEventListener("keydown", onKey);
  }, []);
}
