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
