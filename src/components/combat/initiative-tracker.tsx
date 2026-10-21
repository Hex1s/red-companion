import { useState } from "react";
import { useTranslation } from "react-i18next";
import type { Combat } from "@/bindings/Combat";
import type { Combatant } from "@/bindings/Combatant";
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
import { cn } from "@/lib/utils";

const INITIATIVE_MAX = 99;
const DIE_SIDES = 10;

export type FighterInfo = { id: number; name: string; wound: WoundState };

type InitiativeTrackerProps = {
  combat: Combat;
  /** Everyone who exists, in or out of the fight. */
  characters: FighterInfo[];
  onAdd: (id: number, manual: number | null) => void;
  onReroll: (id: number, manual: number | null) => void;
  onSetInitiative: (id: number, initiative: number) => void;
  onShift: (id: number, delta: number) => void;
  onRemove: (id: number) => void;
  onStart: () => void;
  onNextTurn: () => void;
  onEnd: () => void;
};

/** Parses a whole number from an input; `null` for empty or invalid text. */
function parseWhole(text: string): number | null {
  const n = Number(text);
  return text.trim() !== "" && Number.isInteger(n) && n >= 0 ? n : null;
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
}: InitiativeTrackerProps) {
  const { t } = useTranslation();
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
  const die = parseWhole(dieText);
  const dieInvalid = dieText.trim() !== "" && (die === null || die < 1 || die > DIE_SIDES);

  const submit = () => {
    if (id === null || dieInvalid) return;
    onAdd(id, die);
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
}: FighterRowProps) {
  const { t } = useTranslation();
  const [text, setText] = useState(String(combatant.initiative));
  const [dieText, setDieText] = useState("");
  const die = parseWhole(dieText);
  const dieInvalid = dieText.trim() !== "" && (die === null || die < 1 || die > DIE_SIDES);

  // The total commits on blur / Enter; invalid text snaps back.
  const commit = () => {
    const value = parseWhole(text);
    if (value === null || value > INITIATIVE_MAX) {
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
          ? `${t("stats.REF")} ${combatant.reflex} + ${combatant.roll}`
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
        className="num w-16"
      />
      <Button
        size="xs"
        variant="outline"
        disabled={dieInvalid}
        onClick={() => {
          onReroll(combatant.character_id, die);
          setDieText("");
        }}
      >
        {t("initiative.reroll")}
      </Button>
      <Button size="xs" variant="outline" onClick={() => onRemove(combatant.character_id)}>
        {t("initiative.remove")}
      </Button>
    </li>
  );
}
