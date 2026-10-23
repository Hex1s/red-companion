import { useEffect, useState, type ReactNode } from "react";
import { BedDouble, ChevronDown, ChevronUp, Dices, RotateCcw, Swords, UserMinus, Zap } from "lucide-react";
import { useTranslation } from "react-i18next";
import type { CheckRoll } from "@/bindings/CheckRoll";
import type { DeathSave } from "@/bindings/DeathSave";
import { WoundBadge } from "@/components/character/wound-badge";
import { Button } from "@/components/ui/button";
import { FieldBadge } from "@/components/ui/field-badge";
import { Hint } from "@/components/ui/hint";
import { Input } from "@/components/ui/input";
import { RollNote } from "@/components/ui/roll-note";
import type { CharacterEntry } from "@/hooks/use-characters";
import { explosionOf } from "@/lib/dice";
import { cn } from "@/lib/utils";
import { HpBar } from "./hp-bar";

/** Initiative range the backend accepts. */
const INITIATIVE_MAX = 99;
const INITIATIVE_MIN = -99;

type CharacterCardProps = {
  entry: CharacterEntry;
  /** Initiative total; `null` until it is rolled or typed in. */
  initiative: number | null;
  /** Position in the acting order, for the up/down buttons; `null` when not in the order. */
  place: { first: boolean; last: boolean } | null;
  active?: boolean;
  selected?: boolean;
  /** Stands behind cover: the card gets an outline. */
  inCover: boolean;
  /** The cover button with its popover (see `CoverPicker`). */
  coverControl: ReactNode;
  /** The active fighter is mortally wounded and owes this turn's death save. */
  deathSaveDue: boolean;
  /** Result of the last death save of this character, shown until the next turn. */
  lastSave: DeathSave | null;
  onSelect: () => void;
  onDamage: () => void;
  onSetInitiative: (value: number) => void;
  onRollInitiative: () => void;
  /** The d10 behind the initiative; `null` when the total was typed in. */
  initiativeRoll: CheckRoll | null;
  /** What a role added to that roll (Solo's Personal Initiative); 0 when nothing. */
  initiativeBonus: number;
  onShift: (delta: number) => void;
  onStabilize: () => void;
  /** Opens the rest window (HP back over days of rest). */
  onRest: () => void;
  /** Brings a dead character back (1 HP, no penalty, no injuries). */
  onRevive: () => void;
  /** Resolves `true` when the save was made. */
  onDeathSave: (manual: number | null) => Promise<boolean>;
  /** Takes the character out of the session (the character itself is kept). */
  onLeave: () => void;
};

/** Parses a whole number (may be negative); `null` for empty or invalid text. */
function parseSigned(text: string): number | null {
  const n = Number(text);
  return text.trim() !== "" && Number.isInteger(n) ? n : null;
}

