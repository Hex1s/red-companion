import { useState } from "react";
import { useTranslation } from "react-i18next";
import { Button } from "@/components/ui/button";
import { Loader2 } from "@/components/ui/icons";
import { useContentStatus } from "@/hooks/use-content-status";
import { api } from "@/lib/api";
import { contentLang, explainMissing, packsToEnable } from "@/lib/content-gap";
import { cn } from "@/lib/utils";
import { useContentStore } from "@/store/content-store";
import { status } from "@/store/status-store";

type MissingReferenceProps = {
  /** The server collection the stored code points into (`npc_templates`, `martial_arts`...). */
  collection: string;
  /** The collection's list is in memory (not empty): only then can an entry be called absent. */
  listLoaded: boolean;
  className?: string;
};

/**
 * Says why a stored code is not in the references and offers the fix that exists: download the
 * collection, or switch its pack back on. An entry that is simply not there gets an explanation only.
 */
export function MissingReference({ collection, listLoaded, className }: MissingReferenceProps) {
  const { t, i18n } = useTranslation();
  const content = useContentStatus();
  const bump = useContentStore((s) => s.bump);
  const setManagerOpen = useContentStore((s) => s.setManagerOpen);
  const [busy, setBusy] = useState(false);
  const lang = contentLang(i18n.language);
  const reason = explainMissing(collection, lang, content, listLoaded);
  if (!reason || !content) return null;

  const fix = async () => {
    setBusy(true);
    try {
      if (reason === "not_downloaded") {
        await api.syncContent({ langs: [lang], collections: [collection] });
      } else {
        for (const pack of packsToEnable(collection, lang, content)) await api.setPackEnabled(pack, true);
      }
      bump();
    } catch (e) {
      status.fail(e, "content:fix");
    } finally {
      setBusy(false);
    }
  };

  return (
    <span className={cn("flex flex-wrap items-center gap-2 text-xs text-muted-foreground", className)}>
      <span>{t(`content.missing.${reason}`, { collection: t(`content.collection.${collection}`) })}</span>
      {reason === "absent" ? (
        <Button type="button" size="xs" variant="outline" onClick={() => setManagerOpen(true)}>
          {t("content.button")}
        </Button>
      ) : (
        <Button type="button" size="xs" variant="outline" disabled={busy} onClick={() => void fix()}>
          {busy && <Loader2 className="size-3.5 animate-spin" />}
          {t(`content.missing.fix_${reason}`)}
        </Button>
      )}
    </span>
  );
}
