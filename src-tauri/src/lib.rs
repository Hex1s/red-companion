use std::path::PathBuf;
use std::sync::Mutex;

use rules_core::RulesInfo;
use rules_core::catalog::{Armor, BodyArea, CriticalInjury, NpcTemplate, NpcTier, Skill, Weapon};
use rules_core::character::{Character, CharacterKind, CharacterRecord, Derived};
use rules_core::damage::DamageOutcome;
use rules_core::damage::WoundState;
use rules_core::damage_event::{self, DamageApplied, DamageRequest, InjuryRoll};
use rules_core::data_source::{DataSource, Lang, SeedDataSource};
use rules_core::dice::{
    CheckRoll, DiceError, DiceNotation, RollResult, ThreadDice, check_manual, check_total, roll, roll_check,
    roll_manual,
};
use rules_core::skill_check::{self, SkillCheck};
use rules_core::scene::{CoverPreset, Scene, SceneRecord, SessionInfo, cover_presets};
use rules_core::encounter_transfer::{self, EncountersImported, EncountersSaved};
use rules_core::transfer::{self, ExportFile, ExportSaved, ExportedCharacter};
use rules_core::vitals::{self, DeathSaveApplied, STABILIZE_SKILLS, StabilizeApplied};
use rules_core::encounter::{
    self, Assessment, Encounter, EncounterRecord, TemplateSource, UserTemplateDoc, UserTemplateRecord,
};
use storage::{CharacterRow, DocRow, DocTable, Storage};
use tauri::{AppHandle, Manager, State};
use tauri_plugin_opener::OpenerExt;

/// Folder the export files live in. It is created at startup, so the GM always knows where
/// exports go and where imports come from.
pub struct ExportsDir(pub PathBuf);

/// User database, opened once at startup in the app data directory.
pub struct AppStorage(pub Mutex<Storage>);

/// Language of server content (names of cover, armor, NPCs...). Set by the UI
/// from its own language; there is one server per language, the built-in
/// snapshot stands in for it.
pub struct ContentLang(pub Mutex<Lang>);

/// Reference data source in the current content language.
fn content(state: &State<'_, ContentLang>) -> Result<SeedDataSource, String> {
    let lang = *state.0.lock().map_err(|_| "content language lock poisoned".to_string())?;
    Ok(SeedDataSource::new(lang))
}

/// Switches the content language (`ru`, `en`, `ru-RU`...); unknown codes mean English.
#[tauri::command]
fn set_content_lang(state: State<'_, ContentLang>, code: String) -> Result<(), String> {
    *state.0.lock().map_err(|_| "content language lock poisoned".to_string())? = Lang::from_code(&code);
    Ok(())
}

#[tauri::command]
fn rules_info() -> RulesInfo {
    rules_core::rules_info()
}

#[tauri::command]
fn list_skills() -> Result<Vec<Skill>, String> {
    SeedDataSource::default().skills().map(|c| c.entries).map_err(|e| e.to_string())
}

#[tauri::command]
fn list_armor(lang: State<'_, ContentLang>) -> Result<Vec<Armor>, String> {
    content(&lang)?.armor().map(|c| c.entries).map_err(|e| e.to_string())
}

#[tauri::command]
fn list_weapons(lang: State<'_, ContentLang>) -> Result<Vec<Weapon>, String> {
    content(&lang)?.weapons().map(|c| c.entries).map_err(|e| e.to_string())
}

#[tauri::command]
fn list_npc_templates(lang: State<'_, ContentLang>) -> Result<Vec<NpcTemplate>, String> {
    content(&lang)?.npc_templates().map(|c| c.entries).map_err(|e| e.to_string())
}

/// Clones a ready-made NPC into a new NPC sheet: own HP and SP, so one
/// template can be cloned many times. `notes` is the localized weapons and
/// gear text composed by the UI.
#[tauri::command]
fn create_npc_from_template(
    state: State<'_, AppStorage>,
    lang: State<'_, ContentLang>,
    template_id: String,
    name: String,
    notes: String,
) -> Result<CharacterRecord, String> {
    let templates = content(&lang)?.npc_templates().map_err(|e| e.to_string())?;
    let template = templates
        .entries
        .iter()
        .find(|t| t.code == template_id)
        .ok_or_else(|| format!("NPC template {template_id:?} not found"))?;
    let skills = SeedDataSource::default().skills().map_err(|e| e.to_string())?;
    let character = template.to_character(&name, &notes, &skills.entries);
    character.validate().map_err(|e| e.to_string())?;
    let data = serde_json::to_string(&character).map_err(|e| e.to_string())?;
    let storage = lock(&state)?;
    let id = storage
        .insert_character(storage::CharacterKind::Npc, &character.name, &data)
        .map_err(|e| e.to_string())?;
    join_active_session(&storage, id)?;
    Ok(CharacterRecord::new(id, character))
}

fn user_template_record(row: &DocRow) -> Result<UserTemplateRecord, String> {
    let doc: UserTemplateDoc = serde_json::from_str(&row.data).map_err(|e| e.to_string())?;
    Ok(UserTemplateRecord { id: row.id, tier: doc.tier, character: doc.character })
}

fn encounter_record(row: &DocRow) -> Result<EncounterRecord, String> {
    let encounter: Encounter = serde_json::from_str(&row.data).map_err(|e| e.to_string())?;
    Ok(EncounterRecord { id: row.id, encounter })
}

#[tauri::command]
fn list_user_templates(state: State<'_, AppStorage>) -> Result<Vec<UserTemplateRecord>, String> {
    let rows = lock(&state)?.list_docs(DocTable::UserTemplates).map_err(|e| e.to_string())?;
    rows.iter().map(user_template_record).collect()
}

/// Saves a copy of a character's sheet as the GM's own NPC template (fresh HP and armor, no
/// injuries). With `template_id` the existing template is overwritten instead of adding a new one.
#[tauri::command]
fn save_character_as_template(
    state: State<'_, AppStorage>,
    character_id: i64,
    template_id: Option<i64>,
) -> Result<UserTemplateRecord, String> {
    let storage = lock(&state)?;
    let row = storage
        .get_character(character_id)
        .map_err(|e| e.to_string())?
        .ok_or_else(|| format!("character {character_id} not found"))?;
    let character = parse_row(&row)?.as_fresh();
    character.validate().map_err(|e| e.to_string())?;
    // Overwriting keeps the tier the GM already chose.
    let tier = match template_id {
        Some(id) => storage
            .get_doc(DocTable::UserTemplates, id)
            .map_err(|e| e.to_string())?
            .map(|row| user_template_record(&row))
            .transpose()?
            .map_or(NpcTier::Mook, |t| t.tier),
        None => NpcTier::Mook,
    };
    let data = serde_json::to_string(&UserTemplateDoc { tier, character: character.clone() })
        .map_err(|e| e.to_string())?;
    let id = match template_id {
        Some(id) => {
            if !storage
                .update_doc(DocTable::UserTemplates, id, &character.name, &data)
                .map_err(|e| e.to_string())?
            {
                return Err(format!("template {id} not found"));
            }
            id
        }
        None => storage
            .insert_doc(DocTable::UserTemplates, &character.name, &data)
            .map_err(|e| e.to_string())?,
    };
    Ok(UserTemplateRecord { id, tier, character })
}