/** Card of a session character on the right rail: initiative, HP, armor, cover, damage. */
export function CharacterCard({
  entry,
  initiative,
  place,
  active,
  selected,
  inCover,
  coverControl,
  deathSaveDue,
  lastSave,
  onSelect,
  onDamage,
  onSetInitiative,
  onRollInitiative,
  initiativeRoll,
  initiativeBonus,
  onShift,
  onStabilize,
  onRest,
  onRevive,
  onDeathSave,
  onLeave,
}: CharacterCardProps) {
  const { t } = useTranslation();
  const { character, derived } = entry;
  const [text, setText] = useState(initiative === null ? "" : String(initiative));
  useEffect(() => setText(initiative === null ? "" : String(initiative)), [initiative]);

  // The total commits on blur / Enter; invalid text snaps back.
  const commit = () => {
    const value = parseSigned(text);
    if (text.trim() === "" || value === null || value > INITIATIVE_MAX || value < INITIATIVE_MIN) {
      setText(initiative === null ? "" : String(initiative));
    } else if (value !== initiative) {
      onSetInitiative(value);
    }
  };

  // A click anywhere on the card selects it, except on its own controls.
  const select = (e: React.MouseEvent) => {
    if ((e.target as HTMLElement).closest("button, input, select, textarea, a, [role=combobox]")) return;
    onSelect();
  };
  const stop = (e: React.SyntheticEvent) => e.stopPropagation();
  // Only a mortally wounded character (0 HP) is stabilised; the dead can be brought back.
  const mortal = derived.wound_state === "mortal";
  // Solo's Combat Awareness is on and some points are placed: a badge lists what they do.
  const hasSoloEffects =
    derived.solo_initiative > 0 ||
    derived.solo_deflection > 0 ||
    derived.solo_perception > 0 ||
    derived.solo_precision > 0 ||
    derived.solo_spot_weakness > 0;
  const dead = derived.wound_state === "dead";

  return (
    <div
      className={cn(
        "cut cursor-pointer space-y-2 p-3 text-left [--cut:8px]",
        inCover && "[--cut-border:var(--cover)]",
        selected && !active && !inCover && "[--cut-border:var(--muted-foreground)]",
        active && "[--cut-border:var(--primary)]",
        derived.wound_state === "dead" && "opacity-60",
      )}
      onClick={select}
    >
      <div className="flex items-center gap-1.5">
        <Hint
          label={
            initiativeBonus !== 0 ? (
              <span className="block max-w-64">
                {t("card.initiative_hint")}
                <span className="num mt-1 block text-primary">
                  {t("solo.initiative_from", { value: initiativeBonus })}
                </span>
              </span>
            ) : (
              t("card.initiative_hint")
            )
          }
        >
          <Input
            inputMode="numeric"
            value={text}
            placeholder="—"
            aria-label={t("initiative.total")}
            onChange={(e) => setText(e.target.value)}
            onBlur={commit}
            onKeyDown={(e) => e.key === "Enter" && (e.target as HTMLElement).blur()}
            className="num h-7 w-12 px-1 text-center text-base font-bold text-primary"
          />
        </Hint>
        {explosionOf(initiativeRoll) && (
          <Hint label={<RollNote roll={initiativeRoll} />}>
            <Zap
              className={cn(
                "size-3.5 shrink-0",
                explosionOf(initiativeRoll)?.direction === "up" ? "text-primary" : "text-state-serious",
              )}
            />
          </Hint>
        )}
        <Hint label={t("card.initiative_roll", { reflex: character.stats.reflex })}>
          <Button
            type="button"
            variant="ghost"
            size="icon-xs"
            aria-label={t("card.initiative_roll", { reflex: character.stats.reflex })}
            onClick={onRollInitiative}
          >
            <Dices />
          </Button>
        </Hint>
        {place && (
          <div className="flex flex-col">
            <Hint label={t("initiative.up")}>
              <Button
                type="button"
                variant="ghost"
                size="icon-xs"
                className="h-3 w-5"
                disabled={place.first}
                aria-label={t("initiative.up")}
                onClick={() => onShift(-1)}
              >
                <ChevronUp />
              </Button>
            </Hint>
            <Hint label={t("initiative.down")}>
              <Button
                type="button"
                variant="ghost"
                size="icon-xs"
                className="h-3 w-5"
                disabled={place.last}
                aria-label={t("initiative.down")}
                onClick={() => onShift(1)}
              >
                <ChevronDown />
              </Button>
            </Hint>
          </div>
        )}
        <span className="min-w-0 flex-1 truncate font-display text-sm font-semibold uppercase">
          {character.name}
        </span>
        <Hint label={t("sessions.leave")}>
          <Button
            type="button"
            variant="ghost"
            size="icon-xs"
            aria-label={t("sessions.leave")}
            onClick={onLeave}
          >
            <UserMinus />
          </Button>
        </Hint>
      </div>

      <HpBar current={character.hp} max={derived.max_hp} />

      <div className="num flex items-center gap-3 text-xs text-muted-foreground">
        <span>
          {t("armor.head")} {character.armor_head.sp}
        </span>
        <span>
          {t("armor.body")} {character.armor_body.sp}
        </span>
        <WoundBadge state={derived.wound_state} className="ml-auto text-xs" />
      </div>
      {hasSoloEffects && (
        <Hint
          label={
            <ul className="num space-y-0.5">
              {derived.solo_initiative > 0 && <li>{t("solo.initiative", { value: derived.solo_initiative })}</li>}
              {derived.solo_deflection > 0 && <li>{t("solo.deflection", { value: derived.solo_deflection })}</li>}
              {derived.solo_perception > 0 && <li>{t("solo.perception", { value: derived.solo_perception })}</li>}
              {derived.solo_precision > 0 && <li>{t("solo.precision", { value: derived.solo_precision })}</li>}
              {derived.solo_spot_weakness > 0 && (
                <li>{t("solo.spot_weakness", { value: derived.solo_spot_weakness })}</li>
              )}
            </ul>
          }
        >
          <p className="font-display text-xs font-semibold text-primary uppercase">{t("solo.badge")}</p>
        </Hint>
      )}
      {derived.cyberpsychosis && (
        <Hint label={t("humanity.cyberpsychosis_hint")}>
          <p className="font-display text-xs font-semibold text-state-mortal uppercase">
            {t("humanity.cyberpsychosis")} <span className="num">{derived.humanity}</span>
          </p>
        </Hint>
      )}

      <div className="flex items-center gap-1.5">
        {mortal && (
          <Button type="button" size="xs" variant="outline" onClick={onStabilize}>
            {t("initiative.stabilize")}
          </Button>
        )}
        {!dead && !mortal && character.hp < derived.max_hp && (
          <Hint label={t("rest.hint")}>
            <Button type="button" size="xs" variant="outline" onClick={onRest}>
              <BedDouble />
              {t("rest.button")}
            </Button>
          </Hint>
        )}
        {dead && (
          <Hint label={t("card.revive_hint")}>
            <Button type="button" size="xs" variant="outline" onClick={onRevive}>
              <RotateCcw />
              {t("card.revive")}
            </Button>
          </Hint>
        )}
        <span className="ml-auto flex items-center gap-1.5">
          {/* The popover is a portal: its clicks must not reach the card. */}
          <span onClick={stop}>{coverControl}</span>
          {!dead && (
            <Hint label={t("combat.damage")}>
              <Button type="button" size="xs" onClick={onDamage}>
                <Swords />
                {t("card.damage")}
              </Button>
            </Hint>
          )}
        </span>
      </div>

      {deathSaveDue && active && mortal && <DeathSaveRow character={entry} onRoll={onDeathSave} />}
      {lastSave && !deathSaveDue && (
        <p className="num text-xs">
          {t("initiative.death_save")}: d10 {lastSave.roll.dice[0]} + {lastSave.penalty} —{" "}
          <span className={lastSave.passed ? "text-state-ok" : "text-state-dead"}>
            {t(lastSave.passed ? "initiative.death_save_passed" : "initiative.death_save_failed")}
          </span>
        </p>
      )}
    </div>
  );
}

