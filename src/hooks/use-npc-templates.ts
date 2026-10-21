import { useEffect, useState } from "react";
import type { NpcTemplate } from "@/bindings/NpcTemplate";
import { api, hasBackend } from "@/lib/api";

/** Ready-made NPC templates from the Rust data layer (loaded once). */
export function useNpcTemplates(): NpcTemplate[] {
  const [templates, setTemplates] = useState<NpcTemplate[]>([]);

  useEffect(() => {
    if (!hasBackend) return;
    api.listNpcTemplates().then(setTemplates, () => setTemplates([]));
  }, []);

  return templates;
}
