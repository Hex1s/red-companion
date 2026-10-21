import { useMemo, useState } from "react";
import { useTranslation } from "react-i18next";
import type { NpcTemplate } from "@/bindings/NpcTemplate";
import type { NpcTier } from "@/bindings/NpcTier";
import { Button } from "@/components/ui/button";
import {
  Dialog,
  DialogContent,
  DialogDescription,
  DialogFooter,
  DialogHeader,
  DialogTitle,
} from "@/components/ui/dialog";
import { Input } from "@/components/ui/input";
import { Label } from "@/components/ui/label";
import { cn } from "@/lib/utils";

const TIERS: NpcTier[] = ["mook", "lieutenant", "mini_boss", "boss"];
const COUNT_MIN = 1;
const COUNT_MAX = 10;

export type CloneRequest = {
  templateId: string;
  /** Names of the new NPCs, one per clone. */
  names: string[];
  /** Localized weapons and gear text for the sheet's notes. */
  notes: string;
  intoCombat: boolean;
};

type NpcTemplateDialogProps = {
  open: boolean;
  templates: NpcTemplate[];
  /** Names of every existing character, to number the clones after them. */
  existingNames: string[];
  onClone: (request: CloneRequest) => Promise<void>;
  onClose: () => void;
};

/** Escapes text for use inside a RegExp. */
const escapeRegExp = (text: string) => text.replace(/[.*+?^${}()|[\]\\]/g, "\\$&");

/** "Booster" for the first one alone, otherwise "Booster 2", "Booster 3"... after the existing ones. */
function cloneNames(base: string, count: number, existing: string[]): string[] {
  const same = new RegExp(`^${escapeRegExp(base)}( \\d+)?$`);
  const taken = existing.filter((n) => same.test(n)).length;
  if (taken === 0 && count === 1) return [base];
  return Array.from({ length: count }, (_, i) => `${base} ${taken + i + 1}`);
}

/** Picker for the ready-made NPCs (book p. 412-416): clone one or several, optionally straight into the fight. */
export function NpcTemplateDialog({
  open,
  templates,
  existingNames,
  onClone,
  onClose,
}: NpcTemplateDialogProps) {
  const { t } = useTranslation();
  const [selectedId, setSelectedId] = useState<string | null>(null);
  const [count, setCount] = useState("1");
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState<string | null>(null);

  const selected = templates.find((x) => x.id === selectedId) ?? null;
  const countValue = Number(count);
  const countValid =
    Number.isInteger(countValue) && countValue >= COUNT_MIN && countValue <= COUNT_MAX;

  const groups = useMemo(
    () => TIERS.map((tier) => [tier, templates.filter((x) => x.tier === tier)] as const),
    [templates],
  );

  const notesOf = (template: NpcTemplate) =>
    [
      `${t("npc.notes.weapons")}: ${template.weapons
        .map((w) => `${t(w.name_key)} ${w.damage}`)
        .join(", ")}`,
      `${t("npc.notes.gear")}: ${t(template.gear_key)}`,
      template.notes_key ? t(template.notes_key) : null,
    ]
      .filter(Boolean)
      .join("\n");

  const clone = async (intoCombat: boolean) => {
    if (!selected || !countValid) return;
    setBusy(true);
    setError(null);
    try {
      await onClone({
        templateId: selected.id,
        names: cloneNames(t(selected.name_key), countValue, existingNames),
        notes: notesOf(selected),
        intoCombat,
      });
      onClose();
    } catch (e) {
      setError(t("npc.clone_error", { error: e instanceof Error ? e.message : String(e) }));
    } finally {
      setBusy(false);
    }
  };

  return (
    <Dialog open={open} onOpenChange={(o) => !o && onClose()}>
      <DialogContent className="sm:max-w-2xl" aria-describedby={undefined}>
        <DialogHeader>
          <DialogTitle>{t("npc.dialog_title")}</DialogTitle>
          <DialogDescription>{t("npc.dialog_hint")}</DialogDescription>
        </DialogHeader>

        <div className="max-h-[50vh] space-y-3 overflow-y-auto pr-1">
          {groups.map(([tier, items]) =>
            items.length === 0 ? null : (
              <div key={tier} className="space-y-1">
                <div className="text-xs text-muted-foreground">{t(`npc.tier.${tier}`)}</div>
                <ul className="space-y-1">
                  {items.map((item) => (
                    <li key={item.id}>
                      <button
                        type="button"
                        onClick={() => setSelectedId(item.id)}
                        className={cn(
                          "cut flex w-full items-baseline gap-3 px-3 py-1.5 text-left [--cut:4px]",
                          item.id === selectedId && "[--cut-border:var(--primary)]",
                        )}
                      >
                        <span className="flex-1 font-display text-sm font-semibold uppercase">
                          {t(item.name_key)}
                        </span>
                        <span className="num text-xs text-muted-foreground">
                          {t("armor.head")} {item.armor_head_sp} / {t("armor.body")} {item.armor_body_sp}
                        </span>
                      </button>
                    </li>
                  ))}
                </ul>
              </div>
            ),
          )}
        </div>

        {selected && (
          <p className="whitespace-pre-line text-xs text-muted-foreground">{notesOf(selected)}</p>
        )}
        {error && <p className="text-sm text-destructive">{error}</p>}

        <DialogFooter className="items-center">
          <Label htmlFor="npc-clone-count" className="mr-auto font-normal">
            {t("npc.count")}
            <Input
              id="npc-clone-count"
              type="number"
              min={COUNT_MIN}
              max={COUNT_MAX}
              value={count}
              onChange={(e) => setCount(e.target.value)}
              aria-invalid={!countValid}
              className="num ml-2 inline-block h-8 w-16"
            />
          </Label>
          <Button type="button" variant="outline" onClick={onClose}>
            {t("characters.cancel")}
          </Button>
          <Button
            type="button"
            variant="outline"
            disabled={!selected || !countValid || busy}
            onClick={() => void clone(false)}
          >
            {t("npc.clone")}
          </Button>
          <Button
            type="button"
            disabled={!selected || !countValid || busy}
            onClick={() => void clone(true)}
          >
            {t("npc.clone_to_combat")}
          </Button>
        </DialogFooter>
      </DialogContent>
    </Dialog>
  );
}
