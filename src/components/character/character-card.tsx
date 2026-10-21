import { useTranslation } from "react-i18next";
import { Button } from "@/components/ui/button";
import { cn } from "@/lib/utils";
import { HpBar } from "./hp-bar";

export type CharacterCardData = {
  id: string;
  name: string;
  /** Set by the initiative tracker (1.6); absent outside combat. */
  initiative?: number;
  hp: number;
  maxHp: number;
  spHead: number;
  spBody: number;
};

type CharacterCardProps = {
  character: CharacterCardData;
  active?: boolean;
  selected?: boolean;
  onSelect?: () => void;
  onDamage?: () => void;
};

/** Narrow card for the initiative rail on the right. */
export function CharacterCard({
  character,
  active,
  selected,
  onSelect,
  onDamage,
}: CharacterCardProps) {
  const { t } = useTranslation();
  return (
    <div
      className={cn(
        "cut [--cut:8px] cursor-pointer space-y-2 p-3 text-left",
        active && "[--cut-border:var(--primary)]",
        selected && !active && "[--cut-border:var(--muted-foreground)]",
      )}
      onClick={onSelect}
    >
      <div className="flex items-baseline justify-between gap-2">
        <span className="truncate font-display text-sm font-semibold uppercase">
          {character.name}
        </span>
        {character.initiative !== undefined && (
          <span className="num text-lg font-bold text-primary">{character.initiative}</span>
        )}
      </div>
      <HpBar current={character.hp} max={character.maxHp} />
      <div className="num flex gap-3 text-xs text-muted-foreground">
        <span>
          {t("armor.head")} {character.spHead}
        </span>
        <span>
          {t("armor.body")} {character.spBody}
        </span>
      </div>
      <Button
        size="xs"
        variant="outline"
        className="w-full"
        onClick={(e) => {
          e.stopPropagation();
          onDamage?.();
        }}
      >
        {t("combat.damage")}
      </Button>
    </div>
  );
}