/// Files an own template under a tier (it counts for that much in the encounter assessment).
#[tauri::command]
fn set_user_template_tier(
    state: State<'_, AppStorage>,
    id: i64,
    tier: NpcTier,
) -> Result<UserTemplateRecord, String> {
    let storage = lock(&state)?;
    let row = storage
        .get_doc(DocTable::UserTemplates, id)
        .map_err(|e| e.to_string())?
        .ok_or_else(|| format!("template {id} not found"))?;
    let record = user_template_record(&row)?;
    let data = serde_json::to_string(&UserTemplateDoc { tier, character: record.character.clone() })
        .map_err(|e| e.to_string())?;
    storage.update_doc(DocTable::UserTemplates, id, &row.name, &data).map_err(|e| e.to_string())?;
    Ok(UserTemplateRecord { id, tier, character: record.character })
}

/// Which tier the stats of a sheet are closest to (information only; the GM picks the tier).
#[tauri::command]
fn tier_hint(character: Character) -> Result<encounter::TierHint, String> {
    let skills = SeedDataSource::default().skills().map_err(|e| e.to_string())?;
    Ok(encounter::tier_hint(&character, &skills.entries))
}

/// How strong an encounter is against `players` edgerunners (information only). Lines whose
/// template no longer exists are left out and counted in `skipped`.
#[tauri::command]
fn assess_encounter(
    state: State<'_, AppStorage>,
    lang: State<'_, ContentLang>,
    encounter: Encounter,
    players: u32,
) -> Result<Assessment, String> {
    let book = content(&lang)?.npc_templates().map_err(|e| e.to_string())?;
    let storage = lock(&state)?;
    let (mut weight, mut skipped) = (0, 0);
    for line in &encounter.entries {
        let tier = match line.source {
            TemplateSource::Book => {
                book.entries.iter().find(|t| t.code == line.template).map(|t| t.tier)
            }
            TemplateSource::Custom => match line.template.parse::<i64>() {
                Ok(id) => storage
                    .get_doc(DocTable::UserTemplates, id)
                    .map_err(|e| e.to_string())?
                    .map(|row| user_template_record(&row))
                    .transpose()?
                    .map(|t| t.tier),
                Err(_) => None,
            },
        };
        match tier {
            Some(tier) => weight += encounter::tier_weight(tier) * line.count,
            None => skipped += line.count,
        }
    }
    encounter::assess(weight, players, skipped).map_err(|e| e.to_string())
}

#[tauri::command]
fn delete_user_template(state: State<'_, AppStorage>, id: i64) -> Result<(), String> {
    lock(&state)?.delete_doc(DocTable::UserTemplates, id).map_err(|e| e.to_string())?;
    Ok(())
}

/// Clones the GM's own template into a new NPC sheet under `name`.
#[tauri::command]
fn create_npc_from_user_template(
    state: State<'_, AppStorage>,
    template_id: i64,
    name: String,
) -> Result<CharacterRecord, String> {
    let storage = lock(&state)?;
    let row = storage
        .get_doc(DocTable::UserTemplates, template_id)
        .map_err(|e| e.to_string())?
        .ok_or_else(|| format!("template {template_id} not found"))?;
    let mut character = user_template_record(&row)?.character.as_fresh();
    character.name = name.trim().to_string();
    character.validate().map_err(|e| e.to_string())?;
    let data = serde_json::to_string(&character).map_err(|e| e.to_string())?;
    let id = storage
        .insert_character(storage::CharacterKind::Npc, &character.name, &data)
        .map_err(|e| e.to_string())?;
    join_active_session(&storage, id)?;
    Ok(CharacterRecord::new(id, character))
}

#[tauri::command]
fn list_encounters(state: State<'_, AppStorage>) -> Result<Vec<EncounterRecord>, String> {
    let rows = lock(&state)?.list_docs(DocTable::Encounters).map_err(|e| e.to_string())?;
    rows.iter().map(encounter_record).collect()
}

/// Creates an encounter (`id` is `None`) or overwrites an existing one.
#[tauri::command]
fn save_encounter(
    state: State<'_, AppStorage>,
    id: Option<i64>,
    mut encounter: Encounter,
) -> Result<EncounterRecord, String> {
    encounter.normalize().map_err(|e| e.to_string())?;
    let data = serde_json::to_string(&encounter).map_err(|e| e.to_string())?;
    let storage = lock(&state)?;
    let id = match id {
        Some(id) => {
            if !storage
                .update_doc(DocTable::Encounters, id, &encounter.name, &data)
                .map_err(|e| e.to_string())?
            {
                return Err(format!("encounter {id} not found"));
            }
            id
        }
        None => storage
            .insert_doc(DocTable::Encounters, &encounter.name, &data)
            .map_err(|e| e.to_string())?,
    };
    Ok(EncounterRecord { id, encounter })
}

#[tauri::command]
fn delete_encounter(state: State<'_, AppStorage>, id: i64) -> Result<(), String> {
    lock(&state)?.delete_doc(DocTable::Encounters, id).map_err(|e| e.to_string())?;
    Ok(())
}

fn storage_kind(kind: CharacterKind) -> storage::CharacterKind {
    match kind {
        CharacterKind::Pc => storage::CharacterKind::Pc,
        CharacterKind::Npc => storage::CharacterKind::Npc,
    }
}

/// Parses a stored character. Filler NPCs have no mortal wound: at 0 HP they are dead.
fn parse_row(row: &CharacterRow) -> Result<Character, String> {
    let mut character: Character = serde_json::from_str(&row.data).map_err(|e| e.to_string())?;
    if row.kind == storage::CharacterKind::Npc {
        character.settle_npc();
    }
    Ok(character)
}

fn to_record(row: CharacterRow) -> Result<CharacterRecord, String> {
    let character = parse_row(&row)?;
    Ok(CharacterRecord::new(row.id, character))
}

fn lock<'a>(
    state: &'a State<'_, AppStorage>,
) -> Result<std::sync::MutexGuard<'a, Storage>, String> {
    state.0.lock().map_err(|_| "storage lock poisoned".to_string())
}

