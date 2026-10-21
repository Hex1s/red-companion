import { useTranslation } from "react-i18next";
import type { WoundState } from "@/bindings/WoundState";
import { cn } from "@/lib/utils";

const TONE: Record<WoundState, string> = {
  healthy: "text-state-ok",
  light: "text-state-light",
  serious: "text-state-serious",
  mortal: "text-state-mortal",
  dead: "text-state-dead",
};

type WoundBadgeProps = { state: WoundState; className?: string };

/** Wound state label coloured by its `state-*` token. */
export function WoundBadge({ state, className }: WoundBadgeProps) {
  const { t } = useTranslation();
  return (
    <span className={cn("font-display text-sm font-semibold uppercase", TONE[state], className)}>
      {t(`glossary.wound.${state}`)}
    </span>
  );
}
