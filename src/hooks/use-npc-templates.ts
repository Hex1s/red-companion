import { useEffect, useState } from "react";
import { useTranslation } from "react-i18next";
import type { NpcTemplate } from "@/bindings/NpcTemplate";
import { api, hasBackend } from "@/lib/api";

/** Ready-made NPC templates (server content): reloaded when the language changes. */
export function useNpcTemplates(): NpcTemplate[] {
  const { i18n } = useTranslation();
  const [templates, setTemplates] = useState<NpcTemplate[]>([]);

  useEffect(() => {
    if (!hasBackend) return;
    api.listNpcTemplates(i18n.language).then(setTemplates, () => setTemplates([]));
  }, [i18n.language]);

  return templates;
}
