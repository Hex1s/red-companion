import { useState } from "react";
import { useTranslation } from "react-i18next";
import type { StabilizeApplied } from "@/bindings/StabilizeApplied";
import { WoundBadge } from "@/components/character/wound-badge";
import { Button } from "@/components/ui/button";
import { Dialog, DialogContent, DialogDescription, DialogTitle } from "@/components/ui/dialog";
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
import { formatCheck, parseFaces, validD10Faces } from "@/lib/dice";

/** Skills that can stabilise a wound (p. 222); ids as in the skill catalog. */
const SKILLS = ["first_aid", "paramedic"] as const;

type StabilizeDialogProps = {
  /** Who is being stabilised; `null` keeps the dialog closed. */
  target: CharacterEntry | null;
  /** Everyone who may help (including the target themselves). */
  helpers: CharacterEntry[];
  onDone: (applied: StabilizeApplied) => void;
  onClose: () => void;
};

/** Stabilisation: helper's TECH + First Aid / Paramedic + d10 against the wound's DV. */
export function StabilizeDialog({ target, helpers, onDone, onClose }: StabilizeDialogProps) {
  return (
    <Dialog open={target !== null} onOpenChange={(open) => !open && onClose()}>
      <DialogContent className="sm:max-w-lg" aria-describedby={undefined}>
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

function StabilizeForm({ target, helpers, onDone, onClose }: StabilizeFormProps) {
  const { t } = useTranslation();
  const [helperId, setHelperId] = useState(String(target.id));
  const [skill, setSkill] = useState<(typeof SKILLS)[number]>("first_aid");
  const [dieText, setDieText] = useState("");
  const [result, setResult] = useState<StabilizeApplied["result"] | null>(null);
  const [error, setError] = useState<string | null>(null);

  const faces = parseFaces(dieText);
  const invalid = faces !== null && !validD10Faces(faces);
  const helper = helpers.find((h) => String(h.id) === helperId);
  const level = helper?.character.skills[skill] ?? 0;

  const roll = async () => {
    if (invalid || !helper) return;
    try {
      const applied = await api.stabilize(target.id, helper.id, skill, faces);
      setResult(applied.result);
      setError(null);
      onDone(applied);
    } catch (e) {
      setError(errorMessage(e));
    }
  };

  return (
    <div className="space-y-4">
      <DialogTitle className="text-xl font-bold">{t("stabilize.title")}</DialogTitle>
      <DialogDescription className="flex flex-wrap items-center gap-3">
        <span className="font-display font-semibold uppercase">{target.character.name}</span>
        <WoundBadge state={target.derived.wound_state} />
      </DialogDescription>

      <div className="flex flex-wrap items-end gap-3">
        <div className="space-y-1">
          <span className="block text-xs text-muted-foreground uppercase">
            {t("stabilize.helper")}
          </span>
          <Select value={helperId} onValueChange={setHelperId}>
            <SelectTrigger className="w-52">
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
        </div>
        <div className="space-y-1">
          <span className="block text-xs text-muted-foreground uppercase">
            {t("stabilize.skill")}
          </span>
          <Select value={skill} onValueChange={(v) => setSkill(v as (typeof SKILLS)[number])}>
            <SelectTrigger className="w-44">
              <SelectValue />
            </SelectTrigger>
            <SelectContent>
              {SKILLS.map((id) => (
                <SelectItem key={id} value={id}>
                  {t(`skill.items.${id}`)}
                </SelectItem>
              ))}
            </SelectContent>
          </Select>
        </div>
        <div className="space-y-1">
          <span className="block text-xs text-muted-foreground uppercase">{t("initiative.die")}</span>
          <Input
            inputMode="numeric"
            value={dieText}
            placeholder={t("initiative.die_random")}
            title={t("initiative.die_hint")}
            aria-invalid={invalid}
            onChange={(e) => setDieText(e.target.value)}
            onKeyDown={(e) => e.key === "Enter" && void roll()}
            className="num w-28"
          />
        </div>
      </div>

      <p className="num text-sm text-muted-foreground">
        {t("stabilize.formula", { tech: helper?.character.stats.tech ?? 0, level })}
      </p>

      {result && (
        <div className="cut space-y-1 p-3 [--cut:8px]">
          <p className="num text-sm">
            {result.bonus} + {formatCheck(result.roll)} = {result.total} / {t("glossary.dv")}{" "}
            {result.dv}
          </p>
          <p
            className={
              result.success ? "font-semibold text-state-ok" : "font-semibold text-state-mortal"
            }
          >
            {t(result.success ? "stabilize.success" : "stabilize.failure")}
            {result.revived && ` ${t("stabilize.revived")}`}
          </p>
        </div>
      )}
      {error && <p className="text-destructive">{error}</p>}

      <div className="flex justify-end gap-2">
        <Button variant="outline" onClick={onClose}>
          {t("common.close")}
        </Button>
        <Button disabled={invalid || !helper} onClick={() => void roll()}>
          {t("stabilize.roll")}
        </Button>
      </div>
    </div>
  );
}
