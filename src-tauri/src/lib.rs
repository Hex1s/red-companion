use std::path::PathBuf;
use std::sync::Mutex;

mod error;
mod picker;
mod portrait;
mod startup;
#[cfg(desktop)]
mod updater;
// Android updates through its own APK flow (ROADMAP 8.6); the commands exist so the UI code is the same.
#[cfg(not(desktop))]
#[path = "updater_mobile.rs"]
mod updater;

use error::CmdError;
use rules_core::RulesInfo;
use rules_core::catalog::{
    Ammo, Armor, Article, BodyArea, ClothingStyle, CriticalInjury, GearItem, MagazineRow, MartialArt, NpcTemplate, NpcTier, PriceCategory, RangeTable, Role,
    Skill, Specialty, StreetDrug, Weapon, WeaponMod,
};
use rules_core::character::{Character, CharacterKind, CharacterRecord, Derived};
use rules_core::damage::DamageOutcome;
use rules_core::damage::WoundState;
use rules_core::damage_event::{self, DamageApplied, DamageRequest, InjuryRoll};
use rules_core::data_source::{DataSource, Lang, SeedDataSource};
use rules_core::dice::{
    CheckRoll, DiceNotation, RollResult, ThreadDice, check_manual, check_total, roll, roll_check,
    roll_manual,
};
use rules_core::skill_check::{self, SkillCheck};
use rules_core::scene::{
    CoverPreset, DeletedCharacter, ExplorerNode, NodeKind, Scene, SceneRecord, SceneSnapshot,
    SessionInfo, cover_presets,
};
use rules_core::encounter_transfer::{self, EncountersImported, EncountersSaved};
use rules_core::transfer::{self, ExportFile, ExportKind, ExportSaved, ExportedCharacter};
use rules_core::healing::{self, HealApplied, HealKind, RestApplied, RestGoal, RestPlan};
use rules_core::humanity::{self, HumanityLossApplied, TherapyApplied, TherapyInfo, TherapyKind};
use rules_core::catalog::TreatmentSkill;
use rules_core::vitals::{self, DeathSaveApplied, STABILIZE_SKILLS, StabilizeApplied};
use rules_core::encounter::{
    self, Assessment, Encounter, EncounterRecord, TemplateSource, UserTemplateDoc, UserTemplateRecord,
};
use storage::{CharacterRow, DocRow, DocTable, Storage};
use tauri::{AppHandle, Manager, State};
#[cfg(desktop)]
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
fn content(state: &State<'_, ContentLang>) -> Result<SeedDataSource, CmdError> {
    let lang = *state.0.lock().map_err(|_| CmdError::locked("content language"))?;
    Ok(SeedDataSource::new(lang))
}

/// Switches the content language (`ru`, `en`, `ru-RU`...); unknown codes mean English.
#[tauri::command]
fn set_content_lang(state: State<'_, ContentLang>, code: String) -> Result<(), CmdError> {
    *state.0.lock().map_err(|_| CmdError::locked("content language"))? = Lang::from_code(&code);
    Ok(())
}

#[tauri::command]
fn rules_info() -> RulesInfo {
    rules_core::rules_info()
}

#[tauri::command]
fn list_skills() -> Result<Vec<Skill>, CmdError> {
    SeedDataSource::default().skills().map(|c| c.entries).map_err(CmdError::from)
}

#[tauri::command]
fn list_articles() -> Result<Vec<Article>, CmdError> {
    SeedDataSource::default().articles().map(|c| c.entries).map_err(CmdError::from)
}

#[tauri::command]
fn list_range_tables() -> Result<Vec<RangeTable>, CmdError> {
    SeedDataSource::default().range_tables().map(|c| c.entries).map_err(CmdError::from)
}

#[tauri::command]
fn list_martial_arts(lang: State<'_, ContentLang>) -> Result<Vec<MartialArt>, CmdError> {
    content(&lang)?.martial_arts().map(|c| c.entries).map_err(CmdError::from)
}

#[tauri::command]
fn list_roles(lang: State<'_, ContentLang>) -> Result<Vec<Role>, CmdError> {
    content(&lang)?.roles().map(|c| c.entries).map_err(CmdError::from)
}

#[tauri::command]
fn list_armor(lang: State<'_, ContentLang>) -> Result<Vec<Armor>, CmdError> {
    content(&lang)?.armor().map(|c| c.entries).map_err(CmdError::from)
}

#[tauri::command]
fn list_weapons(lang: State<'_, ContentLang>) -> Result<Vec<Weapon>, CmdError> {
    content(&lang)?.weapons().map(|c| c.entries).map_err(CmdError::from)
}

#[tauri::command]
fn list_gear(lang: State<'_, ContentLang>) -> Result<Vec<GearItem>, CmdError> {
    content(&lang)?.gear().map(|c| c.entries).map_err(CmdError::from)
}

#[tauri::command]
fn list_ammo(lang: State<'_, ContentLang>) -> Result<Vec<Ammo>, CmdError> {
    content(&lang)?.ammo().map(|c| c.entries).map_err(CmdError::from)
}

#[tauri::command]
fn list_weapon_mods(lang: State<'_, ContentLang>) -> Result<Vec<WeaponMod>, CmdError> {
    content(&lang)?.weapon_mods().map(|c| c.entries).map_err(CmdError::from)
}

/// Magazine sizes by weapon class (core table, the same in every language).
#[tauri::command]
fn list_magazines() -> Result<Vec<MagazineRow>, CmdError> {
    SeedDataSource::default().magazines().map(|c| c.entries).map_err(CmdError::from)
}

/// Suggested specialties of skills (languages, instruments); the sheet also accepts typed text.
#[tauri::command]
fn list_specialties(lang: State<'_, ContentLang>) -> Result<Vec<Specialty>, CmdError> {
    content(&lang)?.specialties().map(|c| c.entries).map_err(CmdError::from)
}

#[tauri::command]
fn list_clothing(lang: State<'_, ContentLang>) -> Result<Vec<ClothingStyle>, CmdError> {
    content(&lang)?.clothing().map(|c| c.entries).map_err(CmdError::from)
}

#[tauri::command]
fn list_street_drugs(lang: State<'_, ContentLang>) -> Result<Vec<StreetDrug>, CmdError> {
    content(&lang)?.street_drugs().map(|c| c.entries).map_err(CmdError::from)
}

/// Price categories with their minimum prices (core table); the UI derives a price's category from it.
#[tauri::command]
fn list_price_categories() -> Result<Vec<PriceCategory>, CmdError> {
    SeedDataSource::default().price_categories().map(|c| c.entries).map_err(CmdError::from)
}

#[tauri::command]
fn list_npc_templates(lang: State<'_, ContentLang>) -> Result<Vec<NpcTemplate>, CmdError> {
    content(&lang)?.npc_templates().map(|c| c.entries).map_err(CmdError::from)
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
    session_id: Option<i64>,
) -> Result<CharacterRecord, CmdError> {
    let templates = content(&lang)?.npc_templates()?;
    let template = templates
        .entries
        .iter()
        .find(|t| t.code == template_id)
        .ok_or_else(|| CmdError::not_found("npc_template", template_id))?;
    let skills = SeedDataSource::default().skills()?;
    let character = template.to_character(&name, &notes, &skills.entries);
    character.validate()?;
    let data = serde_json::to_string(&character)?;
    let storage = lock(&state)?;
    let id = spawn_instance(&storage, &character.name, &data, session_id)?;
    Ok(CharacterRecord::new(id, character))
}

/// Stores a fresh NPC copy from a template as an instance of a session (`session_id`, or the active
/// one): it lies in that session and goes away with it. Player characters are never instances.
fn spawn_instance(storage: &Storage, name: &str, data: &str, session_id: Option<i64>) -> Result<i64, CmdError> {
    let session = match session_id {
        Some(id) => {
            if !storage.container(id)?.is_some_and(|c| c.kind == storage::ContainerKind::Session) {
                return Err(CmdError::not_found("session", id));
            }
            id
        }
        None => ensure_session(storage)?,
    };
    let id = storage.insert_instance(storage::CharacterKind::Npc, name, data, session)?;
    storage.add_member(session, id)?;
    Ok(id)
}

fn user_template_record(row: &DocRow) -> Result<UserTemplateRecord, CmdError> {
    let doc: UserTemplateDoc = serde_json::from_str(&row.data)?;
    Ok(UserTemplateRecord { id: row.id, tier: doc.tier, character: doc.character, based_on: doc.based_on })
}

fn encounter_record(row: &DocRow) -> Result<EncounterRecord, CmdError> {
    let encounter: Encounter = serde_json::from_str(&row.data)?;
    Ok(EncounterRecord { id: row.id, encounter })
}

#[tauri::command]
fn list_user_templates(state: State<'_, AppStorage>) -> Result<Vec<UserTemplateRecord>, CmdError> {
    let rows = lock(&state)?.list_docs(DocTable::UserTemplates)?;
    rows.iter().map(user_template_record).collect()
}

/// Saves a copy of a character's sheet as the GM's own NPC template (fresh HP and armor, no
/// injuries). With `template_id` the existing template is overwritten instead of adding a new one.
#[tauri::command]
fn save_character_as_template(
    state: State<'_, AppStorage>,
    character_id: i64,
    template_id: Option<i64>,
) -> Result<UserTemplateRecord, CmdError> {
    let storage = lock(&state)?;
    let row = storage
        .get_character(character_id)
        ?
        .ok_or_else(|| CmdError::not_found("character", character_id))?;
    let character = parse_row(&row)?.as_fresh();
    character.validate()?;
    // Overwriting keeps the tier the GM already chose, and the book template it replaces.
    let existing = match template_id {
        Some(id) => storage
            .get_doc(DocTable::UserTemplates, id)
            ?
            .map(|row| user_template_record(&row))
            .transpose()?,
        None => None,
    };
    let tier = existing.as_ref().map_or(NpcTier::Mook, |t| t.tier);
    let based_on = existing.and_then(|t| t.based_on);
    let data = serde_json::to_string(&UserTemplateDoc { tier, character: character.clone(), based_on: based_on.clone() })
        ?;
    let id = match template_id {
        Some(id) => {
            if !storage
                .update_doc(DocTable::UserTemplates, id, &character.name, &data)
                ?
            {
                return Err(CmdError::not_found("template", id));
            }
            id
        }
        None => storage
            .insert_doc(DocTable::UserTemplates, &character.name, &data)
            ?,
    };
    Ok(UserTemplateRecord { id, tier, character, based_on })
}

/// Files an own template under a tier (it counts for that much in the encounter assessment).
#[tauri::command]
fn set_user_template_tier(
    state: State<'_, AppStorage>,
    id: i64,
    tier: NpcTier,
) -> Result<UserTemplateRecord, CmdError> {
    let storage = lock(&state)?;
    let row = storage
        .get_doc(DocTable::UserTemplates, id)
        ?
        .ok_or_else(|| CmdError::not_found("template", id))?;
    let record = user_template_record(&row)?;
    let doc = UserTemplateDoc { tier, character: record.character.clone(), based_on: record.based_on.clone() };
    storage.update_doc(DocTable::UserTemplates, id, &row.name, &serde_json::to_string(&doc)?)?;
    Ok(UserTemplateRecord { id, tier, character: record.character, based_on: record.based_on })
}

