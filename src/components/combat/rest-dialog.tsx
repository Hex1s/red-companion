import { useEffect, useState } from "react";
import { useTranslation } from "react-i18next";
import type { RestApplied } from "@/bindings/RestApplied";
import type { RestGoal } from "@/bindings/RestGoal";
import type { RestPlan } from "@/bindings/RestPlan";
import { Button } from "@/components/ui/button";
import { ErrorNotice } from "@/components/ui/error-notice";
import { Dialog, DialogContent, DialogDescription, DialogTitle } from "@/components/ui/dialog";
import { FieldBadge } from "@/components/ui/field-badge";
import { Input } from "@/components/ui/input";
import type { CharacterEntry } from "@/hooks/use-characters";
import { errorMessage } from "@/lib/errors";
import { api } from "@/lib/api";

/** Pause after typing before Rust is asked what the rest gives. */
const PREVIEW_DELAY_MS = 150;
/** Longest rest in days; the same limit the rules enforce. */
const MAX_DAYS = 365;

type RestMode = "hp" | "days";

type RestDialogProps = {
  /** The character who rests; the window is open while it is set. */
  target: CharacterEntry | null;
  onDone: (applied: RestApplied) => void;
  onClose: () => void;
};

/** "Rest": either the wanted HP or a number of days in, the other one and the resulting HP out (the rules count them). */
export function RestDialog({ target, onDone, onClose }: RestDialogProps) {
  return (
    <Dialog open={target !== null} onOpenChange={(open) => !open && onClose()}>
      <DialogContent className="sm:max-w-sm" aria-describedby={undefined}>
        {target && <RestForm key={target.id} target={target} onDone={onDone} onClose={onClose} />}
      </DialogContent>
    </Dialog>
  );
}

type RestFormProps = Omit<RestDialogProps, "target"> & { target: CharacterEntry };

function RestForm({ target, onDone, onClose }: RestFormProps) {
  const { t } = useTranslation();
  const { hp, name } = { hp: target.character.hp, name: target.character.name };
  const maxHp = target.derived.max_hp;
  // At full HP there is nothing to restore up to, so the rest starts as a number of days.
  const [mode, setMode] = useState<RestMode>(hp < maxHp ? "hp" : "days");
  const [hpText, setHpText] = useState(String(maxHp));
  const [daysText, setDaysText] = useState("1");
  const [plan, setPlan] = useState<RestPlan | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [busy, setBusy] = useState(false);

  const text = mode === "hp" ? hpText : daysText;
  const value = text.trim() === "" ? null : Number(text);
  const rangeOk =
    value !== null &&
    Number.isInteger(value) &&
    (mode === "hp" ? value > hp && value <= maxHp : value >= 1 && value <= MAX_DAYS);
  const goal: RestGoal | null = !rangeOk
    ? null
    : mode === "hp"
      ? { kind: "hp", target: value }
      : { kind: "days", days: value };
  const goalKey = goal === null ? "" : `${goal.kind}:${goal.kind === "hp" ? goal.target : goal.days}`;

  useEffect(() => {
    if (goal === null) {
      setPlan(null);
      return;
    }
    const timer = setTimeout(() => {
      api.restPreview(target.id, goal).then(
        (p) => {
          setPlan(p);
          setError(null);
        },
        (e: unknown) => {
          setPlan(null);
          setError(errorMessage(e));
        },
      );
    }, PREVIEW_DELAY_MS);
    return () => clearTimeout(timer);
    // `goalKey` stands for `goal`: a new object with the same numbers must not restart the timer.
  }, [goalKey, target.id]);

  const apply = async () => {
    if (goal === null) return;
    setBusy(true);
    try {
      onDone(await api.restCharacter(target.id, goal));
      onClose();
    } catch (e) {
      setError(errorMessage(e));
    } finally {
      setBusy(false);
    }
  };

  const canApply = goal !== null && plan !== null && !busy;
  const setText = mode === "hp" ? setHpText : setDaysText;

  return (
    <div className="space-y-4">
      <div className="space-y-1">
        <DialogTitle className="text-xl font-bold">{t("rest.title")}</DialogTitle>
        <DialogDescription className="flex items-center gap-3">
          <span className="font-display font-semibold text-foreground uppercase">{name}</span>
          <span className="num">
            {hp} / {maxHp}
          </span>
        </DialogDescription>
      </div>

      <div className="grid grid-cols-2 gap-2" role="group" aria-label={t("rest.title")}>
        {(["hp", "days"] as const).map((m) => (
          <Button
            key={m}
            type="button"
            size="sm"
            variant={mode === m ? "default" : "outline"}
            aria-pressed={mode === m}
            disabled={m === "hp" && hp >= maxHp}
            onClick={() => setMode(m)}
          >
            {t(`rest.mode_${m}`)}
          </Button>
        ))}
      </div>

      <label className="flex items-center gap-3 text-sm">
        <span className="flex-1">{t(mode === "hp" ? "rest.target" : "rest.days_input")}</span>
        <FieldBadge
          key={mode}
          error={
            rangeOk || text.trim() === ""
              ? null
              : t(mode === "hp" ? "rest.target_range" : "rest.days_range", {
                  min: hp + 1,
                  max: mode === "hp" ? maxHp : MAX_DAYS,
                })
          }
          className="w-24"
        >
          {(invalid) => (
            <Input
              autoFocus
              inputMode="numeric"
              value={text}
              aria-invalid={invalid}
              onChange={(e) => setText(e.target.value)}
              onKeyDown={(e) => {
                if (e.key === "Enter" && canApply) void apply();
              }}
              className="num w-24"
            />
          )}
        </FieldBadge>
      </label>

      {plan && (
        <div className="space-y-1 text-sm">
          <p className="num text-muted-foreground">
            {t("rest.per_day", { hp: plan.hp_per_day })}
          </p>
          {mode === "hp" && (
            <p className="flex justify-between">
              <span>{t("rest.days")}</span>
              <span className="num font-semibold">{plan.days}</span>
            </p>
          )}
          <p className="flex justify-between">
            <span>{t("rest.result")}</span>
            <span className="num font-semibold">
              {plan.final_hp} / {plan.max_hp}
            </span>
          </p>
          {mode === "hp" && plan.final_hp > plan.target_hp && (
            <p className="text-xs text-muted-foreground">{t("rest.overshoot")}</p>
          )}
        </div>
      )}
      <p className="text-xs text-muted-foreground">{t("rest.ends_day")}</p>
      <ErrorNotice message={error} />

      <Button type="button" className="w-full" disabled={!canApply} onClick={() => void apply()}>
        {t("rest.apply")}
      </Button>
    </div>
  );
}
