use std::sync::Mutex;

use rules_core::catalog::Skill;
use rules_core::character::{Character, CharacterKind, CharacterRecord};
use rules_core::data_source::{DataSource, SeedDataSource};
use rules_core::RulesInfo;
use storage::{CharacterRow, Storage};
use tauri::{Manager, State};

/// User database, opened once at startup in the app data directory.
pub struct AppStorage(pub Mutex<Storage>);

#[tauri::command]
fn rules_info() -> RulesInfo {
    rules_core::rules_info()
}

#[tauri::command]
fn list_skills() -> Result<Vec<Skill>, String> {
    SeedDataSource.skills().map(|c| c.entries).map_err(|e| e.to_string())
}

fn storage_kind(kind: CharacterKind) -> storage::CharacterKind {
    match kind {
        CharacterKind::Pc => storage::CharacterKind::Pc,
        CharacterKind::Npc => storage::CharacterKind::Npc,
    }
}

fn to_record(row: CharacterRow) -> Result<CharacterRecord, String> {
    let character: Character = serde_json::from_str(&row.data).map_err(|e| e.to_string())?;
    Ok(CharacterRecord::new(row.id, character))
}

fn lock<'a>(state: &'a State<'_, AppStorage>) -> Result<std::sync::MutexGuard<'a, Storage>, String> {
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
    let id = lock(&state)?
        .insert_character(storage_kind(kind), &character.name, &data)
        .map_err(|e| e.to_string())?;
    Ok(CharacterRecord::new(id, character))
}

#[tauri::command]
fn save_character(
    state: State<'_, AppStorage>,
    id: i64,
    mut character: Character,
) -> Result<CharacterRecord, String> {
    character.name = character.name.trim().to_string();
    character.validate().map_err(|e| e.to_string())?;
    let data = serde_json::to_string(&character).map_err(|e| e.to_string())?;
    if !lock(&state)?.update_character(id, &character.name, &data).map_err(|e| e.to_string())? {
        return Err(format!("character {id} not found"));
    }
    Ok(CharacterRecord::new(id, character))
}

#[tauri::command]
fn delete_character(state: State<'_, AppStorage>, id: i64) -> Result<(), String> {
    lock(&state)?.delete_character(id).map_err(|e| e.to_string())?;
    Ok(())
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .setup(|app| {
            let dir = app.path().app_data_dir()?;
            std::fs::create_dir_all(&dir)?;
            let storage = Storage::open(&dir.join("red-companion.db"))?;
            app.manage(AppStorage(Mutex::new(storage)));
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            rules_info,
            list_skills,
            list_characters,
            create_character,
            save_character,
            delete_character
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
