import { Play, SkipForward, Swords } from "@/components/ui/icons";
import { useTranslation } from "react-i18next";
import type { Combat } from "@/bindings/Combat";
import { HpBar } from "@/components/character/hp-bar";
import { Button } from "@/components/ui/button";
import type { CharacterEntry } from "@/hooks/use-characters";

type CombatDockProps = {
  combat: Combat | null;
  /** Whose turn it is; `null` before the fight starts. */
  active: CharacterEntry | null;
  hasCharacters: boolean;
  /** Opens the full combat column. */
  onExpand: () => void;
  onStart: () => void;
  onNextTurn: () => void;
  onDamage: (id: number) => void;
};

/**
 * Bottom bar of the narrow layout: the turn, the active character's HP and the two actions used
 * every round. The rest of the combat column opens from it as a drawer.
 */
export function CombatDock({ combat, active, hasCharacters, onExpand, onStart, onNextTurn, onDamage }: CombatDockProps) {
  const { t } = useTranslation();
  const started = (combat?.round ?? 0) > 0;

  return (
    <div className="flex items-stretch gap-2 border-t bg-background p-2">
      <button
        type="button"
        aria-label={t("rail.title")}
        onClick={onExpand}
        className="flex min-h-11 min-w-0 flex-1 items-center gap-3 border px-3 text-left outline-none hover:bg-accent focus-visible:ring-2 focus-visible:ring-ring"
      >
        <Swords className="size-5 shrink-0 text-muted-foreground" />
        <span className="min-w-0 flex-1">
          <span className="flex items-baseline gap-2 text-xs text-muted-foreground uppercase">
            {started ? (
              <>
                {t("initiative.round")} <span className="num text-sm text-emphasis">{combat?.round}</span>
              </>
            ) : (
              t("initiative.title")
            )}
            {active && (
              <span className="min-w-0 flex-1 truncate font-display text-xs font-semibold text-foreground">
                {active.character.name}
              </span>
            )}
          </span>
          {active && <HpBar current={active.character.hp} max={active.derived.max_hp} className="mt-1" />}
        </span>
      </button>
      {started ? (
        <>
          <Button type="button" variant="outline" className="h-auto min-h-11" disabled={!active} onClick={() => active && onDamage(active.id)}>
            {t("card.damage")}
          </Button>
          <Button type="button" className="h-auto min-h-11" aria-label={t("initiative.next_turn")} onClick={onNextTurn}>
            <SkipForward />
          </Button>
        </>
      ) : (
        <Button
          type="button"
          className="h-auto min-h-11"
          disabled={!hasCharacters}
          aria-label={t("initiative.start")}
          onClick={onStart}
        >
          <Play />
        </Button>
      )}
    </div>
  );
}
