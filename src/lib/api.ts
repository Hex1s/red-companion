import { invoke } from "@tauri-apps/api/core";
import type { Character } from "@/bindings/Character";
import type { CharacterKind } from "@/bindings/CharacterKind";
import type { CharacterRecord } from "@/bindings/CharacterRecord";
import type { CoverPreset } from "@/bindings/CoverPreset";
import type { SceneRecord } from "@/bindings/SceneRecord";
import type { Skill } from "@/bindings/Skill";

/** True inside the Tauri window; false in plain `npm run dev` in a browser. */
export const hasBackend = "__TAURI_INTERNALS__" in globalThis;

export const api = {
  listSkills: () => invoke<Skill[]>("list_skills"),
  listCharacters: (kind: CharacterKind) => invoke<CharacterRecord[]>("list_characters", { kind }),
  createCharacter: (kind: CharacterKind, name: string) =>
    invoke<CharacterRecord>("create_character", { kind, name }),
  saveCharacter: (id: number, character: Character) =>
    invoke<CharacterRecord>("save_character", { id, character }),
  deleteCharacter: (id: number) => invoke<void>("delete_character", { id }),
  getScene: () => invoke<SceneRecord>("get_scene"),
  listCoverPresets: () => invoke<CoverPreset[]>("list_cover_presets"),
  addCover: (preset: string | null, name: string, hp: number | null) =>
    invoke<SceneRecord>("add_cover", { preset, name, hp }),
  editCover: (coverId: number, name: string, hp: number, maxHp: number) =>
    invoke<SceneRecord>("edit_cover", { coverId, name, hp, maxHp }),
  removeCover: (coverId: number) => invoke<SceneRecord>("remove_cover", { coverId }),
  setCharacterCover: (characterId: number, coverId: number | null) =>
    invoke<SceneRecord>("set_character_cover", { characterId, coverId }),
};
