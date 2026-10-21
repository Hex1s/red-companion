import { invoke } from "@tauri-apps/api/core";
import type { Character } from "@/bindings/Character";
import type { CharacterKind } from "@/bindings/CharacterKind";
import type { CharacterRecord } from "@/bindings/CharacterRecord";

/** True inside the Tauri window; false in plain `npm run dev` in a browser. */
export const hasBackend = "__TAURI_INTERNALS__" in globalThis;

export const api = {
  listCharacters: (kind: CharacterKind) => invoke<CharacterRecord[]>("list_characters", { kind }),
  createCharacter: (kind: CharacterKind, name: string) =>
    invoke<CharacterRecord>("create_character", { kind, name }),
  saveCharacter: (id: number, character: Character) =>
    invoke<CharacterRecord>("save_character", { id, character }),
  deleteCharacter: (id: number) => invoke<void>("delete_character", { id }),
};