type DeathSaveRowProps = {
  character: CharacterEntry;
  onRoll: (manual: number | null) => Promise<boolean>;
};

/** Start of a mortally wounded character's turn: the death save, die faces optional. */
function DeathSaveRow({ character, onRoll }: DeathSaveRowProps) {
  const { t } = useTranslation();
  const [text, setText] = useState("");
  const face = text.trim() === "" ? null : Number(text);
  const invalid = face !== null && (!Number.isInteger(face) || face < 1 || face > 10);

  return (
    <div className="space-y-1 border-t pt-2">
      <p className="num text-xs text-state-mortal">
        {t("initiative.death_save_rule", {
          body: character.character.stats.body,
          penalty: character.character.death_save_penalty,
        })}
      </p>
      <div className="flex items-center gap-2">
        <FieldBadge error={invalid ? t("validation.die") : null}>
          {(shown) => (
            <Input
              inputMode="numeric"
              value={text}
              placeholder={t("initiative.die_random")}
              aria-label={t("initiative.die")}
              aria-invalid={shown}
              onChange={(e) => setText(e.target.value)}
              className="num h-7 w-24"
            />
          )}
        </FieldBadge>
        <Button
          type="button"
          size="xs"
          disabled={invalid}
          onClick={async () => {
            if (await onRoll(face)) setText("");
          }}
        >
          {t("initiative.death_save")}
        </Button>
      </div>
    </div>
  );
}
