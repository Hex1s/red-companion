import { useCallback, useEffect, useState } from "react";
import type { Character } from "@/bindings/Character";
import type { CharacterKind } from "@/bindings/CharacterKind";
import type { CharacterRecord } from "@/bindings/CharacterRecord";
import { api, hasBackend } from "@/lib/api";

const KINDS: CharacterKind[] = ["pc", "npc"];

export type CharacterEntry = CharacterRecord & { kind: CharacterKind };

const message = (e: unknown) => (e instanceof Error ? e.message : String(e));

/** Loads all characters from the Rust backend and wraps the CRUD commands. */
export function useCharacters() {
  const [entries, setEntries] = useState<CharacterEntry[]>([]);
  const [error, setError] = useState<string | null>(null);

  const reload = useCallback(async () => {
    if (!hasBackend) return;
    try {
      const lists = await Promise.all(KINDS.map((kind) => api.listCharacters(kind)));
      setEntries(lists.flatMap((records, i) => records.map((r) => ({ ...r, kind: KINDS[i] }))));
      setError(null);
    } catch (e) {
      setError(message(e));
    }
  }, []);

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
      await api.deleteCharacter(id);
      await reload();
    },
    [reload],
  );

  return { entries, error, create, save, remove, reload };
}

export { message as errorMessage };
