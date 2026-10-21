import { useState } from "react";
import { useTranslation } from "react-i18next";
import type { Combat } from "@/bindings/Combat";
import type { Combatant } from "@/bindings/Combatant";
import type { DeathSave } from "@/bindings/DeathSave";
import type { WoundState } from "@/bindings/WoundState";
import { WoundBadge } from "@/components/character/wound-badge";
import { Button } from "@/components/ui/button";
import { Input } from "@/components/ui/input";
import {
  Select,
  SelectContent,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from "@/components/ui/select";
import { formatCheck, parseFaces, validD10Faces } from "@/lib/dice";
import { cn } from "@/lib/utils";

const INITIATIVE_MAX = 99;
const INITIATIVE_MIN = -99;

export type FighterInfo = {
  id: number;
  name: string;
  wound: WoundState;
  /** BODY and the current death save penalty, for the death save hint. */
  body: number;
  deathSavePenalty: number;
};

type InitiativeTrackerProps = {
  combat: Combat;
  /** Everyone who exists, in or out of the fight. */
  characters: FighterInfo[];
  onAdd: (id: number, manual: number[] | null) => void;
  onReroll: (id: number, manual: number[] | null) => void;
  onSetInitiative: (id: number, initiative: number) => void;
  onShift: (id: number, delta: number) => void;
  onRemove: (id: number) => void;
  onStart: () => void;
  onNextTurn: () => void;
  onEnd: () => void;
  /** Rolls the death save; resolves with the result, `null` if it failed to run. */
  onDeathSave: (id: number, manual: number | null) => Promise<DeathSave | null>;
  onStabilize: (id: number) => void;
};

/** Parses a whole number (may be negative) from an input; `null` for empty or invalid text. */
function parseSigned(text: string): number | null {
  const n = Number(text);
  return text.trim() !== "" && Number.isInteger(n) ? n : null;
}

/** Faces for a d10 check typed by hand: `null` = roll randomly, `false` = invalid text. */
function manualFaces(text: string): number[] | null | false {
  const faces = parseFaces(text);
  if (faces === null) return null;
  return validD10Faces(faces) ? faces : false;
}

/** Initiative order, round counter and turn controls of the current fight. */
export function InitiativeTracker({
  combat,
  characters,
  onAdd,
  onReroll,
  onSetInitiative,
  onShift,
  onRemove,
  onStart,
  onNextTurn,
  onEnd,
  onDeathSave,
  onStabilize,
}: InitiativeTrackerProps) {
  const { t } = useTranslation();
  const [lastSave, setLastSave] = useState<{ id: number; save: DeathSave } | null>(null);
  const started = combat.round > 0;
  const outside = characters.filter((c) => !combat.combatants.some((f) => f.character_id === c.id));

  return (
    <section className="space-y-4">
      <div className="flex flex-wrap items-center gap-3">
        <h3 className="mr-auto text-lg font-bold">{t("initiative.title")}</h3>
        {started && (
          <span className="text-sm text-muted-foreground uppercase">
            {t("initiative.round")} <span className="num text-primary">{combat.round}</span>
          </span>
        )}
        {started ? (
          <>
            <Button onClick={onNextTurn}>{t("initiative.next_turn")}</Button>
            <Button variant="outline" onClick={onEnd}>
              {t("initiative.end")}
            </Button>
          </>
        ) : (
          <>
            <Button disabled={combat.combatants.length === 0} onClick={onStart}>
              {t("initiative.start")}
            </Button>
            {combat.combatants.length > 0 && (
              <Button variant="outline" onClick={onEnd}>
                {t("initiative.clear")}
              </Button>
            )}
          </>
        )}
      </div>

      {combat.death_save_due && combat.active !== null && (
        <DeathSavePanel
          fighter={characters.find((c) => c.id === combat.active)}
          onRoll={async (manual) => {
            const id = combat.active as number;
            const save = await onDeathSave(id, manual);
            if (save) setLastSave({ id, save });
            return save !== null;
          }}
        />
      )}
      {!combat.death_save_due && lastSave && lastSave.id === combat.active && (
        <DeathSaveResult
          name={characters.find((c) => c.id === lastSave.id)?.name ?? ""}
          save={lastSave.save}
        />
      )}

      <AddFighter outside={outside} onAdd={onAdd} />

      {combat.combatants.length === 0 ? (
        <p className="text-muted-foreground">{t("initiative.empty")}</p>
      ) : (
        <ol className="space-y-2">
          {combat.combatants.map((c, index) => {
            const info = characters.find((f) => f.id === c.character_id);
            return (
              <FighterRow
                key={`${c.character_id}:${c.initiative}`}
                combatant={c}
                name={info?.name ?? String(c.character_id)}
                wound={info?.wound ?? "dead"}
                active={combat.active === c.character_id}
                first={index === 0}
                last={index === combat.combatants.length - 1}
                onReroll={onReroll}
                onSetInitiative={onSetInitiative}
                onShift={onShift}
                onRemove={onRemove}
                onStabilize={onStabilize}
              />
            );
          })}
        </ol>
      )}
    </section>
  );
}

type AddFighterProps = {
  outside: FighterInfo[];
  onAdd: InitiativeTrackerProps["onAdd"];
};

function AddFighter({ outside, onAdd }: AddFighterProps) {
  const { t } = useTranslation();
  const [chosen, setChosen] = useState<string>("");
  const [dieText, setDieText] = useState("");

  const id = outside.find((c) => String(c.id) === chosen)?.id ?? null;
  const faces = manualFaces(dieText);
  const dieInvalid = faces === false;

  const submit = () => {
    if (id === null || faces === false) return;
    onAdd(id, faces);
    setChosen("");
    setDieText("");
  };

  return (
    <div className="cut flex flex-wrap items-end gap-3 p-3 [--cut:8px]">
      <div className="space-y-1">
        <span className="block text-xs text-muted-foreground uppercase">
          {t("initiative.fighter")}
        </span>
        <Select value={chosen} onValueChange={setChosen}>
          <SelectTrigger className="w-60" disabled={outside.length === 0}>
            <SelectValue placeholder={t("initiative.pick")} />
          </SelectTrigger>
          <SelectContent>
            {outside.map((c) => (
              <SelectItem key={c.id} value={String(c.id)}>
                {c.name}
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
          onChange={(e) => setDieText(e.target.value)}
          onKeyDown={(e) => e.key === "Enter" && submit()}
          aria-invalid={dieInvalid}
          title={t("initiative.die_hint")}
          className="num w-28"
        />
      </div>
      <Button disabled={id === null || dieInvalid} onClick={submit}>
        {t("initiative.add")}
      </Button>
    </div>
  );
}

type FighterRowProps = {
  combatant: Combatant;
  name: string;
  wound: WoundState;
  active: boolean;
  first: boolean;
  last: boolean;
  onReroll: InitiativeTrackerProps["onReroll"];
  onSetInitiative: InitiativeTrackerProps["onSetInitiative"];
  onShift: InitiativeTrackerProps["onShift"];
  onRemove: InitiativeTrackerProps["onRemove"];
  onStabilize: InitiativeTrackerProps["onStabilize"];
};

function FighterRow({
  combatant,
  name,
  wound,
  active,
  first,
  last,
  onReroll,
  onSetInitiative,
  onShift,
  onRemove,
  onStabilize,
}: FighterRowProps) {
  const { t } = useTranslation();
  const [text, setText] = useState(String(combatant.initiative));
  const [dieText, setDieText] = useState("");
  const faces = manualFaces(dieText);
  const dieInvalid = faces === false;

  // The total commits on blur / Enter; invalid text snaps back.
  const commit = () => {
    const value = parseSigned(text);
    if (value === null || value > INITIATIVE_MAX || value < INITIATIVE_MIN) {
      setText(String(combatant.initiative));
    } else if (value !== combatant.initiative) {
      onSetInitiative(combatant.character_id, value);
    }
  };

  return (
    <li
      className={cn(
        "cut flex flex-wrap items-center gap-3 p-3 [--cut:8px]",
        active && "[--cut-border:var(--primary)]",
        wound === "dead" && "opacity-60",
      )}
    >
      <Input
        inputMode="numeric"
        value={text}
        onChange={(e) => setText(e.target.value)}
        onBlur={commit}
        onKeyDown={(e) => e.key === "Enter" && (e.target as HTMLElement).blur()}
        aria-label={t("initiative.total")}
        className="num w-16 text-center text-lg font-bold text-primary"
      />
      <span className="num text-xs text-muted-foreground">
        {combatant.roll !== null
          ? `${t("stats.REF")} ${combatant.reflex} + ${formatCheck(combatant.roll)}`
          : t("initiative.manual")}
      </span>
      <span className="mr-auto truncate font-display text-sm font-semibold uppercase">
        {name}
        {active && <span className="ml-2 text-primary">{t("initiative.turn")}</span>}
      </span>
      <WoundBadge state={wound} />
      <Button
        size="xs"
        variant="outline"
        disabled={first}
        aria-label={t("initiative.up")}
        onClick={() => onShift(combatant.character_id, -1)}
      >
        ↑
      </Button>
      <Button
        size="xs"
        variant="outline"
        disabled={last}
        aria-label={t("initiative.down")}
        onClick={() => onShift(combatant.character_id, 1)}
      >
        ↓
      </Button>
      <Input
        inputMode="numeric"
        value={dieText}
        placeholder={t("initiative.die")}
        aria-label={t("initiative.die")}
        aria-invalid={dieInvalid}
        onChange={(e) => setDieText(e.target.value)}
        title={t("initiative.die_hint")}
        className="num w-20"
      />
      <Button
        size="xs"
        variant="outline"
        disabled={dieInvalid}
        onClick={() => {
          if (faces === false) return;
          onReroll(combatant.character_id, faces);
          setDieText("");
        }}
      >
        {t("initiative.reroll")}
      </Button>
      {wound !== "healthy" && wound !== "dead" && (
        <Button size="xs" variant="outline" onClick={() => onStabilize(combatant.character_id)}>
          {t("initiative.stabilize")}
        </Button>
      )}
      <Button size="xs" variant="outline" onClick={() => onRemove(combatant.character_id)}>
        {t("initiative.remove")}
      </Button>
    </li>
  );
}

type DeathSavePanelProps = {
  fighter: FighterInfo | undefined;
  /** Resolves `true` when the save was made. */
  onRoll: (manual: number | null) => Promise<boolean>;
};

/** Start of a mortally wounded fighter's turn: the death save with manual die entry. */
function DeathSavePanel({ fighter, onRoll }: DeathSavePanelProps) {
  const { t } = useTranslation();
  const [text, setText] = useState("");
  const face = text.trim() === "" ? null : Number(text);
  const invalid = face !== null && (!Number.isInteger(face) || face < 1 || face > 10);
  if (!fighter) return null;

  return (
    <div className="cut flex flex-wrap items-center gap-3 p-3 [--cut:8px] [--cut-border:var(--state-mortal)]">
      <span className="font-display text-sm font-semibold uppercase">{fighter.name}</span>
      <span className="text-sm text-state-mortal">{t("initiative.death_save_due")}</span>
      <span className="num mr-auto text-xs text-muted-foreground">
        {t("initiative.death_save_rule", { body: fighter.body, penalty: fighter.deathSavePenalty })}
      </span>
      <Input
        inputMode="numeric"
        value={text}
        placeholder={t("initiative.die_random")}
        aria-label={t("initiative.die")}
        aria-invalid={invalid}
        onChange={(e) => setText(e.target.value)}
        className="num w-28"
      />
      <Button
        disabled={invalid}
        onClick={async () => {
          if (await onRoll(face)) setText("");
        }}
      >
        {t("initiative.death_save")}
      </Button>
    </div>
  );
}

type DeathSaveResultProps = { name: string; save: DeathSave };

function DeathSaveResult({ name, save }: DeathSaveResultProps) {
  const { t } = useTranslation();
  return (
    <p className="num text-sm">
      <span className="font-display font-semibold uppercase">{name}</span>{" "}
      {t("initiative.death_save")}: d10 {save.roll.dice[0]} + {save.penalty} —{" "}
      <span className={save.passed ? "text-state-ok" : "text-state-dead"}>
        {t(save.passed ? "initiative.death_save_passed" : "initiative.death_save_failed")}
      </span>
    </p>
  );
}
