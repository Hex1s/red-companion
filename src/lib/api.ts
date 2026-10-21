import { invoke } from "@tauri-apps/api/core";
import type { Character } from "@/bindings/Character";
import type { CharacterKind } from "@/bindings/CharacterKind";
import type { CharacterRecord } from "@/bindings/CharacterRecord";
import type { BodyArea } from "@/bindings/BodyArea";
import type { CoverPreset } from "@/bindings/CoverPreset";
import type { CriticalInjury } from "@/bindings/CriticalInjury";
import type { DamageApplied } from "@/bindings/DamageApplied";
import type { DamageOutcome } from "@/bindings/DamageOutcome";
import type { DamageRequest } from "@/bindings/DamageRequest";
import type { DeathSaveApplied } from "@/bindings/DeathSaveApplied";
import type { InjuryRoll } from "@/bindings/InjuryRoll";
import type { RollResult } from "@/bindings/RollResult";
import type { SceneRecord } from "@/bindings/SceneRecord";
import type { StabilizeApplied } from "@/bindings/StabilizeApplied";
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
  listInjuries: () => invoke<CriticalInjury[]>("list_injuries"),
  /** Rolls `notation`, or builds the result from faces read off physical dice. */
  rollDice: (notation: string, manual: number[] | null) =>
    invoke<RollResult>("roll_dice", { notation, manual }),
  rollInjury: (characterId: number, area: BodyArea, manual: number[] | null) =>
    invoke<InjuryRoll>("roll_injury", { characterId, area, manual }),
  previewDamage: (characterId: number, request: DamageRequest) =>
    invoke<DamageOutcome>("preview_damage", { characterId, request }),
  applyDamage: (characterId: number, request: DamageRequest, injuryId: string | null) =>
    invoke<DamageApplied>("apply_damage", { characterId, request, injuryId }),
  undoDamage: () => invoke<DamageApplied>("undo_damage"),
  /** Adds to the fight with REF + 1d10; `manual` = faces of physical dice (a second one after 10 or 1). */
  combatAdd: (characterId: number, manual: number[] | null) =>
    invoke<SceneRecord>("combat_add", { characterId, manual }),
  combatReroll: (characterId: number, manual: number[] | null) =>
    invoke<SceneRecord>("combat_reroll", { characterId, manual }),
  combatSetInitiative: (characterId: number, initiative: number) =>
    invoke<SceneRecord>("combat_set_initiative", { characterId, initiative }),
  combatShift: (characterId: number, delta: number) =>
    invoke<SceneRecord>("combat_shift", { characterId, delta }),
  combatRemove: (characterId: number) => invoke<SceneRecord>("combat_remove", { characterId }),
  combatStart: () => invoke<SceneRecord>("combat_start"),
  combatNextTurn: () => invoke<SceneRecord>("combat_next_turn"),
  combatEnd: () => invoke<SceneRecord>("combat_end"),
  deathSave: (characterId: number, manual: number | null) =>
    invoke<DeathSaveApplied>("death_save", { characterId, manual }),
  stabilize: (targetId: number, helperId: number, skill: string, manual: number[] | null) =>
    invoke<StabilizeApplied>("stabilize", { targetId, helperId, skill, manual }),
};
