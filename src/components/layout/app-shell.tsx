import { useState, type DragEvent, type ReactNode } from "react";
import { useTranslation } from "react-i18next";
import { Drawer } from "@/components/ui/drawer";
import { useLayoutMode } from "@/hooks/use-layout-mode";
import { GlitchFilters } from "@/components/layout/glitch-filters";
import { StatusArea } from "@/components/layout/status-area";
import { CHARACTER_DRAG_TYPE, TEMPLATE_DRAG_TYPE, draggedCharacter, draggedTemplate } from "@/lib/drag";
import { cn } from "@/lib/utils";

type AppShellProps = {
  header: ReactNode;
  /** Left panel with the session/folder tree; the grid column is dropped while it is hidden. */
  explorer: ReactNode;
  /** Pinned explorer column, wide layout only; the narrower layouts use the drawer below. */
  explorerOpen: boolean;
  explorerDrawerOpen: boolean;
  onExplorerDrawerChange: (open: boolean) => void;
  railDrawerOpen: boolean;
  onRailDrawerChange: (open: boolean) => void;
  main: ReactNode;
  rail: ReactNode;
  /** Bottom bar of the narrow layout; opens the rail drawer. */
  dock: ReactNode;
  /** A character from the explorer was dropped on the rail: it joins the fight. */
  onRailDrop: (characterId: number) => void;
  /** An NPC template was dropped on the rail: a fresh copy joins the fight. */
  onRailDropTemplate: (templateKey: string) => void;
};

/**
 * Shell: header, explorer on the left, main text area, right rail with character cards.
 * Panels follow the layout mode: wide = three columns; medium = the explorer is a drawer;
 * narrow = both panels are drawers.
 */
export function AppShell({
  header,
  explorer,
  explorerOpen,
  explorerDrawerOpen,
  onExplorerDrawerChange,
  railDrawerOpen,
  onRailDrawerChange,
  main,
  rail,
  dock,
  onRailDrop,
  onRailDropTemplate,
}: AppShellProps) {
  const { t } = useTranslation();
  const mode = useLayoutMode();
  const [railTarget, setRailTarget] = useState(false);

  const showExplorerColumn = mode === "wide" && explorerOpen;
  const showRailColumn = mode !== "narrow";

  // Drag-and-drop is a mouse affordance: the inline rail accepts drops, the drawer does not.
  const railDropProps = {
    onDragOver: (e: DragEvent) => {
      const types = e.dataTransfer.types;
      if (!types.includes(CHARACTER_DRAG_TYPE) && !types.includes(TEMPLATE_DRAG_TYPE)) return;
      e.preventDefault();
      e.dataTransfer.dropEffect = "copy";
      setRailTarget(true);
    },
    onDragLeave: (e: DragEvent) => {
      if (!e.currentTarget.contains(e.relatedTarget as Node | null)) setRailTarget(false);
    },
    onDrop: (e: DragEvent) => {
      const id = draggedCharacter(e);
      const template = draggedTemplate(e);
      setRailTarget(false);
      if (id !== null) {
        e.preventDefault();
        onRailDrop(id);
      } else if (template !== null) {
        e.preventDefault();
        onRailDropTemplate(template);
      }
    },
  };

  return (
    <div className={cn("grid h-dvh overflow-hidden bg-background pt-[env(safe-area-inset-top)] pr-[env(safe-area-inset-right)] pb-[env(safe-area-inset-bottom)] pl-[env(safe-area-inset-left)] text-foreground", mode === "narrow" ? "grid-rows-[auto_minmax(0,1fr)_auto]" : "grid-rows-[auto_minmax(0,1fr)]")}>
      <GlitchFilters />
      <header className="relative flex items-center gap-2 border-b px-3 py-2 after:absolute after:bottom-[-1px] after:left-0 after:h-0.5 after:w-24 after:bg-brand mid:gap-4 mid:px-4">{header}</header>
      <div
        className={cn(
          "grid min-h-0 grid-rows-[minmax(0,1fr)]",
          showExplorerColumn && "grid-cols-[250px_minmax(0,1fr)_320px]",
          !showExplorerColumn && mode === "wide" && "grid-cols-[minmax(0,1fr)_320px]",
          mode === "medium" && "grid-cols-[minmax(0,1fr)_280px]",
          mode === "narrow" && "grid-cols-[minmax(0,1fr)]",
        )}
      >
        {showExplorerColumn && (
          <aside className="min-h-0 overflow-y-auto border-r bg-card/40 p-2">{explorer}</aside>
        )}
        {/* The scroll lives in <main>; the status area is a sibling, so it stays in the corner. */}
        <div className="relative min-h-0 min-w-0">
          <main className="h-full overflow-y-auto p-4 mid:p-6">{main}</main>
          <StatusArea />
        </div>
        {showRailColumn && (
          <aside
            className={cn(
              "min-h-0 space-y-3 overflow-y-auto border-l bg-card/40 p-3 pt-4 pl-4",
              railTarget && "outline-2 -outline-offset-2 outline-primary",
            )}
            {...railDropProps}
          >
            {rail}
          </aside>
        )}
      </div>
      {mode === "narrow" && dock}
      {mode !== "wide" && (
        <Drawer
          side="left"
          title={t("explorer.title")}
          open={explorerDrawerOpen}
          onOpenChange={onExplorerDrawerChange}
        >
          <div className="min-h-0 flex-1 overflow-y-auto p-2 pt-10">{explorer}</div>
        </Drawer>
      )}
      {mode === "narrow" && (
        <Drawer
          side="right"
          title={t("rail.title")}
          open={railDrawerOpen}
          onOpenChange={onRailDrawerChange}
        >
          <div className="min-h-0 flex-1 space-y-3 overflow-y-auto p-3 pt-10">{rail}</div>
        </Drawer>
      )}
    </div>
  );
}
