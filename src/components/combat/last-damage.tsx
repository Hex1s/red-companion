import { useTranslation } from "react-i18next";
import type { DamageEvent } from "@/bindings/DamageEvent";
import { Button } from "@/components/ui/button";

type LastDamageProps = {
  event: DamageEvent;
  onUndo: () => void;
};

/** The scene's last damage event with its undo button (only the last one can be undone). */
export function LastDamage({ event, onUndo }: LastDamageProps) {
  const { t } = useTranslation();
  const { outcome } = event;
  return (
    <div className="cut flex flex-wrap items-center gap-3 p-3 [--cut:8px]">
      <span className="text-xs text-muted-foreground uppercase">{t("damage.last")}</span>
      <span className="font-display text-sm font-semibold uppercase">{event.character_name}</span>
      <span className="num text-primary">−{outcome.hp_lost}</span>
      <span className="num text-xs text-muted-foreground">
        {event.hp_before} → {outcome.new_hp}
      </span>
      <Button size="xs" variant="outline" className="ml-auto" onClick={onUndo}>
        {t("damage.undo")}
      </Button>
    </div>
  );
}
