import { useEffect, useState } from "react";
import { useTranslation } from "react-i18next";
import type { RestApplied } from "@/bindings/RestApplied";
import type { RestPlan } from "@/bindings/RestPlan";
import { Button } from "@/components/ui/button";
import { Dialog, DialogContent, DialogDescription, DialogTitle } from "@/components/ui/dialog";
import { FieldBadge } from "@/components/ui/field-badge";
import { Input } from "@/components/ui/input";
import type { CharacterEntry } from "@/hooks/use-characters";
import { errorMessage } from "@/hooks/use-characters";
import { api } from "@/lib/api";

/** Pause after typing before Rust is asked how many days the rest takes. */
const PREVIEW_DELAY_MS = 150;

type RestDialogProps = {
  /** The character who rests; the window is open while it is set. */
  target: CharacterEntry | null;
  onDone: (applied: RestApplied) => void;
  onClose: () => void;
};

/** "Rest": the wanted HP in, days of rest and the resulting HP out (the rules count them). */
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
  const [text, setText] = useState(String(maxHp));
  const [plan, setPlan] = useState<RestPlan | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [busy, setBusy] = useState(false);

  const wanted = text.trim() === "" ? null : Number(text);
  const rangeOk = wanted !== null && Number.isInteger(wanted) && wanted > hp && wanted <= maxHp;

  useEffect(() => {
    if (!rangeOk) {
      setPlan(null);
      return;
    }
    const timer = setTimeout(() => {
      api.restPreview(target.id, wanted).then(
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
  }, [rangeOk, wanted, target.id]);

  const apply = async () => {
    if (!rangeOk) return;
    setBusy(true);
    try {
      onDone(await api.restCharacter(target.id, wanted));
      onClose();
    } catch (e) {
      setError(errorMessage(e));
    } finally {
      setBusy(false);
    }
  };

  const canApply = rangeOk && plan !== null && !busy;

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

      <label className="flex items-center gap-3 text-sm">
        <span className="flex-1">{t("rest.target")}</span>
        <FieldBadge
          error={rangeOk || text.trim() === "" ? null : t("rest.target_range", { min: hp + 1, max: maxHp })}
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
          <p className="flex justify-between">
            <span>{t("rest.days")}</span>
            <span className="num font-semibold">{plan.days}</span>
          </p>
          <p className="flex justify-between">
            <span>{t("rest.result")}</span>
            <span className="num font-semibold">
              {plan.final_hp} / {plan.max_hp}
            </span>
          </p>
          {plan.final_hp > plan.target_hp && (
            <p className="text-xs text-muted-foreground">{t("rest.overshoot")}</p>
          )}
        </div>
      )}
      <p className="text-xs text-muted-foreground">{t("rest.ends_day")}</p>
      {error && <p className="text-sm text-destructive">{error}</p>}

      <Button type="button" className="w-full" disabled={!canApply} onClick={() => void apply()}>
        {t("rest.apply")}
      </Button>
    </div>
  );
}
