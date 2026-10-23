import type { ReactNode } from "react";
import { StatusArea } from "@/components/layout/status-area";

type AppShellProps = {
  header: ReactNode;
  main: ReactNode;
  rail: ReactNode;
};

/** Shell: header with tabs, main text area, right rail with character cards. */
export function AppShell({ header, main, rail }: AppShellProps) {
  return (
    <div className="grid h-dvh grid-rows-[auto_minmax(0,1fr)] overflow-hidden bg-background text-foreground">
      <header className="flex items-center gap-4 border-b px-4 py-2">{header}</header>
      <div className="grid min-h-0 grid-cols-[minmax(0,1fr)_320px] grid-rows-[minmax(0,1fr)]">
        {/* The scroll lives in <main>; the status area is a sibling, so it stays in the corner. */}
        <div className="relative min-h-0 min-w-0">
          <main className="h-full overflow-y-auto p-6">{main}</main>
          <StatusArea />
        </div>
        <aside className="min-h-0 space-y-3 overflow-y-auto border-l bg-card/40 p-3">{rail}</aside>
      </div>
    </div>
  );
}
