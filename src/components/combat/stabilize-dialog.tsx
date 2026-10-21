import { useState } from "react";
import { Dices } from "lucide-react";
import { useTranslation } from "react-i18next";
import type { StabilizeApplied } from "@/bindings/StabilizeApplied";
import { Button } from "@/components/ui/button";
import { Dialog, DialogContent, DialogDescription, DialogTitle } from "@/components/ui/dialog";
import { FieldBadge } from "@/components/ui/field-badge";
import { Hint } from "@/components/ui/hint";
import { Input } from "@/components/ui/input";
import {
  Select,
  SelectContent,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from "@/components/ui/select";
import type { CharacterEntry } from "@/hooks/use-characters";
import { errorMessage } from "@/hooks/use-characters";
import { api } from "@/lib/api";

/** The skills that stabilise (p. 222); must match `STABILIZE_SKILLS` in Rust. */
const SKILLS = ["first_aid", "paramedic"] as const;
type StabilizeSkill = (typeof SKILLS)[number];

/** Lowest and highest total of a d10 check: 1 - 10 and 10 + 10 (see `check_total` in Rust). */
const TOTAL_MIN = -9;
const TOTAL_MAX = 20;

type StabilizeDialogProps = {
  /** The character being stabilised; the window is open while it is set. */
  target: CharacterEntry | null;
  /** Everyone who can be the one helping. */
  helpers: CharacterEntry[];
  onDone: (applied: StabilizeApplied) => void;
  onClose: () => void;
};

/** "Stabilize": who helps, roll or declare it done. Closes on success. */
export function StabilizeDialog({ target, helpers, onDone, onClose }: StabilizeDialogProps) {
  return (
    <Dialog open={target !== null} onOpenChange={(open) => !open && onClose()}>
      <DialogContent className="sm:max-w-sm" aria-describedby={undefined}>
        {target && (
          <StabilizeForm
            key={target.id}
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

type StabilizeFormProps = Omit<StabilizeDialogProps, "target"> & { target: CharacterEntry };

/** Someone other than the target if possible (a wounded one rarely patches themselves up). */
function defaultHelper(target: CharacterEntry, helpers: CharacterEntry[]): CharacterEntry {
  return helpers.find((h) => h.id !== target.id && h.derived.wound_state !== "dead") ?? target;
}

function StabilizeForm({ target, helpers, onDone, onClose }: StabilizeFormProps) {
  const { t } = useTranslation();
  const [helperId, setHelperId] = useState(() => String(defaultHelper(target, helpers).id));
  const [skill, setSkill] = useState<StabilizeSkill | null>(null);
  const [totalText, setTotalText] = useState("");
  const [failed, setFailed] = useState<StabilizeApplied["result"] | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [busy, setBusy] = useState(false);

  const dv = target.derived.stabilization_dv;
  const helper = helpers.find((h) => String(h.id) === helperId) ?? target;
  const bonusOf = (id: StabilizeSkill) =>
    helper.character.stats.tech + (helper.character.skills[id] ?? 0);
  // The better of the two skills unless the GM picked one.
  const chosen: StabilizeSkill =
    skill ?? (bonusOf("paramedic") > bonusOf("first_aid") ? "paramedic" : "first_aid");

  const total = totalText.trim() === "" ? null : Number(totalText);
  const totalInvalid =
    total !== null && (!Number.isInteger(total) || total < TOTAL_MIN || total > TOTAL_MAX);

  const run = async (manualTotal: number | null, granted: boolean) => {
    setBusy(true);
    try {
      const applied = await api.stabilize(target.id, helper.id, chosen, manualTotal, granted);
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

  return (
    <div className="space-y-4">
      <div className="space-y-1">
        <DialogTitle className="text-xl font-bold">{t("stabilize.title")}</DialogTitle>
        <DialogDescription className="flex items-center gap-3">
          <span className="font-display font-semibold text-foreground uppercase">
            {target.character.name}
          </span>
          {dv !== null && (
            <span className="num">
              {t("stabilize_dialog.dv")} {dv}
            </span>
          )}
        </DialogDescription>
      </div>

      <div className="flex flex-wrap items-center gap-2">
        <Select value={helperId} onValueChange={setHelperId}>
          <SelectTrigger className="w-44" aria-label={t("stabilize.helper")}>
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
        {SKILLS.map((id) => (
          <Hint
            key={id}
            label={t("stabilize.formula_short", {
              tech: helper.character.stats.tech,
              level: helper.character.skills[id] ?? 0,
            })}
          >
            <Button
              type="button"
              size="sm"
              variant={chosen === id ? "default" : "outline"}
              onClick={() => setSkill(id)}
            >
              {t(`skill.items.${id}`)}
              <span className="num">+{bonusOf(id)}</span>
            </Button>
          </Hint>
        ))}
      </div>

      <div className="flex items-center gap-2">
        <Button type="button" disabled={busy} onClick={() => void run(null, false)}>
          <Dices />
          {t("stabilize.roll")}
        </Button>
        <Hint label={t("stabilize_dialog.total_hint")}>
          <FieldBadge
            error={totalInvalid ? t("validation.check_total") : null}
            className="w-20"
          >
            {(invalid) => (
              <Input
                inputMode="numeric"
                value={totalText}
                placeholder="d10"
                aria-label={t("stabilize_dialog.total_label")}
                aria-invalid={invalid}
                onChange={(e) => setTotalText(e.target.value)}
                onKeyDown={(e) => {
                  if (e.key === "Enter" && total !== null && !totalInvalid && !busy) {
                    void run(total, false);
                  }
                }}
                className="num w-20"
              />
            )}
          </FieldBadge>
        </Hint>
      </div>

      {failed && (
        <p className="num text-sm text-state-mortal">
          {failed.total} / {t("stabilize_dialog.dv")} {failed.dv} — {t("stabilize_dialog.failed")}
        </p>
      )}
      {error && <p className="text-sm text-destructive">{error}</p>}

      <Hint label={t("stabilize_dialog.declare_hint")}>
        <Button
          type="button"
          variant="outline"
          className="w-full"
          disabled={busy}
          onClick={() => void run(null, true)}
        >
          {t("stabilize.title")}
        </Button>
      </Hint>
    </div>
  );
}
