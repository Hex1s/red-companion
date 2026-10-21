import { useEffect, useMemo, useState } from "react";
import { useTranslation } from "react-i18next";
import type { AimedTarget } from "@/bindings/AimedTarget";
import type { ArmorMode } from "@/bindings/ArmorMode";
import type { BodyArea } from "@/bindings/BodyArea";
import type { CriticalInjury } from "@/bindings/CriticalInjury";
import type { DamageApplied } from "@/bindings/DamageApplied";
import type { DamageEvent } from "@/bindings/DamageEvent";
import type { DamageOutcome } from "@/bindings/DamageOutcome";
import type { DamageRequest } from "@/bindings/DamageRequest";
import type { DamageStep } from "@/bindings/DamageStep";
import type { InjuryRoll } from "@/bindings/InjuryRoll";
import type { RollResult } from "@/bindings/RollResult";
import type { Scene } from "@/bindings/Scene";
import { HpBar } from "@/components/character/hp-bar";
import { WoundBadge } from "@/components/character/wound-badge";
import { Button } from "@/components/ui/button";
import { Checkbox } from "@/components/ui/checkbox";
import { Dialog, DialogContent, DialogDescription, DialogTitle } from "@/components/ui/dialog";
import { Input } from "@/components/ui/input";
import {
  Select,
  SelectContent,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from "@/components/ui/select";
import { Tabs, TabsList, TabsTrigger } from "@/components/ui/tabs";
import type { CharacterEntry } from "@/hooks/use-characters";
import { errorMessage } from "@/hooks/use-characters";
import { api } from "@/lib/api";
import { cn } from "@/lib/utils";

type HitSpot = "body" | "head" | "head_aimed" | "held_item" | "leg";
type InputMode = "number" | "dice";

const SPOTS: Record<HitSpot, { area: BodyArea; aimed: AimedTarget | null }> = {
  body: { area: "body", aimed: null },
  head: { area: "head", aimed: null },
  head_aimed: { area: "head", aimed: "head" },
  held_item: { area: "body", aimed: "held_item" },
  leg: { area: "body", aimed: "leg" },
};
const ARMOR_MODES: ArmorMode[] = ["full", "half_up", "ignored"];
const NO_COVER = "none";
const DEFAULT_NOTATION = "5d6";

/** Whole number >= 0 from an input; `null` for empty or invalid text. */
function parseCount(text: string): number | null {
  const n = Number(text);
  return text.trim() !== "" && Number.isInteger(n) && n >= 0 ? n : null;
}

/** Faces typed as "3 6 6" or "3,6,6"; `null` for empty text, `NaN` entries for junk. */
function parseFaces(text: string): number[] | null {
  const parts = text.split(/[\s,]+/).filter(Boolean);
  return parts.length === 0 ? null : parts.map(Number);
}

type DamageDialogProps = {
  /** The character taking damage; the window is open while it is set. */
  target: CharacterEntry | null;
  scene: Scene;
  injuries: CriticalInjury[];
  onApplied: (result: DamageApplied) => void;
  onUndo: () => Promise<void>;
  onClose: () => void;
};

/** "Damage taken": enter a hit, see why it did what it did, apply or undo. */
export function DamageDialog({
  target,
  scene,
  injuries,
  onApplied,
  onUndo,
  onClose,
}: DamageDialogProps) {
  const { t } = useTranslation();
  return (
    <Dialog open={target !== null} onOpenChange={(open) => !open && onClose()}>
      <DialogContent className="p-0 sm:max-w-4xl" aria-describedby={undefined}>
        {target && (
          <DamageForm
            key={target.id}
            target={target}
            scene={scene}
            injuries={injuries}
            onApplied={onApplied}
            onUndo={onUndo}
            onClose={onClose}
            title={t("combat.damage")}
          />
        )}
      </DialogContent>
    </Dialog>
  );
}

type DamageFormProps = Omit<DamageDialogProps, "target"> & {
  target: CharacterEntry;
  title: string;
};

function DamageForm({ target, scene, injuries, onApplied, onUndo, onClose, title }: DamageFormProps) {
  const { t } = useTranslation();
  const characterId = target.id;
  const owned = target.character.injuries;

  const [mode, setMode] = useState<InputMode>("number");
  const [valueText, setValueText] = useState("");
  const [manualCritical, setManualCritical] = useState(false);
  const [notation, setNotation] = useState(DEFAULT_NOTATION);
  const [facesText, setFacesText] = useState("");
  const [roll, setRoll] = useState<RollResult | null>(null);
  const [spot, setSpot] = useState<HitSpot>("body");
  const [armorMode, setArmorMode] = useState<ArmorMode>("full");
  const [explosive, setExplosive] = useState(false);
  const [coverId, setCoverId] = useState<string>(
    () => String(scene.covers.find((c) => c.characters.includes(characterId))?.id ?? NO_COVER),
  );
  const [injuryId, setInjuryId] = useState<string | null>(null);
  const [injuryRoll, setInjuryRoll] = useState<InjuryRoll | null>(null);
  const [injuryFaces, setInjuryFaces] = useState("");
  const [preview, setPreview] = useState<DamageOutcome | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [busy, setBusy] = useState(false);
  const [applied, setApplied] = useState<DamageEvent | null>(null);

  const { area, aimed } = SPOTS[spot];

  const request = useMemo<DamageRequest | null>(() => {
    let damage: DamageRequest["damage"];
    if (mode === "number") {
      const value = parseCount(valueText);
      if (value === null) return null;
      damage = { kind: "total", value };
    } else {
      if (roll === null) return null;
      damage = { kind: "rolled", roll };
    }
    return {
      damage,
      area,
      aimed,
      armor_mode: armorMode,
      explosive,
      manual_critical: mode === "number" && manualCritical,
      cover_id: coverId === NO_COVER ? null : Number(coverId),
    };
  }, [mode, valueText, roll, area, aimed, armorMode, explosive, manualCritical, coverId]);

  // The backend recomputes the hit on every change: the UI never does the maths.
  useEffect(() => {
    if (request === null) {
      setPreview(null);
      return;
    }
    let live = true;
    api
      .previewDamage(characterId, request)
      .then((outcome) => {
        if (!live) return;
        setPreview(outcome);
        setError(null);
      })
      .catch((e: unknown) => {
        if (!live) return;
        setPreview(null);
        setError(errorMessage(e));
      });
    return () => {
      live = false;
    };
  }, [characterId, request]);

  const critical = preview?.critical ?? false;
  const freeInjuries = injuries.filter(
    (i) => i.area === area && !owned.includes(i.id) && i.id !== preview?.forced_injury,
  );
  const chosenInjury = critical ? (injuries.find((i) => i.id === injuryId) ?? null) : null;
  const needsInjury = critical && chosenInjury === null && freeInjuries.length > 0;
  const canApply = preview !== null && !busy && !needsInjury;

  const guard = async (action: () => Promise<void>) => {
    setBusy(true);
    try {
      await action();
      setError(null);
    } catch (e) {
      setError(errorMessage(e));
    } finally {
      setBusy(false);
    }
  };

  const rollDamage = () =>
    guard(async () => {
      const faces = parseFaces(facesText);
      setRoll(await api.rollDice(notation, faces));
    });

  const rollInjury = (manual: number[] | null) =>
    guard(async () => {
      const result = await api.rollInjury(characterId, area, manual);
      setInjuryRoll(result);
      setInjuryId(result.injury_id);
    });

  const pickInjury = (id: string) => {
    setInjuryId(id);
    setInjuryRoll(null);
  };

  const apply = () =>
    guard(async () => {
      if (request === null) return;
      const result = await api.applyDamage(characterId, request, chosenInjury?.id ?? null);
      setApplied(result.scene.scene.last_damage);
      onApplied(result);
    });

  const undo = () =>
    guard(async () => {
      await onUndo();
      onClose();
    });

  const resetInjury = () => {
    setInjuryId(null);
    setInjuryRoll(null);
    setInjuryFaces("");
  };

  const stateLabel = (state: DamageOutcome["wound_after"]) => t(`glossary.wound.${state}`);

  return (
    <div className="max-h-[88vh] space-y-4 overflow-y-auto p-6">
      <div className="space-y-1">
        <DialogTitle className="text-xl font-bold">{title}</DialogTitle>
        <DialogDescription className="flex flex-wrap items-center gap-3">
          <span className="font-display text-base font-semibold text-foreground uppercase">
            {target.character.name}
          </span>
          <span className="w-48">
            <HpBar current={target.character.hp} max={target.derived.max_hp} />
          </span>
          <WoundBadge state={target.derived.wound_state} />
        </DialogDescription>
      </div>

      <div className="grid gap-6 md:grid-cols-2">
        <fieldset disabled={applied !== null} className="space-y-4">
          <Tabs value={mode} onValueChange={(v) => setMode(v as InputMode)}>
            <TabsList>
              <TabsTrigger value="number">{t("damage.mode.number")}</TabsTrigger>
              <TabsTrigger value="dice">{t("damage.mode.dice")}</TabsTrigger>
            </TabsList>
          </Tabs>

          {mode === "number" ? (
            <div className="space-y-2">
              <Field label={t("damage.amount")}>
                <Input
                  autoFocus
                  inputMode="numeric"
                  value={valueText}
                  onChange={(e) => setValueText(e.target.value)}
                  aria-invalid={valueText.trim() !== "" && parseCount(valueText) === null}
                  className="num w-28"
                />
              </Field>
              <CheckRow
                label={t("damage.manual_critical")}
                checked={manualCritical}
                onChange={setManualCritical}
              />
            </div>
          ) : (
            <div className="space-y-2">
              <div className="flex flex-wrap items-end gap-2">
                <Field label={t("damage.notation")}>
                  <Input
                    value={notation}
                    onChange={(e) => {
                      setNotation(e.target.value);
                      setRoll(null);
                    }}
                    className="num w-28"
                  />
                </Field>
                <Field label={t("damage.manual_faces")}>
                  <Input
                    value={facesText}
                    placeholder="3 6 6 2 1"
                    onChange={(e) => setFacesText(e.target.value)}
                    className="num w-40"
                  />
                </Field>
                <Button disabled={busy} onClick={rollDamage}>
                  {facesText.trim() ? t("damage.accept") : t("common.roll")}
                </Button>
              </div>
              {roll && <DiceResult roll={roll} />}
            </div>
          )}

          <Field label={t("damage.spot.label")}>
            <Select
              value={spot}
              onValueChange={(v) => {
                setSpot(v as HitSpot);
                resetInjury();
              }}
            >
              <SelectTrigger className="w-full">
                <SelectValue />
              </SelectTrigger>
              <SelectContent>
                {(Object.keys(SPOTS) as HitSpot[]).map((s) => (
                  <SelectItem key={s} value={s}>
                    {t(`damage.spot.${s}`)}
                  </SelectItem>
                ))}
              </SelectContent>
            </Select>
          </Field>

          <Field label={t("glossary.armor_mode.label")}>
            <Select value={armorMode} onValueChange={(v) => setArmorMode(v as ArmorMode)}>
              <SelectTrigger className="w-full">
                <SelectValue />
              </SelectTrigger>
              <SelectContent>
                {ARMOR_MODES.map((m) => (
                  <SelectItem key={m} value={m}>
                    {t(`damage.armor_mode.${m}`)}
                  </SelectItem>
                ))}
              </SelectContent>
            </Select>
          </Field>

          <Field label={t("glossary.cover")}>
            <Select value={coverId} onValueChange={setCoverId}>
              <SelectTrigger className="w-full">
                <SelectValue />
              </SelectTrigger>
              <SelectContent>
                <SelectItem value={NO_COVER}>{t("damage.no_cover")}</SelectItem>
                {scene.covers.map((c) => (
                  <SelectItem key={c.id} value={String(c.id)}>
                    {c.name || t("scene.custom")} · {c.hp}/{c.max_hp}
                  </SelectItem>
                ))}
              </SelectContent>
            </Select>
          </Field>
          {coverId !== NO_COVER && (
            <CheckRow label={t("damage.explosive")} checked={explosive} onChange={setExplosive} />
          )}
        </fieldset>

        <div className="space-y-4">
          {preview ? (
            <Outcome outcome={preview} target={target} stateLabel={stateLabel} />
          ) : (
            <p className="text-muted-foreground">{t("damage.enter_hint")}</p>
          )}

          {critical && !applied && (
            <section className="cut space-y-3 p-3 [--cut:8px] [--cut-border:var(--primary)]">
              <h4 className="text-sm font-semibold text-primary">{t("glossary.critical_injury")}</h4>
              <div className="flex flex-wrap items-end gap-2">
                <Button size="xs" disabled={busy} onClick={() => rollInjury(null)}>
                  {t("damage.injury_roll")}
                </Button>
                <Field label={t("damage.injury_manual")}>
                  <Input
                    value={injuryFaces}
                    placeholder="3 4"
                    onChange={(e) => setInjuryFaces(e.target.value)}
                    className="num w-24"
                  />
                </Field>
                <Button
                  size="xs"
                  variant="outline"
                  disabled={busy || parseFaces(injuryFaces) === null}
                  onClick={() => rollInjury(parseFaces(injuryFaces))}
                >
                  {t("damage.accept")}
                </Button>
              </div>
              {injuryRoll && <DiceResult roll={injuryRoll.roll} />}
              <Select value={injuryId ?? undefined} onValueChange={pickInjury}>
                <SelectTrigger className="w-full">
                  <SelectValue placeholder={t("damage.injury_pick")} />
                </SelectTrigger>
                <SelectContent>
                  {freeInjuries.map((i) => (
                    <SelectItem key={i.id} value={i.id}>
                      <span className="num">{i.roll}</span> · {t(i.name_key)}
                    </SelectItem>
                  ))}
                </SelectContent>
              </Select>
              {freeInjuries.length === 0 && (
                <p className="text-sm text-muted-foreground">{t("damage.no_free_injury")}</p>
              )}
              {chosenInjury && (
                <p className="text-sm">
                  <span className="font-semibold">{t(chosenInjury.name_key)}.</span>{" "}
                  {t(chosenInjury.effect_key)}
                </p>
              )}
            </section>
          )}

          {error && <p className="text-sm text-destructive">{error}</p>}
        </div>
      </div>

      <div className="flex justify-end gap-2">
        {applied ? (
          <>
            <Button variant="outline" disabled={busy} onClick={undo}>
              {t("damage.undo")}
            </Button>
            <Button onClick={onClose}>{t("damage.close")}</Button>
          </>
        ) : (
          <>
            <Button variant="outline" onClick={onClose}>
              {t("characters.cancel")}
            </Button>
            <Button disabled={!canApply} onClick={apply}>
              {t("damage.apply")}
            </Button>
          </>
        )}
      </div>
    </div>
  );
}

type FieldProps = { label: string; children: React.ReactNode };

function Field({ label, children }: FieldProps) {
  return (
    <label className="block space-y-1">
      <span className="block text-xs text-muted-foreground uppercase">{label}</span>
      {children}
    </label>
  );
}

type CheckRowProps = { label: string; checked: boolean; onChange: (checked: boolean) => void };

function CheckRow({ label, checked, onChange }: CheckRowProps) {
  return (
    <label className="flex items-center gap-2 text-sm">
      <Checkbox checked={checked} onCheckedChange={(v) => onChange(v === true)} />
      {label}
    </label>
  );
}

/** Dice faces of a roll; maximum faces (the sixes that make a critical) are red. */
function DiceResult({ roll }: { roll: RollResult }) {
  const { t } = useTranslation();
  return (
    <div className="num flex flex-wrap items-center gap-1 text-sm">
      {roll.dice.map((face, i) => (
        <span
          key={i}
          className={cn(
            "cut flex h-7 w-7 items-center justify-center [--cut:4px]",
            face === roll.notation.sides && roll.notation.sides === 6 && "[--cut-border:var(--primary)] text-primary",
          )}
        >
          {face}
        </span>
      ))}
      <span className="ml-2 text-muted-foreground">
        = {roll.total}
        {roll.source === "manual" && ` (${t("damage.manual")})`}
      </span>
    </div>
  );
}

type OutcomeProps = {
  outcome: DamageOutcome;
  target: CharacterEntry;
  stateLabel: (state: DamageOutcome["wound_after"]) => string;
};

/** Result of the hit: numbers on top, the step log below. */
function Outcome({ outcome, target, stateLabel }: OutcomeProps) {
  const { t } = useTranslation();
  const changed = outcome.wound_after !== outcome.wound_before;
  return (
    <section className="space-y-3">
      <div className="cut space-y-2 p-3 [--cut:8px]">
        <div className="flex items-baseline justify-between gap-3">
          <span className="text-xs text-muted-foreground uppercase">{t("damage.result")}</span>
          <span className="num text-2xl font-bold text-primary">−{outcome.hp_lost}</span>
        </div>
        <HpBar current={outcome.new_hp} max={target.derived.max_hp} />
        {changed && (
          <p className="text-sm">
            {stateLabel(outcome.wound_before)} → {stateLabel(outcome.wound_after)}
          </p>
        )}
        {outcome.cover_hp_after !== null && (
          <p className="num text-xs text-muted-foreground">
            {t("glossary.cover")}: {outcome.cover_hp_after} {t("scene.hp")}
          </p>
        )}
      </div>
      <div>
        <h4 className="mb-1 text-xs text-muted-foreground">{t("damage.log")}</h4>
        <ol className="num space-y-1 text-xs">
          {outcome.steps.map((step, i) => (
            <li key={i}>{stepText(step, t, stateLabel)}</li>
          ))}
        </ol>
      </div>
    </section>
  );
}

type Translate = ReturnType<typeof useTranslation>["t"];

/** One log line in words. The backend sends numbers only. */
function stepText(
  step: DamageStep,
  t: Translate,
  stateLabel: (state: DamageOutcome["wound_after"]) => string,
): string {
  switch (step.kind) {
    case "damage":
      return t(step.critical_dice ? "damage.step.damage_critical" : "damage.step.damage", step);
    case "cover_absorbed":
      return t(
        step.destroyed ? "damage.step.cover_absorbed_destroyed" : "damage.step.cover_absorbed",
        step,
      );
    case "armor":
      return t(`damage.step.armor_${step.mode}`, step);
    case "wound_state_changed":
      return t("damage.step.wound_state_changed", {
        from: stateLabel(step.from),
        to: stateLabel(step.to),
      });
    default:
      return t(`damage.step.${step.kind}`, step);
  }
}
