import { Ellipsis } from "@/components/ui/icons";
import { useTranslation } from "react-i18next";
import {
  ContextMenuItem,
  ContextMenuPortal,
  ContextMenuSeparator,
  ContextMenuSub,
  ContextMenuSubContent,
  ContextMenuSubTrigger,
} from "@/components/ui/context-menu";
import {
  DropdownMenu,
  DropdownMenuContent,
  DropdownMenuItem,
  DropdownMenuPortal,
  DropdownMenuSeparator,
  DropdownMenuSub,
  DropdownMenuSubContent,
  DropdownMenuSubTrigger,
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
    }
  /** A nested list ("Move to…"): the touch replacement for dragging onto a target. */
  | { type: "sub"; key: string; label: string; entries: MenuEntry[]; disabled?: boolean };

export const item = (
  key: string,
  label: string,
  onSelect: () => void,
  options: { disabled?: boolean; destructive?: boolean } = {},
): MenuEntry => ({ type: "item", key, label, onSelect, ...options });

export const submenu = (
  key: string,
  label: string,
  entries: MenuEntry[],
  options: { disabled?: boolean } = {},
): MenuEntry => ({ type: "sub", key, label, entries: tidy(entries), ...options });

export const separator = (key: string): MenuEntry => ({ type: "separator", key });

/** Drops separators that would open, close or double the list (menus assemble entries by condition). */
export function tidy(entries: MenuEntry[]): MenuEntry[] {
  return entries.filter((entry, i) => {
    if (entry.type !== "separator") return true;
    const before = entries[i - 1];
    const after = entries[i + 1];
    return before !== undefined && before.type !== "separator" && after !== undefined && after.type !== "separator";
  });
}

export function ContextEntries({ entries }: { entries: MenuEntry[] }) {
  return entries.map((entry) => {
    if (entry.type === "separator") return <ContextMenuSeparator key={entry.key} />;
    if (entry.type === "sub") {
      return (
        <ContextMenuSub key={entry.key}>
          <ContextMenuSubTrigger disabled={entry.disabled}>{entry.label}</ContextMenuSubTrigger>
          <ContextMenuPortal>
            <ContextMenuSubContent>
              <ContextEntries entries={entry.entries} />
            </ContextMenuSubContent>
          </ContextMenuPortal>
        </ContextMenuSub>
      );
    }
    return (
      <ContextMenuItem
        key={entry.key}
        disabled={entry.disabled}
        variant={entry.destructive ? "destructive" : "default"}
        onSelect={entry.onSelect}
      >
        {entry.label}
      </ContextMenuItem>
    );
  });
}

function DropdownEntries({ entries }: { entries: MenuEntry[] }) {
  return entries.map((entry) => {
    if (entry.type === "separator") return <DropdownMenuSeparator key={entry.key} />;
    if (entry.type === "sub") {
      return (
        <DropdownMenuSub key={entry.key}>
          <DropdownMenuSubTrigger disabled={entry.disabled}>{entry.label}</DropdownMenuSubTrigger>
          <DropdownMenuPortal>
            <DropdownMenuSubContent>
              <DropdownEntries entries={entry.entries} />
            </DropdownMenuSubContent>
          </DropdownMenuPortal>
        </DropdownMenuSub>
      );
    }
    return (
      <DropdownMenuItem
        key={entry.key}
        disabled={entry.disabled}
        variant={entry.destructive ? "destructive" : "default"}
        onSelect={entry.onSelect}
      >
        {entry.label}
      </DropdownMenuItem>
    );
  });
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
            "flex size-6 touch:size-11 shrink-0 items-center justify-center text-muted-foreground outline-none hover:text-foreground focus-visible:opacity-100 data-[state=open]:opacity-100",
            alwaysVisible ? "opacity-100" : "opacity-0 group-hover:opacity-100 no-hover:opacity-100",
          )}
        >
          {icon ?? <Ellipsis className="size-4" />}
        </button>
      </DropdownMenuTrigger>
      <DropdownMenuContent align="start">
        <DropdownEntries entries={entries} />
      </DropdownMenuContent>
    </DropdownMenu>
  );
}
