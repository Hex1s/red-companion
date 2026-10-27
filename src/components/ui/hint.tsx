import { useEffect, useState } from "react";
import type { ComponentProps, ReactNode } from "react";
import { Tooltip, TooltipContent, TooltipTrigger } from "@/components/ui/tooltip";
import { useNoHover } from "@/hooks/use-media-query";

type HintProps = {
  /** Tooltip content; a plain string or a small block of text. */
  label: ReactNode;
  side?: ComponentProps<typeof TooltipContent>["side"];
  children: ReactNode;
};

/** How long a tapped hint stays on screen; a finger has no "leave" to close it by. */
const TAP_HINT_MS = 3000;

/**
 * Wraps one element (a button, an input) into a tooltip shown on hover and focus. Where the
 * primary input cannot hover (finger, stylus) the hint opens on a tap instead; the tap still
 * reaches the element, so a button acts and shows its hint at the same time.
 */
export function Hint({ label, side, children }: HintProps) {
  const noHover = useNoHover();
  const [open, setOpen] = useState(false);

  useEffect(() => {
    if (!open) return;
    const timer = window.setTimeout(() => setOpen(false), TAP_HINT_MS);
    // Any touch closes it; a tap on the trigger itself closes it here and reopens in `onClick`.
    const close = () => setOpen(false);
    document.addEventListener("pointerdown", close, true);
    return () => {
      window.clearTimeout(timer);
      document.removeEventListener("pointerdown", close, true);
    };
  }, [open]);

  if (!noHover) {
    return (
      <Tooltip>
        <TooltipTrigger asChild>{children}</TooltipTrigger>
        <TooltipContent side={side}>{label}</TooltipContent>
      </Tooltip>
    );
  }
  // Radix closes a tooltip on every click of its trigger and ignores touch hovers, so the open
  // state is ours and its own change requests are dropped.
  return (
    <Tooltip open={open} onOpenChange={() => undefined}>
      <TooltipTrigger asChild onClick={() => setOpen(true)}>
        {children}
      </TooltipTrigger>
      <TooltipContent side={side}>{label}</TooltipContent>
    </Tooltip>
  );
}
