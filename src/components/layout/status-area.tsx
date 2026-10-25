import { useState } from "react";
import { AlertCircle, Check, Info, Loader2, X } from "lucide-react";
import { useTranslation } from "react-i18next";
import { Button } from "@/components/ui/button";
import { useStatusStore, type StatusItem, type StatusKind } from "@/store/status-store";
import { cn } from "@/lib/utils";

const ICON: Record<StatusKind, typeof Check> = {
  loading: Loader2,
  success: Check,
  error: AlertCircle,
  info: Info,
};
const TONE: Record<StatusKind, string> = {
  loading: "text-muted-foreground",
  success: "text-state-ok",
  error: "text-destructive-text",
  info: "text-foreground",
};
const BORDER: Record<StatusKind, string> = {
  loading: "[--cut-border:var(--border)]",
  success: "[--cut-border:var(--state-ok)]",
  error: "[--cut-border:var(--destructive)]",
  info: "[--cut-border:var(--muted-foreground)]",
};

/**
 * Status area in the bottom right corner of the work area: saving (spinner → check), results
 * of actions, errors. The parent is `relative`; messages never take part in the page layout.
 */
export function StatusArea() {
  const items = useStatusStore((s) => s.items);
  if (items.length === 0) return null;
  return (
    <div className="pointer-events-none absolute right-4 bottom-4 z-40 flex max-w-sm flex-col items-end gap-2">
      {items.map((item) => (
        <StatusMessage key={item.id} item={item} />
      ))}
    </div>
  );
}

function StatusMessage({ item }: { item: StatusItem }) {
  const { t } = useTranslation();
  const dismiss = useStatusStore((s) => s.dismiss);
  const Icon = ICON[item.kind];
  const [open, setOpen] = useState(false);
  return (
    <div
      role={item.kind === "error" ? "alert" : "status"}
      className={cn(
        "cut pointer-events-auto flex items-start gap-2 px-3 py-2 text-sm [--cut:8px] duration-200",
        BORDER[item.kind],
        item.closing
          ? "animate-out fade-out-0 slide-out-to-bottom-2"
          : "animate-in fade-in-0 slide-in-from-bottom-2",
      )}
    >
      <Icon
        className={cn("mt-0.5 size-4 shrink-0", TONE[item.kind], item.kind === "loading" && "animate-spin")}
      />
      <div className="min-w-0 flex-1">
        <span className="break-words">{item.text}</span>
        {item.details && open && (
          <pre className="mt-1 whitespace-pre-wrap break-words text-xs text-muted-foreground select-text">
            {item.details}
          </pre>
        )}
      </div>
      {item.action && (
        <Button
          type="button"
          variant="link"
          size="xs"
          className="shrink-0"
          onClick={item.action.run}
        >
          {item.action.label}
        </Button>
      )}
      {item.details && (
        <Button
          type="button"
          variant="link"
          size="xs"
          className="shrink-0"
          aria-expanded={open}
          onClick={() => setOpen((v) => !v)}
        >
          {t("status.details")}
        </Button>
      )}
      {item.kind === "error" && (
        <Button
          type="button"
          variant="ghost"
          size="icon-xs"
          className="shrink-0"
          aria-label={t("status.close")}
          onClick={() => dismiss(item.id)}
        >
          <X />
        </Button>
      )}
    </div>
  );
}
