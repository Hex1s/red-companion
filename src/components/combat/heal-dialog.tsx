import { useState } from "react";
import { Dices } from "lucide-react";
import { useTranslation } from "react-i18next";
import type { CriticalInjury } from "@/bindings/CriticalInjury";
import type { HealApplied } from "@/bindings/HealApplied";
import type { HealKind } from "@/bindings/HealKind";
import type { TreatmentSkill } from "@/bindings/TreatmentSkill";
import { Button } from "@/components/ui/button";
import { ErrorNotice } from "@/components/ui/error-notice";
import { Dialog, DialogContent, DialogDescription, DialogTitle } from "@/components/ui/dialog";
import { FieldBadge } from "@/components/ui/field-badge";
import { Hint } from "@/components/ui/hint";
import { RollNote } from "@/components/ui/roll-note";
import { Input } from "@/components/ui/input";
import {
  Select,
  SelectContent,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from "@/components/ui/select";
import type { CharacterEntry } from "@/hooks/use-characters";
import { errorMessage } from "@/lib/errors";
import { api } from "@/lib/api";

/** Lowest and highest total of a d10 check: 1 - 10 and 10 + 10 (see `check_total` in Rust). */
const TOTAL_MIN = -9;
const TOTAL_MAX = 20;

/** What the window heals: one injury of a character, by first aid or by treatment. */
export type HealTarget = { character: CharacterEntry; injury: CriticalInjury; kind: HealKind };

type HealDialogProps = {
  target: HealTarget | null;
  /** Everyone who can be the healer. */
  helpers: CharacterEntry[];
  onDone: (applied: HealApplied) => void;
  onClose: () => void;
};

/** "Heal injury": who heals and with what skill, roll or declare it done. Closes on success. */
export function HealDialog({ target, helpers, onDone, onClose }: HealDialogProps) {
  return (
    <Dialog open={target !== null} onOpenChange={(open) => !open && onClose()}>
      <DialogContent className="sm:max-w-sm" aria-describedby={undefined}>
        {target && (
          <HealForm
            key={`${target.character.id}:${target.injury.id}:${target.kind}`}
            target={target}
            helpers={helpers}
            onDone={onDone}
            onClose={onClose}
          />
        )}
      </DialogContent>
    </Dialog>
  );
}

type HealFormProps = Omit<HealDialogProps, "target"> & { target: HealTarget };

function HealForm({ target, helpers, onDone, onClose }: HealFormProps) {
  const { t } = useTranslation();
  const { character: patient, injury, kind } = target;
  const options = kind === "first_aid" ? injury.first_aid : injury.treatment;
  const [helperId, setHelperId] = useState(() => String(patient.id));
  const [skill, setSkill] = useState<TreatmentSkill>(options[0]?.skill ?? "first_aid");
  const [surgeryText, setSurgeryText] = useState("");
  const [totalText, setTotalText] = useState("");
  const [failed, setFailed] = useState<HealApplied["result"] | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [busy, setBusy] = useState(false);

  const helper = helpers.find((h) => String(h.id) === helperId) ?? patient;
  const dv = options.find((o) => o.skill === skill)?.dv ?? null;
  const skillName = (id: TreatmentSkill) =>
    id === "surgery" ? t("heal.skill_surgery") : t(`skill.items.${id}`);
  // First aid and paramedic add TECH + the sheet level; surgery comes from a Medtech's Medicine
  // (Rust adds it when no bonus is sent), anyone else types it in.
  const helperIsMedtech = helper.character.roles.some((r) => r.role === "medtech");
  const sheetBonus = (id: TreatmentSkill) =>
    helper.character.stats.tech +
    (id === "surgery" ? helper.derived.surgery_skill : (helper.character.skills[id] ?? 0));

  const surgery = skill === "surgery";
  const surgeryBonus = surgeryText.trim() === "" ? null : Number(surgeryText);
  const surgeryInvalid =
    surgery &&
    (surgeryBonus === null ? !helperIsMedtech : !Number.isInteger(surgeryBonus));

  const total = totalText.trim() === "" ? null : Number(totalText);
  const totalInvalid =
    total !== null && (!Number.isInteger(total) || total < TOTAL_MIN || total > TOTAL_MAX);

  const run = async (manualTotal: number | null, granted: boolean) => {
    setBusy(true);
    try {
      const applied = await api.healInjury(
        patient.id,
        helper.id,
        injury.id,
        kind,
        skill,
        surgery ? surgeryBonus : null,
        manualTotal,
        granted,
      );
      onDone(applied);
      if (applied.result.success) onClose();
      else setFailed(applied.result);
      setError(null);
    } catch (e) {
      setError(errorMessage(e));
    } finally {
      setBusy(false);
    }
  };

  const title =
    kind === "first_aid" && injury.quick_fix_removes_effect ? t("heal.quick_fix") : t(`heal.${kind}`);
  const canRoll = !busy && !surgeryInvalid;

  return (
    <div className="space-y-4">
      <div className="space-y-1">
        <DialogTitle className="text-xl font-bold">{title}</DialogTitle>
        <DialogDescription className="flex flex-wrap items-center gap-x-3">
          <span className="font-display font-semibold text-foreground uppercase">
            {patient.character.name}
          </span>
          <span>{t(`injury.${injury.id}.name`, { defaultValue: injury.id })}</span>
          {dv !== null && (
            <span className="num">
              {t("heal.dv")} {dv}
            </span>
          )}
        </DialogDescription>
      </div>

      <div className="flex flex-wrap items-center gap-2">
        <Select value={helperId} onValueChange={setHelperId}>
          <SelectTrigger className="w-44" aria-label={t("heal.helper")}>
            <SelectValue />
          </SelectTrigger>
          <SelectContent>
            {helpers.map((h) => (
              <SelectItem key={h.id} value={String(h.id)}>
                {h.character.name}
              </SelectItem>
            ))}
          </SelectContent>
        </Select>
        {options.map((o) => (
          <Hint
            key={o.skill}
            label={
              o.skill === "surgery"
                ? helperIsMedtech
                  ? t("heal.surgery_medtech_hint", { skill: helper.derived.surgery_skill })
                  : t("heal.surgery_bonus_hint")
                : t("stabilize.formula_short", {
                    tech: helper.character.stats.tech,
                    level: helper.character.skills[o.skill] ?? 0,
                  })
            }
          >
            <Button
              type="button"
              size="sm"
              variant={skill === o.skill ? "default" : "outline"}
              onClick={() => setSkill(o.skill)}
            >
              {skillName(o.skill)}
              <span className="num">
                {o.skill === "surgery" && !helperIsMedtech
                  ? `${t("heal.dv")} ${o.dv}`
                  : `+${sheetBonus(o.skill)}`}
              </span>
            </Button>
          </Hint>
        ))}
      </div>

      {surgery && (
        <Hint label={t("heal.surgery_bonus_hint")}>
          <FieldBadge
            error={surgeryInvalid && surgeryText.trim() !== "" ? t("validation.check_total") : null}
            className="w-32"
          >
            {(invalid) => (
              <Input
                inputMode="numeric"
                value={surgeryText}
                placeholder={
                  helperIsMedtech ? String(sheetBonus("surgery")) : t("heal.surgery_bonus")
                }
                aria-label={t("heal.surgery_bonus")}
                aria-invalid={invalid}
                onChange={(e) => setSurgeryText(e.target.value)}
                className="num w-32"
              />
            )}
          </FieldBadge>
        </Hint>
      )}

      <div className="flex items-center gap-2">
        <Button type="button" disabled={!canRoll} onClick={() => void run(null, false)}>
          <Dices />
          {t("heal.roll")}
        </Button>
        <Hint label={t("stabilize_dialog.total_hint")}>
          <FieldBadge error={totalInvalid ? t("validation.check_total") : null} className="w-20">
            {(invalid) => (
              <Input
                inputMode="numeric"
                value={totalText}
                placeholder="d10"
                aria-label={t("stabilize_dialog.total_label")}
                aria-invalid={invalid}
                onChange={(e) => setTotalText(e.target.value)}
                onKeyDown={(e) => {
                  if (e.key === "Enter" && total !== null && !totalInvalid && canRoll) {
                    void run(total, false);
                  }
                }}
                className="num w-20"
              />
            )}
          </FieldBadge>
        </Hint>
      </div>

      {failed && failed.roll && (
        <p className="num text-sm text-state-mortal-text">
          {t("heal.result", {
            bonus: failed.bonus,
            die: failed.roll.total,
            total: failed.total,
            dv: failed.dv,
          })}{" "}
          — {t("heal.failed")}
        </p>
      )}
      {failed && <RollNote roll={failed.roll} />}
      <ErrorNotice message={error} />

      <Hint label={t("heal.declare_hint")}>
        <Button
          type="button"
          variant="outline"
          className="w-full"
          disabled={busy}
          onClick={() => void run(null, true)}
        >
          {t("heal.declare")}
        </Button>
      </Hint>
    </div>
  );
}
