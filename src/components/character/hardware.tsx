import type { ReactNode } from "react";
import { cn } from "@/lib/utils";

const PINS = 5;

type PinsProps = { className?: string };

/** A column of chip legs: short gold bars, spread along the side. */
function Pins({ className }: PinsProps) {
  return (
    <span aria-hidden className={cn("absolute inset-y-2.5 flex flex-col justify-between", className)}>
      {Array.from({ length: PINS }, (_, i) => (
        <span key={i} className="h-0.5 w-2 bg-brand/70" />
      ))}
    </span>
  );
}

type ChipProps = {
  /** The stat's short name, big and red: the first thing the eye reads. */
  title: ReactNode;
  /** The full name, small, under the number. */
  caption?: ReactNode;
  /** The value, usually an input. */
  children: ReactNode;
  className?: string;
};

/**
 * A microchip: legs on both sides, a notch marking pin 1, a die outline inside. Holds one stat.
 * Flat shapes only (the `cut` corners, 1px lines), nothing raster, so it costs the bundle nothing.
 */
export function Chip({ title, caption, children, className }: ChipProps) {
  return (
    <div className={cn("relative px-2", className)}>
      <Pins className="left-0" />
      <Pins className="right-0" />
      <div className="cut mx-auto aspect-square [--cut:6px]">
        <div className="relative flex h-full flex-col justify-center space-y-1 px-1.5">
          {/* Pin 1 and the die: what makes the box a chip rather than a card. */}
          <span aria-hidden className="absolute top-1.5 left-1.5 size-1 bg-muted-foreground/60" />
          <span aria-hidden className="pointer-events-none absolute inset-1 border border-border/60" />
          <div className="relative text-center font-display text-base leading-none font-semibold tracking-wider text-destructive-text uppercase">
            {title}
          </div>
          <div className="relative">{children}</div>
          {caption && (
            <div className="relative truncate text-center text-[10px] leading-tight text-muted-foreground">{caption}</div>
          )}
        </div>
      </div>
    </div>
  );
}

/** Contact fingers along the bottom of a module; the gap after the sixth is the key notch. */
const FINGERS = 12;
const KEY_AFTER = 6;

type ModuleProps = {
  label: ReactNode;
  value: ReactNode;
  /** A glyph for the socket on the right (what the number is about). */
  icon?: ReactNode;
  className?: string;
};

/**
 * A module card for a derived number: a gold bracket in the corner, the caption and the value, a socket with
 * a glyph on the right, a row of gold contacts with a key notch along the bottom.
 */
export function ModuleTile({ label, value, icon, className }: ModuleProps) {
  return (
    <div className={cn("cut relative [--cut:6px]", className)}>
      <span aria-hidden className="absolute top-1 left-1 size-2 border-t border-l border-brand" />
      <div className="flex items-center gap-2 px-3 pt-3 pb-5">
        <div className="min-w-0 flex-1 space-y-1">
          <div className="text-[11px] leading-tight tracking-wide text-muted-foreground uppercase">{label}</div>
          <div className="num text-2xl leading-none font-bold">{value}</div>
        </div>
        {icon && (
          <span
            aria-hidden
            className="cut grid size-9 shrink-0 place-items-center text-muted-foreground [--cut-fill:var(--muted)] [--cut:5px] [&_svg]:size-4"
          >
            {icon}
          </span>
        )}
      </div>
      <span aria-hidden className="absolute inset-x-3 bottom-1.5 flex gap-0.5">
        {Array.from({ length: FINGERS }, (_, i) => (
          <span key={i} className={cn("h-1.5 flex-1 bg-brand/60", i === KEY_AFTER && "ml-1.5")} />
        ))}
      </span>
    </div>
  );
}