/// Saves the edited sheet of an own template (name, stats, gear...). The template keeps its tier and
/// stays a fresh sheet: full HP and armor, no injuries, so every copy starts clean.
#[tauri::command]
fn update_user_template(
    state: State<'_, AppStorage>,
    id: i64,
    mut character: Character,
) -> Result<UserTemplateRecord, CmdError> {
    character.name = character.name.trim().to_string();
    character.validate()?;
    let role_catalog = SeedDataSource::default().roles()?.entries;
    rules_core::roles::validate_against(&character.roles, &role_catalog)?;
    let storage = lock(&state)?;
    let row = storage
        .get_doc(DocTable::UserTemplates, id)?
        .ok_or_else(|| CmdError::not_found("template", id))?;
    let current = user_template_record(&row)?;
    let (tier, based_on) = (current.tier, current.based_on);
    let character = character.as_fresh();
    let data = serde_json::to_string(&UserTemplateDoc { tier, character: character.clone(), based_on: based_on.clone() })?;
    storage.update_doc(DocTable::UserTemplates, id, &character.name, &data)?;
    Ok(UserTemplateRecord { id, tier, character, based_on })
}

/// The own template that replaces the book template `code`, if the GM has edited it.
fn book_override(storage: &Storage, code: &str) -> Result<Option<UserTemplateRecord>, CmdError> {
    for row in storage.list_docs(DocTable::UserTemplates)? {
        let record = user_template_record(&row)?;
        if record.based_on.as_deref() == Some(code) {
            return Ok(Some(record));
        }
    }
    Ok(None)
}

/// The sheet a book template starts from, so it can be opened in the editor before anything is
/// changed (nothing is stored until the first edit, see `save_book_template_copy`).
#[tauri::command]
fn book_template_sheet(
    lang: State<'_, ContentLang>,
    code: String,
    notes: String,
) -> Result<Character, CmdError> {
    let templates = content(&lang)?.npc_templates()?;
    let template = templates
        .entries
        .iter()
        .find(|t| t.code == code)
        .ok_or_else(|| CmdError::not_found("npc_template", code))?;
    let skills = SeedDataSource::default().skills()?;
    Ok(template.to_character(&template.name, &notes, &skills.entries))
}

/// The GM edited a book template: the edited sheet is stored as their copy, which takes the book
/// one's place everywhere (the original stays untouched and comes back when the copy is deleted).
/// Further edits overwrite the same copy.
#[tauri::command]
fn save_book_template_copy(
    state: State<'_, AppStorage>,
    lang: State<'_, ContentLang>,
    code: String,
    mut character: Character,
) -> Result<UserTemplateRecord, CmdError> {
    character.name = character.name.trim().to_string();
    character.validate()?;
    let role_catalog = SeedDataSource::default().roles()?.entries;
    rules_core::roles::validate_against(&character.roles, &role_catalog)?;
    let templates = content(&lang)?.npc_templates()?;
    let tier = templates
        .entries
        .iter()
        .find(|t| t.code == code)
        .ok_or_else(|| CmdError::not_found("npc_template", code.clone()))?
        .tier;
    let character = character.as_fresh();
    let storage = lock(&state)?;
    let based_on = Some(code.clone());
    match book_override(&storage, &code)? {
        Some(existing) => {
            let doc = UserTemplateDoc { tier: existing.tier, character: character.clone(), based_on: based_on.clone() };
            storage.update_doc(DocTable::UserTemplates, existing.id, &character.name, &serde_json::to_string(&doc)?)?;
            Ok(UserTemplateRecord { id: existing.id, tier: existing.tier, character, based_on })
        }
        None => {
            let doc = UserTemplateDoc { tier, character: character.clone(), based_on: based_on.clone() };
            let id = storage.insert_doc(DocTable::UserTemplates, &character.name, &serde_json::to_string(&doc)?)?;
            Ok(UserTemplateRecord { id, tier, character, based_on })
        }
    }
}

/// Which tier the stats of a sheet are closest to (information only; the GM picks the tier).
#[tauri::command]
fn tier_hint(character: Character) -> Result<encounter::TierHint, CmdError> {
    let skills = SeedDataSource::default().skills()?;
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
) -> Result<Assessment, CmdError> {
    let book = content(&lang)?.npc_templates()?;
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
                    ?
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
    encounter::assess(weight, players, skipped).map_err(CmdError::from)
}

#[tauri::command]
fn delete_user_template(state: State<'_, AppStorage>, id: i64) -> Result<(), CmdError> {
    lock(&state)?.delete_doc(DocTable::UserTemplates, id)?;
    Ok(())
}

/// Clones the GM's own template into a new NPC sheet under `name`.
#[tauri::command]
fn create_npc_from_user_template(
    state: State<'_, AppStorage>,
    template_id: i64,
    name: String,
    session_id: Option<i64>,
) -> Result<CharacterRecord, CmdError> {
    let storage = lock(&state)?;
    let row = storage
        .get_doc(DocTable::UserTemplates, template_id)
        ?
        .ok_or_else(|| CmdError::not_found("template", template_id))?;
    let mut character = user_template_record(&row)?.character.as_fresh();
    character.name = name.trim().to_string();
    character.validate()?;
    let data = serde_json::to_string(&character)?;
    let id = spawn_instance(&storage, &character.name, &data, session_id)?;
    Ok(CharacterRecord::new(id, character))
}

#[tauri::command]
fn list_encounters(state: State<'_, AppStorage>) -> Result<Vec<EncounterRecord>, CmdError> {
    let rows = lock(&state)?.list_docs(DocTable::Encounters)?;
    rows.iter().map(encounter_record).collect()
}

/// Creates an encounter (`id` is `None`) or overwrites an existing one.
#[tauri::command]
fn save_encounter(
    state: State<'_, AppStorage>,
    id: Option<i64>,
    mut encounter: Encounter,
) -> Result<EncounterRecord, CmdError> {
    encounter.normalize()?;
    let data = serde_json::to_string(&encounter)?;
    let storage = lock(&state)?;
    let id = match id {
        Some(id) => {
            if !storage
                .update_doc(DocTable::Encounters, id, &encounter.name, &data)
                ?
            {
                return Err(CmdError::not_found("encounter", id));
            }
            id
        }
        None => storage
            .insert_doc(DocTable::Encounters, &encounter.name, &data)
            ?,
    };
    Ok(EncounterRecord { id, encounter })
}

#[tauri::command]
fn delete_encounter(state: State<'_, AppStorage>, id: i64) -> Result<(), CmdError> {
    lock(&state)?.delete_doc(DocTable::Encounters, id)?;
    Ok(())
}

fn storage_kind(kind: CharacterKind) -> storage::CharacterKind {
    match kind {
        CharacterKind::Pc => storage::CharacterKind::Pc,
        CharacterKind::Npc => storage::CharacterKind::Npc,
    }
}

fn rules_kind(kind: storage::CharacterKind) -> CharacterKind {
    match kind {
        storage::CharacterKind::Pc => CharacterKind::Pc,
        storage::CharacterKind::Npc => CharacterKind::Npc,
    }
}

/// Parses a stored character. Filler NPCs have no mortal wound: at 0 HP they are dead.
fn parse_row(row: &CharacterRow) -> Result<Character, CmdError> {
    let mut character: Character = serde_json::from_str(&row.data)?;
    if row.kind == storage::CharacterKind::Npc {
        character.settle_npc();
    }
    Ok(character)
}

fn to_record(row: CharacterRow) -> Result<CharacterRecord, CmdError> {
    let character = parse_row(&row)?;
    Ok(CharacterRecord::new(row.id, character))
}

fn lock<'a>(
    state: &'a State<'_, AppStorage>,
) -> Result<std::sync::MutexGuard<'a, Storage>, CmdError> {
    state.0.lock().map_err(|_| CmdError::locked("storage"))
}

#[tauri::command]
fn list_characters(
    state: State<'_, AppStorage>,
    kind: CharacterKind,
) -> Result<Vec<CharacterRecord>, CmdError> {
    let rows = lock(&state)?.list_characters(storage_kind(kind))?;
    rows.into_iter().map(to_record).collect()
}

/// Quick create: a valid default sheet with the given name.
#[tauri::command]
fn create_character(
    state: State<'_, AppStorage>,
    kind: CharacterKind,
    name: String,
) -> Result<CharacterRecord, CmdError> {
    let character = Character::quick(&name);
    character.validate()?;
    let data = serde_json::to_string(&character)?;
    let storage = lock(&state)?;
    let id = storage
        .insert_character(storage_kind(kind), &character.name, &data)
        ?;
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
) -> Result<CharacterRecord, CmdError> {
    character.name = character.name.trim().to_string();
    character.validate()?;
    let role_catalog = SeedDataSource::default().roles()?.entries;
    rules_core::roles::validate_against(&character.roles, &role_catalog)?;
    let storage = lock(&state)?;
    let kind = storage
        .get_character(id)
        ?
        .ok_or_else(|| CmdError::not_found("character", id))?
        .kind;
    if kind == storage::CharacterKind::Npc {
        character.settle_npc();
    }
    let data = serde_json::to_string(&character)?;
    if !storage.update_character(id, &character.name, &data)? {
        return Err(CmdError::not_found("character", id));
    }
    Ok(CharacterRecord::new(id, character))
}

/// Where files of one type live: a sub-folder of the exports folder.
fn kind_dir(root: &std::path::Path, kind: ExportKind) -> std::path::PathBuf {
    root.join(kind.folder())
}

/// Path of an export file the UI named: a typed file lies in the folder of its type, a file from
/// before types in the exports folder itself. `None` for any other name (paths, other types).
fn resolve_export(root: &std::path::Path, kind: ExportKind, name: &str) -> Option<std::path::PathBuf> {
    if transfer::is_typed_file_name(kind, name) {
        Some(kind_dir(root, kind).join(name))
    } else if transfer::is_legacy_file_name(kind, name) {
        Some(root.join(name))
    } else {
        None
    }
}

/// `name.character.redcompanion.json`, or `name-2.character.redcompanion.json` and so on when it
/// already exists in `dir`.
fn free_export_name(dir: &std::path::Path, name: String, suffix: &str) -> String {
    if !dir.join(&name).exists() {
        return name;
    }
    let stem = name.strip_suffix(suffix).unwrap_or(&name).to_string();
    let mut n = 2;
    loop {
        let candidate = format!("{stem}-{n}{suffix}");
        if !dir.join(&candidate).exists() {
            return candidate;
        }
        n += 1;
    }
}

