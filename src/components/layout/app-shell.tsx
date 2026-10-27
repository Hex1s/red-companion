import { useState, type ReactNode } from "react";
import { GlitchFilters } from "@/components/layout/glitch-filters";
import { StatusArea } from "@/components/layout/status-area";
import { CHARACTER_DRAG_TYPE, TEMPLATE_DRAG_TYPE, draggedCharacter, draggedTemplate } from "@/lib/drag";
import { cn } from "@/lib/utils";

type AppShellProps = {
  header: ReactNode;
  /** Left panel with the session/folder tree; the grid column is dropped while it is hidden. */
  explorer: ReactNode;
  explorerOpen: boolean;
  main: ReactNode;
  rail: ReactNode;
  /** A character from the explorer was dropped on the rail: it joins the fight. */
  onRailDrop: (characterId: number) => void;
  /** An NPC template was dropped on the rail: a fresh copy joins the fight. */
  onRailDropTemplate: (templateKey: string) => void;
};

/** Shell: header, explorer on the left, main text area, right rail with character cards. */
export function AppShell({ header, explorer, explorerOpen, main, rail, onRailDrop, onRailDropTemplate }: AppShellProps) {
  const [railTarget, setRailTarget] = useState(false);
  return (
    <div className="grid h-dvh grid-rows-[auto_minmax(0,1fr)] overflow-hidden bg-background text-foreground">
      <GlitchFilters />
      <header className="relative flex items-center gap-4 border-b px-4 py-2 after:absolute after:bottom-[-1px] after:left-0 after:h-0.5 after:w-24 after:bg-brand">{header}</header>
      <div
        className={cn(
          "grid min-h-0 grid-rows-[minmax(0,1fr)]",
          explorerOpen
            ? "grid-cols-[250px_minmax(0,1fr)_320px]"
            : "grid-cols-[minmax(0,1fr)_320px]",
        )}
      >
        {explorerOpen && (
          <aside className="min-h-0 overflow-y-auto border-r bg-card/40 p-2">{explorer}</aside>
        )}
        {/* The scroll lives in <main>; the status area is a sibling, so it stays in the corner. */}
        <div className="relative min-h-0 min-w-0">
          <main className="h-full overflow-y-auto p-6">{main}</main>
          <StatusArea />
        </div>
        <aside
          className={cn(
            "min-h-0 space-y-3 overflow-y-auto border-l bg-card/40 p-3 pt-4 pl-4",
            railTarget && "outline-2 -outline-offset-2 outline-primary",
          )}
          onDragOver={(e) => {
            const types = e.dataTransfer.types;
            if (!types.includes(CHARACTER_DRAG_TYPE) && !types.includes(TEMPLATE_DRAG_TYPE)) return;
            e.preventDefault();
            e.dataTransfer.dropEffect = "copy";
            setRailTarget(true);
          }}
          onDragLeave={(e) => {
            if (!e.currentTarget.contains(e.relatedTarget as Node | null)) setRailTarget(false);
          }}
          onDrop={(e) => {
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
          }}
        >
          {rail}
        </aside>
      </div>
    </div>
  );
}