#[tauri::command]
fn list_characters(
    state: State<'_, AppStorage>,
    kind: CharacterKind,
) -> Result<Vec<CharacterRecord>, String> {
    let rows = lock(&state)?.list_characters(storage_kind(kind)).map_err(|e| e.to_string())?;
    rows.into_iter().map(to_record).collect()
}

/// Quick create: a valid default sheet with the given name.
#[tauri::command]
fn create_character(
    state: State<'_, AppStorage>,
    kind: CharacterKind,
    name: String,
) -> Result<CharacterRecord, String> {
    let character = Character::quick(&name);
    character.validate().map_err(|e| e.to_string())?;
    let data = serde_json::to_string(&character).map_err(|e| e.to_string())?;
    let storage = lock(&state)?;
    let id = storage
        .insert_character(storage_kind(kind), &character.name, &data)
        .map_err(|e| e.to_string())?;
    join_active_session(&storage, id)?;
    Ok(CharacterRecord::new(id, character))
}

/// Derived numbers of a sheet that is being edited (not yet saved): Rust stays the only place
/// that knows the formulas.
#[tauri::command]
fn preview_derived(character: Character) -> Derived {
    character.derived()
}

#[tauri::command]
fn save_character(
    state: State<'_, AppStorage>,
    id: i64,
    mut character: Character,
) -> Result<CharacterRecord, String> {
    character.name = character.name.trim().to_string();
    character.validate().map_err(|e| e.to_string())?;
    let storage = lock(&state)?;
    let kind = storage
        .get_character(id)
        .map_err(|e| e.to_string())?
        .ok_or_else(|| format!("character {id} not found"))?
        .kind;
    if kind == storage::CharacterKind::Npc {
        character.settle_npc();
    }
    let data = serde_json::to_string(&character).map_err(|e| e.to_string())?;
    if !storage.update_character(id, &character.name, &data).map_err(|e| e.to_string())? {
        return Err(format!("character {id} not found"));
    }
    Ok(CharacterRecord::new(id, character))
}

/// `name.redcompanion.json`, or `name (2).redcompanion.json` and so on when it already exists.
fn free_export_name(dir: &std::path::Path, name: String, suffix: &str) -> String {
    if !dir.join(&name).exists() {
        return name;
    }
    let stem = name.strip_suffix(suffix).unwrap_or(&name).to_string();
    (2..)
        .map(|n| format!("{stem} ({n}){suffix}"))
        .find(|candidate| !dir.join(candidate).exists())
        .expect("an unused file name exists")
}

/// Writes characters (`ids`, or everyone when `None`) into a new file in the exports folder.
#[tauri::command]
fn export_characters(
    state: State<'_, AppStorage>,
    dir: State<'_, ExportsDir>,
    ids: Option<Vec<i64>>,
) -> Result<ExportSaved, String> {
    let storage = lock(&state)?;
    let mut items = Vec::new();
    for kind in [CharacterKind::Pc, CharacterKind::Npc] {
        for row in storage.list_characters(storage_kind(kind)).map_err(|e| e.to_string())? {
            if ids.as_ref().is_some_and(|ids| !ids.contains(&row.id)) {
                continue;
            }
            let character: Character = serde_json::from_str(&row.data).map_err(|e| e.to_string())?;
            items.push(ExportedCharacter { kind, character });
        }
    }
    let count = u32::try_from(items.len()).unwrap_or(u32::MAX);
    // One character is named after itself, several after "characters".
    let base = match items.as_slice() {
        [only] => only.character.name.clone(),
        _ => String::new(),
    };
    let json = transfer::export(items).map_err(|e| e.to_string())?;
    std::fs::create_dir_all(&dir.0).map_err(|e| e.to_string())?;
    let name = free_export_name(&dir.0, transfer::export_file_name(&base), transfer::EXPORT_SUFFIX);
    let path = dir.0.join(&name);
    std::fs::write(&path, json).map_err(|e| e.to_string())?;
    Ok(ExportSaved { name, path: path.display().to_string(), count })
}

/// Files of the exports folder accepted by `is_name`, the newest first.
fn list_files(dir: &std::path::Path, is_name: fn(&str) -> bool) -> Result<Vec<ExportFile>, String> {
    let Ok(entries) = std::fs::read_dir(dir) else { return Ok(Vec::new()) };
    let mut files: Vec<ExportFile> = entries
        .filter_map(|entry| {
            let entry = entry.ok()?;
            let meta = entry.metadata().ok()?;
            let name = entry.file_name().into_string().ok()?;
            if !meta.is_file() || !is_name(&name) {
                return None;
            }
            let modified_ms = meta
                .modified()
                .ok()
                .and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok())
                .map_or(0, |d| u64::try_from(d.as_millis()).unwrap_or(u64::MAX));
            Some(ExportFile { name, modified_ms, size: meta.len() })
        })
        .collect();
    files.sort_by(|a, b| b.modified_ms.cmp(&a.modified_ms).then_with(|| a.name.cmp(&b.name)));
    Ok(files)
}

/// Character export files in the exports folder, the newest first.
#[tauri::command]
fn list_exports(dir: State<'_, ExportsDir>) -> Result<Vec<ExportFile>, String> {
    list_files(&dir.0, transfer::is_export_file_name)
}

/// Writes the chosen encounters and own templates into a new `.redencounters.json` file. An
/// encounter always takes the own templates it uses along, so the file stands on its own.
#[tauri::command]
fn export_encounters(
    state: State<'_, AppStorage>,
    dir: State<'_, ExportsDir>,
    encounter_ids: Vec<i64>,
    template_ids: Vec<i64>,
) -> Result<EncountersSaved, String> {
    let storage = lock(&state)?;
    let encounters: Vec<Encounter> = storage
        .list_docs(DocTable::Encounters)
        .map_err(|e| e.to_string())?
        .iter()
        .filter(|row| encounter_ids.contains(&row.id))
        .map(|row| encounter_record(row).map(|r| r.encounter))
        .collect::<Result<_, _>>()?;
    let mut wanted = template_ids;
    for id in encounter_transfer::used_template_ids(&encounters) {
        if !wanted.contains(&id) {
            wanted.push(id);
        }
    }
    let templates: Vec<(i64, NpcTier, Character)> = storage
        .list_docs(DocTable::UserTemplates)
        .map_err(|e| e.to_string())?
        .iter()
        .filter(|row| wanted.contains(&row.id))
        .map(|row| user_template_record(row).map(|r| (r.id, r.tier, r.character)))
        .collect::<Result<_, _>>()?;
    if encounters.is_empty() && templates.is_empty() {
        return Err("nothing to export".to_string());
    }
    let counts = (
        u32::try_from(encounters.len()).unwrap_or(u32::MAX),
        u32::try_from(templates.len()).unwrap_or(u32::MAX),
    );
    // One encounter or one template is named after itself, anything more after "encounters".
    let base = match (encounters.as_slice(), templates.as_slice()) {
        ([only], []) => only.name.clone(),
        ([], [(_, _, only)]) => only.name.clone(),
        _ => String::new(),
    };
    let json = encounter_transfer::export(templates, encounters).map_err(|e| e.to_string())?;
    std::fs::create_dir_all(&dir.0).map_err(|e| e.to_string())?;
    let name = free_export_name(&dir.0, encounter_transfer::file_name(&base), encounter_transfer::SUFFIX);
    let path = dir.0.join(&name);
    std::fs::write(&path, json).map_err(|e| e.to_string())?;
    Ok(EncountersSaved {
        name,
        path: path.display().to_string(),
        encounters: counts.0,
        templates: counts.1,
    })
}

