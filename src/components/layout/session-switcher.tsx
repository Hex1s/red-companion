import { useState } from "react";
import { useTranslation } from "react-i18next";
import type { SessionInfo } from "@/bindings/SessionInfo";
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
import { Select, SelectContent, SelectItem, SelectTrigger, SelectValue } from "@/components/ui/select";

const NAME_MAX = 60;

type SessionSwitcherProps = {
  sessions: SessionInfo[];
  disabled?: boolean;
  onSwitch: (id: number) => void;
  onCreate: (name: string) => Promise<boolean>;
  onRename: (id: number, name: string) => Promise<boolean>;
  onDelete: (id: number) => Promise<boolean>;
};

type Dialogs = "create" | "rename" | "delete" | null;

/** Header control: pick the active session, make a new one, rename or delete the current one. */
export function SessionSwitcher({
  sessions,
  disabled,
  onSwitch,
  onCreate,
  onRename,
  onDelete,
}: SessionSwitcherProps) {
  const { t } = useTranslation();
  const active = sessions.find((s) => s.active) ?? null;
  const [dialog, setDialog] = useState<Dialogs>(null);
  const [name, setName] = useState("");
  const trimmed = name.trim();
  const valid = trimmed.length >= 1 && trimmed.length <= NAME_MAX;

  const open = (kind: Exclude<Dialogs, null>) => {
    setName(
      kind === "create"
        ? t("sessions.default_name", { n: sessions.length + 1 })
        : kind === "rename"
          ? (active?.name ?? "")
          : "",
    );
    setDialog(kind);
  };

  const submit = async () => {
    if (!active) return;
    const ok =
      dialog === "create"
        ? await onCreate(trimmed)
        : dialog === "rename"
          ? await onRename(active.id, trimmed)
          : await onDelete(active.id);
    // A failure is shown in the status area; the dialog stays so the name can be fixed.
    if (ok) setDialog(null);
  };

  return (
    <div className="flex items-center gap-1">
      <Select
        value={active ? String(active.id) : ""}
        disabled={disabled || sessions.length === 0}
        onValueChange={(id) => onSwitch(Number(id))}
      >
        <SelectTrigger className="h-8 w-48" aria-label={t("sessions.current")}>
          <SelectValue placeholder={t("sessions.current")} />
        </SelectTrigger>
        <SelectContent>
          {sessions.map((s) => (
            <SelectItem key={s.id} value={String(s.id)}>
              {s.name}
            </SelectItem>
          ))}
        </SelectContent>
      </Select>
      <Button size="xs" variant="outline" disabled={disabled} onClick={() => open("create")}>
        {t("sessions.new")}
      </Button>
      <Button size="xs" variant="outline" disabled={disabled || !active} onClick={() => open("rename")}>
        {t("sessions.rename")}
      </Button>
      <Button size="xs" variant="outline" disabled={disabled || !active} onClick={() => open("delete")}>
        {t("sessions.delete")}
      </Button>

      <Dialog open={dialog !== null} onOpenChange={(o) => !o && setDialog(null)}>
        <DialogContent>
          <DialogHeader>
            <DialogTitle>{t(`sessions.${dialog ?? "create"}_title`)}</DialogTitle>
            {dialog === "delete" && (
              <DialogDescription>{t("sessions.delete_body", { name: active?.name ?? "" })}</DialogDescription>
            )}
          </DialogHeader>
          {dialog !== "delete" && (
            <Input
              autoFocus
              value={name}
              maxLength={NAME_MAX}
              aria-label={t("sessions.name")}
              onChange={(e) => setName(e.target.value)}
              onKeyDown={(e) => e.key === "Enter" && valid && void submit()}
            />
          )}
          <DialogFooter>
            <Button type="button" variant="outline" onClick={() => setDialog(null)}>
              {t("characters.cancel")}
            </Button>
            <Button
              type="button"
              disabled={dialog !== "delete" && !valid}
              onClick={() => void submit()}
            >
              {t(dialog === "create" ? "sessions.create" : dialog === "rename" ? "sessions.save" : "sessions.delete")}
            </Button>
          </DialogFooter>
        </DialogContent>
      </Dialog>
    </div>
  );
}
