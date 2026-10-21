use std::sync::Mutex;

use rules_core::catalog::Skill;
use rules_core::character::{Character, CharacterKind, CharacterRecord};
use rules_core::data_source::{DataSource, SeedDataSource};
use rules_core::scene::{cover_presets, CoverPreset, Scene, SceneRecord};
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
    let storage = lock(&state)?;
    storage.delete_character(id).map_err(|e| e.to_string())?;
    // A deleted character must not linger behind a cover.
    if let Some(row) = storage.latest_scene().map_err(|e| e.to_string())? {
        let mut scene: Scene = serde_json::from_str(&row.data).map_err(|e| e.to_string())?;
        scene.unlink_character(id);
        save_scene_row(&storage, row.id, &scene)?;
    }
    Ok(())
}

fn save_scene_row(storage: &Storage, id: i64, scene: &Scene) -> Result<(), String> {
    let data = serde_json::to_string(scene).map_err(|e| e.to_string())?;
    storage.save_scene(id, &data).map_err(|e| e.to_string())?;
    Ok(())
}

/// Loads the working scene (creating a session and an empty scene on first
/// run), applies `change` and saves only when it succeeds.
fn with_scene(
    state: &State<'_, AppStorage>,
    change: impl FnOnce(&mut Scene) -> Result<(), String>,
) -> Result<SceneRecord, String> {
    let storage = lock(state)?;
    let (id, mut scene) = match storage.latest_scene().map_err(|e| e.to_string())? {
        Some(row) => {
            let scene: Scene = serde_json::from_str(&row.data).map_err(|e| e.to_string())?;
            scene.validate().map_err(|e| e.to_string())?;
            (row.id, scene)
        }
        None => {
            let scene = Scene::default();
            let session = storage.create_session("Session").map_err(|e| e.to_string())?;
            let data = serde_json::to_string(&scene).map_err(|e| e.to_string())?;
            let id = storage.create_scene(session, "Scene", &data).map_err(|e| e.to_string())?;
            (id, scene)
        }
    };
    change(&mut scene)?;
    save_scene_row(&storage, id, &scene)?;
    Ok(SceneRecord { id, scene })
}

#[tauri::command]
fn get_scene(state: State<'_, AppStorage>) -> Result<SceneRecord, String> {
    with_scene(&state, |_| Ok(()))
}

#[tauri::command]
fn list_cover_presets() -> Result<Vec<CoverPreset>, String> {
    cover_presets(&SeedDataSource).map_err(|e| e.to_string())
}

/// Adds a cover from a preset (`preset` = its id) or a hand-made one (`None`,
/// then `hp` is required). `hp` overrides the preset default.
#[tauri::command]
fn add_cover(
    state: State<'_, AppStorage>,
    preset: Option<String>,
    name: String,
    hp: Option<u32>,
) -> Result<SceneRecord, String> {
    let preset = match preset {
        Some(id) => Some(
            cover_presets(&SeedDataSource)
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
            delete_character,
            get_scene,
            list_cover_presets,
            add_cover,
            edit_cover,
            remove_cover,
            set_character_cover
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
