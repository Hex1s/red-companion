import { useEffect, useMemo, useState } from "react";
import { Shield } from "lucide-react";
import { useTranslation } from "react-i18next";
import type { AimedTarget } from "@/bindings/AimedTarget";
import type { ArmorMode } from "@/bindings/ArmorMode";
import type { BodyArea } from "@/bindings/BodyArea";
import type { CriticalInjury } from "@/bindings/CriticalInjury";
import type { DamageApplied } from "@/bindings/DamageApplied";
import type { DamageOutcome } from "@/bindings/DamageOutcome";
import type { DamageRequest } from "@/bindings/DamageRequest";
import type { DeflectionInfo } from "@/bindings/DeflectionInfo";
import type { RollResult } from "@/bindings/RollResult";
import { DiceButton } from "@/components/combat/dice-button";
import { InjuryIcon } from "@/components/combat/injury-icons";
import { Button } from "@/components/ui/button";
import { Checkbox } from "@/components/ui/checkbox";
import { Dialog, DialogContent, DialogDescription, DialogTitle } from "@/components/ui/dialog";
import { FieldBadge } from "@/components/ui/field-badge";
import { Hint } from "@/components/ui/hint";
import { Input } from "@/components/ui/input";
import type { CharacterEntry } from "@/hooks/use-characters";
import { errorMessage } from "@/hooks/use-characters";
import { api } from "@/lib/api";
import { cn } from "@/lib/utils";

const ARMOR_MODES: ArmorMode[] = ["full", "half_up", "ignored"];
const AIMS: Record<BodyArea, AimedTarget[]> = {
  body: ["held_item", "leg"],
  head: ["head"],
};
const DEFAULT_NOTATION = "5d6";

/** Whole number >= 0 from an input; `null` for empty or invalid text. */
function parseCount(text: string): number | null {
  const n = Number(text);
  return text.trim() !== "" && Number.isInteger(n) && n >= 0 ? n : null;
}

type DamageDialogProps = {
  /** The character taking damage; the window is open while it is set. */
  target: CharacterEntry | null;
  /** Cover the target stands behind (it takes the hit first), if any. */
  cover: { id: number; label: string } | null;
  injuries: CriticalInjury[];
  onApplied: (result: DamageApplied) => void;
  onClose: () => void;
};

/** "Damage taken": the fastest way to enter a hit. The result shows up on the session rail. */
export function DamageDialog({ target, cover, injuries, onApplied, onClose }: DamageDialogProps) {
  return (
    <Dialog open={target !== null} onOpenChange={(open) => !open && onClose()}>
      <DialogContent className="sm:max-w-md" aria-describedby={undefined}>
        {target && (
          <DamageForm
            key={target.id}
            target={target}
            cover={cover}
            injuries={injuries}
            onApplied={onApplied}
            onClose={onClose}
          />
        )}
      </DialogContent>
    </Dialog>
  );
}

type DamageFormProps = Omit<DamageDialogProps, "target"> & { target: CharacterEntry };

