import { useEffect, useState } from "react";
import { Dices } from "lucide-react";
import { useTranslation } from "react-i18next";
import type { TherapyApplied } from "@/bindings/TherapyApplied";
import type { TherapyInfo } from "@/bindings/TherapyInfo";
import type { TherapyKind } from "@/bindings/TherapyKind";
import { Button } from "@/components/ui/button";
import { ErrorNotice } from "@/components/ui/error-notice";
import { Dialog, DialogContent, DialogDescription, DialogTitle } from "@/components/ui/dialog";
import { FieldBadge } from "@/components/ui/field-badge";
import { Hint } from "@/components/ui/hint";
import { Input } from "@/components/ui/input";
import type { CharacterEntry } from "@/hooks/use-characters";
import { errorMessage } from "@/lib/errors";
import { api } from "@/lib/api";
import { parseFaces } from "@/lib/dice";

/** Lowest and highest total of a d10 check: 1 - 10 and 10 + 10 (see `check_total` in Rust). */
const TOTAL_MIN = -9;
const TOTAL_MAX = 20;

type TherapyDialogProps = {
  /** The character in therapy; the window is open while it is set. */
  target: CharacterEntry | null;
  onDone: (applied: TherapyApplied) => void;
  onClose: () => void;
};

/** "Therapy": a week with a Medtech, a check against the DV, and humanity back on success. */
export function TherapyDialog({ target, onDone, onClose }: TherapyDialogProps) {
  return (
    <Dialog open={target !== null} onOpenChange={(open) => !open && onClose()}>
      <DialogContent className="sm:max-w-md" aria-describedby={undefined}>
        {target && <TherapyForm key={target.id} target={target} onDone={onDone} onClose={onClose} />}
      </DialogContent>
    </Dialog>
  );
}

type TherapyFormProps = Omit<TherapyDialogProps, "target"> & { target: CharacterEntry };

function TherapyForm({ target, onDone, onClose }: TherapyFormProps) {
  const { t } = useTranslation();
  const { derived, character } = target;
  const [therapies, setTherapies] = useState<TherapyInfo[]>([]);
  const [kind, setKind] = useState<TherapyKind>("standard");
  const [bonusText, setBonusText] = useState("");
  const [totalText, setTotalText] = useState("");
  const [facesText, setFacesText] = useState("");
  const [failed, setFailed] = useState<TherapyApplied["result"] | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [busy, setBusy] = useState(false);

  useEffect(() => {
    api.listTherapies().then(setTherapies, (e: unknown) => setError(errorMessage(e)));
  }, []);

  const info = therapies.find((x) => x.kind === kind) ?? null;
  const diceCount = info ? Number(info.dice.split("d")[0]) : 0;

  const bonus = bonusText.trim() === "" ? 0 : Number(bonusText);
  const bonusInvalid = !Number.isInteger(bonus);
  const total = totalText.trim() === "" ? null : Number(totalText);
  const totalInvalid =
    total !== null && (!Number.isInteger(total) || total < TOTAL_MIN || total > TOTAL_MAX);
  const faces = parseFaces(facesText);
  const facesInvalid =
    faces !== null &&
    (faces.length !== diceCount || faces.some((f) => !Number.isInteger(f) || f < 1 || f > 6));
  const full = derived.humanity >= derived.max_humanity;
  const canRun = info !== null && !busy && !bonusInvalid && !facesInvalid && !full;

  const run = async (manualTotal: number | null, granted: boolean) => {
    setBusy(true);
    try {
      const applied = await api.therapy(target.id, kind, bonus, manualTotal, granted, faces);
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
        <DialogTitle className="text-xl font-bold">{t("humanity.therapy_title")}</DialogTitle>
        <DialogDescription className="flex items-center gap-3">
          <span className="font-display font-semibold text-foreground uppercase">
            {character.name}
          </span>
          <span className="num">
            {t("glossary.humanity")} {derived.humanity} / {derived.max_humanity}
          </span>
        </DialogDescription>
      </div>

      <div className="flex gap-2">
        {therapies.map((x) => (
          <Button
            key={x.kind}
            type="button"
            size="sm"
            variant={kind === x.kind ? "default" : "outline"}
            onClick={() => {
              setKind(x.kind);
              setFacesText("");
              setFailed(null);
            }}
          >
            {t(`humanity.kind.${x.kind}`)}
          </Button>
        ))}
      </div>

      {info && (
        <div className="space-y-1 text-sm">
          <p className="flex justify-between">
            <span>{t("humanity.dv")}</span>
            <span className="num font-semibold">{info.dv}</span>
          </p>
          <p className="flex justify-between">
            <span>{t("humanity.restores")}</span>
            <span className="num font-semibold">{info.dice}</span>
          </p>
          <p className="flex justify-between">
            <span>{t("humanity.price")}</span>
            <span className="num font-semibold">
              {info.price_eb} {t("humanity.eb")}
            </span>
          </p>
          <p className="flex justify-between">
            <span>{t("humanity.materials")}</span>
            <span className="num font-semibold">
              {info.materials_eb} {t("humanity.eb")}
            </span>
          </p>
          <p className="text-xs text-muted-foreground">{t("humanity.therapy_note")}</p>
        </div>
      )}

      <label className="flex items-center gap-3 text-sm">
        <span className="flex-1">{t("humanity.bonus")}</span>
        <Hint label={t("humanity.bonus_hint")}>
          <FieldBadge error={bonusInvalid ? t("validation.check_total") : null} className="w-20">
            {(invalid) => (
              <Input
                inputMode="numeric"
                value={bonusText}
                placeholder="0"
                aria-invalid={invalid}
                onChange={(e) => setBonusText(e.target.value)}
                className="num w-20"
              />
            )}
          </FieldBadge>
        </Hint>
      </label>

      <div className="flex items-center gap-2">
        <Button type="button" disabled={!canRun} onClick={() => void run(null, false)}>
          <Dices />
          {t("humanity.roll")}
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
                  if (e.key === "Enter" && total !== null && !totalInvalid && canRun) {
                    void run(total, false);
                  }
                }}
                className="num w-20"
              />
            )}
          </FieldBadge>
        </Hint>
        <Hint label={t("humanity.faces_hint", { dice: info?.dice ?? "" })}>
          <FieldBadge
            error={facesInvalid ? t("humanity.faces_invalid", { dice: info?.dice ?? "" }) : null}
            className="w-28"
          >
            {(invalid) => (
              <Input
                value={facesText}
                placeholder={info?.dice ?? ""}
                aria-label={t("humanity.faces_label")}
                aria-invalid={invalid}
                onChange={(e) => setFacesText(e.target.value)}
                className="num w-28"
              />
            )}
          </FieldBadge>
        </Hint>
      </div>

      {full && <p className="text-sm text-muted-foreground">{t("humanity.already_full")}</p>}
      {failed && failed.check && (
        <p className="num text-sm text-state-mortal">
          {t("heal.result", {
            bonus: failed.bonus,
            die: failed.check.total,
            total: failed.total,
            dv: failed.dv,
          })}{" "}
          — {t("humanity.failed")}
        </p>
      )}
      <ErrorNotice message={error} />

      <Hint label={t("humanity.declare_hint")}>
        <Button
          type="button"
          variant="outline"
          className="w-full"
          disabled={!canRun}
          onClick={() => void run(null, true)}
        >
          {t("humanity.declare")}
        </Button>
      </Hint>
    </div>
  );
}
