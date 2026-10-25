import { useEffect, useRef, useState } from "react";
import { useTranslation } from "react-i18next";
import type { Encounter } from "@/bindings/Encounter";
import type { EncounterEntry } from "@/bindings/EncounterEntry";
import type { EncounterRecord } from "@/bindings/EncounterRecord";
import type { NpcTemplate } from "@/bindings/NpcTemplate";
import type { UserTemplateRecord } from "@/bindings/UserTemplateRecord";
import { Button } from "@/components/ui/button";
import { ErrorNotice } from "@/components/ui/error-notice";
import {
  Dialog,
  DialogContent,
  DialogDescription,
  DialogFooter,
  DialogHeader,
  DialogTitle,
} from "@/components/ui/dialog";
import { FieldBadge } from "@/components/ui/field-badge";
import { Input } from "@/components/ui/input";
import { NumberInput } from "@/components/ui/number-input";
import { Label } from "@/components/ui/label";
import {
  Select,
  SelectContent,
  SelectGroup,
  SelectItem,
  SelectLabel,
  SelectTrigger,
  SelectValue,
} from "@/components/ui/select";
import { errorMessage } from "@/lib/errors";
import { templateKey, templateLabel } from "@/lib/encounter";

/** Same limits as `rules_core::encounter`. */
const COUNT_MIN = 1;
const COUNT_MAX = 20;

type Line = { key: number; template: string; count: string };

type EncounterEditorProps = {
  open: boolean;
  /** `null` creates a new encounter. */
  initial: EncounterRecord | null;
  bookTemplates: NpcTemplate[];
  userTemplates: UserTemplateRecord[];
  onSave: (id: number | null, encounter: Encounter) => Promise<unknown>;
  onClose: () => void;
};

const toEntry = (line: Line): EncounterEntry => {
  const [source, template] = line.template.split(/:(.*)/s);
  return {
    source: source === "custom" ? "custom" : "book",
    template,
    count: Number(line.count),
  };
};

