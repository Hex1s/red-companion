use std::sync::Mutex;

use rules_core::RulesInfo;
use rules_core::catalog::{BodyArea, CriticalInjury, Skill};
use rules_core::character::{Character, CharacterKind, CharacterRecord};
use rules_core::damage::DamageOutcome;
use rules_core::damage_event::{self, DamageApplied, DamageRequest, InjuryRoll};
use rules_core::data_source::{DataSource, SeedDataSource};
use rules_core::dice::{DiceError, DiceNotation, RollResult, ThreadDice, roll, roll_manual};
use rules_core::initiative::INITIATIVE_DIE;
use rules_core::scene::{CoverPreset, Scene, SceneRecord, cover_presets};
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

/// Loads the working scene, creating a session and an empty scene on first run.
fn load_scene(storage: &Storage) -> Result<(i64, Scene), String> {
    match storage.latest_scene().map_err(|e| e.to_string())? {
        Some(row) => {
            let scene: Scene = serde_json::from_str(&row.data).map_err(|e| e.to_string())?;
            scene.validate().map_err(|e| e.to_string())?;
            Ok((row.id, scene))
        }
        None => {
            let scene = Scene::default();
            let session = storage.create_session("Session").map_err(|e| e.to_string())?;
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

fn load_character(storage: &Storage, id: i64) -> Result<Option<Character>, String> {
    match storage.get_character(id).map_err(|e| e.to_string())? {
        Some(row) => Ok(Some(serde_json::from_str(&row.data).map_err(|e| e.to_string())?)),
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

/// Rolls the initiative die, or takes the face read off a physical die.
fn initiative_die(manual: Option<u32>) -> Result<u32, String> {
    let notation = DiceNotation { count: 1, sides: INITIATIVE_DIE, modifier: 0 };
    let result = match manual {
        Some(face) => roll_manual(notation, &[face]).map_err(|e| e.to_string())?,
        None => roll(notation, &mut ThreadDice::new()),
    };
    Ok(result.dice[0])
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

/// Adds a character to the fight with REF + 1d10 (`manual` = the die face).
#[tauri::command]
fn combat_add(
    state: State<'_, AppStorage>,
    character_id: i64,
    manual: Option<u32>,
) -> Result<SceneRecord, String> {
    with_combat(&state, |scene, storage| {
        let reflex = require_character(storage, character_id)?.stats.reflex;
        let die = initiative_die(manual)?;
        scene.combat.add(character_id, reflex, die).map_err(|e| e.to_string())
    })
}

#[tauri::command]
fn combat_reroll(
    state: State<'_, AppStorage>,
    character_id: i64,
    manual: Option<u32>,
) -> Result<SceneRecord, String> {
    with_combat(&state, |scene, storage| {
        let reflex = require_character(storage, character_id)?.stats.reflex;
        let die = initiative_die(manual)?;
        scene.combat.reroll(character_id, reflex, die).map_err(|e| e.to_string())
    })
}

/// Sets the initiative total by hand.
#[tauri::command]
fn combat_set_initiative(
    state: State<'_, AppStorage>,
    character_id: i64,
    initiative: u32,
) -> Result<SceneRecord, String> {
    with_combat(&state, |scene, _| {
        scene.combat.set_initiative(character_id, initiative).map_err(|e| e.to_string())
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
        scene.combat.remove(character_id, &skip).map_err(|e| e.to_string())
    })
}

#[tauri::command]
fn combat_start(state: State<'_, AppStorage>) -> Result<SceneRecord, String> {
    with_combat(&state, |scene, storage| {
        let skip = dead_ids(storage, scene)?;
        scene.combat.start(&skip).map_err(|e| e.to_string())
    })
}

#[tauri::command]
fn combat_next_turn(state: State<'_, AppStorage>) -> Result<SceneRecord, String> {
    with_combat(&state, |scene, storage| {
        let skip = dead_ids(storage, scene)?;
        scene.combat.next_turn(&skip).map_err(|e| e.to_string())
    })
}

#[tauri::command]
fn combat_end(state: State<'_, AppStorage>) -> Result<SceneRecord, String> {
    with_combat(&state, |scene, _| {
        scene.combat.end();
        Ok(())
    })
}

fn injury_table() -> Result<Vec<CriticalInjury>, String> {
    SeedDataSource.critical_injuries().map(|c| c.entries).map_err(|e| e.to_string())
}

#[tauri::command]
fn list_injuries() -> Result<Vec<CriticalInjury>, String> {
    injury_table()
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
    let mut character = require_character(&storage, character_id)?;
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
    character.validate().map_err(|e| e.to_string())?;
    store_character(&storage, character_id, &character)?;
    save_scene_row(&storage, scene_id, &scene)?;
    Ok(DamageApplied {
        character: Some(CharacterRecord::new(character_id, character)),
        scene: SceneRecord { id: scene_id, scene },
    })
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
            set_character_cover,
            list_injuries,
            roll_dice,
            roll_injury,
            preview_damage,
            apply_damage,
            undo_damage,
            combat_add,
            combat_reroll,
            combat_set_initiative,
            combat_shift,
            combat_remove,
            combat_start,
            combat_next_turn,
            combat_end
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