/// Encounter files in the exports folder, the newest first.
#[tauri::command]
fn list_encounter_exports(dir: State<'_, ExportsDir>) -> Result<Vec<ExportFile>, String> {
    list_files(&dir.0, encounter_transfer::is_file_name)
}

/// Imports an encounters file of the exports folder as new templates and encounters (nothing is
/// merged by name). The whole file is validated before anything is stored.
#[tauri::command]
fn import_encounter_file(
    state: State<'_, AppStorage>,
    dir: State<'_, ExportsDir>,
    name: String,
) -> Result<EncountersImported, String> {
    if !encounter_transfer::is_file_name(&name) {
        return Err(format!("{name:?} is not an encounters file name"));
    }
    let json = std::fs::read_to_string(dir.0.join(&name)).map_err(|e| e.to_string())?;
    let file = encounter_transfer::parse_import(&json, &SeedDataSource::default())
        .map_err(|e| e.to_string())?;
    let storage = lock(&state)?;
    let mut new_ids = std::collections::HashMap::new();
    let mut templates = Vec::new();
    for t in file.templates {
        let doc = UserTemplateDoc { tier: t.tier, character: t.character };
        let data = serde_json::to_string(&doc).map_err(|e| e.to_string())?;
        let id = storage
            .insert_doc(DocTable::UserTemplates, &doc.character.name, &data)
            .map_err(|e| e.to_string())?;
        new_ids.insert(t.key, id);
        templates.push(UserTemplateRecord { id, tier: doc.tier, character: doc.character });
    }
    let mut encounters = Vec::new();
    for mut encounter in file.encounters {
        encounter_transfer::remap_templates(&mut encounter, &new_ids);
        let data = serde_json::to_string(&encounter).map_err(|e| e.to_string())?;
        let id = storage
            .insert_doc(DocTable::Encounters, &encounter.name, &data)
            .map_err(|e| e.to_string())?;
        encounters.push(EncounterRecord { id, encounter });
    }
    Ok(EncountersImported { templates, encounters })
}

/// Imports a file of the exports folder as new characters; the whole file is validated
/// before anything is stored. Only a bare file name is accepted.
#[tauri::command]
fn import_export_file(
    state: State<'_, AppStorage>,
    dir: State<'_, ExportsDir>,
    name: String,
) -> Result<Vec<CharacterRecord>, String> {
    if !transfer::is_export_file_name(&name) {
        return Err(format!("{name:?} is not an export file name"));
    }
    let json = std::fs::read_to_string(dir.0.join(&name)).map_err(|e| e.to_string())?;
    let items = transfer::parse_import(&json, &SeedDataSource::default()).map_err(|e| e.to_string())?;
    let storage = lock(&state)?;
    let mut records = Vec::new();
    for mut item in items {
        if item.kind == CharacterKind::Npc {
            item.character.settle_npc();
        }
        let data = serde_json::to_string(&item.character).map_err(|e| e.to_string())?;
        let id = storage
            .insert_character(storage_kind(item.kind), &item.character.name, &data)
            .map_err(|e| e.to_string())?;
        join_active_session(&storage, id)?;
        records.push(CharacterRecord::new(id, item.character));
    }
    Ok(records)
}

/// Shows the exports folder in the system file manager.
#[tauri::command]
fn open_exports_folder(app: AppHandle, dir: State<'_, ExportsDir>) -> Result<(), String> {
    std::fs::create_dir_all(&dir.0).map_err(|e| e.to_string())?;
    app.opener().open_path(dir.0.display().to_string(), None::<&str>).map_err(|e| e.to_string())
}

#[tauri::command]
fn delete_character(state: State<'_, AppStorage>, id: i64) -> Result<(), String> {
    let storage = lock(&state)?;
    // Membership rows go with the character (ON DELETE CASCADE).
    storage.delete_character(id).map_err(|e| e.to_string())?;
    // A deleted character must not linger behind a cover or in a fight, in any session.
    for row in storage.all_scenes().map_err(|e| e.to_string())? {
        let mut scene: Scene = serde_json::from_str(&row.data).map_err(|e| e.to_string())?;
        scene.unlink_character(id);
        if scene.combat.combatants.iter().any(|c| c.character_id == id) {
            let skip = dead_ids(&storage, &scene)?;
            scene.combat.remove(id, &skip).map_err(|e| e.to_string())?;
        }
        save_scene_row(&storage, row.id, &scene)?;
    }
    Ok(())
}

fn save_scene_row(storage: &Storage, id: i64, scene: &Scene) -> Result<(), String> {
    let data = serde_json::to_string(scene).map_err(|e| e.to_string())?;
    storage.save_scene(id, &data).map_err(|e| e.to_string())?;
    Ok(())
}

const SESSION_NAME_MAX: usize = 60;

fn clean_session_name(name: &str) -> Result<String, String> {
    let name = name.trim();
    if name.is_empty() || name.chars().count() > SESSION_NAME_MAX {
        return Err(format!("session name must be 1..{SESSION_NAME_MAX} characters"));
    }
    Ok(name.to_string())
}

/// Creates a session with an empty scene; returns the session id.
fn new_session(storage: &Storage, name: &str) -> Result<i64, String> {
    let name = clean_session_name(name)?;
    let session = storage.create_session(&name).map_err(|e| e.to_string())?;
    let data = serde_json::to_string(&Scene::default()).map_err(|e| e.to_string())?;
    storage.create_scene(session, "Scene", &data).map_err(|e| e.to_string())?;
    Ok(session)
}

