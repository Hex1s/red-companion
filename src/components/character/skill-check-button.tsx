import { useSkillText } from "@/components/character/skill-hint";
import { useState } from "react";
import { Dices } from "@/components/ui/icons";
import { useTranslation } from "react-i18next";
import type { SkillCheck } from "@/bindings/SkillCheck";
import { Button } from "@/components/ui/button";
import { FieldBadge } from "@/components/ui/field-badge";
import { Hint } from "@/components/ui/hint";
import { Input } from "@/components/ui/input";
import { RollNote } from "@/components/ui/roll-note";
import { Popover, PopoverContent, PopoverTrigger } from "@/components/ui/popover";
import { api } from "@/lib/api";
import { formatCheck } from "@/lib/dice";
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
  /** Why the check cannot be rolled now (an invalid level on the sheet); `null` = it can. */
  disabledReason?: string | null;
};

/** Quick check of a skill: STAT + level + d10, rolled in the app or typed in from a physical die. */
export function SkillCheckButton({ characterId, skillKey, label, disabledReason = null }: SkillCheckButtonProps) {
  const { t } = useTranslation();
  const { summary } = useSkillText(skillKey.split(":")[0]);
  const [open, setOpen] = useState(false);
  const [totalText, setTotalText] = useState("");
  const [result, setResult] = useState<SkillCheck | null>(null);
  const [busy, setBusy] = useState(false);

  const total = totalText.trim() === "" ? null : Number(totalText);
  const totalInvalid = total !== null && (!Number.isInteger(total) || total < TOTAL_MIN || total > TOTAL_MAX);

  const run = async (manualTotal: number | null) => {
    setBusy(true);
    try {
      const check = await api.skillCheck(characterId, skillKey, manualTotal);
      setResult(check);
      setTotalText("");
      // Results also go to the status area; every roll is its own message.
      status.show({
        kind: "info",
        id: `skill-check:${Date.now()}`,
        text: t(check.role_bonus !== 0 ? "skill_check.toast_role" : "skill_check.toast", {
          name: label,
          stat: t(`stats.${STAT_ABBR[check.stat]}`),
          statValue: check.stat_value,
          level: check.level,
          role: check.role_bonus,
          die: formatCheck(check.die),
          total: check.total,
        }),
      });
    } catch (e) {
      status.fail(e, "skill-check");
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
      <Hint label={disabledReason ?? t("skill_check.tooltip")}>
        {/* A disabled button gets no pointer events, so the tooltip hangs on the wrapper. */}
        <span className="inline-flex">
          <PopoverTrigger asChild>
            <Button
              type="button"
              size="xs"
              variant="outline"
              disabled={disabledReason !== null}
              aria-label={disabledReason ?? t("skill_check.tooltip")}
            >
              <Dices />
            </Button>
          </PopoverTrigger>
        </span>
      </Hint>
      <PopoverContent align="end" className="w-72 space-y-3">
        <p className="text-sm">{label}</p>
        {summary && <p className="text-xs text-muted-foreground">{summary}</p>}
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
            <p className="num text-2xl font-bold text-emphasis">{result.total}</p>
            <p className="num text-xs text-muted-foreground">
              {t(result.role_bonus !== 0 ? "skill_check.formula_role" : "skill_check.formula", {
                stat: t(`stats.${STAT_ABBR[result.stat]}`),
                statValue: result.stat_value,
                level: result.level,
                role: result.role_bonus,
                die: formatCheck(result.die),
              })}
            </p>
            <RollNote roll={result.die} />
          </div>
        )}
      </PopoverContent>
    </Popover>
  );
}
