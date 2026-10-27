import { useMediaQuery } from "@/hooks/use-media-query";

/**
 * How many panels fit next to the main text. Thresholds match the `mid:` and `wide:` Tailwind
 * variants in `index.css`; keep the two in sync.
 *
 * - `wide` (≥ 1280 px): explorer, main area and combat column side by side.
 * - `medium` (900–1279 px): the explorer is a slide-in panel, the combat column stays (narrower).
 * - `narrow` (< 900 px): both panels slide in; combat is a bottom bar.
 */
export type LayoutMode = "wide" | "medium" | "narrow";

export const WIDE_MIN_PX = 1280;
export const MEDIUM_MIN_PX = 900;

export function useLayoutMode(): LayoutMode {
  const wide = useMediaQuery(`(min-width: ${WIDE_MIN_PX}px)`);
  const medium = useMediaQuery(`(min-width: ${MEDIUM_MIN_PX}px)`);
  if (wide) return "wide";
  return medium ? "medium" : "narrow";
}

/** A landscape tablet leaves about 600 px of height: vertical chrome has to shrink there. */
export function useShortScreen(): boolean {
  return useMediaQuery("(max-height: 640px)");
}