/// The session the app works with. If none is active, the first one becomes active; with no
/// sessions at all a default one is created (the UI normally creates a localized one first).
fn ensure_session(storage: &Storage) -> Result<i64, String> {
    if let Some(id) = storage.active_session().map_err(|e| e.to_string())? {
        return Ok(id);
    }
    let first = storage.list_sessions().map_err(|e| e.to_string())?.first().map(|s| s.id);
    let id = match first {
        Some(id) => id,
        None => new_session(storage, "Session")?,
    };
    storage.set_active_session(id).map_err(|e| e.to_string())?;
    Ok(id)
}

/// Puts a new character into the active session.
fn join_active_session(storage: &Storage, character_id: i64) -> Result<(), String> {
    let session = ensure_session(storage)?;
    storage.add_member(session, character_id).map_err(|e| e.to_string())
}

/// Like `session_infos`, but an empty database stays empty: the UI creates the first session
/// itself so that its name follows the interface language.
fn list_session_infos(storage: &Storage) -> Result<Vec<SessionInfo>, String> {
    if storage.list_sessions().map_err(|e| e.to_string())?.is_empty() {
        return Ok(Vec::new());
    }
    session_infos(storage)
}

fn session_infos(storage: &Storage) -> Result<Vec<SessionInfo>, String> {
    ensure_session(storage)?;
    let rows = storage.list_sessions().map_err(|e| e.to_string())?;
    Ok(rows.into_iter().map(|r| SessionInfo { id: r.id, name: r.name, active: r.active }).collect())
}

/// Loads the scene of the active session.
fn load_scene(storage: &Storage) -> Result<(i64, Scene), String> {
    let session = ensure_session(storage)?;
    match storage.active_scene().map_err(|e| e.to_string())? {
        Some(row) => {
            let scene: Scene = serde_json::from_str(&row.data).map_err(|e| e.to_string())?;
            scene.validate().map_err(|e| e.to_string())?;
            Ok((row.id, scene))
        }
        None => {
            let scene = Scene::default();
            let data = serde_json::to_string(&scene).map_err(|e| e.to_string())?;
            let id = storage.create_scene(session, "Scene", &data).map_err(|e| e.to_string())?;
            Ok((id, scene))
        }
    }
}

/// Loads the working scene, applies `change` and saves only when it succeeds.
fn with_scene(
    state: &State<'_, AppStorage>,
    change: impl FnOnce(&mut Scene) -> Result<(), String>,
) -> Result<SceneRecord, String> {
    let storage = lock(state)?;
    let (id, mut scene) = load_scene(&storage)?;
    change(&mut scene)?;
    save_scene_row(&storage, id, &scene)?;
    Ok(SceneRecord { id, scene })
}

#[tauri::command]
fn list_sessions(state: State<'_, AppStorage>) -> Result<Vec<SessionInfo>, String> {
    let storage = lock(&state)?;
    list_session_infos(&storage)
}

/// Creates a session with an empty scene and switches to it.
#[tauri::command]
fn create_session(state: State<'_, AppStorage>, name: String) -> Result<Vec<SessionInfo>, String> {
    let storage = lock(&state)?;
    let id = new_session(&storage, &name)?;
    storage.set_active_session(id).map_err(|e| e.to_string())?;
    session_infos(&storage)
}

#[tauri::command]
fn rename_session(
    state: State<'_, AppStorage>,
    id: i64,
    name: String,
) -> Result<Vec<SessionInfo>, String> {
    let storage = lock(&state)?;
    let name = clean_session_name(&name)?;
    if !storage.rename_session(id, &name).map_err(|e| e.to_string())? {
        return Err(format!("session {id} not found"));
    }
    session_infos(&storage)
}

#[tauri::command]
fn switch_session(state: State<'_, AppStorage>, id: i64) -> Result<Vec<SessionInfo>, String> {
    let storage = lock(&state)?;
    if !storage.set_active_session(id).map_err(|e| e.to_string())? {
        return Err(format!("session {id} not found"));
    }
    session_infos(&storage)
}

/// Deletes a session with its scene and fight. Its characters stay in the common list. Deleting
/// the active session activates another one; the list comes back empty after the last one.
#[tauri::command]
fn delete_session(state: State<'_, AppStorage>, id: i64) -> Result<Vec<SessionInfo>, String> {
    let storage = lock(&state)?;
    if !storage.delete_session(id).map_err(|e| e.to_string())? {
        return Err(format!("session {id} not found"));
    }
    if storage.list_sessions().map_err(|e| e.to_string())?.is_empty() {
        return Ok(Vec::new());
    }
    session_infos(&storage)
}

/// Ids of the active session's characters, in the order they joined.
#[tauri::command]
fn session_members(state: State<'_, AppStorage>) -> Result<Vec<i64>, String> {
    let storage = lock(&state)?;
    let session = ensure_session(&storage)?;
    storage.session_members(session).map_err(|e| e.to_string())
}

#[tauri::command]
fn add_to_session(state: State<'_, AppStorage>, character_id: i64) -> Result<Vec<i64>, String> {
    let storage = lock(&state)?;
    if storage.get_character(character_id).map_err(|e| e.to_string())?.is_none() {
        return Err(format!("character {character_id} not found"));
    }
    join_active_session(&storage, character_id)?;
    let session = ensure_session(&storage)?;
    storage.session_members(session).map_err(|e| e.to_string())
}

/// Takes a character out of the active session: out of its covers and fight too. The character
/// itself stays in the common list.
#[tauri::command]
fn remove_from_session(
    state: State<'_, AppStorage>,
    character_id: i64,
) -> Result<SceneRecord, String> {
    let storage = lock(&state)?;
    let session = ensure_session(&storage)?;
    storage.remove_member(session, character_id).map_err(|e| e.to_string())?;
    let (id, mut scene) = load_scene(&storage)?;
    scene.unlink_character(character_id);
    if scene.combat.combatants.iter().any(|c| c.character_id == character_id) {
        let skip = dead_ids(&storage, &scene)?;
        scene.combat.remove(character_id, &skip).map_err(|e| e.to_string())?;
        refresh_death_save_due(&mut scene, &storage)?;
    }
    save_scene_row(&storage, id, &scene)?;
    Ok(SceneRecord { id, scene })
}

#[tauri::command]
fn get_scene(state: State<'_, AppStorage>) -> Result<SceneRecord, String> {
    with_scene(&state, |_| Ok(()))
}

#[tauri::command]
fn list_cover_presets(lang: State<'_, ContentLang>) -> Result<Vec<CoverPreset>, String> {
    cover_presets(&content(&lang)?).map_err(|e| e.to_string())
}

