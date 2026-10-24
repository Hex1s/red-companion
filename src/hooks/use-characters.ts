import { useCallback, useEffect, useState } from "react";
import type { Character } from "@/bindings/Character";
import type { CharacterKind } from "@/bindings/CharacterKind";
import type { CharacterRecord } from "@/bindings/CharacterRecord";
import { api, hasBackend } from "@/lib/api";

const KINDS: CharacterKind[] = ["pc", "npc"];

export type CharacterEntry = CharacterRecord & { kind: CharacterKind };

/**
 * Loads all characters from the Rust backend and wraps the CRUD commands. `members` are the ids
 * of the active session's characters; nothing is asked until a session is active (`sessionId`),
 * and everything is reloaded when it changes.
 */
export function useCharacters(sessionId: number | null) {
  const [entries, setEntries] = useState<CharacterEntry[]>([]);
  const [members, setMembers] = useState<number[]>([]);
  const [error, setError] = useState<unknown>(null);

  const reload = useCallback(async () => {
    if (!hasBackend || sessionId === null) return;
    try {
      const [lists, ids] = await Promise.all([
        Promise.all(KINDS.map((kind) => api.listCharacters(kind))),
        api.sessionMembers(),
      ]);
      setEntries(lists.flatMap((records, i) => records.map((r) => ({ ...r, kind: KINDS[i] }))));
      setMembers(ids);
      setError(null);
    } catch (e) {
      setError(e ?? "unknown");
    }
  }, [sessionId]);

  useEffect(() => {
    void reload();
  }, [reload]);

  const addToSession = useCallback(async (id: number) => {
    setMembers(await api.addToSession(id));
  }, []);

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
      await api.deleteCharacter(id);
      await reload();
    },
    [reload],
  );

  return { entries, members, error, create, save, remove, reload, addToSession };
}
