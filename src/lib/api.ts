import { invoke } from "@tauri-apps/api/core";
import type { Character } from "@/bindings/Character";
import type { CharacterKind } from "@/bindings/CharacterKind";
import type { CharacterRecord } from "@/bindings/CharacterRecord";
import type { Armor } from "@/bindings/Armor";
import type { BodyArea } from "@/bindings/BodyArea";
import type { CoverPreset } from "@/bindings/CoverPreset";
import type { CriticalInjury } from "@/bindings/CriticalInjury";
import type { DamageApplied } from "@/bindings/DamageApplied";
import type { DamageOutcome } from "@/bindings/DamageOutcome";
import type { DamageRequest } from "@/bindings/DamageRequest";
import type { Derived } from "@/bindings/Derived";
import type { Encounter } from "@/bindings/Encounter";
import type { EncounterRecord } from "@/bindings/EncounterRecord";
import type { Assessment } from "@/bindings/Assessment";
import type { NpcTier } from "@/bindings/NpcTier";
import type { ExportFile } from "@/bindings/ExportFile";
import type { UserTemplateRecord } from "@/bindings/UserTemplateRecord";
import type { ExportSaved } from "@/bindings/ExportSaved";
import type { DeathSaveApplied } from "@/bindings/DeathSaveApplied";
import type { InjuryRoll } from "@/bindings/InjuryRoll";
import type { RollResult } from "@/bindings/RollResult";
import type { SceneRecord } from "@/bindings/SceneRecord";
import type { StabilizeApplied } from "@/bindings/StabilizeApplied";
import type { NpcTemplate } from "@/bindings/NpcTemplate";
import type { Skill } from "@/bindings/Skill";
import type { SkillCheck } from "@/bindings/SkillCheck";
import type { Weapon } from "@/bindings/Weapon";

/**
 * Server content (cover, armor, NPCs...) comes in the content language, which
 * follows the UI language: the backend is told first, then asked.
 */
async function inLanguage<T>(lang: string, load: () => Promise<T>): Promise<T> {
  await invoke("set_content_lang", { code: lang });
  return load();
}

/** True inside the Tauri window; false in plain `npm run dev` in a browser. */
export const hasBackend = "__TAURI_INTERNALS__" in globalThis;

export const api = {
  listArmor: (lang: string) => inLanguage(lang, () => invoke<Armor[]>("list_armor")),
  listWeapons: (lang: string) => inLanguage(lang, () => invoke<Weapon[]>("list_weapons")),
  listSkills: () => invoke<Skill[]>("list_skills"),
  listNpcTemplates: (lang: string) =>
    inLanguage(lang, () => invoke<NpcTemplate[]>("list_npc_templates")),
  /** Clones a ready-made NPC into a new NPC sheet; `notes` is the localized weapons and gear text. */
  createNpcFromTemplate: (templateId: string, name: string, notes: string) =>
    invoke<CharacterRecord>("create_npc_from_template", { templateId, name, notes }),
  listUserTemplates: () => invoke<UserTemplateRecord[]>("list_user_templates"),
  /** Copies a character's sheet (fresh HP and armor) into the GM's own templates; `templateId` overwrites one. */
  saveCharacterAsTemplate: (characterId: number, templateId: number | null) =>
    invoke<UserTemplateRecord>("save_character_as_template", { characterId, templateId }),
  setUserTemplateTier: (id: number, tier: NpcTier) =>
    invoke<UserTemplateRecord>("set_user_template_tier", { id, tier }),
  /** How strong an encounter is against `players` edgerunners; information only. */
  assessEncounter: (encounter: Encounter, players: number) =>
    invoke<Assessment>("assess_encounter", { encounter, players }),
  deleteUserTemplate: (id: number) => invoke<void>("delete_user_template", { id }),
  createNpcFromUserTemplate: (templateId: number, name: string) =>
    invoke<CharacterRecord>("create_npc_from_user_template", { templateId, name }),
  listEncounters: () => invoke<EncounterRecord[]>("list_encounters"),
  /** Creates an encounter (`id` null) or overwrites one. */
  saveEncounter: (id: number | null, encounter: Encounter) =>
    invoke<EncounterRecord>("save_encounter", { id, encounter }),
  deleteEncounter: (id: number) => invoke<void>("delete_encounter", { id }),
  listCharacters: (kind: CharacterKind) => invoke<CharacterRecord[]>("list_characters", { kind }),
  createCharacter: (kind: CharacterKind, name: string) =>
    invoke<CharacterRecord>("create_character", { kind, name }),
  saveCharacter: (id: number, character: Character) =>
    invoke<CharacterRecord>("save_character", { id, character }),
  deleteCharacter: (id: number) => invoke<void>("delete_character", { id }),
  /** Writes `ids` (or everyone when `null`) into a new file in the exports folder. */
  exportCharacters: (ids: number[] | null) => invoke<ExportSaved>("export_characters", { ids }),
  /** Export files in the exports folder, the newest first. */
  listExports: () => invoke<ExportFile[]>("list_exports"),
  /** Imports one file of the exports folder as new characters; rejects the whole file if anything is invalid. */
  importExportFile: (name: string) => invoke<CharacterRecord[]>("import_export_file", { name }),
  openExportsFolder: () => invoke<void>("open_exports_folder"),
  getScene: () => invoke<SceneRecord>("get_scene"),
  listCoverPresets: (lang: string) =>
    inLanguage(lang, () => invoke<CoverPreset[]>("list_cover_presets")),
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
  /** Brings a dead character back: 1 HP, no death save penalty, no injuries. */
  reviveCharacter: (id: number) => invoke<CharacterRecord>("revive_character", { id }),
  undoDamage: () => invoke<DamageApplied>("undo_damage"),
  /** Rolls REF + 1d10: adds to the order, or re-rolls; `manual` = faces of physical dice. */
  combatRoll: (characterId: number, manual: number[] | null) =>
    invoke<SceneRecord>("combat_roll", { characterId, manual }),
  /** Derived numbers of an unsaved sheet; the formulas live in Rust. */
  previewDerived: (character: Character) => invoke<Derived>("preview_derived", { character }),
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
  /**
   * Stabilisation: the helper's TECH + skill + d10 against the wound's DV. `manualTotal` is the
   * die typed in from physical dice, `granted` skips the roll (the GM declares success).
   */
  stabilize: (
    targetId: number,
    helperId: number,
    skill: string,
    manualTotal: number | null,
    granted: boolean,
  ) => invoke<StabilizeApplied>("stabilize", { targetId, helperId, skill, manualTotal, granted }),
  /** STAT + skill level + d10 of a saved sheet; `manualTotal` = the die typed in from a physical one. */
  skillCheck: (characterId: number, skill: string, manualTotal: number | null) =>
    invoke<SkillCheck>("skill_check", { characterId, skill, manualTotal }),
};