function DamageForm({ target, cover, injuries, onApplied, onClose }: DamageFormProps) {
  const { t } = useTranslation();
  const characterId = target.id;
  const owned = target.character.injuries;

  const [valueText, setValueText] = useState("");
  const [roll, setRoll] = useState<RollResult | null>(null);
  const [notation, setNotation] = useState(DEFAULT_NOTATION);
  const [area, setArea] = useState<BodyArea>("body");
  const [aimed, setAimed] = useState<AimedTarget | null>(null);
  const [armorMode, setArmorMode] = useState<ArmorMode>("full");
  const [explosive, setExplosive] = useState(false);
  const [manualCritical, setManualCritical] = useState(false);
  const [deflect, setDeflect] = useState(false);
  const [deflection, setDeflection] = useState<DeflectionInfo | null>(null);
  const [injuryId, setInjuryId] = useState<string | null>(null);
  const [injuryRoll, setInjuryRoll] = useState<RollResult | null>(null);
  const [preview, setPreview] = useState<DamageOutcome | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [busy, setBusy] = useState(false);

  const request = useMemo<DamageRequest | null>(() => {
    let damage: DamageRequest["damage"];
    if (roll !== null) {
      damage = { kind: "rolled", roll };
    } else {
      const value = parseCount(valueText);
      if (value === null) return null;
      damage = { kind: "total", value };
    }
    return {
      damage,
      area,
      aimed,
      armor_mode: armorMode,
      explosive,
      manual_critical: roll === null && manualCritical,
      cover_id: cover?.id ?? null,
      deflect: deflect && (deflection?.amount ?? 0) > 0,
    };
  }, [roll, valueText, area, aimed, armorMode, explosive, manualCritical, cover, deflect, deflection]);

  // Solo's Damage Deflection: ticked for the first hit of the round in a running fight; outside
  // a fight (no rounds) the GM ticks it.
  useEffect(() => {
    let live = true;
    api.deflectionInfo(characterId).then(
      (info) => {
        if (!live) return;
        setDeflection(info);
        setDeflect(info.amount > 0 && info.first_this_round);
      },
      () => undefined,
    );
    return () => {
      live = false;
    };
  }, [characterId]);

  // Rust decides what the hit does (two sixes, forced injuries); the form only asks.
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
  // Any injury of the area can be picked, even one the target has (a second leg); the random
  // 2d6 roll skips the ones already on the sheet.
  const pickable = injuries
    .filter((i) => i.area === area && i.id !== preview?.forced_injury)
    .sort((a, b) => a.roll - b.roll);
  const freeInjuries = pickable.filter((i) => !owned.includes(i.id));
  const chosenInjury = critical ? (injuries.find((i) => i.id === injuryId) ?? null) : null;

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

  const rollDamage = (dice: string, manual: number[] | null) =>
    guard(async () => {
      const result = await api.rollDice(dice, manual);
      setRoll(result);
      setValueText(String(result.total));
    });

  const rollInjury = (manual: number[] | null) =>
    guard(async () => {
      const result = await api.rollInjury(characterId, area, manual);
      setInjuryRoll(result.roll);
      setInjuryId(result.injury_id);
    });

  const chooseArea = (next: BodyArea) => {
    setArea(next);
    setAimed(null);
    setInjuryId(null);
    setInjuryRoll(null);
  };

  const apply = () =>
    guard(async () => {
      if (request === null || preview === null) return;
      let injury = chosenInjury?.id ?? null;
      // A critical hit with no injury picked gets a random one: the fast path.
      if (critical && injury === null && freeInjuries.length > 0) {
        injury = (await api.rollInjury(characterId, area, null)).injury_id;
      }
      const result = await api.applyDamage(characterId, request, injury);
      onApplied(result);
      onClose();
    });

  // Zero damage changes nothing: there is nothing to apply (or to undo later).
  const canApply = preview !== null && !busy && (parseCount(valueText) ?? 0) > 0;
  const damageInvalid = valueText.trim() !== "" && parseCount(valueText) === null;

  return (
    <form
      className="space-y-4"
      onSubmit={(e) => {
        e.preventDefault();
        if (canApply) void apply();
      }}
    >
      <div className="space-y-1">
        <DialogTitle className="text-xl font-bold">{t("combat.damage")}</DialogTitle>
        <DialogDescription className="flex items-center gap-3">
          <span className="font-display text-base font-semibold text-foreground uppercase">
            {target.character.name}
          </span>
          {cover && (
            <span className="flex items-center gap-1 text-xs text-cover">
              <Shield className="size-3.5" />
              {cover.label}
            </span>
          )}
        </DialogDescription>
      </div>

      <div className="flex items-end gap-2">
        <label className="block space-y-1">
          <span className="block text-xs text-muted-foreground uppercase">{t("damage.amount")}</span>
          <FieldBadge error={damageInvalid ? t("validation.amount") : null}>
            {(invalid) => (
              <Input
                autoFocus
                inputMode="numeric"
                value={valueText}
                onChange={(e) => {
                  setValueText(e.target.value);
                  setRoll(null);
                }}
                aria-invalid={invalid}
                className="num h-10 w-28 text-lg font-bold"
              />
            )}
          </FieldBadge>
        </label>
        <DiceButton
          notation={notation}
          onNotationChange={(n) => {
            setNotation(n);
            setRoll(null);
          }}
          disabled={busy}
          tooltip={t("damage.dice_roll", { notation })}
          onRoll={(dice, manual) => void rollDamage(dice, manual)}
        />
      </div>
      {roll && <DiceResult roll={roll} />}

      <div className="flex flex-wrap items-center gap-2">
        <Segment
          label={t("damage.spot.body")}
          selected={area === "body"}
          onClick={() => chooseArea("body")}
        />
        <Segment
          label={t("damage.spot.head")}
          selected={area === "head"}
          onClick={() => chooseArea("head")}
        />
        <span className="mx-1 h-5 w-px bg-border" />
        {AIMS[area].map((target) => (
          <Hint key={target} label={t(`damage.aimed_hint.${target}`)}>
            <Button
              type="button"
              size="sm"
              variant={aimed === target ? "default" : "outline"}
              onClick={() => setAimed(aimed === target ? null : target)}
            >
              {t(`damage.aimed.${target}`)}
            </Button>
          </Hint>
        ))}
      </div>

      <div className="flex flex-wrap items-center gap-2">
        {ARMOR_MODES.map((mode) => (
          <Hint key={mode} label={t(`damage.armor_mode.${mode}`)}>
            <Button
              type="button"
              size="sm"
              variant={armorMode === mode ? "default" : "outline"}
              onClick={() => setArmorMode(mode)}
            >
              {t(`damage.armor_short.${mode}`)}
            </Button>
          </Hint>
        ))}
      </div>

      <div className="flex flex-wrap items-center gap-x-5 gap-y-2">
        <Hint label={t("damage.critical_hint")}>
          <label className="flex items-center gap-2 text-sm">
            <Checkbox
              checked={critical || manualCritical}
              disabled={roll !== null}
              onCheckedChange={(v) => setManualCritical(v === true)}
            />
            {t("damage.critical")}
          </label>
        </Hint>
        {deflection !== null && deflection.amount > 0 && (
          <Hint label={t("damage.deflect_hint")}>
            <label className="flex items-center gap-2 text-sm">
              <Checkbox checked={deflect} onCheckedChange={(v) => setDeflect(v === true)} />
              {t("damage.deflect", { value: deflection.amount })}
            </label>
          </Hint>
        )}
        <Hint label={t("damage.explosive")}>
          <label className="flex items-center gap-2 text-sm">
            <Checkbox checked={explosive} onCheckedChange={(v) => setExplosive(v === true)} />
            {t("damage.explosive_short")}
          </label>
        </Hint>
      </div>

      {critical && (
        <section className="space-y-2">
          <div className="flex items-center gap-3">
            <h4 className="text-sm font-semibold text-primary">{t("glossary.critical_injury")}</h4>
            <DiceButton
              notation="2d6"
              disabled={busy || freeInjuries.length === 0}
              tooltip={t("damage.injury_random")}
              onRoll={(_, manual) => void rollInjury(manual)}
            />
          </div>
          {pickable.length === 0 ? (
            <p className="text-sm text-muted-foreground">{t("damage.no_free_injury")}</p>
          ) : (
            <>
              <div className="grid grid-cols-6 gap-1.5">
                {pickable.map((injury) => (
                  <Hint
                    key={injury.id}
                    label={
                      <div className="space-y-1">
                        <div className="font-semibold">{t(injury.name_key)}</div>
                        <div className="text-muted-foreground">{t(injury.effect_key)}</div>
                        {owned.includes(injury.id) && (
                          <div className="text-state-serious">{t("damage.injury_owned")}</div>
                        )}
                      </div>
                    }
                  >
                    <Button
                      type="button"
                      size="icon-lg"
                      variant={injuryId === injury.id ? "default" : "outline"}
                      aria-label={t(injury.name_key)}
                      aria-pressed={injuryId === injury.id}
                      onClick={() => {
                        setInjuryId(injuryId === injury.id ? null : injury.id);
                        setInjuryRoll(null);
                      }}
                    >
                      <InjuryIcon id={injury.id} className="size-6" />
                    </Button>
                  </Hint>
                ))}
              </div>
              <p className="text-sm text-muted-foreground">
                {chosenInjury ? (
                  <>
                    <span className="text-foreground">{t(chosenInjury.name_key)}</span>
                    {injuryRoll && <span className="num"> · 2d6 = {injuryRoll.total}</span>}
                  </>
                ) : (
                  t("damage.injury_default")
                )}
              </p>
            </>
          )}
        </section>
      )}

      {error && <p className="text-sm text-destructive">{error}</p>}

      <div className="flex justify-end gap-2">
        <Button type="button" variant="outline" onClick={onClose}>
          {t("characters.cancel")}
        </Button>
        <Button type="submit" disabled={!canApply}>
          {t("damage.apply")}
        </Button>
      </div>
    </form>
  );
}

type SegmentProps = { label: string; selected: boolean; onClick: () => void };

function Segment({ label, selected, onClick }: SegmentProps) {
  return (
    <Button type="button" size="sm" variant={selected ? "default" : "outline"} onClick={onClick}>
      {label}
    </Button>
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
            face === roll.notation.sides &&
              roll.notation.sides === 6 &&
              "[--cut-border:var(--primary)] text-primary",
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
