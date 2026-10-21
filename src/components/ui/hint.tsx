import type { ComponentProps, ReactNode } from "react";
import { Tooltip, TooltipContent, TooltipTrigger } from "@/components/ui/tooltip";

type HintProps = {
  /** Tooltip content; a plain string or a small block of text. */
  label: ReactNode;
  side?: ComponentProps<typeof TooltipContent>["side"];
  children: ReactNode;
};

/** Wraps one element (a button, an input) into a tooltip shown on hover and focus. */
export function Hint({ label, side, children }: HintProps) {
  return (
    <Tooltip>
      <TooltipTrigger asChild>{children}</TooltipTrigger>
      <TooltipContent side={side}>{label}</TooltipContent>
    </Tooltip>
  );
}
