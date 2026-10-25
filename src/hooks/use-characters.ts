import { useCallback, useEffect, useState } from "react";
import type { Character } from "@/bindings/Character";
import type { CharacterKind } from "@/bindings/CharacterKind";
import type { CharacterRecord } from "@/bindings/CharacterRecord";
import type { DeletedCharacter } from "@/bindings/DeletedCharacter";
import { api, hasBackend } from "@/lib/api";

const KINDS: CharacterKind[] = ["pc", "npc"];

export type CharacterEntry = CharacterRecord & { kind: CharacterKind };

/**
 * Loads all characters from the Rust backend and wraps the CRUD commands. Nothing is asked until
 * a session is active (`sessionId`), because the first command would otherwise create a default
 * one. `refreshTree` reloads the explorer: a new character joins the active session, a deleted one
 * leaves every container.
 */
export function useCharacters(sessionId: number | null, refreshTree: () => Promise<void>) {
  const [entries, setEntries] = useState<CharacterEntry[]>([]);
  const [error, setError] = useState<unknown>(null);

  const reload = useCallback(async () => {
    if (!hasBackend || sessionId === null) return;
    try {
      const [lists] = await Promise.all([
        Promise.all(KINDS.map((kind) => api.listCharacters(kind))),
        refreshTree(),
      ]);
      setEntries(lists.flatMap((records, i) => records.map((r) => ({ ...r, kind: KINDS[i] }))));
      setError(null);
    } catch (e) {
      setError(e ?? "unknown");
    }
  }, [sessionId, refreshTree]);

  useEffect(() => {
    void reload();
  }, [reload]);

  const create = useCallback(
    async (kind: CharacterKind, name: string) => {
      const record = await api.createCharacter(kind, name);
      await reload();
      return record;
    },
    [reload],
  );

  const save = useCallback(
    async (id: number, character: Character) => {
      const record = await api.saveCharacter(id, character);
      await reload();
      return record;
    },
    [reload],
  );

  const remove = useCallback(
    async (id: number) => {
      const deleted = await api.deleteCharacter(id);
      await reload();
      return deleted;
    },
    [reload],
  );

  const restore = useCallback(
    async (deleted: DeletedCharacter) => {
      await api.restoreCharacter(deleted);
      await reload();
    },
    [reload],
  );

  return { entries, error, create, save, remove, restore, reload };
}
