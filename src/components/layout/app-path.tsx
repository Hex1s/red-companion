import { useEffect, useRef, useState } from "react";
import { useTranslation } from "react-i18next";

type AppPathProps = {
  /** Path segments after the app name: the tab slug, then the opened item (if any). */
  segments: string[];
};

/** Erasing and typing take a fixed time each, however long the text: long paths just type faster. */
const ERASE_MS = 140;
const TYPE_MS = 260;

/** File-system-like slug: no spaces, no slashes, so every segment stays one path part. */
function slug(text: string): string {
  return text.trim().replace(/[\\/]+/g, "-").replace(/\s+/g, "_");
}

function prefersReducedMotion(): boolean {
  return window.matchMedia("(prefers-reduced-motion: reduce)").matches;
}

/**
 * Text that follows `target` like a shell line: erases back to the common prefix, then types the rest.
 * Each phase has a fixed duration. Returns the visible text and whether it is still moving.
 */
function useTyped(target: string): { text: string; busy: boolean } {
  const [text, setText] = useState("");
  const shown = useRef("");

  useEffect(() => {
    const from = shown.current;
    const show = (next: string) => {
      shown.current = next;
      setText(next);
    };
    if (from === target) return;
    if (prefersReducedMotion()) {
      show(target);
      return;
    }
    let keep = 0;
    while (keep < from.length && keep < target.length && from[keep] === target[keep]) keep++;
    const eraseCount = from.length - keep;
    const typeCount = target.length - keep;
    const eraseMs = eraseCount > 0 ? ERASE_MS : 0;
    const start = performance.now();
    let frame = 0;
    const tick = (now: number) => {
      const elapsed = now - start;
      if (elapsed < eraseMs) {
        show(from.slice(0, from.length - Math.floor((eraseCount * elapsed) / eraseMs)));
        frame = requestAnimationFrame(tick);
        return;
      }
      const typed = Math.min(typeCount, Math.floor((typeCount * (elapsed - eraseMs)) / TYPE_MS));
      show(target.slice(0, keep + typed));
      if (typed < typeCount) frame = requestAnimationFrame(tick);
    };
    frame = requestAnimationFrame(tick);
    return () => cancelAnimationFrame(frame);
  }, [target]);

  return { text, busy: text !== target };
}

/**
 * App title as a shell path that follows the current tab and item and is typed out on every change.
 * The trailing `~` is a cursor: solid while typing, blinking when idle.
 */
export function AppPath({ segments }: AppPathProps) {
  const { t } = useTranslation();
  const parts = [t("app.title").toLowerCase(), ...segments.filter((s) => s !== "").map(slug)];
  const { text, busy } = useTyped(`/${parts.join("/")}/`);
  return (
    <h1
      className="mr-auto flex min-w-0 text-lg font-bold text-emphasis normal-case tracking-normal"
      aria-label={[t("app.title"), ...segments].filter((s) => s !== "").join(" / ")}
    >
      <span aria-hidden="true" className="truncate">
        {text}
      </span>
      <span aria-hidden="true" className={busy ? "text-primary" : "animate-cursor text-primary"}>
        ~
      </span>
    </h1>
  );
}
