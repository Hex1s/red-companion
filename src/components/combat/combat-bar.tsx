import { Play, Square, SkipForward, Undo2 } from "@/components/ui/icons";
import { useTranslation } from "react-i18next";
import type { Combat } from "@/bindings/Combat";
import type { DamageEvent } from "@/bindings/DamageEvent";
import { AttackReference } from "@/components/combat/attack-reference";
import { Button } from "@/components/ui/button";
import { Hint } from "@/components/ui/hint";

type CombatBarProps = {
  combat: Combat;
  /** Anyone in the session at all; there is nothing to start otherwise. */
  hasCharacters: boolean;
  lastDamage: DamageEvent | null;
  onStart: () => void;
  onNextTurn: () => void;
  onEnd: () => void;
  onUndo: () => void;
};

/** Head of the session rail: start / next turn / end of the fight, and the last hit with its undo. */
export function CombatBar({
  combat,
  hasCharacters,
  lastDamage,
  onStart,
  onNextTurn,
  onEnd,
  onUndo,
}: CombatBarProps) {
  const { t } = useTranslation();
  const started = combat.round > 0;

  return (
    // Sticks to the top of the rail so next turn and the initiative are always in reach.
    <div className="sticky top-0 z-10 -mx-3 -mt-3 space-y-2 border-b bg-background px-3 py-3">
      <div className="flex items-center gap-1.5">
        <span className="mr-auto text-xs text-muted-foreground uppercase">
          {started ? (
            <>
              {t("initiative.round")} <span className="num text-sm text-emphasis">{combat.round}</span>
            </>
          ) : (
            t("initiative.title")
          )}
        </span>
        {started ? (
          <>
            <Hint label={t("initiative.next_turn")}>
              <Button type="button" size="icon-sm" aria-label={t("initiative.next_turn")} onClick={onNextTurn}>
                <SkipForward />
              </Button>
            </Hint>
            <Hint label={t("initiative.end")}>
              <Button
                type="button"
                size="icon-sm"
                variant="outline"
                aria-label={t("initiative.end")}
                onClick={onEnd}
              >
                <Square />
              </Button>
            </Hint>
          </>
        ) : (
          <>
            <Hint label={t("rail.start_hint")}>
              <Button
                type="button"
                size="icon-sm"
                disabled={!hasCharacters}
                aria-label={t("initiative.start")}
                onClick={onStart}
              >
                <Play />
              </Button>
            </Hint>
            {combat.combatants.length > 0 && (
              <Hint label={t("initiative.clear")}>
                <Button
                  type="button"
                  size="icon-sm"
                  variant="outline"
                  aria-label={t("initiative.clear")}
                  onClick={onEnd}
                >
                  <Square />
                </Button>
              </Hint>
            )}
          </>
        )}
        <AttackReference />
      </div>

      {lastDamage && <LastHit event={lastDamage} onUndo={onUndo} />}
    </div>
  );
}

type LastHitProps = { event: DamageEvent; onUndo: () => void };

/** The last hit in numbers: what was taken off the HP and the armor. Only this one can be undone. */
function LastHit({ event, onUndo }: LastHitProps) {
  const { t } = useTranslation();
  const { outcome } = event;
  return (
    <div className="cut flex items-center gap-2 p-2 [--cut:6px]">
      <div className="min-w-0 flex-1">
        <div className="truncate font-display text-xs font-semibold uppercase">{event.character_name}</div>
        <div className="num flex flex-wrap gap-x-3 text-xs">
          <span className="text-emphasis">
            −{outcome.hp_lost} {t("scene.hp")}
          </span>
          {outcome.sp_after !== event.sp_before && (
            <span className="text-muted-foreground">
              {t("damage.armor")} {event.sp_before}→{outcome.sp_after}
            </span>
          )}
          {outcome.cover_hp_after !== null && (
            <span className="text-cover">
              {t("glossary.cover")} {outcome.cover_hp_after}
            </span>
          )}
        </div>
      </div>
      <Hint label={t("damage.undo")}>
        <Button type="button" size="icon-xs" variant="outline" aria-label={t("damage.undo")} onClick={onUndo}>
          <Undo2 />
        </Button>
      </Hint>
    </div>
  );
}
