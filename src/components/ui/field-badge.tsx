import { useEffect, useState, type ReactNode } from "react";
import { Hint } from "@/components/ui/hint";
import { cn } from "@/lib/utils";

/** An error appears once the value has stayed wrong this long, or when the field is left. */
export const FIELD_ERROR_DELAY_MS = 700;

type FieldBadgeProps = {
  /** Why the value is wrong; empty or `null` when it is fine. */
  error: string | null | undefined;
  className?: string;
  /** The field itself; `invalid` is the delayed state to paint the field red with. */
  children: (invalid: boolean) => ReactNode;
};

/**
 * Wraps one input: while the value is wrong, a red "!" sits on the input's top right corner
 * (the corner is its centre) and a tooltip says why. It takes no room in the layout, and it
 * only shows after a pause or when focus leaves the field, so it does not blink while the
 * user is still typing. The one place in the design system with a round shape (`rounded-full`).
 */
export function FieldBadge({ error, className, children }: FieldBadgeProps) {
  const [settled, setSettled] = useState(false);
  const [left, setLeft] = useState(false);

  useEffect(() => {
    setSettled(false);
    if (!error) return;
    const timer = window.setTimeout(() => setSettled(true), FIELD_ERROR_DELAY_MS);
    return () => window.clearTimeout(timer);
  }, [error]);

  const invalid = Boolean(error) && (settled || left);

  return (
    <div
      className={cn("relative", className)}
      onFocusCapture={() => setLeft(false)}
      onBlurCapture={() => setLeft(true)}
    >
      {children(invalid)}
      {invalid && (
        <Hint label={error} side="top">
          <span
            role="img"
            aria-label={error ?? undefined}
            className="absolute top-0 right-0 z-10 grid size-4 translate-x-1/2 -translate-y-1/2 cursor-help place-items-center rounded-full bg-destructive text-[10px] leading-none font-bold text-primary-foreground animate-in fade-in-0 zoom-in-75"
          >
            !
          </span>
        </Hint>
      )}
    </div>
  );
}