/// Writes `json` as a new file of this type into its folder and says where it went.
fn write_export(
    root: &std::path::Path,
    kind: ExportKind,
    base: &str,
    json: &str,
) -> Result<(String, std::path::PathBuf), CmdError> {
    let dir = kind_dir(root, kind);
    std::fs::create_dir_all(&dir)?;
    let name = free_export_name(&dir, transfer::typed_file_name(kind, base), &kind.suffix());
    let path = dir.join(&name);
    std::fs::write(&path, json)?;
    Ok((name, path))
}

/// The characters with these ids (everyone when `None`) as export items.
fn export_items(storage: &Storage, ids: Option<&[i64]>) -> Result<Vec<ExportedCharacter>, CmdError> {
    let mut items = Vec::new();
    for kind in [CharacterKind::Pc, CharacterKind::Npc] {
        for row in storage.list_characters(storage_kind(kind))? {
            if ids.is_some_and(|ids| !ids.contains(&row.id)) {
                continue;
            }
            let character: Character = serde_json::from_str(&row.data)?;
            items.push(ExportedCharacter { kind, character });
        }
    }
    Ok(items)
}

/// Turns the raw bytes of a picture (the request body) into a portrait data URI; the sheet stores
/// it with the next save. The frame comes in the `crop-x`, `crop-y`, `crop-size` headers (see
/// `portrait::Crop`); without them the biggest centred square is taken.
#[tauri::command]
fn make_portrait(request: tauri::ipc::Request<'_>) -> Result<String, CmdError> {
    let tauri::ipc::InvokeBody::Raw(bytes) = request.body() else {
        return Err(CmdError::portrait_unreadable());
    };
    let header = |name: &str, default: f32| {
        request.headers().get(name).and_then(|v| v.to_str().ok()).and_then(|v| v.parse().ok()).unwrap_or(default)
    };
    let crop = portrait::Crop { cx: header("crop-x", 0.5), cy: header("crop-y", 0.5), size: header("crop-size", 1.0) };
    portrait::encode(bytes, crop).map_err(|e| match e {
        portrait::PortraitError::TooLarge => CmdError::portrait_too_large(portrait::SOURCE_MAX_BYTES / (1024 * 1024)),
        portrait::PortraitError::Unreadable => CmdError::portrait_unreadable(),
    })
}

/// Writes characters (`ids`, or everyone when `None`) into a new character file.
#[tauri::command]
fn export_characters(
    state: State<'_, AppStorage>,
    dir: State<'_, ExportsDir>,
    ids: Option<Vec<i64>>,
) -> Result<ExportSaved, CmdError> {
    let storage = lock(&state)?;
    let items = export_items(&storage, ids.as_deref())?;
    let count = u32::try_from(items.len()).unwrap_or(u32::MAX);
    // One character is named after itself, several after "characters".
    let base = match items.as_slice() {
        [only] => only.character.name.clone(),
        _ => String::new(),
    };
    let json = transfer::export(items)?;
    let (name, path) = write_export(&dir.0, ExportKind::Character, &base, &json)?;
    Ok(ExportSaved { name, path: path.display().to_string(), count })
}

/// Writes the participants of the active session into a new session file, named after the session.
#[tauri::command]
fn export_session(
    state: State<'_, AppStorage>,
    dir: State<'_, ExportsDir>,
) -> Result<ExportSaved, CmdError> {
    let storage = lock(&state)?;
    let session = ensure_session(&storage)?;
    let members = storage.container_members(session)?;
    let name = storage
        .list_sessions()
        ?
        .into_iter()
        .find(|s| s.id == session)
        .map(|s| s.name)
        .unwrap_or_default();
    let items = export_items(&storage, Some(&members))?;
    let count = u32::try_from(items.len()).unwrap_or(u32::MAX);
    let json = transfer::export(items)?;
    let (name, path) = write_export(&dir.0, ExportKind::Session, &name, &json)?;
    Ok(ExportSaved { name, path: path.display().to_string(), count })
}

/// Files of `dir` accepted by `is_name`, as list entries.
fn files_in(dir: &std::path::Path, is_name: impl Fn(&str) -> bool) -> Vec<ExportFile> {
    let Ok(entries) = std::fs::read_dir(dir) else { return Vec::new() };
    entries
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
        .collect()
}

/// Files an import window of this type offers, the newest first: the typed files of its own
/// folder, and (for characters and encounters) the older untyped ones in the exports folder.
#[tauri::command]
fn list_exports(dir: State<'_, ExportsDir>, kind: ExportKind) -> Vec<ExportFile> {
    let mut files = files_in(&kind_dir(&dir.0, kind), |n| transfer::is_typed_file_name(kind, n));
    files.extend(files_in(&dir.0, |n| transfer::is_legacy_file_name(kind, n)));
    files.sort_by(|a, b| b.modified_ms.cmp(&a.modified_ms).then_with(|| a.name.cmp(&b.name)));
    files
}

