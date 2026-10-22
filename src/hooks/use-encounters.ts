import { useCallback, useEffect, useState } from "react";
import type { Encounter } from "@/bindings/Encounter";
import type { EncounterRecord } from "@/bindings/EncounterRecord";
import type { NpcTier } from "@/bindings/NpcTier";
import type { UserTemplateRecord } from "@/bindings/UserTemplateRecord";
import { api, hasBackend } from "@/lib/api";

const message = (e: unknown) => (e instanceof Error ? e.message : String(e));

/** The GM's own NPC templates and encounters (user data in SQLite) with their CRUD commands. */
export function useEncounters() {
  const [templates, setTemplates] = useState<UserTemplateRecord[]>([]);
  const [encounters, setEncounters] = useState<EncounterRecord[]>([]);
  const [error, setError] = useState<string | null>(null);

  const reload = useCallback(async () => {
    if (!hasBackend) return;
    try {
      const [userTemplates, userEncounters] = await Promise.all([
        api.listUserTemplates(),
        api.listEncounters(),
      ]);
      setTemplates(userTemplates);
      setEncounters(userEncounters);
      setError(null);
    } catch (e) {
      setError(message(e));
    }
  }, []);

  useEffect(() => {
    void reload();
  }, [reload]);

  const saveEncounter = useCallback(
    async (id: number | null, encounter: Encounter) => {
      const record = await api.saveEncounter(id, encounter);
      await reload();
      return record;
    },
    [reload],
  );

  const deleteEncounter = useCallback(
    async (id: number) => {
      await api.deleteEncounter(id);
      await reload();
    },
    [reload],
  );

  const setTemplateTier = useCallback(
    async (id: number, tier: NpcTier) => {
      await api.setUserTemplateTier(id, tier);
      await reload();
    },
    [reload],
  );

  const deleteTemplate = useCallback(
    async (id: number) => {
      await api.deleteUserTemplate(id);
      await reload();
    },
    [reload],
  );

  return { templates, encounters, error, reload, saveEncounter, deleteEncounter, deleteTemplate, setTemplateTier };
}
