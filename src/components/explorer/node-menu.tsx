import { Ellipsis } from "lucide-react";
import { useTranslation } from "react-i18next";
import {
  ContextMenuItem,
  ContextMenuSeparator,
} from "@/components/ui/context-menu";
import {
  DropdownMenu,
  DropdownMenuContent,
  DropdownMenuItem,
  DropdownMenuSeparator,
  DropdownMenuTrigger,
} from "@/components/ui/dropdown-menu";
import { cn } from "@/lib/utils";

/** One line of a node's menu. The same list feeds the right-click menu and the "⋯" button. */
export type MenuEntry =
  | { type: "separator"; key: string }
  | {
      type: "item";
      key: string;
      label: string;
      onSelect: () => void;
      disabled?: boolean;
      destructive?: boolean;
    };

export const item = (
  key: string,
  label: string,
  onSelect: () => void,
  options: { disabled?: boolean; destructive?: boolean } = {},
): MenuEntry => ({ type: "item", key, label, onSelect, ...options });

export const separator = (key: string): MenuEntry => ({ type: "separator", key });

/** Drops separators that would open, close or double the list (menus assemble entries by condition). */
export function tidy(entries: MenuEntry[]): MenuEntry[] {
  return entries.filter((entry, i) => {
    if (entry.type === "item") return true;
    const before = entries[i - 1];
    const after = entries[i + 1];
    return before?.type === "item" && after?.type === "item";
  });
}

export function ContextEntries({ entries }: { entries: MenuEntry[] }) {
  return entries.map((entry) =>
    entry.type === "separator" ? (
      <ContextMenuSeparator key={entry.key} />
    ) : (
      <ContextMenuItem
        key={entry.key}
        disabled={entry.disabled}
        variant={entry.destructive ? "destructive" : "default"}
        onSelect={entry.onSelect}
      >
        {entry.label}
      </ContextMenuItem>
    ),
  );
}

type MoreMenuProps = {
  entries: MenuEntry[];
  /** Also shown without hover (section headers, the selected row). */
  alwaysVisible?: boolean;
  icon?: React.ReactNode;
  label?: string;
};

/** The visible "⋯" button of a row: it opens the same menu as the right click. */
export function MoreMenu({ entries, alwaysVisible, icon, label }: MoreMenuProps) {
  const { t } = useTranslation();
  if (entries.length === 0) return null;
  return (
    <DropdownMenu>
      <DropdownMenuTrigger asChild>
        <button
          type="button"
          aria-label={label ?? t("explorer.actions")}
          className={cn(
            "flex size-6 shrink-0 items-center justify-center text-muted-foreground outline-none hover:text-foreground focus-visible:opacity-100 data-[state=open]:opacity-100",
            alwaysVisible ? "opacity-100" : "opacity-0 group-hover:opacity-100",
          )}
        >
          {icon ?? <Ellipsis className="size-4" />}
        </button>
      </DropdownMenuTrigger>
      <DropdownMenuContent align="start">
        {entries.map((entry) =>
          entry.type === "separator" ? (
            <DropdownMenuSeparator key={entry.key} />
          ) : (
            <DropdownMenuItem
              key={entry.key}
              disabled={entry.disabled}
              variant={entry.destructive ? "destructive" : "default"}
              onSelect={entry.onSelect}
            >
              {entry.label}
            </DropdownMenuItem>
          ),
        )}
      </DropdownMenuContent>
    </DropdownMenu>
  );
}
