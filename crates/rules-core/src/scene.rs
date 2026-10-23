//! Scene: cover objects that outlive a single attack.
//!
//! A cover has its own HP and is shared: several characters may stand behind
//! the same one. The link is separate from the object, so a cover keeps its
//! (shot-up) HP when a character steps away and comes back (ROADMAP, section 0).
//! A character is behind at most one cover at a time.

use serde::{Deserialize, Serialize};
use thiserror::Error;
use ts_rs::TS;

use crate::catalog::CoverThickness;
use crate::damage_event::DamageEvent;
use crate::data_source::{DataResult, DataSource};
use crate::initiative::Combat;

/// Current version of the scene document.
pub const SCHEMA_VERSION: u32 = 1;
/// Code of the shield record for a person used as a shield; its HP is set by the GM.
pub const LIVING_SHIELD: &str = "living_shield";
/// Upper bound for cover HP accepted from the UI (the sturdiest material is 50).
pub const COVER_HP_MAX: u32 = 999;

/// Quick-add entry: a cover material or a shield (portable, corpse, living).
/// Built from server content, so the name is in the content language.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct CoverPreset {
    /// `code` of the content record; scenes refer to presets by it.
    pub id: String,
    pub name: String,
    pub thickness: Option<CoverThickness>,
    /// `None`: the GM has to enter the HP (corpse = BODY, living shield).
    pub hp: Option<u32>,
}