/// Adds a cover from a preset (`preset` = its id) or a hand-made one (`None`,
/// then `hp` is required). `hp` overrides the preset default.
#[tauri::command]
fn add_cover(
    state: State<'_, AppStorage>,
    lang: State<'_, ContentLang>,
    preset: Option<String>,
    name: String,
    hp: Option<u32>,
) -> Result<SceneRecord, String> {
    let preset = match preset {
        Some(id) => Some(
            cover_presets(&content(&lang)?)
                .map_err(|e| e.to_string())?
                .into_iter()
                .find(|p| p.id == id)
                .ok_or_else(|| format!("unknown cover preset {id:?}"))?,
        ),
        None => None,
    };
    with_scene(&state, |scene| {
        scene.add_cover(preset.as_ref(), &name, hp).map(|_| ()).map_err(|e| e.to_string())
    })
}

#[tauri::command]
fn edit_cover(
    state: State<'_, AppStorage>,
    cover_id: u32,
    name: String,
    hp: u32,
    max_hp: u32,
) -> Result<SceneRecord, String> {
    with_scene(&state, |scene| {
        scene.edit_cover(cover_id, &name, hp, max_hp).map_err(|e| e.to_string())
    })
}

#[tauri::command]
fn remove_cover(state: State<'_, AppStorage>, cover_id: u32) -> Result<SceneRecord, String> {
    with_scene(&state, |scene| scene.remove_cover(cover_id).map_err(|e| e.to_string()))
}

/// Puts a character behind a cover, or out of cover with `cover_id = None`.
#[tauri::command]
fn set_character_cover(
    state: State<'_, AppStorage>,
    character_id: i64,
    cover_id: Option<u32>,
) -> Result<SceneRecord, String> {
    with_scene(&state, |scene| {
        scene.set_character_cover(character_id, cover_id).map_err(|e| e.to_string())
    })
}

fn load_character(storage: &Storage, id: i64) -> Result<Option<Character>, String> {
    match storage.get_character(id).map_err(|e| e.to_string())? {
        Some(row) => Ok(Some(parse_row(&row)?)),
        None => Ok(None),
    }
}

fn require_character(storage: &Storage, id: i64) -> Result<Character, String> {
    load_character(storage, id)?.ok_or_else(|| format!("character {id} not found"))
}

fn store_character(storage: &Storage, id: i64, character: &Character) -> Result<(), String> {
    let data = serde_json::to_string(character).map_err(|e| e.to_string())?;
    storage.update_character(id, &character.name, &data).map_err(|e| e.to_string())?;
    Ok(())
}

/// Ids of fighters who cannot take a turn: dead, or no longer in the database.
fn dead_ids(storage: &Storage, scene: &Scene) -> Result<Vec<i64>, String> {
    let mut dead = Vec::new();
    for c in &scene.combat.combatants {
        match load_character(storage, c.character_id)? {
            Some(character) if !character.dead => {}
            _ => dead.push(c.character_id),
        }
    }
    Ok(dead)
}

/// Rolls a d10 check (exploding on 10 and 1), or takes the faces read off
/// physical dice.
fn d10_check(manual: Option<Vec<u32>>) -> Result<CheckRoll, String> {
    match manual {
        Some(faces) => check_manual(&faces).map_err(|e| e.to_string()),
        None => Ok(roll_check(&mut ThreadDice::new())),
    }
}

/// Marks whether the fighter whose turn just began owes a death save.
fn refresh_death_save_due(scene: &mut Scene, storage: &Storage) -> Result<(), String> {
    scene.combat.death_save_due = match scene.combat.active {
        Some(id) => match load_character(storage, id)? {
            Some(c) => c.wound_state() == WoundState::Mortal,
            None => false,
        },
        None => false,
    };
    Ok(())
}

/// Runs a change on the working scene with access to the character storage.
fn with_combat(
    state: &State<'_, AppStorage>,
    change: impl FnOnce(&mut Scene, &Storage) -> Result<(), String>,
) -> Result<SceneRecord, String> {
    let storage = lock(state)?;
    let (id, mut scene) = load_scene(&storage)?;
    change(&mut scene, &storage)?;
    save_scene_row(&storage, id, &scene)?;
    Ok(SceneRecord { id, scene })
}

/// Rolls a character's initiative, REF + 1d10 (`manual` = the die faces): adds them to the
/// order, or re-rolls them if they already have one.
#[tauri::command]
fn combat_roll(
    state: State<'_, AppStorage>,
    character_id: i64,
    manual: Option<Vec<u32>>,
) -> Result<SceneRecord, String> {
    with_combat(&state, |scene, storage| {
        let reflex = require_character(storage, character_id)?.stats.reflex;
        let die = d10_check(manual)?;
        scene.combat.roll(character_id, reflex, die).map_err(|e| e.to_string())
    })
}

/// Sets the initiative total by hand (the field on a card); adds the character to the order
/// if they had none yet.
#[tauri::command]
fn combat_set_initiative(
    state: State<'_, AppStorage>,
    character_id: i64,
    initiative: i32,
) -> Result<SceneRecord, String> {
    with_combat(&state, |scene, storage| {
        let reflex = require_character(storage, character_id)?.stats.reflex;
        scene.combat.set_or_add(character_id, reflex, initiative).map_err(|e| e.to_string())
    })
}

/// Moves a fighter one place earlier (`delta < 0`) or later (`delta > 0`).
#[tauri::command]
fn combat_shift(
    state: State<'_, AppStorage>,
    character_id: i64,
    delta: i32,
) -> Result<SceneRecord, String> {
    with_combat(&state, |scene, _| {
        scene.combat.shift(character_id, delta).map_err(|e| e.to_string())
    })
}

#[tauri::command]
fn combat_remove(state: State<'_, AppStorage>, character_id: i64) -> Result<SceneRecord, String> {
    with_combat(&state, |scene, storage| {
        let skip = dead_ids(storage, scene)?;
        scene.combat.remove(character_id, &skip).map_err(|e| e.to_string())?;
        refresh_death_save_due(scene, storage)
    })
}

/// Starts the fight. Everyone in the active session fights: living characters who have no initiative
/// yet get one rolled here, so the GM only types in the ones rolled with physical dice.
#[tauri::command]
fn combat_start(state: State<'_, AppStorage>) -> Result<SceneRecord, String> {
    with_combat(&state, |scene, storage| {
        let session = ensure_session(storage)?;
        for id in storage.session_members(session).map_err(|e| e.to_string())? {
            let Some(character) = load_character(storage, id)? else { continue };
            if character.dead || scene.combat.is_in_fight(id) {
                continue;
            }
            let die = d10_check(None)?;
            scene.combat.add(id, character.stats.reflex, die).map_err(|e| e.to_string())?;
        }
        let skip = dead_ids(storage, scene)?;
        scene.combat.start(&skip).map_err(|e| e.to_string())?;
        refresh_death_save_due(scene, storage)
    })
}

