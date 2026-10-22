import { useState } from "react";
import { Dices } from "lucide-react";
import { useTranslation } from "react-i18next";
import type { SkillCheck } from "@/bindings/SkillCheck";
import { Button } from "@/components/ui/button";
import { FieldBadge } from "@/components/ui/field-badge";
import { Hint } from "@/components/ui/hint";
import { Input } from "@/components/ui/input";
import { Popover, PopoverContent, PopoverTrigger } from "@/components/ui/popover";
import { errorMessage } from "@/hooks/use-characters";
import { api } from "@/lib/api";
import { STAT_ABBR } from "@/lib/stat-abbr";
import { status } from "@/store/status-store";

/** Lowest and highest total of a d10 check: 1 - 10 and 10 + 10 (see `check_total` in Rust). */
const TOTAL_MIN = -9;
const TOTAL_MAX = 20;

type SkillCheckButtonProps = {
  characterId: number;
  /** Sheet key of the skill: `id` or `id:specialty`. */
  skillKey: string;
  /** Name shown in the popover. */
  label: string;
};

/** Quick check of a skill: STAT + level + d10, rolled in the app or typed in from a physical die. */
export function SkillCheckButton({ characterId, skillKey, label }: SkillCheckButtonProps) {
  const { t } = useTranslation();
  const [open, setOpen] = useState(false);
  const [totalText, setTotalText] = useState("");
  const [result, setResult] = useState<SkillCheck | null>(null);
  const [busy, setBusy] = useState(false);

  const total = totalText.trim() === "" ? null : Number(totalText);
  const totalInvalid = total !== null && (!Number.isInteger(total) || total < TOTAL_MIN || total > TOTAL_MAX);

  const run = async (manualTotal: number | null) => {
    setBusy(true);
    try {
      setResult(await api.skillCheck(characterId, skillKey, manualTotal));
      setTotalText("");
    } catch (e) {
      status.error(errorMessage(e), "skill-check");
    } finally {
      setBusy(false);
    }
  };

  return (
    <Popover
      open={open}
      onOpenChange={(next) => {
        setOpen(next);
        if (next) setResult(null);
      }}
    >
      <Hint label={t("skill_check.tooltip")}>
        <PopoverTrigger asChild>
          <Button type="button" size="xs" variant="outline" aria-label={t("skill_check.tooltip")}>
            <Dices />
          </Button>
        </PopoverTrigger>
      </Hint>
      <PopoverContent align="end" className="w-72 space-y-3">
        <p className="text-sm">{label}</p>
        <div className="flex items-center gap-2">
          <Button type="button" disabled={busy} onClick={() => void run(null)}>
            <Dices />
            {t("skill_check.roll")}
          </Button>
          <Hint label={t("skill_check.total_hint")}>
            <FieldBadge error={totalInvalid ? t("validation.check_total") : null} className="w-20">
              {(invalid) => (
                <Input
                  inputMode="numeric"
                  value={totalText}
                  placeholder="d10"
                  aria-label={t("skill_check.total_label")}
                  aria-invalid={invalid}
                  onChange={(e) => setTotalText(e.target.value)}
                  onKeyDown={(e) => {
                    if (e.key === "Enter" && total !== null && !totalInvalid && !busy) void run(total);
                  }}
                  className="num w-20"
                />
              )}
            </FieldBadge>
          </Hint>
        </div>
        {result && (
          <div className="space-y-1">
            <p className="num text-2xl font-bold text-primary">{result.total}</p>
            <p className="num text-xs text-muted-foreground">
              {t("skill_check.formula", {
                stat: t(`stats.${STAT_ABBR[result.stat]}`),
                statValue: result.stat_value,
                level: result.level,
                die: result.die.total,
              })}
            </p>
          </div>
        )}
      </PopoverContent>
    </Popover>
  );
}