/// Writes the chosen encounters and own templates into a new `.redencounters.json` file. An
/// encounter always takes the own templates it uses along, so the file stands on its own.
#[tauri::command]
fn export_encounters(
    state: State<'_, AppStorage>,
    dir: State<'_, ExportsDir>,
    encounter_ids: Vec<i64>,
    template_ids: Vec<i64>,
) -> Result<EncountersSaved, CmdError> {
    let storage = lock(&state)?;
    let encounters: Vec<Encounter> = storage
        .list_docs(DocTable::Encounters)
        ?
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
        ?
        .iter()
        .filter(|row| wanted.contains(&row.id))
        .map(|row| user_template_record(row).map(|r| (r.id, r.tier, r.character)))
        .collect::<Result<_, _>>()?;
    if encounters.is_empty() && templates.is_empty() {
        return Err(CmdError::nothing_to_export());
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
    let json = encounter_transfer::export(templates, encounters)?;
    let (name, path) = write_export(&dir.0, ExportKind::Encounters, &base, &json)?;
    Ok(EncountersSaved {
        name,
        path: path.display().to_string(),
        encounters: counts.0,
        templates: counts.1,
    })
}

/// Imports an encounters file of the exports folder as new templates and encounters (nothing is
/// merged by name). The whole file is validated before anything is stored.
#[tauri::command]
fn import_encounter_file(
    state: State<'_, AppStorage>,
    dir: State<'_, ExportsDir>,
    name: String,
) -> Result<EncountersImported, CmdError> {
    let path = resolve_export(&dir.0, ExportKind::Encounters, &name)
        .ok_or_else(|| CmdError::wrong_file_kind(&name))?;
    let json = std::fs::read_to_string(path)?;
    store_encounters_file(&state, &json)
}

/// Validates an encounters file and stores its templates and encounters as new ones.
fn store_encounters_file(state: &State<'_, AppStorage>, json: &str) -> Result<EncountersImported, CmdError> {
    let file = encounter_transfer::parse_import(json, &SeedDataSource::default())?;
    let storage = lock(state)?;
    let mut new_ids = std::collections::HashMap::new();
    let mut templates = Vec::new();
    for t in file.templates {
        let doc = UserTemplateDoc { tier: t.tier, character: t.character, based_on: None };
        let data = serde_json::to_string(&doc)?;
        let id = storage
            .insert_doc(DocTable::UserTemplates, &doc.character.name, &data)
            ?;
        new_ids.insert(t.key, id);
        templates.push(UserTemplateRecord { id, tier: doc.tier, character: doc.character, based_on: None });
    }
    let mut encounters = Vec::new();
    for mut encounter in file.encounters {
        encounter_transfer::remap_templates(&mut encounter, &new_ids);
        let data = serde_json::to_string(&encounter)?;
        let id = storage
            .insert_doc(DocTable::Encounters, &encounter.name, &data)
            ?;
        encounters.push(EncounterRecord { id, encounter });
    }
    Ok(EncountersImported { templates, encounters })
}

/// Imports a character or session file of the exports folder as new characters (they join the
/// active session); the whole file is validated before anything is stored. Only a bare file name
/// of that type is accepted.
#[tauri::command]
fn import_export_file(
    state: State<'_, AppStorage>,
    dir: State<'_, ExportsDir>,
    kind: ExportKind,
    name: String,
) -> Result<Vec<CharacterRecord>, CmdError> {
    if kind == ExportKind::Encounters {
        return Err(CmdError::wrong_file_kind(&name));
    }
    let path = resolve_export(&dir.0, kind, &name)
        .ok_or_else(|| CmdError::wrong_file_kind(&name))?;
    let json = std::fs::read_to_string(path)?;
    store_characters_file(&state, &json)
}

/// Validates a characters or session file and stores its characters as new ones.
fn store_characters_file(state: &State<'_, AppStorage>, json: &str) -> Result<Vec<CharacterRecord>, CmdError> {
    let items = transfer::parse_import(json, &SeedDataSource::default())?;
    let storage = lock(state)?;
    let mut records = Vec::new();
    for mut item in items {
        if item.kind == CharacterKind::Npc {
            item.character.settle_npc();
        }
        let data = serde_json::to_string(&item.character)?;
        let id = storage
            .insert_character(storage_kind(item.kind), &item.character.name, &data)
            ?;
        join_active_session(&storage, id)?;
        records.push(CharacterRecord::new(id, item.character));
    }
    Ok(records)
}

/// Lets the user choose a characters or session file in the system dialog and imports it. `None` when the
/// dialog is closed without a choice.
#[tauri::command]
async fn import_picked_file(
    app: AppHandle,
    state: State<'_, AppStorage>,
) -> Result<Option<Vec<CharacterRecord>>, CmdError> {
    let Some(json) = picker::pick_and_read(&app).await? else { return Ok(None) };
    store_characters_file(&state, &json).map(Some)
}

/// Same for an encounters file.
#[tauri::command]
async fn import_picked_encounter_file(
    app: AppHandle,
    state: State<'_, AppStorage>,
) -> Result<Option<EncountersImported>, CmdError> {
    let Some(json) = picker::pick_and_read(&app).await? else { return Ok(None) };
    store_encounters_file(&state, &json).map(Some)
}

/// Hands a file just written to the exports folder over to the system "save as" dialog. False when the
/// dialog is closed without a choice (the copy in the exports folder stays).
#[tauri::command]
async fn deliver_export(
    app: AppHandle,
    dir: State<'_, ExportsDir>,
    kind: ExportKind,
    name: String,
) -> Result<bool, CmdError> {
    let path = resolve_export(&dir.0, kind, &name).ok_or_else(|| CmdError::wrong_file_kind(&name))?;
    let bytes = std::fs::read(path)?;
    picker::save_bytes(&app, &name, bytes).await
}

/// Whether the app runs on a phone or tablet, where files move through system dialogs instead of a folder.
#[tauri::command]
fn is_mobile() -> bool {
    cfg!(mobile)
}

/// Shows the exports folder (or the sub-folder of one type) in the system file manager.
#[cfg(desktop)]
#[tauri::command]
fn open_exports_folder(
    app: AppHandle,
    dir: State<'_, ExportsDir>,
    kind: Option<ExportKind>,
) -> Result<(), CmdError> {
    let target = kind.map_or_else(|| dir.0.clone(), |k| kind_dir(&dir.0, k));
    std::fs::create_dir_all(&target)?;
    app.opener().open_path(target.display().to_string(), None::<&str>).map_err(CmdError::from)
}

/// Android keeps the exports folder in the app sandbox, where no file manager reaches it; files move through
/// the system save/open dialogs instead (ROADMAP 8.3). The command stays so the list of commands is the same.
#[cfg(not(desktop))]
#[tauri::command]
fn open_exports_folder(dir: State<'_, ExportsDir>, kind: Option<ExportKind>) -> Result<(), CmdError> {
    let target = kind.map_or_else(|| dir.0.clone(), |k| kind_dir(&dir.0, k));
    std::fs::create_dir_all(&target)?;
    Ok(())
}

/// Deletes a character at once and returns a snapshot of what went with it, for `restore_character`.
#[tauri::command]
fn delete_character(state: State<'_, AppStorage>, id: i64) -> Result<DeletedCharacter, CmdError> {
    let storage = lock(&state)?;
    delete_character_in(&storage, id)
}

fn delete_character_in(storage: &Storage, id: i64) -> Result<DeletedCharacter, CmdError> {
    let row = storage.get_character(id)?.ok_or_else(|| CmdError::not_found("character", id))?;
    let containers = storage.containers_of(id)?;
    // Membership rows go with the character (ON DELETE CASCADE).
    storage.delete_character(id)?;
    // A deleted character must not linger behind a cover or in a fight, in any session.
    let mut scenes = Vec::new();
    for scene_row in storage.all_scenes()? {
        let mut scene: Scene = serde_json::from_str(&scene_row.data)?;
        let before = serde_json::to_string(&scene)?;
        purge_from_scene(storage, &mut scene, id)?;
        let after = serde_json::to_string(&scene)?;
        if before != after {
            storage.save_scene(scene_row.id, &after)?;
            scenes.push(SceneSnapshot { id: scene_row.id, before, after });
        }
    }
    Ok(DeletedCharacter {
        id,
        kind: rules_kind(row.kind),
        owner_session: row.owner_session,
        name: row.name,
        data: row.data,
        containers,
        scenes,
    })
}

/// Undoes `delete_character`: the character comes back under its old id, with its shortcuts
/// (to containers that still exist) and its place in fights and covers (in scenes nobody has
/// changed since).
#[tauri::command]
fn restore_character(state: State<'_, AppStorage>, deleted: DeletedCharacter) -> Result<(), CmdError> {
    let storage = lock(&state)?;
    restore_character_in(&storage, deleted)
}

fn restore_character_in(storage: &Storage, deleted: DeletedCharacter) -> Result<(), CmdError> {
    // The sheet must still parse; the row is put back exactly as it was.
    serde_json::from_str::<Character>(&deleted.data)?;
    // An instance comes back as one only while its session exists; otherwise it joins the library.
    let owner = match deleted.owner_session {
        Some(session) if storage.container(session)?.is_some() => Some(session),
        _ => None,
    };
    if !storage.restore_character(deleted.id, storage_kind(deleted.kind), &deleted.name, &deleted.data, owner)? {
        return Err(CmdError::already_exists("character", deleted.id));
    }
    for container in deleted.containers {
        if storage.container(container)?.is_some() {
            storage.add_member(container, deleted.id)?;
        }
    }
    let current = storage.all_scenes()?;
    for snapshot in deleted.scenes {
        if current.iter().any(|row| row.id == snapshot.id && row.data == snapshot.after) {
            storage.save_scene(snapshot.id, &snapshot.before)?;
        }
    }
    Ok(())
}

/// Takes a character out of a scene's covers and fight.
fn purge_from_scene(storage: &Storage, scene: &mut Scene, character_id: i64) -> Result<(), CmdError> {
    scene.unlink_character(character_id);
    if scene.combat.combatants.iter().any(|c| c.character_id == character_id) {
        let skip = dead_ids(storage, scene)?;
        scene.combat.remove(character_id, &skip)?;
        // The turn may have passed to a mortally wounded fighter (or to nobody).
        refresh_death_save_due(scene, storage)?;
    }
    Ok(())
}

fn save_scene_row(storage: &Storage, id: i64, scene: &Scene) -> Result<(), CmdError> {
    let data = serde_json::to_string(scene)?;
    storage.save_scene(id, &data)?;
    Ok(())
}

const SESSION_NAME_MAX: usize = 60;

fn clean_session_name(name: &str) -> Result<String, CmdError> {
    let name = name.trim();
    if name.is_empty() || name.chars().count() > SESSION_NAME_MAX {
        return Err(CmdError::name_length(SESSION_NAME_MAX));
    }
    Ok(name.to_string())
}

/// Creates a session with an empty scene; returns the session id.
fn new_session(storage: &Storage, name: &str) -> Result<i64, CmdError> {
    let name = clean_session_name(name)?;
    let session = storage.create_session(&name)?;
    let data = serde_json::to_string(&Scene::default())?;
    storage.create_scene(session, "Scene", &data)?;
    Ok(session)
}

/// The session the app works with. If none is active, the first one becomes active; with no
/// sessions at all a default one is created (the UI normally creates a localized one first).
fn ensure_session(storage: &Storage) -> Result<i64, CmdError> {
    if let Some(id) = storage.active_session()? {
        return Ok(id);
    }
    let first = storage.list_sessions()?.first().map(|s| s.id);
    let id = match first {
        Some(id) => id,
        None => new_session(storage, "Session")?,
    };
    storage.set_active_session(id)?;
    Ok(id)
}

/// Puts a new player character into the active session. NPCs do not live in sessions (the GM
/// keeps them in the library and in folders, and drags them into a fight), so they are skipped.
fn join_active_session(storage: &Storage, character_id: i64) -> Result<(), CmdError> {
    if storage.get_character(character_id)?.is_some_and(|row| row.kind == storage::CharacterKind::Npc) {
        return Ok(());
    }
    let session = ensure_session(storage)?;
    storage.add_member(session, character_id).map_err(CmdError::from)
}

fn session_infos(storage: &Storage) -> Result<Vec<SessionInfo>, CmdError> {
    ensure_session(storage)?;
    let rows = storage.list_sessions()?;
    Ok(rows.into_iter().map(|r| SessionInfo { id: r.id, name: r.name, active: r.active }).collect())
}

/// Loads the scene of the active session.
fn load_scene(storage: &Storage) -> Result<(i64, Scene), CmdError> {
    let session = ensure_session(storage)?;
    match storage.active_scene()? {
        Some(row) => {
            let scene: Scene = serde_json::from_str(&row.data)?;
            scene.validate()?;
            Ok((row.id, scene))
        }
        None => {
            let scene = Scene::default();
            let data = serde_json::to_string(&scene)?;
            let id = storage.create_scene(session, "Scene", &data)?;
            Ok((id, scene))
        }
    }
}

/// Loads the working scene, applies `change` and saves only when it succeeds.
fn with_scene(
    state: &State<'_, AppStorage>,
    change: impl FnOnce(&mut Scene) -> Result<(), CmdError>,
) -> Result<SceneRecord, CmdError> {
    let storage = lock(state)?;
    let (id, mut scene) = load_scene(&storage)?;
    change(&mut scene)?;
    save_scene_row(&storage, id, &scene)?;
    Ok(SceneRecord { id, scene })
}

/// Creates a session with an empty scene and switches to it.
#[tauri::command]
fn create_session(state: State<'_, AppStorage>, name: String) -> Result<Vec<SessionInfo>, CmdError> {
    let storage = lock(&state)?;
    let id = new_session(&storage, &name)?;
    storage.set_active_session(id)?;
    session_infos(&storage)
}

/// Longest session note text, in characters; a guard against a runaway paste, not a design limit.
const SESSION_NOTES_MAX: usize = 100_000;

/// Notes of a session (empty text when none were written).
#[tauri::command]
fn session_notes(state: State<'_, AppStorage>, id: i64) -> Result<String, CmdError> {
    let storage = lock(&state)?;
    match storage.session_notes(id)? {
        Some(notes) => Ok(notes),
        None => Err(CmdError::not_found("session", id)),
    }
}

#[tauri::command]
fn set_session_notes(state: State<'_, AppStorage>, id: i64, notes: String) -> Result<(), CmdError> {
    if notes.chars().count() > SESSION_NOTES_MAX {
        return Err(CmdError::notes_too_long(SESSION_NOTES_MAX));
    }
    let storage = lock(&state)?;
    if !storage.set_session_notes(id, &notes)? {
        return Err(CmdError::not_found("session", id));
    }
    Ok(())
}

#[tauri::command]
fn switch_session(state: State<'_, AppStorage>, id: i64) -> Result<Vec<SessionInfo>, CmdError> {
    let storage = lock(&state)?;
    if !storage.set_active_session(id)? {
        return Err(CmdError::not_found("session", id));
    }
    session_infos(&storage)
}

/// The explorer tree as a flat list: sessions and folders with the characters lying in them.
fn explorer_nodes(storage: &Storage) -> Result<Vec<ExplorerNode>, CmdError> {
    let memberships = storage.all_memberships()?;
    let instances = storage.all_instances()?;
    Ok(storage
        .list_containers()?
        .into_iter()
        .map(|c| ExplorerNode {
            id: c.id,
            kind: match c.kind {
                storage::ContainerKind::Session => NodeKind::Session,
                storage::ContainerKind::Folder => NodeKind::Folder,
            },
            parent_id: c.parent_id,
            name: c.name,
            active: c.active,
            members: memberships.iter().filter(|(n, _)| *n == c.id).map(|(_, ch)| *ch).collect(),
            instances: instances.iter().filter(|(n, _)| *n == c.id).map(|(_, ch)| *ch).collect(),
        })
        .collect())
}

#[tauri::command]
fn explorer_tree(state: State<'_, AppStorage>) -> Result<Vec<ExplorerNode>, CmdError> {
    let storage = lock(&state)?;
    explorer_nodes(&storage)
}

/// Creates a folder at the root, or inside a root folder.
#[tauri::command]
fn create_folder(
    state: State<'_, AppStorage>,
    parent: Option<i64>,
    name: String,
) -> Result<Vec<ExplorerNode>, CmdError> {
    let storage = lock(&state)?;
    let name = clean_session_name(&name)?;
    if storage.create_folder(parent, &name)?.is_none() {
        return Err(CmdError::invalid_folder_parent());
    }
    explorer_nodes(&storage)
}

/// Renames a session or a folder.
#[tauri::command]
fn rename_node(
    state: State<'_, AppStorage>,
    id: i64,
    name: String,
) -> Result<Vec<ExplorerNode>, CmdError> {
    let storage = lock(&state)?;
    let name = clean_session_name(&name)?;
    if !storage.rename_container(id, &name)? {
        return Err(CmdError::not_found("container", id));
    }
    explorer_nodes(&storage)
}

/// Moves a folder to the root (`parent` null) or into a root folder.
#[tauri::command]
fn move_folder(
    state: State<'_, AppStorage>,
    id: i64,
    parent: Option<i64>,
) -> Result<Vec<ExplorerNode>, CmdError> {
    let storage = lock(&state)?;
    if !storage.container(id)?.is_some_and(|c| c.kind == storage::ContainerKind::Folder) {
        return Err(CmdError::not_found("container", id));
    }
    if !storage.move_folder(id, parent)? {
        return Err(CmdError::invalid_folder_parent());
    }
    explorer_nodes(&storage)
}

/// Deletes a session (with its scene and fight) or a folder (with its subfolders). The characters
/// stay in the library. Deleting the active session hands the role to another one.
#[tauri::command]
fn delete_node(state: State<'_, AppStorage>, id: i64) -> Result<Vec<ExplorerNode>, CmdError> {
    let storage = lock(&state)?;
    if !storage.delete_container(id)? {
        return Err(CmdError::not_found("container", id));
    }
    if storage.active_session()?.is_none() && !storage.list_sessions()?.is_empty() {
        ensure_session(&storage)?;
    }
    explorer_nodes(&storage)
}

/// Puts a shortcut to a character into a session or a folder.
#[tauri::command]
fn add_to_container(
    state: State<'_, AppStorage>,
    container_id: i64,
    character_id: i64,
) -> Result<Vec<ExplorerNode>, CmdError> {
    let storage = lock(&state)?;
    if storage.container(container_id)?.is_none() {
        return Err(CmdError::not_found("container", container_id));
    }
    let Some(row) = storage.get_character(character_id)? else {
        return Err(CmdError::not_found("character", character_id));
    };
    // An instance put anywhere but its own session (a folder, another session) is meant to be kept:
    // it becomes an ordinary library character, otherwise it would vanish with its session.
    if row.owner_session.is_some_and(|owner| owner != container_id) {
        storage.set_character_owner(character_id, None)?;
    }
    storage.add_member(container_id, character_id)?;
    explorer_nodes(&storage)
}

/// Keeps an instance: it stops belonging to its session and becomes a library character.
#[tauri::command]
fn save_instance_to_library(
    state: State<'_, AppStorage>,
    character_id: i64,
) -> Result<Vec<ExplorerNode>, CmdError> {
    let storage = lock(&state)?;
    if !storage.set_character_owner(character_id, None)? {
        return Err(CmdError::not_found("character", character_id));
    }
    explorer_nodes(&storage)
}

/// Deletes every instance of a session ("clear NPCs"); returns the snapshots, one per instance, for
/// "undo" (`restore_character` each).
#[tauri::command]
fn clear_session_instances(
    state: State<'_, AppStorage>,
    session_id: i64,
) -> Result<Vec<DeletedCharacter>, CmdError> {
    let storage = lock(&state)?;
    if !storage.container(session_id)?.is_some_and(|c| c.kind == storage::ContainerKind::Session) {
        return Err(CmdError::not_found("session", session_id));
    }
    let mut deleted = Vec::new();
    for id in storage.instances_of(session_id)? {
        deleted.push(delete_character_in(&storage, id)?);
    }
    Ok(deleted)
}

/// Takes the shortcut out of a session or a folder; the character stays in the library. Who
/// fights is a separate matter (the initiative order), so leaving a session changes no fight.
#[tauri::command]
fn remove_from_container(
    state: State<'_, AppStorage>,
    container_id: i64,
    character_id: i64,
) -> Result<Vec<ExplorerNode>, CmdError> {
    let storage = lock(&state)?;
    if storage.container(container_id)?.is_none() {
        return Err(CmdError::not_found("container", container_id));
    }
    // An instance has no life outside its session: taking it out of that session deletes it.
    if storage.get_character(character_id)?.is_some_and(|row| row.owner_session == Some(container_id)) {
        delete_character_in(&storage, character_id)?;
    } else {
        storage.remove_member(container_id, character_id)?;
    }
    explorer_nodes(&storage)
}

/// Solo's Damage Deflection for the "damage taken" window: how much, and whether this is the
/// character's first damage of the round in a running fight.
#[tauri::command]
fn deflection_info(
    state: State<'_, AppStorage>,
    character_id: i64,
) -> Result<damage_event::DeflectionInfo, CmdError> {
    let storage = lock(&state)?;
    let character = require_character(&storage, character_id)?;
    let (_, scene) = load_scene(&storage)?;
    Ok(damage_event::deflection_info(&character, character_id, &scene))
}

#[tauri::command]
fn get_scene(state: State<'_, AppStorage>) -> Result<SceneRecord, CmdError> {
    with_scene(&state, |_| Ok(()))
}

#[tauri::command]
fn list_cover_presets(lang: State<'_, ContentLang>) -> Result<Vec<CoverPreset>, CmdError> {
    cover_presets(&content(&lang)?).map_err(CmdError::from)
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
) -> Result<SceneRecord, CmdError> {
    let preset = match preset {
        Some(id) => Some(
            cover_presets(&content(&lang)?)
                ?
                .into_iter()
                .find(|p| p.id == id)
                .ok_or_else(|| CmdError::unknown_preset(&id))?,
        ),
        None => None,
    };
    with_scene(&state, |scene| {
        scene.add_cover(preset.as_ref(), &name, hp).map(|_| ()).map_err(CmdError::from)
    })
}

#[tauri::command]
fn edit_cover(
    state: State<'_, AppStorage>,
    cover_id: u32,
    name: String,
    hp: u32,
    max_hp: u32,
) -> Result<SceneRecord, CmdError> {
    with_scene(&state, |scene| {
        scene.edit_cover(cover_id, &name, hp, max_hp).map_err(CmdError::from)
    })
}

#[tauri::command]
fn remove_cover(state: State<'_, AppStorage>, cover_id: u32) -> Result<SceneRecord, CmdError> {
    with_scene(&state, |scene| scene.remove_cover(cover_id).map_err(CmdError::from))
}

/// Puts a character behind a cover, or out of cover with `cover_id = None`.
#[tauri::command]
fn set_character_cover(
    state: State<'_, AppStorage>,
    character_id: i64,
    cover_id: Option<u32>,
) -> Result<SceneRecord, CmdError> {
    with_scene(&state, |scene| {
        scene.set_character_cover(character_id, cover_id).map_err(CmdError::from)
    })
}

fn load_character(storage: &Storage, id: i64) -> Result<Option<Character>, CmdError> {
    match storage.get_character(id)? {
        Some(row) => Ok(Some(parse_row(&row)?)),
        None => Ok(None),
    }
}

fn require_character(storage: &Storage, id: i64) -> Result<Character, CmdError> {
    load_character(storage, id)?.ok_or_else(|| CmdError::not_found("character", id))
}

fn store_character(storage: &Storage, id: i64, character: &Character) -> Result<(), CmdError> {
    let data = serde_json::to_string(character)?;
    storage.update_character(id, &character.name, &data)?;
    Ok(())
}

/// Ids of fighters who cannot take a turn: dead, or no longer in the database.
fn dead_ids(storage: &Storage, scene: &Scene) -> Result<Vec<i64>, CmdError> {
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
fn d10_check(manual: Option<Vec<u32>>) -> Result<CheckRoll, CmdError> {
    match manual {
        Some(faces) => check_manual(&faces).map_err(CmdError::from),
        None => Ok(roll_check(&mut ThreadDice::new())),
    }
}

/// Marks whether the fighter whose turn just began owes a death save.
fn refresh_death_save_due(scene: &mut Scene, storage: &Storage) -> Result<(), CmdError> {
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
    change: impl FnOnce(&mut Scene, &Storage) -> Result<(), CmdError>,
) -> Result<SceneRecord, CmdError> {
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
) -> Result<SceneRecord, CmdError> {
    with_combat(&state, |scene, storage| {
        let character = require_character(storage, character_id)?;
        let (reflex, bonus) = (character.stats.reflex, rules_core::roles::initiative_bonus(&character));
        let die = d10_check(manual)?;
        scene
            .combat
            .roll_with_bonus(character_id, reflex, bonus, die)
            .map_err(CmdError::from)?;
        refresh_death_save_due(scene, storage)
    })
}

/// Sets the initiative total by hand (the field on a card); adds the character to the order
/// if they had none yet.
#[tauri::command]
fn combat_set_initiative(
    state: State<'_, AppStorage>,
    character_id: i64,
    initiative: i32,
) -> Result<SceneRecord, CmdError> {
    with_combat(&state, |scene, storage| {
        let reflex = require_character(storage, character_id)?.stats.reflex;
        scene.combat.set_or_add(character_id, reflex, initiative).map_err(CmdError::from)?;
        refresh_death_save_due(scene, storage)
    })
}

/// Puts a fighter at a place in the order (the card was dragged there).
#[tauri::command]
fn combat_move(
    state: State<'_, AppStorage>,
    character_id: i64,
    to: usize,
) -> Result<SceneRecord, CmdError> {
    with_combat(&state, |scene, storage| {
        scene.combat.move_to(character_id, to).map_err(CmdError::from)?;
        refresh_death_save_due(scene, storage)
    })
}

/// Moves a fighter one place earlier (`delta < 0`) or later (`delta > 0`).
#[tauri::command]
fn combat_shift(
    state: State<'_, AppStorage>,
    character_id: i64,
    delta: i32,
) -> Result<SceneRecord, CmdError> {
    with_combat(&state, |scene, _| {
        scene.combat.shift(character_id, delta).map_err(CmdError::from)
    })
}

#[tauri::command]
fn combat_remove(state: State<'_, AppStorage>, character_id: i64) -> Result<SceneRecord, CmdError> {
    with_combat(&state, |scene, storage| {
        let skip = dead_ids(storage, scene)?;
        scene.combat.remove(character_id, &skip)?;
        refresh_death_save_due(scene, storage)
    })
}

/// Puts a character into the fight with a freshly rolled initiative; someone already in the
/// fight keeps theirs (use `combat_roll` to re-roll).
fn add_fighter(scene: &mut Scene, storage: &Storage, id: i64) -> Result<(), CmdError> {
    let character = require_character(storage, id)?;
    if scene.combat.is_in_fight(id) {
        return Ok(());
    }
    let die = d10_check(None)?;
    let bonus = rules_core::roles::initiative_bonus(&character);
    scene
        .combat
        .add_with_bonus(id, character.stats.reflex, bonus, die)
        .map_err(CmdError::from)
}

/// Starts the fight with whoever is in the initiative order. With nobody there, the living
/// members of the active session fight: the GM only drags in the exceptions (absent players,
/// NPCs from a folder). Fighters without initiative get one rolled when added.
fn start_fight(scene: &mut Scene, storage: &Storage) -> Result<(), CmdError> {
    if scene.combat.combatants.is_empty() {
        let session = ensure_session(storage)?;
        for id in storage.container_members(session)? {
            let Some(character) = load_character(storage, id)? else { continue };
            if !character.dead {
                add_fighter(scene, storage, id)?;
            }
        }
    }
    let skip = dead_ids(storage, scene)?;
    scene.combat.start(&skip)?;
    refresh_death_save_due(scene, storage)
}

/// Adds a character to the fight (initiative is rolled for them), before or during the fight.
#[tauri::command]
fn combat_add(state: State<'_, AppStorage>, character_id: i64) -> Result<SceneRecord, CmdError> {
    with_combat(&state, |scene, storage| add_fighter(scene, storage, character_id))
}

/// Starts the fight (see `start_fight` for who takes part).
#[tauri::command]
fn combat_start(state: State<'_, AppStorage>) -> Result<SceneRecord, CmdError> {
    with_combat(&state, start_fight)
}

#[tauri::command]
fn combat_next_turn(state: State<'_, AppStorage>) -> Result<SceneRecord, CmdError> {
    with_combat(&state, |scene, storage| {
        let skip = dead_ids(storage, scene)?;
        scene.combat.next_turn(&skip)?;
        refresh_death_save_due(scene, storage)
    })
}

/// Steps the turn back (a mis-click, a missed action); nothing already dealt is undone.
#[tauri::command]
fn combat_prev_turn(state: State<'_, AppStorage>) -> Result<SceneRecord, CmdError> {
    with_combat(&state, |scene, storage| {
        let skip = dead_ids(storage, scene)?;
        scene.combat.prev_turn(&skip)?;
        refresh_death_save_due(scene, storage)
    })
}

#[tauri::command]
fn combat_end(state: State<'_, AppStorage>) -> Result<SceneRecord, CmdError> {
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
) -> Result<DeathSaveApplied, CmdError> {
    let storage = lock(&state)?;
    let mut character = require_character(&storage, character_id)?;
    let notation = DiceNotation { count: 1, sides: 10, modifier: 0 };
    let roll = match manual {
        Some(face) => roll_manual(notation, &[face])?,
        None => roll(notation, &mut ThreadDice::new()),
    };
    let save = vitals::apply_death_save(&mut character, roll)?;
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
) -> Result<StabilizeApplied, CmdError> {
    let storage = lock(&state)?;
    let mut target = require_character(&storage, target_id)?;
    let base = vitals::base_death_save_penalty(&target.injuries, &injury_table()?);
    let result = if granted {
        vitals::stabilize_granted(&mut target, base)?
    } else {
        if !STABILIZE_SKILLS.contains(&skill.as_str()) {
            return Err(vitals::VitalsError::BadSkill(skill).into());
        }
        let helper = if helper_id == target_id {
            target.clone()
        } else {
            require_character(&storage, helper_id)?
        };
        let bonus = i32::from(helper.stats.tech)
            + i32::from(helper.skills.get(&skill).copied().unwrap_or(0));
        let die = match manual_total {
            Some(total) => check_total(total)?,
            None => roll_check(&mut ThreadDice::new()),
        };
        vitals::stabilize(&mut target, bonus, die, base)?
    };
    store_character(&storage, target_id, &target)?;
    Ok(StabilizeApplied { character: CharacterRecord::new(target_id, target), result })
}

/// One attempt to heal an injury of `target_id`: `kind` first aid or treatment, with `skill`
/// (a way the injury's table row offers). The healer's bonus is TECH + skill level for first aid
/// and paramedic; surgery is a role ability not kept on the sheet, so its `bonus` is typed in.
/// `manual_total` is a physical d10 (explosion already added), `granted` declares success.
#[tauri::command]
#[allow(clippy::too_many_arguments)]
fn heal_injury(
    state: State<'_, AppStorage>,
    target_id: i64,
    helper_id: i64,
    injury_id: String,
    kind: HealKind,
    skill: TreatmentSkill,
    bonus: Option<i32>,
    manual_total: Option<i32>,
    granted: bool,
) -> Result<HealApplied, CmdError> {
    let storage = lock(&state)?;
    let mut target = require_character(&storage, target_id)?;
    let injury = injury_table()?
        .into_iter()
        .find(|i| i.id == injury_id)
        .ok_or_else(|| CmdError::not_found("injury", &injury_id))?;
    let (bonus, roll) = if granted {
        (0, None)
    } else {
        let bonus = match skill {
            TreatmentSkill::Surgery => match bonus {
                Some(b) => b,
                None => {
                    // A Medtech's Medicine gives the Surgery skill (p. 149); anyone else types it.
                    let helper = if helper_id == target_id {
                        target.clone()
                    } else {
                        require_character(&storage, helper_id)?
                    };
                    if helper.role(rules_core::roles::MEDTECH).is_none() {
                        return Err(CmdError::surgery_bonus_missing());
                    }
                    rules_core::roles::surgery_bonus(&helper)
                }
            },
            TreatmentSkill::FirstAid | TreatmentSkill::Paramedic => {
                let id = if skill == TreatmentSkill::FirstAid { "first_aid" } else { "paramedic" };
                let helper = if helper_id == target_id {
                    target.clone()
                } else {
                    require_character(&storage, helper_id)?
                };
                let catalog =
                    SeedDataSource::default().skills()?.entries;
                let (_, stat, level) =
                    skill_check::skill_bonus(&helper, id, &catalog)?;
                i32::from(stat) + i32::from(level)
            }
        };
        let die = match manual_total {
            Some(total) => check_total(total)?,
            None => roll_check(&mut ThreadDice::new()),
        };
        (bonus, Some(die))
    };
    let result = healing::heal_injury(&mut target, &injury, kind, skill, bonus, roll)
        ?;
    store_character(&storage, target_id, &target)?;
    Ok(HealApplied { character: CharacterRecord::new(target_id, target), result })
}

/// The GM puts a critical injury on a character by hand: no roll, no damage, no checks. The same
/// injury may be added again (a second broken leg).
#[tauri::command]
fn add_injury(
    state: State<'_, AppStorage>,
    id: i64,
    injury_id: String,
) -> Result<CharacterRecord, CmdError> {
    let storage = lock(&state)?;
    let mut character = require_character(&storage, id)?;
    let table = injury_table()?;
    let injury = table
        .iter()
        .find(|i| i.id == injury_id)
        .ok_or_else(|| CmdError::not_found("injury", &injury_id))?;
    character.add_injury(&injury.id, injury.death_save_penalty);
    store_character(&storage, id, &character)?;
    Ok(CharacterRecord::new(id, character))
}

/// The GM takes one instance of an injury off by hand (its death save penalty goes with it).
#[tauri::command]
fn remove_injury(
    state: State<'_, AppStorage>,
    id: i64,
    injury_id: String,
) -> Result<CharacterRecord, CmdError> {
    let storage = lock(&state)?;
    let mut character = require_character(&storage, id)?;
    let table = injury_table()?;
    let injury = table
        .iter()
        .find(|i| i.id == injury_id)
        .ok_or_else(|| CmdError::not_found("injury", &injury_id))?;
    if !character.remove_injury(&injury.id, injury.death_save_penalty) {
        return Err(CmdError::injury_not_on_sheet(&injury_id));
    }
    store_character(&storage, id, &character)?;
    Ok(CharacterRecord::new(id, character))
}

/// A Medtech (`helper_id`) gives a drug they picked to `target_id` (p. 150).
#[tauri::command]
fn administer_drug(
    state: State<'_, AppStorage>,
    target_id: i64,
    helper_id: i64,
    drug: String,
) -> Result<rules_core::roles::DrugApplied, CmdError> {
    let storage = lock(&state)?;
    let mut target = require_character(&storage, target_id)?;
    let helper = if helper_id == target_id {
        target.clone()
    } else {
        require_character(&storage, helper_id)?
    };
    let result = rules_core::roles::administer_drug(&helper, &mut target, &drug)?;
    store_character(&storage, target_id, &target)?;
    Ok(rules_core::roles::DrugApplied { character: CharacterRecord::new(target_id, target), result })
}

/// What a rest toward `goal` (HP to reach or days to pass) gives (computed by the rules, nothing is changed).
#[tauri::command]
fn rest_preview(
    state: State<'_, AppStorage>,
    id: i64,
    goal: RestGoal,
) -> Result<RestPlan, CmdError> {
    let character = require_character(&*lock(&state)?, id)?;
    healing::rest_plan_for(&character, goal).map_err(CmdError::from)
}

/// Rests toward `goal`; first aid wears off with the day.
#[tauri::command]
fn rest_character(
    state: State<'_, AppStorage>,
    id: i64,
    goal: RestGoal,
) -> Result<RestApplied, CmdError> {
    let storage = lock(&state)?;
    let mut character = require_character(&storage, id)?;
    let plan = healing::rest_for(&mut character, goal)?;
    store_character(&storage, id, &character)?;
    Ok(RestApplied { character: CharacterRecord::new(id, character), plan })
}

/// Takes humanity off a character (a number the GM decided on, or a roll the UI already made).
/// Going below zero is cyberpsychosis; nothing else changes on the sheet.
#[tauri::command]
fn lose_humanity(
    state: State<'_, AppStorage>,
    id: i64,
    loss: u32,
) -> Result<HumanityLossApplied, CmdError> {
    let storage = lock(&state)?;
    let mut character = require_character(&storage, id)?;
    let loss = humanity::lose_humanity(&mut character, loss)?;
    store_character(&storage, id, &character)?;
    Ok(HumanityLossApplied { character: CharacterRecord::new(id, character), loss })
}

/// The therapies that give humanity back, with their DV, dice and price.
#[tauri::command]
fn list_therapies() -> Vec<TherapyInfo> {
    humanity::therapies()
}

/// One week of therapy. The Medtech's `bonus` is typed in (the Medicine ability is not a sheet
/// skill); `manual_total` is a physical d10 for the check, `granted` declares it a success.
/// `restore_faces` are the faces of the restoration dice (2d6 / 4d6) read off physical dice.
#[tauri::command]
fn therapy(
    state: State<'_, AppStorage>,
    id: i64,
    kind: TherapyKind,
    bonus: i32,
    manual_total: Option<i32>,
    granted: bool,
    restore_faces: Option<Vec<u32>>,
) -> Result<TherapyApplied, CmdError> {
    let storage = lock(&state)?;
    let mut character = require_character(&storage, id)?;
    let check = if granted {
        None
    } else {
        Some(match manual_total {
            Some(total) => check_total(total)?,
            None => roll_check(&mut ThreadDice::new()),
        })
    };
    // The restoration is only rolled when the check succeeds, but a wrong set of faces should be
    // refused before anything happens.
    let notation: DiceNotation = kind.dice().parse()?;
    let restore = match restore_faces {
        Some(faces) => roll_manual(notation, &faces)?,
        None => roll(notation, &mut ThreadDice::new()),
    };
    let result = humanity::therapy(&mut character, kind, bonus, check, Some(restore))
        ?;
    store_character(&storage, id, &character)?;
    Ok(TherapyApplied { character: CharacterRecord::new(id, character), result })
}

fn injury_table() -> Result<Vec<CriticalInjury>, CmdError> {
    SeedDataSource::default().critical_injuries().map(|c| c.entries).map_err(CmdError::from)
}

#[tauri::command]
fn list_injuries() -> Result<Vec<CriticalInjury>, CmdError> {
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
) -> Result<SkillCheck, CmdError> {
    let character = require_character(&*lock(&state)?, character_id)?;
    let die = match manual_total {
        Some(total) => check_total(total)?,
        None => roll_check(&mut ThreadDice::new()),
    };
    let catalog = SeedDataSource::default().skills()?.entries;
    skill_check::skill_check(&character, &skill, &catalog, die).map_err(CmdError::from)
}

/// Rolls `notation` with the OS RNG, or builds the result from faces read off
/// physical dice (`manual`).
#[tauri::command]
fn roll_dice(notation: String, manual: Option<Vec<u32>>) -> Result<RollResult, CmdError> {
    let notation: DiceNotation = notation.parse()?;
    match manual {
        Some(faces) => roll_manual(notation, &faces).map_err(CmdError::from),
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
) -> Result<InjuryRoll, CmdError> {
    let character = require_character(&*lock(&state)?, character_id)?;
    damage_event::roll_injury(
        &injury_table()?,
        area,
        &character.injuries,
        manual.as_deref(),
        &mut ThreadDice::new(),
    )
    .map_err(CmdError::from)
}

/// Calculates a hit against the stored state without saving anything.
#[tauri::command]
fn preview_damage(
    state: State<'_, AppStorage>,
    character_id: i64,
    request: DamageRequest,
) -> Result<DamageOutcome, CmdError> {
    let storage = lock(&state)?;
    let character = require_character(&storage, character_id)?;
    let (_, scene) = load_scene(&storage)?;
    damage_event::evaluate(&character, &scene, &request).map_err(CmdError::from)
}

/// Applies a hit to the character and the scene's cover; saves both or neither.
#[tauri::command]
fn apply_damage(
    state: State<'_, AppStorage>,
    character_id: i64,
    request: DamageRequest,
    injury_id: Option<String>,
) -> Result<DamageApplied, CmdError> {
    let storage = lock(&state)?;
    let row = storage
        .get_character(character_id)
        ?
        .ok_or_else(|| CmdError::not_found("character", character_id))?;
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
    ?;
    // Filler NPCs have no death save: at 0 HP they are dead on the spot.
    if row.kind == storage::CharacterKind::Npc {
        damage_event::kill_downed_npc(&mut character, &mut scene);
    }
    character.validate()?;
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
) -> Result<CharacterRecord, CmdError> {
    let storage = lock(&state)?;
    let mut character = require_character(&storage, id)?;
    vitals::revive(&mut character)?;
    store_character(&storage, id, &character)?;
    Ok(CharacterRecord::new(id, character))
}

/// Undoes the scene's last damage event.
#[tauri::command]
fn undo_damage(state: State<'_, AppStorage>) -> Result<DamageApplied, CmdError> {
    let storage = lock(&state)?;
    let (scene_id, mut scene) = load_scene(&storage)?;
    let event = scene.last_damage.clone().ok_or_else(CmdError::nothing_to_undo)?;
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

/// The database file of the app.
const DB_FILE: &str = "red-companion.db";
/// Data folders (named after the app identifier) and database names of the app under its earlier
/// identifiers, newest first: the Favoterm build, then the first Red Companion build.
const LEGACY_DATA: [(&str, &str); 2] = [("app.favoterm", "favoterm.db"), ("com.user.red-companion", "red-companion.db")];

/// Copies the data of the app under an earlier identifier (database and exports) into the new data
/// folder, once: nothing is done when the new database already exists or there is no old one. The
/// old folder stays untouched. The database goes last, under a temporary name, so that an
/// interrupted copy is simply done again on the next start. True when something was copied.
fn migrate_legacy_data(dir: &std::path::Path, legacy_dir: &std::path::Path, legacy_db: &str) -> std::io::Result<bool> {
    let target = dir.join(DB_FILE);
    let source = legacy_dir.join(legacy_db);
    if target.exists() || !source.is_file() {
        return Ok(false);
    }
    let legacy_exports = legacy_dir.join("exports");
    if legacy_exports.is_dir() && !dir.join("exports").exists() {
        copy_dir(&legacy_exports, &dir.join("exports"))?;
    }
    // A journal left by an interrupted run belongs to the database: it goes along, renamed.
    for extra in ["-journal", "-wal", "-shm"] {
        let from = legacy_dir.join(format!("{legacy_db}{extra}"));
        if from.is_file() {
            std::fs::copy(&from, dir.join(format!("{DB_FILE}{extra}")))?;
        }
    }
    let temporary = dir.join(format!("{DB_FILE}.migrating"));
    std::fs::copy(&source, &temporary)?;
    std::fs::rename(&temporary, &target)?;
    Ok(true)
}

fn copy_dir(from: &std::path::Path, to: &std::path::Path) -> std::io::Result<()> {
    std::fs::create_dir_all(to)?;
    for entry in std::fs::read_dir(from)? {
        let entry = entry?;
        let path = entry.path();
        let destination = to.join(entry.file_name());
        if entry.file_type()?.is_dir() {
            copy_dir(&path, &destination)?;
        } else {
            std::fs::copy(&path, &destination)?;
        }
    }
    Ok(())
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    let builder = tauri::Builder::default();
    // Registered first: a second launch hands over to the running window instead of opening the same database twice.
    #[cfg(desktop)]
    let builder = builder.plugin(tauri_plugin_single_instance::init(|app, _args, _cwd| {
        if let Some(window) = app.get_webview_window("main") {
            let _ = window.unminimize();
            let _ = window.show();
            let _ = window.set_focus();
        }
    }));
    #[cfg(desktop)]
    let builder = builder.plugin(tauri_plugin_updater::Builder::new().build());
    #[cfg(not(desktop))]
    let builder = builder.plugin(updater::plugin());
    builder
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_fs::init())
        .setup(|app| {
            let log_dir = app.path().app_log_dir()?;
            startup::install_panic_hook(log_dir.clone());
            let dir = app.path().app_data_dir()?;
            std::fs::create_dir_all(&dir)?;
            if let Some(parent) = dir.parent() {
                // The newest earlier folder that has a database wins; the rest stay untouched.
                for (folder, db) in LEGACY_DATA {
                    if migrate_legacy_data(&dir, &parent.join(folder), db)? {
                        break;
                    }
                }
            }
            let db_path = dir.join(DB_FILE);
            let storage = match Storage::open(&db_path) {
                Ok(storage) => storage,
                Err(error) => {
                    startup::log_line(&log_dir, &format!("the database did not open: {error}"));
                    startup::show_database_error(app.handle(), &error, &db_path, &log_dir);
                    std::process::exit(1);
                }
            };
            let exports = dir.join("exports");
            std::fs::create_dir_all(&exports)?;
            app.manage(ExportsDir(exports));
            app.manage(AppStorage(Mutex::new(storage)));
            app.manage(ContentLang(Mutex::new(Lang::default())));
            app.manage(updater::PendingUpdate::default());
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            rules_info,
            make_portrait,
            set_content_lang,
            list_skills,
            list_articles,
            list_range_tables,
            list_martial_arts,
            list_roles,
            administer_drug,
            skill_check,
            list_armor,
            list_weapons,
            list_gear,
            list_ammo,
            list_weapon_mods,
            list_magazines,
            list_price_categories,
            list_specialties,
            list_clothing,
            list_street_drugs,
            list_npc_templates,
            create_npc_from_template,
            list_user_templates,
            save_character_as_template,
            delete_user_template,
            set_user_template_tier,
            update_user_template,
            book_template_sheet,
            save_book_template_copy,
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
            restore_character,
            updater::check_update,
            updater::install_update,
            updater::restart_app,
            save_instance_to_library,
            clear_session_instances,
            export_characters,
            list_exports,
            export_session,
            export_encounters,
            import_encounter_file,
            import_export_file,
            open_exports_folder,
            import_picked_file,
            import_picked_encounter_file,
            deliver_export,
            is_mobile,
            create_session,
            switch_session,
            session_notes,
            set_session_notes,
            explorer_tree,
            create_folder,
            rename_node,
            move_folder,
            delete_node,
            add_to_container,
            remove_from_container,
            get_scene,
            list_cover_presets,
            add_cover,
            edit_cover,
            remove_cover,
            set_character_cover,
            list_injuries,
            deflection_info,
            add_injury,
            remove_injury,
            roll_dice,
            roll_injury,
            preview_damage,
            apply_damage,
            undo_damage,
            revive_character,
            combat_roll,
            combat_add,
            preview_derived,
            combat_set_initiative,
            combat_shift,
            combat_move,
            combat_remove,
            combat_start,
            combat_next_turn,
            combat_prev_turn,
            combat_end,
            death_save,
            stabilize,
            heal_injury,
            rest_preview,
            rest_character,
            lose_humanity,
            list_therapies,
            therapy
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A fresh folder under the system temp dir, removed by the test at the end.
    fn temp_dir(tag: &str) -> std::path::PathBuf {
        let nanos = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_nanos();
        let dir = std::env::temp_dir().join(format!("red-companion-test-{tag}-{}-{nanos}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        dir
    }

    #[test]
    fn the_data_of_the_old_app_name_is_copied_once_and_left_in_place() {
        const OLD_DB: &str = "favoterm.db";
        let root = temp_dir("migrate");
        let (old, new) = (root.join("app.favoterm"), root.join("app.red-companion"));
        std::fs::create_dir_all(old.join("exports").join("characters")).unwrap();
        std::fs::create_dir_all(&new).unwrap();
        std::fs::write(old.join(OLD_DB), b"old database").unwrap();
        std::fs::write(old.join("exports").join("characters").join("v.character.redcompanion.json"), b"{}").unwrap();

        assert!(migrate_legacy_data(&new, &old, OLD_DB).unwrap());
        assert_eq!(std::fs::read(new.join(DB_FILE)).unwrap(), b"old database");
        assert!(new.join("exports").join("characters").join("v.character.redcompanion.json").is_file());
        assert!(!new.join(format!("{DB_FILE}.migrating")).exists());
        // The old folder is untouched.
        assert!(old.join(OLD_DB).is_file());

        // A second start changes nothing, even when the old data changed since.
        std::fs::write(old.join(OLD_DB), b"changed").unwrap();
        assert!(!migrate_legacy_data(&new, &old, OLD_DB).unwrap());
        assert_eq!(std::fs::read(new.join(DB_FILE)).unwrap(), b"old database");
        std::fs::remove_dir_all(&root).unwrap();
    }

    #[test]
    fn nothing_is_migrated_without_an_old_database() {
        let root = temp_dir("fresh");
        let (old, new) = (root.join("app.favoterm"), root.join("app.red-companion"));
        std::fs::create_dir_all(&new).unwrap();
        assert!(!migrate_legacy_data(&new, &old, "favoterm.db").unwrap());
        assert!(!new.join(DB_FILE).exists());
        std::fs::remove_dir_all(&root).unwrap();
    }

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
    fn reading_the_tree_of_an_empty_database_creates_no_session() {
        // The UI creates the first session itself, so that its name follows the language.
        let s = storage();
        assert!(explorer_nodes(&s).unwrap().is_empty());
        assert!(s.list_sessions().unwrap().is_empty());
        // Sessions exist but none is active: the tree shows it as it is.
        let one = new_session(&s, "One").unwrap();
        let tree = explorer_nodes(&s).unwrap();
        assert_eq!(tree.len(), 1);
        assert!(!tree[0].active && tree[0].id == one);
    }

    #[test]
    fn the_explorer_tree_lists_sessions_and_folders_with_their_shortcuts() {
        let s = storage();
        let session = new_session(&s, "Night 1").unwrap();
        let folder = s.create_folder(None, "Gangs").unwrap().unwrap();
        let ganger = s.insert_character(storage::CharacterKind::Npc, "Ganger", "{}").unwrap();
        s.add_member(folder, ganger).unwrap();
        s.add_member(session, ganger).unwrap();

        let tree = explorer_nodes(&s).unwrap();
        assert_eq!(tree.len(), 2);
        let node = |id| tree.iter().find(|n| n.id == id).unwrap();
        assert_eq!(node(session).kind, NodeKind::Session);
        assert_eq!(node(folder).kind, NodeKind::Folder);
        assert_eq!(node(folder).members, vec![ganger]);
        assert_eq!(node(session).members, vec![ganger]);
        assert!(tree.iter().all(|n| !n.active));
    }

    fn add_character(s: &Storage, kind: storage::CharacterKind, name: &str) -> i64 {
        let character = Character::quick(name);
        let data = serde_json::to_string(&character).unwrap();
        s.insert_character(kind, name, &data).unwrap()
    }

    #[test]
    fn a_deleted_character_comes_back_with_its_shortcuts_and_place_in_the_fight() {
        let s = storage();
        let pc = add_character(&s, storage::CharacterKind::Pc, "V");
        join_active_session(&s, pc).unwrap();
        let folder = s.create_folder(None, "Crew").unwrap().unwrap();
        s.add_member(folder, pc).unwrap();
        let (scene_id, mut scene) = load_scene(&s).unwrap();
        scene.combat.set_or_add(pc, 6, 12).unwrap();
        save_scene_row(&s, scene_id, &scene).unwrap();

        let deleted = delete_character_in(&s, pc).unwrap();
        assert!(s.get_character(pc).unwrap().is_none());
        assert_eq!(deleted.containers.len(), 2);
        let (_, gone) = load_scene(&s).unwrap();
        assert!(gone.combat.combatants.is_empty());

        restore_character_in(&s, deleted.clone()).unwrap();
        assert_eq!(s.get_character(pc).unwrap().unwrap().name, "V");
        assert_eq!(s.containers_of(pc).unwrap().len(), 2);
        let (_, back) = load_scene(&s).unwrap();
        assert_eq!(back.combat.combatants.len(), 1);

        // A second undo finds the id taken.
        assert_eq!(restore_character_in(&s, deleted).unwrap_err().0.code, "already_exists.character");
    }

    #[test]
    fn an_edited_book_template_is_found_by_its_code_and_old_documents_still_load() {
        let s = storage();
        let character = Character::quick("Booster");
        // A document written before `based_on` existed has no such field.
        let old = serde_json::json!({ "tier": "mook", "character": character }).to_string();
        let plain = s.insert_doc(DocTable::UserTemplates, "Plain", &old).unwrap();
        assert_eq!(user_template_record(&s.get_doc(DocTable::UserTemplates, plain).unwrap().unwrap()).unwrap().based_on, None);

        assert!(book_override(&s, "booster").unwrap().is_none());
        let doc = UserTemplateDoc { tier: NpcTier::Mook, character, based_on: Some("booster".into()) };
        let copy = s.insert_doc(DocTable::UserTemplates, "Booster", &serde_json::to_string(&doc).unwrap()).unwrap();
        assert_eq!(book_override(&s, "booster").unwrap().unwrap().id, copy);
        assert!(book_override(&s, "other").unwrap().is_none());

        // Deleting the copy brings the book template back.
        s.delete_doc(DocTable::UserTemplates, copy).unwrap();
        assert!(book_override(&s, "booster").unwrap().is_none());
    }

    fn spawn_ganger(s: &Storage, name: &str, session: Option<i64>) -> i64 {
        let data = serde_json::to_string(&Character::quick(name)).unwrap();
        spawn_instance(s, name, &data, session).unwrap()
    }

    #[test]
    fn an_instance_lies_in_its_session_and_the_tree_lists_it() {
        let s = storage();
        let night = new_session(&s, "Night").unwrap();
        let day = new_session(&s, "Day").unwrap();
        s.set_active_session(night).unwrap();

        let a = spawn_ganger(&s, "Ganger 1", None);
        let b = spawn_ganger(&s, "Ganger 2", Some(day));
        let kept = add_character(&s, storage::CharacterKind::Npc, "Fixer");

        assert_eq!(s.get_character(a).unwrap().unwrap().owner_session, Some(night));
        assert_eq!(s.get_character(b).unwrap().unwrap().owner_session, Some(day));
        assert_eq!(s.get_character(kept).unwrap().unwrap().owner_session, None);
        let tree = explorer_nodes(&s).unwrap();
        let node = |id| tree.iter().find(|n| n.id == id).unwrap();
        assert_eq!(node(night).members, vec![a]);
        assert_eq!(node(night).instances, vec![a]);
        assert_eq!(node(day).instances, vec![b]);

        let missing = spawn_instance(&s, "X", "{}", Some(999)).unwrap_err();
        assert_eq!(missing.0.code, "not_found.session");
    }

    #[test]
    fn deleting_a_session_takes_its_instances_with_it_but_not_the_library() {
        let s = storage();
        let night = new_session(&s, "Night").unwrap();
        let ganger = spawn_ganger(&s, "Ganger", Some(night));
        let kept = add_character(&s, storage::CharacterKind::Npc, "Fixer");
        s.add_member(night, kept).unwrap();

        s.delete_container(night).unwrap();
        assert!(s.get_character(ganger).unwrap().is_none());
        assert!(s.get_character(kept).unwrap().is_some());
    }

    #[test]
    fn an_undone_instance_comes_back_as_an_instance_or_joins_the_library_when_its_session_is_gone() {
        let s = storage();
        let night = new_session(&s, "Night").unwrap();
        let a = spawn_ganger(&s, "Ganger 1", Some(night));
        let b = spawn_ganger(&s, "Ganger 2", Some(night));

        let deleted = delete_character_in(&s, a).unwrap();
        assert_eq!(deleted.owner_session, Some(night));
        restore_character_in(&s, deleted).unwrap();
        assert_eq!(s.get_character(a).unwrap().unwrap().owner_session, Some(night));
        assert_eq!(s.containers_of(a).unwrap(), vec![night]);

        let deleted = delete_character_in(&s, b).unwrap();
        s.delete_container(night).unwrap();
        restore_character_in(&s, deleted).unwrap();
        assert_eq!(s.get_character(b).unwrap().unwrap().owner_session, None);
    }

    #[test]
    fn undo_leaves_a_scene_alone_when_it_changed_in_the_meantime() {
        let s = storage();
        let pc = add_character(&s, storage::CharacterKind::Pc, "V");
        let other = add_character(&s, storage::CharacterKind::Pc, "Jackie");
        let (scene_id, mut scene) = load_scene(&s).unwrap();
        scene.combat.set_or_add(pc, 6, 12).unwrap();
        save_scene_row(&s, scene_id, &scene).unwrap();

        let deleted = delete_character_in(&s, pc).unwrap();
        let (_, mut changed) = load_scene(&s).unwrap();
        changed.combat.set_or_add(other, 6, 9).unwrap();
        save_scene_row(&s, scene_id, &changed).unwrap();

        restore_character_in(&s, deleted).unwrap();
        let (_, now) = load_scene(&s).unwrap();
        assert_eq!(now.combat.combatants.len(), 1);
        assert_eq!(now.combat.combatants[0].character_id, other);
    }

    #[test]
    fn deleting_a_missing_character_is_not_found() {
        let s = storage();
        assert_eq!(delete_character_in(&s, 99).unwrap_err().0.code, "not_found.character");
    }

    #[test]
    fn npcs_do_not_join_the_session_but_players_do() {
        let s = storage();
        let pc = add_character(&s, storage::CharacterKind::Pc, "V");
        let npc = add_character(&s, storage::CharacterKind::Npc, "Ganger");
        join_active_session(&s, pc).unwrap();
        join_active_session(&s, npc).unwrap();
        let session = ensure_session(&s).unwrap();
        assert_eq!(s.container_members(session).unwrap(), vec![pc]);
    }

    #[test]
    fn a_fight_started_with_nobody_takes_the_session_and_otherwise_only_the_chosen() {
        let s = storage();
        let (v, jackie, absent) = (
            add_character(&s, storage::CharacterKind::Pc, "V"),
            add_character(&s, storage::CharacterKind::Pc, "Jackie"),
            add_character(&s, storage::CharacterKind::Pc, "Absent"),
        );
        let ganger = add_character(&s, storage::CharacterKind::Npc, "Ganger");
        for id in [v, jackie, absent] {
            join_active_session(&s, id).unwrap();
        }

        // Nobody was picked: every living member of the session fights.
        let mut everyone = Scene::default();
        start_fight(&mut everyone, &s).unwrap();
        assert_eq!(everyone.combat.combatants.len(), 3);

        // The GM picked a folder NPC and two players: the absent one stays out.
        let mut picked = Scene::default();
        for id in [v, jackie, ganger] {
            add_fighter(&mut picked, &s, id).unwrap();
        }
        start_fight(&mut picked, &s).unwrap();
        assert!(picked.combat.is_in_fight(ganger));
        assert!(!picked.combat.is_in_fight(absent));
        assert_eq!(picked.combat.combatants.len(), 3);
        assert!(picked.combat.is_started());
    }

    #[test]
    fn adding_a_fighter_twice_keeps_the_first_initiative() {
        let s = storage();
        let v = add_character(&s, storage::CharacterKind::Pc, "V");
        let mut scene = Scene::default();
        add_fighter(&mut scene, &s, v).unwrap();
        let first = scene.combat.combatants[0].initiative;
        add_fighter(&mut scene, &s, v).unwrap();
        assert_eq!(scene.combat.combatants.len(), 1);
        assert_eq!(scene.combat.combatants[0].initiative, first);
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
        assert!(s.container_members(two).unwrap().is_empty());

        // A new character joins the active session only.
        let jackie = s.insert_character(storage::CharacterKind::Pc, "Jackie", "{}").unwrap();
        join_active_session(&s, jackie).unwrap();
        assert_eq!(s.container_members(one).unwrap(), vec![v]);
        assert_eq!(s.container_members(two).unwrap(), vec![jackie]);

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
        s.delete_container(two).unwrap();
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
