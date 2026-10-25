import { invoke } from "@tauri-apps/api/core";
import type { Character } from "@/bindings/Character";
import type { CharacterKind } from "@/bindings/CharacterKind";
import type { CharacterRecord } from "@/bindings/CharacterRecord";
import type { DeletedCharacter } from "@/bindings/DeletedCharacter";
import type { Armor } from "@/bindings/Armor";
import type { Role } from "@/bindings/Role";
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
import type { TierHint } from "@/bindings/TierHint";
import type { NpcTier } from "@/bindings/NpcTier";
import type { DeflectionInfo } from "@/bindings/DeflectionInfo";
import type { ExportFile } from "@/bindings/ExportFile";
import type { ExportKind } from "@/bindings/ExportKind";
import type { UserTemplateRecord } from "@/bindings/UserTemplateRecord";
import type { ExportSaved } from "@/bindings/ExportSaved";
import type { EncountersImported } from "@/bindings/EncountersImported";
import type { EncountersSaved } from "@/bindings/EncountersSaved";
import type { DeathSaveApplied } from "@/bindings/DeathSaveApplied";
import type { InjuryRoll } from "@/bindings/InjuryRoll";
import type { RollResult } from "@/bindings/RollResult";
import type { SceneRecord } from "@/bindings/SceneRecord";
import type { ExplorerNode } from "@/bindings/ExplorerNode";
import type { SessionInfo } from "@/bindings/SessionInfo";
import type { StabilizeApplied } from "@/bindings/StabilizeApplied";
import type { NpcTemplate } from "@/bindings/NpcTemplate";
import type { Skill } from "@/bindings/Skill";
import type { SkillCheck } from "@/bindings/SkillCheck";
import type { DrugApplied } from "@/bindings/DrugApplied";
import type { HealApplied } from "@/bindings/HealApplied";
import type { HealKind } from "@/bindings/HealKind";
import type { HumanityLossApplied } from "@/bindings/HumanityLossApplied";
import type { TherapyApplied } from "@/bindings/TherapyApplied";
import type { TherapyInfo } from "@/bindings/TherapyInfo";
import type { TherapyKind } from "@/bindings/TherapyKind";
import type { RestApplied } from "@/bindings/RestApplied";
import type { RestPlan } from "@/bindings/RestPlan";
import type { TreatmentSkill } from "@/bindings/TreatmentSkill";
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
  listRoles: (lang: string) => inLanguage(lang, () => invoke<Role[]>("list_roles")),
  listNpcTemplates: (lang: string) =>
    inLanguage(lang, () => invoke<NpcTemplate[]>("list_npc_templates")),
  /** Clones a ready-made NPC into a new NPC sheet; `notes` is the localized weapons and gear text. */
  /** A fresh NPC copy (an instance) in `sessionId`, or in the active session when `null`. */
  createNpcFromTemplate: (templateId: string, name: string, notes: string, sessionId: number | null = null) =>
    invoke<CharacterRecord>("create_npc_from_template", { templateId, name, notes, sessionId }),
  /** The tier a sheet's stats are closest to; information only. */
  tierHint: (character: Character) => invoke<TierHint>("tier_hint", { character }),
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
  createNpcFromUserTemplate: (templateId: number, name: string, sessionId: number | null = null) =>
    invoke<CharacterRecord>("create_npc_from_user_template", { templateId, name, sessionId }),
  /** The sheet a book template starts from, to open it in the editor before the first edit. */
  bookTemplateSheet: (code: string, notes: string) =>
    invoke<Character>("book_template_sheet", { code, notes }),
  /** The first (and every later) edit of a book template: stores the GM's copy, which replaces it. */
  saveBookTemplateCopy: (code: string, character: Character) =>
    invoke<UserTemplateRecord>("save_book_template_copy", { code, character }),
  /** Saves the edited sheet of an own template. */
  updateUserTemplate: (id: number, character: Character) =>
    invoke<UserTemplateRecord>("update_user_template", { id, character }),
  /** Keeps an instance: it becomes a library character and no longer goes away with its session. */
  saveInstanceToLibrary: (characterId: number) =>
    invoke<ExplorerNode[]>("save_instance_to_library", { characterId }),
  /** Deletes every instance of a session; the snapshots restore them one by one. */
  clearSessionInstances: (sessionId: number) =>
    invoke<DeletedCharacter[]>("clear_session_instances", { sessionId }),
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
  /** Deletes at once; the snapshot it returns is what `restoreCharacter` needs to undo it. */
  deleteCharacter: (id: number) => invoke<DeletedCharacter>("delete_character", { id }),
  restoreCharacter: (deleted: DeletedCharacter) => invoke<void>("restore_character", { deleted }),
  /** Writes `ids` (or everyone when `null`) into a new file in the exports folder. */
  exportCharacters: (ids: number[] | null) => invoke<ExportSaved>("export_characters", { ids }),
  /** Writes the participants of the active session into a session file named after the session. */
  exportSession: () => invoke<ExportSaved>("export_session"),
  /** Files an import window of this type offers (its own folder, plus older untyped files), newest first. */
  listExports: (kind: ExportKind) => invoke<ExportFile[]>("list_exports", { kind }),
  /** Imports one character or session file as new characters; rejects the whole file if anything is invalid. */
  importExportFile: (kind: ExportKind, name: string) =>
    invoke<CharacterRecord[]>("import_export_file", { kind, name }),
  /** Shows the exports folder, or the sub-folder of one type, in the file manager. */
  openExportsFolder: (kind?: ExportKind) =>
    invoke<void>("open_exports_folder", { kind: kind ?? null }),
  /**
   * Writes encounters and own NPC templates into a new `*.encounters.redcompanion.json` file; the own
   * templates an encounter uses go along with it.
   */
  exportEncounters: (encounterIds: number[], templateIds: number[]) =>
    invoke<EncountersSaved>("export_encounters", { encounterIds, templateIds }),
  /** Imports one encounters file as new templates and encounters; rejects the whole file if anything is invalid. */
  importEncounterFile: (name: string) =>
    invoke<EncountersImported>("import_encounter_file", { name }),
  /** Creates a session with an empty scene and switches to it. */
  createSession: (name: string) => invoke<SessionInfo[]>("create_session", { name }),
  sessionNotes: (id: number) => invoke<string>("session_notes", { id }),
  setSessionNotes: (id: number, notes: string) => invoke<void>("set_session_notes", { id, notes }),
  switchSession: (id: number) => invoke<SessionInfo[]>("switch_session", { id }),
  /** The explorer tree as a flat list: sessions and folders with the characters lying in them. */
  explorerTree: () => invoke<ExplorerNode[]>("explorer_tree"),
  /** Creates a folder at the root (`parent` null) or inside a root folder. */
  createFolder: (parent: number | null, name: string) =>
    invoke<ExplorerNode[]>("create_folder", { parent, name }),
  renameNode: (id: number, name: string) => invoke<ExplorerNode[]>("rename_node", { id, name }),
  /** Moves a folder to the root (`parent` null) or into a root folder (one level of nesting). */
  moveFolder: (id: number, parent: number | null) =>
    invoke<ExplorerNode[]>("move_folder", { id, parent }),
  /** Deletes a session (with its scene and fight) or a folder; the characters stay. */
  deleteNode: (id: number) => invoke<ExplorerNode[]>("delete_node", { id }),
  /** Puts a shortcut to a character into a session or a folder. */
  addToContainer: (containerId: number, characterId: number) =>
    invoke<ExplorerNode[]>("add_to_container", { containerId, characterId }),
  /** Takes the shortcut out (a session's covers and fight too); the character is kept. */
  removeFromContainer: (containerId: number, characterId: number) =>
    invoke<ExplorerNode[]>("remove_from_container", { containerId, characterId }),
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
  /**
   * One attempt to heal an injury with `skill` (a way its table row offers). First aid and
   * paramedic use the healer's TECH + level; surgery needs the typed-in `bonus`. `manualTotal`
   * is a physical d10, `granted` counts it as a success without a roll.
   */
  /** A Medtech gives a drug they picked (p. 150); the rules apply the effect and the dose limit. */
  administerDrug: (targetId: number, helperId: number, drug: string) =>
    invoke<DrugApplied>("administer_drug", { targetId, helperId, drug }),
  healInjury: (
    targetId: number,
    helperId: number,
    injuryId: string,
    kind: HealKind,
    skill: TreatmentSkill,
    bonus: number | null,
    manualTotal: number | null,
    granted: boolean,
  ) =>
    invoke<HealApplied>("heal_injury", {
      targetId,
      helperId,
      injuryId,
      kind,
      skill,
      bonus,
      manualTotal,
      granted,
    }),
  /** Solo's Damage Deflection for the damage window: how much, and whether it is the first hit of the round. */
  deflectionInfo: (characterId: number) =>
    invoke<DeflectionInfo>("deflection_info", { characterId }),
  /** The GM puts an injury on the sheet by hand; it may repeat (a second broken leg). */
  addInjury: (id: number, injuryId: string) =>
    invoke<CharacterRecord>("add_injury", { id, injuryId }),
  /** The GM takes one instance of an injury off by hand. */
  removeInjury: (id: number, injuryId: string) =>
    invoke<CharacterRecord>("remove_injury", { id, injuryId }),
  /** Days of rest needed to reach `targetHp`; changes nothing. */
  restPreview: (id: number, targetHp: number) => invoke<RestPlan>("rest_preview", { id, targetHp }),
  restCharacter: (id: number, targetHp: number) =>
    invoke<RestApplied>("rest_character", { id, targetHp }),
  /** Takes humanity off; below zero is cyberpsychosis. */
  loseHumanity: (id: number, loss: number) =>
    invoke<HumanityLossApplied>("lose_humanity", { id, loss }),
  listTherapies: () => invoke<TherapyInfo[]>("list_therapies"),
  /**
   * A week of therapy. `bonus` is the Medtech's typed-in modifier; `manualTotal` a physical d10
   * for the check, `granted` declares it passed; `restoreFaces` the faces of the 2d6 / 4d6.
   */
  therapy: (
    id: number,
    kind: TherapyKind,
    bonus: number,
    manualTotal: number | null,
    granted: boolean,
    restoreFaces: number[] | null,
  ) => invoke<TherapyApplied>("therapy", { id, kind, bonus, manualTotal, granted, restoreFaces }),
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
  /** Puts a character into the fight with a rolled initiative; someone already there keeps theirs. */
  combatAdd: (characterId: number) => invoke<SceneRecord>("combat_add", { characterId }),
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
