import { useEffect, useRef, useState, type ReactNode } from "react";
import { RotateCcw, Swords } from "lucide-react";
import { useTranslation } from "react-i18next";
import type { CheckRoll } from "@/bindings/CheckRoll";
import type { DeathSave } from "@/bindings/DeathSave";
import { MarkPicker, type MarkChange } from "@/components/character/color-mark";
import { WoundBadge } from "@/components/character/wound-badge";
import { item, MoreMenu, separator, tidy } from "@/components/explorer/node-menu";
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
  /** Sets or clears (`null`) the colour label and the portrait. */
  onSetMark: (change: MarkChange) => void;
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
  /** Opens the humanity loss window. */
  onLoseHumanity: () => void;
  /** Opens the therapy window. */
  onTherapy: () => void;
  /** The character lies in the active session, so it can be taken out of it. */
  inSession: boolean;
  /** Takes the character out of the active session (the character itself is kept). */
  onLeaveSession: () => void;
  /** Set for an instance (a temporary copy from a template): the menu offers to keep it instead of "leave session". */
  onKeepInLibrary?: () => void;
  onExport: () => void;
  /** Saves a copy as one of the GM's own templates; only NPCs offer it. */
  onSaveAsTemplate: () => void;
  /** Deletes the character; the status area offers to undo. */
  onDelete: () => void;
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
  onSetMark,
  onDamage,
  onSetInitiative,
  onRollInitiative,
  initiativeRoll,
  initiativeBonus,
  onShift,
  onStabilize,
  onRest,
  onLoseHumanity,
  onTherapy,
  inSession,
  onLeaveSession,
  onKeepInLibrary,
  onExport,
  onSaveAsTemplate,
  onDelete,
  onRevive,
  onDeathSave,
  onLeave,
}: CharacterCardProps) {
  const { t } = useTranslation();
  const { character, derived } = entry;
  const [text, setText] = useState(initiative === null ? "" : String(initiative));
  useEffect(() => setText(initiative === null ? "" : String(initiative)), [initiative]);

  // HP went down (damage, not healing): the card glitches once. The ref skips the first render.
  const [hit, setHit] = useState(false);
  const lastHp = useRef(character.hp);
  useEffect(() => {
    if (character.hp < lastHp.current) setHit(true);
    lastHp.current = character.hp;
  }, [character.hp]);

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
  // Rest needs a living character who is not mortally wounded (stabilise first); a day passes even at full HP.
  const canRest = !dead && !mortal;
  const menu = tidy([
    item("roll-initiative", t("card.initiative_roll", { reflex: character.stats.reflex }), onRollInitiative),
    ...(place
      ? [
          item("order-up", t("initiative.up"), () => onShift(-1), { disabled: place.first }),
          item("order-down", t("initiative.down"), () => onShift(1), { disabled: place.last }),
        ]
      : []),
    item("leave-fight", t("initiative.remove_from_fight"), onLeave),
    separator("s0"),
    item("rest", t("rest.button"), onRest, { disabled: !canRest }),
    item("stabilize", t("initiative.stabilize"), onStabilize, { disabled: !mortal }),
    item("humanity-loss", t("humanity.loss_button"), onLoseHumanity, { disabled: dead }),
    item("therapy", t("humanity.therapy_button"), onTherapy, {
      disabled: dead || derived.humanity >= derived.max_humanity,
    }),
    separator("s1"),
    onKeepInLibrary
      ? item("keep", t("explorer.keep_in_library"), onKeepInLibrary)
      : item("leave-session", t("sessions.leave"), onLeaveSession, { disabled: !inSession }),
    item("export", t("characters.export_selected"), onExport),
    ...(entry.kind === "npc" ? [item("template", t("encounters.save_as_template"), onSaveAsTemplate)] : []),
    separator("s2"),
    item("delete", t("characters.delete"), onDelete, { destructive: true }),
  ]);

  return (
    <div
      className={cn(
        "cut relative animate-in cursor-pointer p-3 text-left duration-200 fade-in-0 slide-in-from-right-4 [--cut:8px]",
        inCover && "[--cut-border:var(--cover)]",
        selected && !active && !inCover && "[--cut-border:var(--muted-foreground)]",
        active && "[--cut-border:var(--primary)]",
        derived.wound_state === "dead" && "opacity-60",
      )}
      onClick={select}
      style={character.color ? ({ "--mark": character.color } as React.CSSProperties) : undefined}
    >
      {character.color && <span aria-hidden className="absolute inset-y-3 left-0 z-10 w-1 bg-(--mark)" />}
      {/* The glitch plays on this inner box: `glitch-hit` would replace the card's own entry animation. */}
      <div
        className={cn("space-y-2", hit && "glitch-hit")}
        onAnimationEnd={(e) => e.target === e.currentTarget && e.animationName === "glitch-shake" && setHit(false)}
      >
      <div className="flex items-center gap-1.5">
        <Hint
          label={
            initiativeBonus !== 0 || explosionOf(initiativeRoll) ? (
              <span className="block max-w-64">
                {t("card.initiative_hint")}
                {initiativeBonus !== 0 && (
                  <span className="num mt-1 block text-emphasis">
                    {t("solo.initiative_from", { value: initiativeBonus })}
                  </span>
                )}
                {/* A d10 that exploded is told here, not with an icon that takes room on the row. */}
                {explosionOf(initiativeRoll) && (
                  <span className="mt-1 block">
                    <RollNote roll={initiativeRoll} />
                  </span>
                )}
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
            className="num h-7 w-12 px-1 text-center text-base font-bold text-emphasis"
          />
        </Hint>
        {/* Popovers are portals: their clicks must not reach the card. */}
        <span onClick={stop}>
          <MarkPicker
            color={character.color}
            portrait={character.portrait}
            name={character.name}
            markClassName="size-8"
            onChange={onSetMark}
          />
        </span>
        <span className="min-w-0 flex-1 truncate font-display text-sm font-semibold uppercase">
          {character.name}
        </span>
        {/* Popovers and menus are portals: their clicks must not reach the card. */}
        <span onClick={stop}>
          <MoreMenu entries={menu} alwaysVisible />
        </span>
      </div>

      <div className="flex items-center gap-2">
        <HpBar current={character.hp} max={derived.max_hp} className="flex-1" />
        <WoundBadge state={derived.wound_state} className="w-20 shrink-0 text-right text-xs" />
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
          <p className="font-display text-xs font-semibold text-emphasis uppercase">{t("solo.badge")}</p>
        </Hint>
      )}
      {derived.cyberpsychosis && (
        <Hint label={t("humanity.cyberpsychosis_hint")}>
          <p className="font-display text-xs font-semibold text-state-mortal-text uppercase">
            {t("humanity.cyberpsychosis")} <span className="num">{derived.humanity}</span>
          </p>
        </Hint>
      )}

      <div className="flex items-center gap-1.5">
        <span className="num flex items-center gap-3 text-xs text-muted-foreground">
          <span>
            {t("armor.head")} {character.armor_head.sp}
          </span>
          <span>
            {t("armor.body")} {character.armor_body.sp}
          </span>
        </span>
        {mortal && (
          <Button type="button" size="xs" variant="outline" onClick={onStabilize}>
            {t("initiative.stabilize")}
          </Button>
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
          {/* Popovers and menus are portals: their clicks must not reach the card. */}
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
      <p className="num text-xs text-state-mortal-text">
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
