import { useState } from "react";
import { useTranslation } from "react-i18next";
import type { HumanityLossApplied } from "@/bindings/HumanityLossApplied";
import type { RollResult } from "@/bindings/RollResult";
import { DiceButton } from "@/components/combat/dice-button";
import { Button } from "@/components/ui/button";
import { ErrorNotice } from "@/components/ui/error-notice";
import { Dialog, DialogContent, DialogDescription, DialogTitle } from "@/components/ui/dialog";
import { FieldBadge } from "@/components/ui/field-badge";
import { Input } from "@/components/ui/input";
import type { CharacterEntry } from "@/hooks/use-characters";
import { errorMessage } from "@/lib/errors";
import { api } from "@/lib/api";

/** Largest loss accepted in one go: well past a whole character's humanity. */
const LOSS_MAX = 999;

type LoseHumanityDialogProps = {
  /** The character who loses humanity; the window is open while it is set. */
  target: CharacterEntry | null;
  onDone: (applied: HumanityLossApplied) => void;
  onClose: () => void;
};

/** "Humanity loss": a number the GM decided on, or a roll (2d6 for an implant and so on). */
export function LoseHumanityDialog({ target, onDone, onClose }: LoseHumanityDialogProps) {
  return (
    <Dialog open={target !== null} onOpenChange={(open) => !open && onClose()}>
      <DialogContent className="sm:max-w-sm" aria-describedby={undefined}>
        {target && <LossForm key={target.id} target={target} onDone={onDone} onClose={onClose} />}
      </DialogContent>
    </Dialog>
  );
}

type LossFormProps = Omit<LoseHumanityDialogProps, "target"> & { target: CharacterEntry };

function LossForm({ target, onDone, onClose }: LossFormProps) {
  const { t } = useTranslation();
  const { derived, character } = target;
  const [text, setText] = useState("");
  const [notation, setNotation] = useState("2d6");
  const [roll, setRoll] = useState<RollResult | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [busy, setBusy] = useState(false);

  const loss = text.trim() === "" ? null : Number(text);
  const lossOk = loss !== null && Number.isInteger(loss) && loss >= 1 && loss <= LOSS_MAX;

  const rollLoss = async (dice: string, manual: number[] | null) => {
    setBusy(true);
    try {
      const rolled = await api.rollDice(dice, manual);
      setRoll(rolled);
      setText(String(rolled.total));
      setError(null);
    } catch (e) {
      setError(errorMessage(e));
    } finally {
      setBusy(false);
    }
  };

  const apply = async () => {
    if (!lossOk) return;
    setBusy(true);
    try {
      onDone(await api.loseHumanity(target.id, loss));
      onClose();
    } catch (e) {
      setError(errorMessage(e));
    } finally {
      setBusy(false);
    }
  };

  const after = derived.humanity - (lossOk ? loss : 0);

  return (
    <div className="space-y-4">
      <div className="space-y-1">
        <DialogTitle className="text-xl font-bold">{t("humanity.loss_title")}</DialogTitle>
        <DialogDescription className="flex items-center gap-3">
          <span className="font-display font-semibold text-foreground uppercase">
            {character.name}
          </span>
          <span className="num">
            {t("glossary.humanity")} {derived.humanity} / {derived.max_humanity}
          </span>
        </DialogDescription>
      </div>

      <div className="flex items-center gap-2">
        <label className="flex flex-1 items-center gap-3 text-sm">
          <span className="flex-1">{t("humanity.loss_amount")}</span>
          <FieldBadge
            error={lossOk || text.trim() === "" ? null : t("humanity.loss_range", { max: LOSS_MAX })}
            className="w-24"
          >
            {(invalid) => (
              <Input
                autoFocus
                inputMode="numeric"
                value={text}
                aria-invalid={invalid}
                onChange={(e) => {
                  setText(e.target.value);
                  setRoll(null);
                }}
                onKeyDown={(e) => {
                  if (e.key === "Enter" && lossOk && !busy) void apply();
                }}
                className="num w-24"
              />
            )}
          </FieldBadge>
        </label>
        <DiceButton
          notation={notation}
          onNotationChange={setNotation}
          disabled={busy}
          tooltip={t("humanity.loss_roll", { notation })}
          onRoll={(dice, manual) => void rollLoss(dice, manual)}
        />
      </div>
      {roll && (
        <p className="num text-sm text-muted-foreground">
          {roll.notation.count}d{roll.notation.sides}: {roll.dice.join(" + ")} = {roll.total}
        </p>
      )}

      {lossOk && (
        <p className="num text-sm">
          {t("humanity.after", { from: derived.humanity, to: after })}
          {after < 0 && <span className="ml-2 text-state-mortal-text">{t("humanity.cyberpsychosis")}</span>}
        </p>
      )}
      <ErrorNotice message={error} />

      <Button type="button" className="w-full" disabled={!lossOk || busy} onClick={() => void apply()}>
        {t("humanity.loss_apply")}
      </Button>
    </div>
  );
}