/// All presets, in book order: materials (p. 182), then shields (p. 184,
/// including the corpse and the living shield). Without downloaded content
/// the list is empty and only hand-made covers remain.
pub fn cover_presets(ds: &impl DataSource) -> DataResult<Vec<CoverPreset>> {
    let mut presets: Vec<CoverPreset> = ds
        .cover_materials()?
        .entries
        .into_iter()
        .map(|m| CoverPreset {
            id: m.code,
            name: m.name,
            thickness: Some(m.thickness),
            hp: Some(m.hp),
        })
        .collect();
    presets.extend(ds.shields()?.entries.into_iter().map(|s| CoverPreset {
        id: s.code,
        name: s.name,
        thickness: None,
        hp: s.hp,
    }));
    Ok(presets)
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct CoverObject {
    /// Unique within the scene, never reused.
    pub id: u32,
    /// Custom label; empty means "show the preset name".
    pub name: String,
    /// Id of the preset it was made from; `None` for a hand-made cover.
    pub preset: Option<String>,
    /// Current HP; 0 means destroyed (no longer protects).
    pub hp: u32,
    pub max_hp: u32,
    /// Ids of characters standing behind it.
    #[ts(type = "Array<number>")]
    pub characters: Vec<i64>,
}

impl CoverObject {
    pub fn is_destroyed(&self) -> bool {
        self.hp == 0
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct Scene {
    pub schema_version: u32,
    #[serde(default)]
    pub covers: Vec<CoverObject>,
    #[serde(default = "first_cover_id")]
    pub next_cover_id: u32,
    /// Last applied hit, kept so it can be undone (only this one).
    #[serde(default)]
    pub last_damage: Option<DamageEvent>,
    /// Initiative order and round counter of the current fight.
    #[serde(default)]
    pub combat: Combat,
}

fn first_cover_id() -> u32 {
    1
}

/// A stored scene as the UI sees it.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct SceneRecord {
    #[ts(type = "number")]
    pub id: i64,
    pub scene: Scene,
}

/// A session as the UI lists it: a named folder with its own scene and members.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct SessionInfo {
    #[ts(type = "number")]
    pub id: i64,
    pub name: String,
    /// The session the app is working with; exactly one is active.
    pub active: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum SceneError {
    #[error("cover {0} not found")]
    CoverNotFound(u32),
    #[error("cover HP must be between 1 and {COVER_HP_MAX}")]
    BadHp,
    #[error("preset {0:?} needs HP: it has no default")]
    HpRequired(String),
    #[error("unsupported scene schema version {0}")]
    UnsupportedVersion(u32),
}

impl Default for Scene {
    fn default() -> Self {
        Self {
            schema_version: SCHEMA_VERSION,
            covers: Vec::new(),
            next_cover_id: 1,
            last_damage: None,
            combat: Combat::default(),
        }
    }
}

impl Scene {
    pub fn validate(&self) -> Result<(), SceneError> {
        if self.schema_version != SCHEMA_VERSION {
            return Err(SceneError::UnsupportedVersion(self.schema_version));
        }
        Ok(())
    }

    pub fn cover(&self, id: u32) -> Option<&CoverObject> {
        self.covers.iter().find(|c| c.id == id)
    }

    fn cover_mut(&mut self, id: u32) -> Result<&mut CoverObject, SceneError> {
        self.covers.iter_mut().find(|c| c.id == id).ok_or(SceneError::CoverNotFound(id))
    }

    /// Adds a cover at full HP. `hp` overrides the preset default and is
    /// required when the preset has none.
    pub fn add_cover(
        &mut self,
        preset: Option<&CoverPreset>,
        name: &str,
        hp: Option<u32>,
    ) -> Result<u32, SceneError> {
        let hp = match (hp, preset) {
            (Some(hp), _) => hp,
            (None, Some(p)) => p.hp.ok_or_else(|| SceneError::HpRequired(p.id.clone()))?,
            (None, None) => return Err(SceneError::BadHp),
        };
        check_hp(hp)?;
        let id = self.next_cover_id;
        self.next_cover_id += 1;
        self.covers.push(CoverObject {
            id,
            name: name.trim().to_string(),
            preset: preset.map(|p| p.id.clone()),
            hp,
            max_hp: hp,
            characters: Vec::new(),
        });
        Ok(id)
    }

    /// Edits a cover. `max_hp` must be >= 1; `hp` is clamped to it.
    pub fn edit_cover(
        &mut self,
        id: u32,
        name: &str,
        hp: u32,
        max_hp: u32,
    ) -> Result<(), SceneError> {
        check_hp(max_hp)?;
        let cover = self.cover_mut(id)?;
        cover.name = name.trim().to_string();
        cover.max_hp = max_hp;
        cover.hp = hp.min(max_hp);
        Ok(())
    }

    /// Stores the HP left after an attack (see `damage::DamageOutcome`).
    pub fn set_hp(&mut self, id: u32, hp: u32) -> Result<(), SceneError> {
        let cover = self.cover_mut(id)?;
        cover.hp = hp.min(cover.max_hp);
        Ok(())
    }

    pub fn remove_cover(&mut self, id: u32) -> Result<(), SceneError> {
        let before = self.covers.len();
        self.covers.retain(|c| c.id != id);
        if self.covers.len() == before {
            return Err(SceneError::CoverNotFound(id));
        }
        Ok(())
    }

    /// Puts a character behind a cover (or out of any cover with `None`).
    /// The previous link is dropped; cover HP is not touched.
    pub fn set_character_cover(
        &mut self,
        character_id: i64,
        cover_id: Option<u32>,
    ) -> Result<(), SceneError> {
        if let Some(id) = cover_id {
            self.cover(id).ok_or(SceneError::CoverNotFound(id))?;
        }
        self.unlink_character(character_id);
        if let Some(id) = cover_id {
            self.cover_mut(id)?.characters.push(character_id);
        }
        Ok(())
    }

    /// Removes a character from every cover (used when it is deleted).
    pub fn unlink_character(&mut self, character_id: i64) {
        for cover in &mut self.covers {
            cover.characters.retain(|&c| c != character_id);
        }
    }

    /// The cover a character stands behind, if any.
    pub fn cover_of(&self, character_id: i64) -> Option<&CoverObject> {
        self.covers.iter().find(|c| c.characters.contains(&character_id))
    }
}

fn check_hp(hp: u32) -> Result<(), SceneError> {
    if (1..=COVER_HP_MAX).contains(&hp) { Ok(()) } else { Err(SceneError::BadHp) }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::catalog::BodyArea;
    use crate::damage::{ArmorMode, DamageInput, DamageRoll, TargetState, resolve_damage};
    use crate::data_source::SeedDataSource;

    fn preset(id: &str) -> CoverPreset {
        cover_presets(&SeedDataSource::default()).unwrap().into_iter().find(|p| p.id == id).unwrap()
    }

    #[test]
    fn presets_cover_materials_shields_and_living_shield() {
        let presets = cover_presets(&SeedDataSource::default()).unwrap();
        assert_eq!(presets.len(), 11 + 2 + 1);
        assert_eq!(preset("concrete_thick").hp, Some(25));
        assert_eq!(preset("bulletproof_shield").hp, Some(10));
        // Corpse (= BODY) and living shield have no default HP.
        assert_eq!(preset("corpse").hp, None);
        assert_eq!(preset(LIVING_SHIELD).hp, None);
        let mut ids: Vec<_> = presets.iter().map(|p| p.id.clone()).collect();
        ids.sort();
        ids.dedup();
        assert_eq!(ids.len(), presets.len());
    }

    #[test]
    fn add_from_preset_uses_default_or_override() {
        let mut scene = Scene::default();
        let a = scene.add_cover(Some(&preset("wood_thin")), "", None).unwrap();
        let b = scene.add_cover(Some(&preset("wood_thin")), " Crate ", Some(8)).unwrap();
        assert_eq!((scene.cover(a).unwrap().hp, scene.cover(a).unwrap().max_hp), (5, 5));
        assert_eq!(scene.cover(b).unwrap().name, "Crate");
        assert_eq!(scene.cover(b).unwrap().max_hp, 8);
        assert_ne!(a, b);
    }

    #[test]
    fn preset_without_default_requires_hp() {
        let mut scene = Scene::default();
        assert_eq!(
            scene.add_cover(Some(&preset("corpse")), "", None),
            Err(SceneError::HpRequired("corpse".into()))
        );
        assert!(scene.add_cover(Some(&preset("corpse")), "", Some(6)).is_ok());
        assert_eq!(scene.add_cover(None, "Hand-made", None), Err(SceneError::BadHp));
        assert_eq!(scene.add_cover(None, "x", Some(0)), Err(SceneError::BadHp));
    }

    #[test]
    fn ids_are_not_reused_after_removal() {
        let mut scene = Scene::default();
        let a = scene.add_cover(None, "a", Some(10)).unwrap();
        scene.remove_cover(a).unwrap();
        let b = scene.add_cover(None, "b", Some(10)).unwrap();
        assert_ne!(a, b);
        assert_eq!(scene.remove_cover(a), Err(SceneError::CoverNotFound(a)));
    }

    #[test]
    fn several_characters_share_one_cover_and_one_character_has_one_cover() {
        let mut scene = Scene::default();
        let wall = scene.add_cover(None, "wall", Some(25)).unwrap();
        let car = scene.add_cover(None, "car", Some(25)).unwrap();
        scene.set_character_cover(1, Some(wall)).unwrap();
        scene.set_character_cover(2, Some(wall)).unwrap();
        assert_eq!(scene.cover(wall).unwrap().characters, vec![1, 2]);

        scene.set_character_cover(1, Some(car)).unwrap();
        assert_eq!(scene.cover(wall).unwrap().characters, vec![2]);
        assert_eq!(scene.cover_of(1).unwrap().id, car);

        scene.set_character_cover(1, None).unwrap();
        assert!(scene.cover_of(1).is_none());
        assert_eq!(scene.set_character_cover(1, Some(99)), Err(SceneError::CoverNotFound(99)));
    }

    #[test]
    fn cover_remembers_hp_when_character_leaves_and_returns() {
        let mut scene = Scene::default();
        let wall = scene.add_cover(Some(&preset("concrete_thick")), "", None).unwrap();
        scene.set_character_cover(7, Some(wall)).unwrap();

        // Shot for 17 (p. 182 example): 25 -> 8.
        scene.set_hp(wall, 8).unwrap();
        scene.set_character_cover(7, None).unwrap();
        scene.set_character_cover(7, Some(wall)).unwrap();
        assert_eq!(scene.cover(wall).unwrap().hp, 8);
    }

    #[test]
    fn damage_result_feeds_back_into_the_cover() {
        // Same flow the damage window will use: cover HP in, cover HP out.
        let mut scene = Scene::default();
        let wall = scene.add_cover(Some(&preset("concrete_thick")), "", None).unwrap();
        scene.set_character_cover(7, Some(wall)).unwrap();

        let cover = scene.cover_of(7).unwrap();
        let input = DamageInput {
            damage: DamageRoll::Total { value: 17 },
            area: BodyArea::Body,
            aimed: None,
            armor_mode: ArmorMode::Full,
            explosive: false,
            manual_critical: false,
            cover_hp: Some(cover.hp),
            deflection: 0,
        };
        let target = TargetState { hp: 40, max_hp: 40, sp_head: 0, sp_body: 0, injuries: vec![] };
        let out = resolve_damage(&input, &target);
        let id = cover.id;
        scene.set_hp(id, out.cover_hp_after.unwrap()).unwrap();
        assert_eq!(scene.cover(wall).unwrap().hp, 8);
    }

    #[test]
    fn edit_clamps_hp_and_set_hp_never_exceeds_max() {
        let mut scene = Scene::default();
        let c = scene.add_cover(None, "c", Some(20)).unwrap();
        scene.edit_cover(c, "c2", 15, 10).unwrap();
        assert_eq!((scene.cover(c).unwrap().hp, scene.cover(c).unwrap().max_hp), (10, 10));
        scene.set_hp(c, 500).unwrap();
        assert_eq!(scene.cover(c).unwrap().hp, 10);
        assert_eq!(scene.edit_cover(c, "c", 1, 0), Err(SceneError::BadHp));
        assert_eq!(scene.edit_cover(9, "c", 1, 5), Err(SceneError::CoverNotFound(9)));
    }

    #[test]
    fn zero_hp_means_destroyed() {
        let mut scene = Scene::default();
        let c = scene.add_cover(None, "c", Some(5)).unwrap();
        scene.set_hp(c, 0).unwrap();
        assert!(scene.cover(c).unwrap().is_destroyed());
    }

    #[test]
    fn unlink_character_clears_all_covers() {
        let mut scene = Scene::default();
        let c = scene.add_cover(None, "c", Some(5)).unwrap();
        scene.set_character_cover(3, Some(c)).unwrap();
        scene.unlink_character(3);
        assert!(scene.cover(c).unwrap().characters.is_empty());
    }

    #[test]
    fn scene_roundtrips_through_json() {
        let mut scene = Scene::default();
        let c = scene.add_cover(Some(&preset("wood_thin")), "", None).unwrap();
        scene.set_character_cover(4, Some(c)).unwrap();
        let json = serde_json::to_string(&scene).unwrap();
        assert_eq!(serde_json::from_str::<Scene>(&json).unwrap(), scene);
        // An empty document (as the scenes table defaults to) is not a valid scene
        // without a version, but old documents with only the version still load.
        let minimal: Scene = serde_json::from_str(r#"{"schema_version":1}"#).unwrap();
        assert_eq!(minimal, Scene::default());
    }
}
