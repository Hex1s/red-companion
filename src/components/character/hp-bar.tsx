import { useState } from "react";
import type { WoundState } from "@/bindings/WoundState";
import { cn } from "@/lib/utils";

type HpBarProps = {
  current: number;
  max: number;
  className?: string;
};

/** HP bar. Colour follows wound thresholds: < max, < half, < 1 (see docs/ROADMAP.md). */
export function HpBar({ current, max, className }: HpBarProps) {
  const ratio = Math.max(0, Math.min(1, current / max));
  const tone =
    current < 1
      ? "bg-state-mortal"
      : current < max / 2
        ? "bg-state-serious"
        : current < max
          ? "bg-state-light"
          : "bg-state-ok";

  return (
    <div className={cn("flex items-center gap-2", className)}>
      <div className="h-1.5 flex-1 bg-muted">
        <div className={cn("h-full transition-[width,background-color] duration-300", tone)} style={{ width: `${ratio * 100}%` }} />
      </div>
      {/* Fixed width for up to 3+3 digits, so the bar does not creep when HP gains or loses a digit. */}
      <span className="num min-w-[7ch] text-right text-xs text-muted-foreground">
        {current}/{max}
      </span>
    </div>
  );
}

const SIGNAL_TONE: Record<WoundState, string> = {
  healthy: "bg-state-ok",
  light: "bg-state-light",
  serious: "bg-state-serious",
  mortal: "bg-state-mortal",
  dead: "bg-state-dead",
};

type HpSignalProps = {
  current: number;
  max: number;
  state: WoundState;
  className?: string;
};

/**
 * HP as a trace on the board: a thin line that ends in a pad, filled up to the current HP. It idles
 * with a pixel now and then (calm when healthy, stuttering when wounded), a seriously wounded one
 * flickers and glitches once in a long while, a mortally wounded one is a flat line with a blinking pad.
 * The effects are rare on purpose; with "reduce motion" they are off (see index.css).
 */
export function HpSignal({ current, max, state, className }: HpSignalProps) {
  const ratio = Math.max(0, Math.min(1, current / max));
  // Cards must not pulse in unison: each starts at its own point of the cycle.
  const [phase] = useState(() => -Math.random() * 13);
  const alive = state !== "dead";
  const failing = state === "serious" || state === "mortal";

  return (
    <div className={cn("flex items-center gap-2", className)}>
      <div
        className={cn("relative h-3 flex-1", failing && "signal-glitch")}
        style={{ "--sig-delay": `${phase}s` } as React.CSSProperties}
      >
        <span className="absolute inset-x-0 top-1/2 h-px -translate-y-1/2 bg-border" />
        <span className="absolute top-1/2 right-0 size-1.5 -translate-y-1/2 border border-border bg-card" />
        <span
          className={cn(
            "absolute top-1/2 left-0 h-0.5 -translate-y-1/2 transition-[width,background-color] duration-300",
            SIGNAL_TONE[state],
            state === "serious" && "signal-flicker",
          )}
          style={{ width: `${ratio * 100}%` }}
        >
          {ratio > 0 && <span className={cn("absolute top-1/2 right-0 size-1.5 -translate-y-1/2", SIGNAL_TONE[state])} />}
          {ratio > 0 && alive && (
            <span
              className={cn(
                "absolute top-1/2 left-0 size-[3px] -translate-y-1/2 bg-emphasis opacity-0",
                state === "healthy" ? "signal-pulse" : "signal-pulse-jerky",
              )}
            />
          )}
        </span>
        {state === "mortal" && (
          <span className="animate-cursor absolute top-1/2 left-0 size-1.5 -translate-y-1/2 bg-state-mortal" />
        )}
      </div>
      {/* Fixed width for up to 3+3 digits, so the line does not creep when HP gains or loses a digit. */}
      <span className="num min-w-[7ch] text-right text-xs text-muted-foreground">
        {current}/{max}
      </span>
    </div>
  );
}