#[tauri::command]
fn combat_next_turn(state: State<'_, AppStorage>) -> Result<SceneRecord, String> {
    with_combat(&state, |scene, storage| {
        let skip = dead_ids(storage, scene)?;
        scene.combat.next_turn(&skip).map_err(|e| e.to_string())?;
        refresh_death_save_due(scene, storage)
    })
}

#[tauri::command]
fn combat_end(state: State<'_, AppStorage>) -> Result<SceneRecord, String> {
    with_combat(&state, |scene, _| {
        scene.combat.end();
        Ok(())
    })
}

/// Death save of a mortally wounded character (`manual` = the d10 face).
#[tauri::command]
fn death_save(
    state: State<'_, AppStorage>,
    character_id: i64,
    manual: Option<u32>,
) -> Result<DeathSaveApplied, String> {
    let storage = lock(&state)?;
    let mut character = require_character(&storage, character_id)?;
    let notation = DiceNotation { count: 1, sides: 10, modifier: 0 };
    let roll = match manual {
        Some(face) => roll_manual(notation, &[face]).map_err(|e| e.to_string())?,
        None => roll(notation, &mut ThreadDice::new()),
    };
    let save = vitals::apply_death_save(&mut character, roll).map_err(|e| e.to_string())?;
    store_character(&storage, character_id, &character)?;
    let (scene_id, mut scene) = load_scene(&storage)?;
    if scene.combat.active == Some(character_id) {
        scene.combat.death_save_due = false;
        save_scene_row(&storage, scene_id, &scene)?;
    }
    Ok(DeathSaveApplied {
        character: CharacterRecord::new(character_id, character),
        save,
        scene: SceneRecord { id: scene_id, scene },
    })
}

/// Stabilisation against the DV of the target wound: the helper TECH + `skill` level + d10
/// (rolled here, or `manual_total` typed in from physical dice), or `granted` when the GM just
/// declares the target stabilised.
#[tauri::command]
fn stabilize(
    state: State<'_, AppStorage>,
    target_id: i64,
    helper_id: i64,
    skill: String,
    manual_total: Option<i32>,
    granted: bool,
) -> Result<StabilizeApplied, String> {
    let storage = lock(&state)?;
    let mut target = require_character(&storage, target_id)?;
    let base = vitals::base_death_save_penalty(&target.injuries, &injury_table()?);
    let result = if granted {
        vitals::stabilize_granted(&mut target, base).map_err(|e| e.to_string())?
    } else {
        if !STABILIZE_SKILLS.contains(&skill.as_str()) {
            return Err(vitals::VitalsError::BadSkill(skill).to_string());
        }
        let helper = if helper_id == target_id {
            target.clone()
        } else {
            require_character(&storage, helper_id)?
        };
        let bonus = i32::from(helper.stats.tech)
            + i32::from(helper.skills.get(&skill).copied().unwrap_or(0));
        let die = match manual_total {
            Some(total) => check_total(total).map_err(|e| e.to_string())?,
            None => roll_check(&mut ThreadDice::new()),
        };
        vitals::stabilize(&mut target, bonus, die, base).map_err(|e| e.to_string())?
    };
    store_character(&storage, target_id, &target)?;
    Ok(StabilizeApplied { character: CharacterRecord::new(target_id, target), result })
}

fn injury_table() -> Result<Vec<CriticalInjury>, String> {
    SeedDataSource::default().critical_injuries().map(|c| c.entries).map_err(|e| e.to_string())
}

#[tauri::command]
fn list_injuries() -> Result<Vec<CriticalInjury>, String> {
    injury_table()
}

/// Skill check of a saved sheet: STAT + level + a d10 check, rolled here or typed in
/// from a physical die (`manual_total`, the explosion already added up).
#[tauri::command]
fn skill_check(
    state: State<'_, AppStorage>,
    character_id: i64,
    skill: String,
    manual_total: Option<i32>,
) -> Result<SkillCheck, String> {
    let character = require_character(&*lock(&state)?, character_id)?;
    let die = match manual_total {
        Some(total) => check_total(total).map_err(|e| e.to_string())?,
        None => roll_check(&mut ThreadDice::new()),
    };
    let catalog = SeedDataSource::default().skills().map_err(|e| e.to_string())?.entries;
    skill_check::skill_check(&character, &skill, &catalog, die).map_err(|e| e.to_string())
}

/// Rolls `notation` with the OS RNG, or builds the result from faces read off
/// physical dice (`manual`).
#[tauri::command]
fn roll_dice(notation: String, manual: Option<Vec<u32>>) -> Result<RollResult, String> {
    let notation: DiceNotation = notation.parse().map_err(|e: DiceError| e.to_string())?;
    match manual {
        Some(faces) => roll_manual(notation, &faces).map_err(|e| e.to_string()),
        None => Ok(roll(notation, &mut ThreadDice::new())),
    }
}

/// Rolls (or takes manual 2d6 faces for) the critical injury of a hit.
#[tauri::command]
fn roll_injury(
    state: State<'_, AppStorage>,
    character_id: i64,
    area: BodyArea,
    manual: Option<Vec<u32>>,
) -> Result<InjuryRoll, String> {
    let character = require_character(&*lock(&state)?, character_id)?;
    damage_event::roll_injury(
        &injury_table()?,
        area,
        &character.injuries,
        manual.as_deref(),
        &mut ThreadDice::new(),
    )
    .map_err(|e| e.to_string())
}

/// Calculates a hit against the stored state without saving anything.
#[tauri::command]
fn preview_damage(
    state: State<'_, AppStorage>,
    character_id: i64,
    request: DamageRequest,
) -> Result<DamageOutcome, String> {
    let storage = lock(&state)?;
    let character = require_character(&storage, character_id)?;
    let (_, scene) = load_scene(&storage)?;
    damage_event::evaluate(&character, &scene, &request).map_err(|e| e.to_string())
}

/// Applies a hit to the character and the scene's cover; saves both or neither.
#[tauri::command]
fn apply_damage(
    state: State<'_, AppStorage>,
    character_id: i64,
    request: DamageRequest,
    injury_id: Option<String>,
) -> Result<DamageApplied, String> {
    let storage = lock(&state)?;
    let row = storage
        .get_character(character_id)
        .map_err(|e| e.to_string())?
        .ok_or_else(|| format!("character {character_id} not found"))?;
    let mut character = parse_row(&row)?;
    let (scene_id, mut scene) = load_scene(&storage)?;
    damage_event::apply(
        &mut character,
        character_id,
        &mut scene,
        &request,
        injury_id.as_deref(),
        &injury_table()?,
    )
    .map_err(|e| e.to_string())?;
    // Filler NPCs have no death save: at 0 HP they are dead on the spot.
    if row.kind == storage::CharacterKind::Npc {
        damage_event::kill_downed_npc(&mut character, &mut scene);
    }
    character.validate().map_err(|e| e.to_string())?;
    store_character(&storage, character_id, &character)?;
    save_scene_row(&storage, scene_id, &scene)?;
    Ok(DamageApplied {
        character: Some(CharacterRecord::new(character_id, character)),
        scene: SceneRecord { id: scene_id, scene },
    })
}

