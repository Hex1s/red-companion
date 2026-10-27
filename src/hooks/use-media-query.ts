import { useSyncExternalStore } from "react";

/** Subscribes to a CSS media query; re-renders only when its match flips. */
export function useMediaQuery(query: string): boolean {
  return useSyncExternalStore(
    (onChange) => {
      const list = window.matchMedia(query);
      list.addEventListener("change", onChange);
      return () => list.removeEventListener("change", onChange);
    },
    () => window.matchMedia(query).matches,
    () => false,
  );
}

/**
 * Touch is detected by input type, not by width: a tablet with a mouse and a touch laptop both
 * behave correctly. Mirrors the `touch` / `no-hover` variants in `index.css`.
 */
export function useCoarsePointer(): boolean {
  return useMediaQuery("(pointer: coarse)");
}

/** The primary input cannot hover (finger, stylus): hover-only affordances need a tap variant. */
export function useNoHover(): boolean {
  return useMediaQuery("(hover: none)");
}
