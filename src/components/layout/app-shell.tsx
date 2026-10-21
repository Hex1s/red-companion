import type { ReactNode } from "react";

type AppShellProps = {
  header: ReactNode;
  main: ReactNode;
  rail: ReactNode;
};

/** Shell: header with tabs, main text area, right rail with character cards. */
export function AppShell({ header, main, rail }: AppShellProps) {
  return (
    <div className="grid h-screen grid-rows-[auto_1fr] bg-background text-foreground">
      <header className="flex items-center gap-4 border-b px-4 py-2">{header}</header>
      <div className="grid min-h-0 grid-cols-[1fr_280px]">
        <main className="min-w-0 overflow-y-auto p-6">{main}</main>
        <aside className="space-y-3 overflow-y-auto border-l bg-card/40 p-3">{rail}</aside>
      </div>
    </div>
  );
}
