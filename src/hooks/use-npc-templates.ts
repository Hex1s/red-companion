import { useCallback, useEffect, useState } from "react";
import { useTranslation } from "react-i18next";
import type { NpcTemplate } from "@/bindings/NpcTemplate";
import { api, hasBackend } from "@/lib/api";
import { useContentStore } from "@/store/content-store";
import { errorMessage } from "@/lib/errors";
import { status } from "@/store/status-store";

/** Ready-made NPC templates (server content): reloaded when the language changes. */
export function useNpcTemplates(): NpcTemplate[] {
  const { t, i18n } = useTranslation();
  const [templates, setTemplates] = useState<NpcTemplate[]>([]);
  const contentVersion = useContentStore((s) => s.version);

  const load = useCallback(() => {
    api.listNpcTemplates(i18n.language).then(
      (list) => {
        setTemplates(list);
        status.dismiss("npc-templates:load");
      },
      (e: unknown) => {
        setTemplates([]);
        status.fail(e, "npc-templates:load", {
          text: t("npc.load_error", { error: errorMessage(e) }),
          retry: { label: t("status.retry"), run: load },
        });
      },
    );
  }, [i18n.language, t, contentVersion]);

  useEffect(() => {
    if (hasBackend) load();
  }, [load]);

  return templates;
}