/** Encounter editor: name, note and lines "template x count" (book or own templates). */
export function EncounterEditor({
  open,
  initial,
  bookTemplates,
  userTemplates,
  onSave,
  onClose,
}: EncounterEditorProps) {
  const { t } = useTranslation();
  const nextKey = useRef(0);
  const [name, setName] = useState("");
  const [notes, setNotes] = useState("");
  const [lines, setLines] = useState<Line[]>([]);
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState<string | null>(null);

  const newLine = (template: string, count: string): Line => ({
    key: nextKey.current++,
    template,
    count,
  });

  // Every opening starts from the encounter being edited (or an empty form).
  useEffect(() => {
    if (!open) return;
    setName(initial?.encounter.name ?? "");
    setNotes(initial?.encounter.notes ?? "");
    setLines(
      (initial?.encounter.entries ?? []).map((e) =>
        newLine(templateKey(e.source, e.template), String(e.count)),
      ),
    );
    setError(null);
    // eslint-disable-next-line react-hooks/exhaustive-deps -- reset only when the dialog opens
  }, [open, initial]);

  const known = new Set([
    ...bookTemplates.map((x) => templateKey("book", x.code)),
    ...userTemplates.map((x) => templateKey("custom", String(x.id))),
  ]);
  const countValid = (line: Line) => {
    const n = Number(line.count);
    return Number.isInteger(n) && n >= COUNT_MIN && n <= COUNT_MAX;
  };
  const nameValid = name.trim().length > 0;
  const canSave =
    nameValid && lines.length > 0 && lines.every((l) => countValid(l) && known.has(l.template));

  const addLine = () => {
    const first =
      bookTemplates.length > 0
        ? templateKey("book", bookTemplates[0].code)
        : userTemplates.length > 0
          ? templateKey("custom", String(userTemplates[0].id))
          : null;
    if (first) setLines((prev) => [...prev, newLine(first, "1")]);
  };

  const update = (key: number, patch: Partial<Line>) =>
    setLines((prev) => prev.map((l) => (l.key === key ? { ...l, ...patch } : l)));

  const save = async () => {
    if (!canSave) return;
    setBusy(true);
    setError(null);
    try {
      await onSave(initial?.id ?? null, { name, notes, entries: lines.map(toEntry) });
      onClose();
    } catch (e) {
      setError(t("encounters.save_error", { error: errorMessage(e) }));
    } finally {
      setBusy(false);
    }
  };

  return (
    <Dialog open={open} onOpenChange={(o) => !o && onClose()}>
      <DialogContent className="sm:max-w-2xl" aria-describedby={undefined}>
        <DialogHeader>
          <DialogTitle>
            {initial ? t("encounters.edit_title") : t("encounters.new_title")}
          </DialogTitle>
          <DialogDescription>{t("encounters.editor_hint")}</DialogDescription>
        </DialogHeader>

        <div className="space-y-3">
          <div className="space-y-1">
            <Label htmlFor="encounter-name">{t("encounters.name")}</Label>
            <FieldBadge error={nameValid ? null : t("characters.validation.name")}>
              {(invalid) => (
                <Input
                  id="encounter-name"
                  value={name}
                  onChange={(e) => setName(e.target.value)}
                  aria-invalid={invalid}
                />
              )}
            </FieldBadge>
          </div>

          <div className="space-y-1">
            <Label htmlFor="encounter-notes">{t("encounters.notes")}</Label>
            <textarea
              id="encounter-notes"
              rows={2}
              value={notes}
              onChange={(e) => setNotes(e.target.value)}
              className="w-full border border-input bg-transparent px-3 py-2 text-sm outline-none focus-visible:border-ring"
            />
          </div>

          <div className="space-y-2">
            <div className="text-xs text-muted-foreground uppercase">{t("encounters.lines")}</div>
            <ul className="max-h-[32vh] space-y-2 overflow-y-auto pr-1">
              {lines.map((line) => {
                const missing = !known.has(line.template);
                return (
                  <li key={line.key} className="flex items-center gap-2">
                    <Select
                      value={line.template}
                      onValueChange={(v) => update(line.key, { template: v })}
                    >
                      <SelectTrigger className="flex-1" aria-label={t("encounters.template")}>
                        <SelectValue>
                          {missing
                            ? t("encounters.missing_template")
                            : templateLabel(line.template, bookTemplates, userTemplates)}
                        </SelectValue>
                      </SelectTrigger>
                      <SelectContent>
                        {bookTemplates.length > 0 && (
                          <SelectGroup>
                            <SelectLabel>{t("encounters.book_group")}</SelectLabel>
                            {bookTemplates.map((x) => (
                              <SelectItem key={x.code} value={templateKey("book", x.code)}>
                                {x.name}
                              </SelectItem>
                            ))}
                          </SelectGroup>
                        )}
                        {userTemplates.length > 0 && (
                          <SelectGroup>
                            <SelectLabel>{t("encounters.own_group")}</SelectLabel>
                            {userTemplates.map((x) => (
                              <SelectItem
                                key={x.id}
                                value={templateKey("custom", String(x.id))}
                              >
                                {x.character.name}
                              </SelectItem>
                            ))}
                          </SelectGroup>
                        )}
                      </SelectContent>
                    </Select>
                    <FieldBadge
                      error={countValid(line) ? null : t("validation.count", { max: COUNT_MAX })}
                    >
                      {(invalid) => (
                        <NumberInput
                          min={COUNT_MIN}
                          max={COUNT_MAX}
                          value={line.count}
                          onChange={(e) => update(line.key, { count: e.target.value })}
                          aria-label={t("npc.count")}
                          aria-invalid={invalid}
                          className="num h-8 w-16"
                        />
                      )}
                    </FieldBadge>
                    <Button
                      type="button"
                      size="xs"
                      variant="outline"
                      onClick={() => setLines((prev) => prev.filter((l) => l.key !== line.key))}
                    >
                      {t("encounters.remove_line")}
                    </Button>
                  </li>
                );
              })}
            </ul>
            <Button
              type="button"
              size="xs"
              variant="outline"
              disabled={bookTemplates.length === 0 && userTemplates.length === 0}
              onClick={addLine}
            >
              {t("encounters.add_line")}
            </Button>
          </div>
        </div>

        <ErrorNotice message={error} />

        <DialogFooter>
          <Button type="button" variant="outline" onClick={onClose}>
            {t("characters.cancel")}
          </Button>
          <Button type="button" disabled={!canSave || busy} onClick={() => void save()}>
            {t("encounters.save")}
          </Button>
        </DialogFooter>
      </DialogContent>
    </Dialog>
  );
}
