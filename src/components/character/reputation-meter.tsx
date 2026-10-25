import { Minus, Plus } from "lucide-react";
import { useTranslation } from "react-i18next";
import { Button } from "@/components/ui/button";
import { Hint } from "@/components/ui/hint";
import { REPUTATION_MAX } from "@/lib/character-schema";
import { cn } from "@/lib/utils";

type ReputationMeterProps = {
  value: number;
  onChange: (value: number) => void;
};

/** Steps from the middle outwards: 1..10. */
const STEPS = Array.from({ length: REPUTATION_MAX }, (_, i) => i + 1);

type StepProps = { step: number; level: number; side: "bad" | "good"; onPick: (value: number) => void };

/** One step of the scale: its own box, lit when the reputation has reached it. */
function Step({ step, level, side, onPick }: StepProps) {
  const target = side === "good" ? step : -step;
  const lit = side === "good" ? level >= step : level <= -step;
  return (
    <button
      type="button"
      tabIndex={-1}
      aria-label={String(target)}
      onClick={() => onPick(level === target ? target + (side === "good" ? -1 : 1) : target)}
      className={cn(
        "h-5 min-w-0 flex-1 border transition-colors",
        lit ? (side === "good" ? "border-state-ok bg-state-ok" : "border-destructive bg-destructive") : "bg-muted/40 hover:bg-muted",
      )}
    />
  );
}

/**
 * Reputation as a scale of 20 separate steps (-10..-1 | 1..10), "−" on the left and "+" on the
 * right, the value in the middle: green above zero, red below. A click on a step sets it.
 */
export function ReputationMeter({ value, onChange }: ReputationMeterProps) {
  const { t } = useTranslation();
  const level = Number.isFinite(value) ? Math.max(-REPUTATION_MAX, Math.min(REPUTATION_MAX, value)) : 0;

  return (
    <Hint label={<span className="block max-w-64">{t("reputation.field_hint")}</span>}>
      <div className="cut flex items-center gap-3 p-3 [--cut:6px]">
        <span className="w-24 shrink-0 text-xs text-muted-foreground">{t("reputation.field")}</span>
        <Button
          type="button"
          size="icon-xs"
          variant="outline"
          aria-label={t("reputation.decrease")}
          disabled={level <= -REPUTATION_MAX}
          onClick={() => onChange(level - 1)}
        >
          <Minus />
        </Button>
        <div
          role="meter"
          aria-label={t("reputation.field")}
          aria-valuemin={-REPUTATION_MAX}
          aria-valuemax={REPUTATION_MAX}
          aria-valuenow={level}
          className="flex min-w-0 flex-1 items-center gap-3"
        >
          <div className="flex min-w-0 flex-1 flex-row-reverse gap-1">
            {STEPS.map((s) => (
              <Step key={s} step={s} level={level} side="bad" onPick={onChange} />
            ))}
          </div>
          <span
            className={cn(
              "num w-10 shrink-0 text-center text-2xl font-bold",
              level > 0 && "text-state-ok",
              level < 0 && "text-destructive",
            )}
          >
            {level}
          </span>
          <div className="flex min-w-0 flex-1 gap-1">
            {STEPS.map((s) => (
              <Step key={s} step={s} level={level} side="good" onPick={onChange} />
            ))}
          </div>
        </div>
        <Button
          type="button"
          size="icon-xs"
          variant="outline"
          aria-label={t("reputation.increase")}
          disabled={level >= REPUTATION_MAX}
          onClick={() => onChange(level + 1)}
        >
          <Plus />
        </Button>
      </div>
    </Hint>
  );
}
