import { useTranslation } from "react-i18next";
import { Button } from "@/components/ui/button";
import type { CharacterEntry } from "@/hooks/use-characters";
import { cn } from "@/lib/utils";

type CharacterListProps = {
  entries: CharacterEntry[];
  /** Ids of the active session's characters. */
  members: number[];
  selectedId: number | null;
  onSelect: (id: number) => void;
  onAdd: (id: number) => void;
  onLeave: (id: number) => void;
};

/** Every saved character: open its sheet, add it to the active session or take it out. */
export function CharacterList({
  entries,
  members,
  selectedId,
  onSelect,
  onAdd,
  onLeave,
}: CharacterListProps) {
  const { t } = useTranslation();
  if (entries.length === 0) return null;
  return (
    <ul className="max-h-56 space-y-1 overflow-y-auto">
      {entries.map((e) => {
        const inSession = members.includes(e.id);
        return (
          <li
            key={e.id}
            className={cn(
              "cut flex items-center gap-3 px-3 py-1.5 [--cut:4px]",
              e.id === selectedId && "[--cut-border:var(--muted-foreground)]",
            )}
          >
            <button
              type="button"
              className="min-w-0 flex-1 truncate text-left font-display text-sm font-semibold uppercase"
              onClick={() => onSelect(e.id)}
            >
              {e.character.name}
            </button>
            <span className="text-xs uppercase text-muted-foreground">{t(`characters.kind.${e.kind}`)}</span>
            <Button
              size="xs"
              variant="outline"
              onClick={() => (inSession ? onLeave(e.id) : onAdd(e.id))}
            >
              {t(inSession ? "sessions.leave_short" : "sessions.join")}
            </Button>
          </li>
        );
      })}
    </ul>
  );
}