/// Brings a dead character (a PC or an NPC) back: 1 HP, no death save penalty, no injuries.
#[tauri::command]
fn revive_character(
    state: State<'_, AppStorage>,
    id: i64,
) -> Result<CharacterRecord, String> {
    let storage = lock(&state)?;
    let mut character = require_character(&storage, id)?;
    vitals::revive(&mut character).map_err(|e| e.to_string())?;
    store_character(&storage, id, &character)?;
    Ok(CharacterRecord::new(id, character))
}

/// Undoes the scene's last damage event.
#[tauri::command]
fn undo_damage(state: State<'_, AppStorage>) -> Result<DamageApplied, String> {
    let storage = lock(&state)?;
    let (scene_id, mut scene) = load_scene(&storage)?;
    let event = scene.last_damage.clone().ok_or("nothing to undo")?;
    let character = match load_character(&storage, event.character_id)? {
        Some(mut character) => {
            damage_event::undo_character(&mut character, &event);
            store_character(&storage, event.character_id, &character)?;
            Some(CharacterRecord::new(event.character_id, character))
        }
        None => None,
    };
    damage_event::undo_scene(&mut scene, &event);
    save_scene_row(&storage, scene_id, &scene)?;
    Ok(DamageApplied { character, scene: SceneRecord { id: scene_id, scene } })
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .setup(|app| {
            let dir = app.path().app_data_dir()?;
            std::fs::create_dir_all(&dir)?;
            let storage = Storage::open(&dir.join("red-companion.db"))?;
            let exports = dir.join("exports");
            std::fs::create_dir_all(&exports)?;
            app.manage(ExportsDir(exports));
            app.manage(AppStorage(Mutex::new(storage)));
            app.manage(ContentLang(Mutex::new(Lang::default())));
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            rules_info,
            set_content_lang,
            list_skills,
            skill_check,
            list_armor,
            list_weapons,
            list_npc_templates,
            create_npc_from_template,
            list_user_templates,
            save_character_as_template,
            delete_user_template,
            set_user_template_tier,
            assess_encounter,
            tier_hint,
            create_npc_from_user_template,
            list_encounters,
            save_encounter,
            delete_encounter,
            list_characters,
            create_character,
            save_character,
            delete_character,
            export_characters,
            list_exports,
            export_encounters,
            list_encounter_exports,
            import_encounter_file,
            import_export_file,
            open_exports_folder,
            list_sessions,
            create_session,
            rename_session,
            switch_session,
            delete_session,
            session_members,
            add_to_session,
            remove_from_session,
            get_scene,
            list_cover_presets,
            add_cover,
            edit_cover,
            remove_cover,
            set_character_cover,
            list_injuries,
            roll_dice,
            roll_injury,
            preview_damage,
            apply_damage,
            undo_damage,
            revive_character,
            combat_roll,
            preview_derived,
            combat_set_initiative,
            combat_shift,
            combat_remove,
            combat_start,
            combat_next_turn,
            combat_end,
            death_save,
            stabilize
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}

#[cfg(test)]
mod tests {
    use super::*;

    fn storage() -> Storage {
        Storage::open_in_memory().unwrap()
    }

    #[test]
    fn first_use_creates_one_active_session_with_an_empty_scene() {
        let s = storage();
        let id = ensure_session(&s).unwrap();
        assert_eq!(ensure_session(&s).unwrap(), id);
        let infos = session_infos(&s).unwrap();
        assert_eq!(infos.len(), 1);
        assert!(infos[0].active);
        let (_, scene) = load_scene(&s).unwrap();
        assert!(scene.covers.is_empty());
    }

    #[test]
    fn listing_an_empty_database_creates_no_session() {
        let s = storage();
        assert!(list_session_infos(&s).unwrap().is_empty());
        assert!(s.list_sessions().unwrap().is_empty());
        // Sessions exist but none is active: the first one is activated.
        let one = new_session(&s, "One").unwrap();
        let infos = list_session_infos(&s).unwrap();
        assert_eq!(infos.len(), 1);
        assert!(infos[0].active && infos[0].id == one);
    }

    #[test]
    fn each_session_has_its_own_scene_and_members() {
        let s = storage();
        let one = new_session(&s, "One").unwrap();
        s.set_active_session(one).unwrap();
        let v = s.insert_character(storage::CharacterKind::Pc, "V", "{}").unwrap();
        join_active_session(&s, v).unwrap();
        let (scene_one, mut scene) = load_scene(&s).unwrap();
        scene.add_cover(None, "Car", Some(20)).unwrap();
        save_scene_row(&s, scene_one, &scene).unwrap();

        let two = new_session(&s, "Two").unwrap();
        s.set_active_session(two).unwrap();
        let (scene_two, other) = load_scene(&s).unwrap();
        assert_ne!(scene_one, scene_two);
        assert!(other.covers.is_empty(), "cover must stay in session one");
        assert!(s.session_members(two).unwrap().is_empty());

        // A new character joins the active session only.
        let ganger = s.insert_character(storage::CharacterKind::Npc, "Ganger", "{}").unwrap();
        join_active_session(&s, ganger).unwrap();
        assert_eq!(s.session_members(one).unwrap(), vec![v]);
        assert_eq!(s.session_members(two).unwrap(), vec![ganger]);

        // Back in session one the cover is still there.
        s.set_active_session(one).unwrap();
        assert_eq!(load_scene(&s).unwrap().1.covers.len(), 1);
    }

    #[test]
    fn deleting_the_active_session_activates_another() {
        let s = storage();
        let one = new_session(&s, "One").unwrap();
        let two = new_session(&s, "Two").unwrap();
        s.set_active_session(two).unwrap();
        s.delete_session(two).unwrap();
        assert_eq!(ensure_session(&s).unwrap(), one);
        assert!(session_infos(&s).unwrap()[0].active);
    }

    #[test]
    fn session_names_are_trimmed_and_limited() {
        assert_eq!(clean_session_name("  Night 1 ").unwrap(), "Night 1");
        assert!(clean_session_name("   ").is_err());
        assert!(clean_session_name(&"x".repeat(SESSION_NAME_MAX + 1)).is_err());
        assert!(clean_session_name(&"x".repeat(SESSION_NAME_MAX)).is_ok());
    }
}
