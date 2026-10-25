import { useTranslation } from "react-i18next";
import { Dialog, DialogContent, DialogDescription, DialogHeader, DialogTitle } from "@/components/ui/dialog";
import { HOTKEYS, HOTKEY_IDS, formatHotkey, type HotkeyGroup } from "@/lib/hotkeys";

const GROUPS: HotkeyGroup[] = ["interface", "combat", "navigation"];

type ShortcutsDialogProps = {
  open: boolean;
  onOpenChange: (open: boolean) => void;
};

/** The cheat sheet: every shortcut of the registry, by group. */
export function ShortcutsDialog({ open, onOpenChange }: ShortcutsDialogProps) {
  const { t } = useTranslation();
  return (
    <Dialog open={open} onOpenChange={onOpenChange}>
      <DialogContent className="max-w-md">
        <DialogHeader>
          <DialogTitle>{t("hotkeys.title")}</DialogTitle>
          <DialogDescription>{t("hotkeys.rule")}</DialogDescription>
        </DialogHeader>
        <div className="space-y-4">
          {GROUPS.map((group) => (
            <section key={group} className="space-y-1">
              <h3 className="text-xs text-muted-foreground">{t(`hotkeys.group.${group}`)}</h3>
              <ul className="divide-y divide-border">
                {HOTKEY_IDS.filter((id) => HOTKEYS[id].group === group).map((id) => (
                  <li key={id} className="flex items-center justify-between gap-3 py-1.5 text-sm">
                    <span>{t(`hotkeys.action.${id}`)}</span>
                    <kbd className="num shrink-0 border border-border bg-muted px-1.5 py-0.5 text-xs">
                      {formatHotkey(id)}
                    </kbd>
                  </li>
                ))}
              </ul>
            </section>
          ))}
        </div>
      </DialogContent>
    </Dialog>
  );
}
