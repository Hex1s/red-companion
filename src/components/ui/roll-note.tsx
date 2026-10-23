import { Zap } from "lucide-react";
import { useTranslation } from "react-i18next";
import type { CheckRoll } from "@/bindings/CheckRoll";
import { explosionOf } from "@/lib/dice";
import { cn } from "@/lib/utils";

type RollNoteProps = {
  roll: CheckRoll | null | undefined;
  className?: string;
};

/**
 * Says that a d10 check exploded: a natural 10 added one more die, a natural 1 took one away.
 * Renders nothing for an ordinary roll, so callers can place it unconditionally.
 */
export function RollNote({ roll, className }: RollNoteProps) {
  const { t } = useTranslation();
  const boom = explosionOf(roll);
  if (!boom) return null;
  return (
    <p
      className={cn(
        "num flex items-start gap-1 text-xs",
        boom.direction === "up" ? "text-primary" : "text-state-serious",
        className,
      )}
    >
      <Zap className="mt-0.5 size-3 shrink-0" />
      {t(boom.direction === "up" ? "roll.explode_up" : "roll.explode_down", {
        first: boom.first,
        extra: boom.extra,
      })}
    </p>
  );
}
