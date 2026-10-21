//! Reference-data schemas (collections of game entries) and dataset versioning.
//!
//! Numbers live in the data without any language; names and descriptions are
//! i18n keys. Every entry records where in the book it came from. The same
//! types are exported as JSON Schema (see `schemas/`) and will be the base of
//! the future OpenAPI contract.

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use thiserror::Error;
use ts_rs::TS;

/// Where a value comes from in the rulebook.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema, TS)]
#[ts(export)]
pub struct Source {
    /// Printed page number (PDF page minus one).
    pub page: u32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema, TS)]
#[ts(export)]
#[serde(rename_all = "snake_case")]
pub enum BodyArea {
    Body,
    Head,
}

/// A critical injury table row (2d6, separate tables for body and head).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema, TS)]
#[ts(export)]
pub struct CriticalInjury {
    /// Stable id, never translated (e.g. `broken_leg`).
    pub id: String,
    pub area: BodyArea,
    /// 2d6 result, 2..=12.
    pub roll: u8,
    pub name_key: String,
    pub effect_key: String,
    /// Ways to stabilise the injury until the end of the day; any one option
    /// works. Empty when first aid is impossible ("НЕТ" in the table).
    pub first_aid: Vec<TreatmentOption>,
    /// Ways to heal the injury for good; any one option works. Empty when the
    /// table only offers a quick fix (see `quick_fix_removes_effect`).
    pub treatment: Vec<TreatmentOption>,
    /// "Quick Fix removes the effect" instead of a regular treatment.
    pub quick_fix_removes_effect: bool,
    /// The injury raises the death-save penalty by 1 ("Штраф Спасброска
    /// увеличивается на 1").
    pub death_save_penalty: bool,
    pub source: Source,
}

/// Medical skill that can treat an injury.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema, TS)]
#[ts(export)]
#[serde(rename_all = "snake_case")]
pub enum TreatmentSkill {
    FirstAid,
    Paramedic,
    Surgery,
}

/// One way to treat an injury: skill check against a DV.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema, TS)]
#[ts(export)]
pub struct TreatmentOption {
    pub skill: TreatmentSkill,
    pub dv: u8,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema, TS)]
#[ts(export)]
#[serde(rename_all = "snake_case")]
pub enum CoverThickness {
    Thick,
    Thin,
}

/// Cover material at a given thickness (book p. 182). Combinations with 0 HP
/// (thin drywall) are not cover and are not stored.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema, TS)]
#[ts(export)]
pub struct CoverMaterial {
    /// Unique per material and thickness (e.g. `metal_thick`).
    pub id: String,
    pub name_key: String,
    pub thickness: CoverThickness,
    pub hp: u32,
    pub source: Source,
}

/// Armor type (book p. 184-185). Any type can be worn on the head or on the
    /// body; armor does not stack: only the best SP of an area counts.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema, TS)]
#[ts(export)]
pub struct Armor {
    pub id: String,
    pub name_key: String,
    pub sp: u8,
    /// Penalty to REF, DEX and MOVE (0, 2 or 4); applied once, the highest
    /// of the worn armor.
    pub penalty: u8,
    pub source: Source,
}

/// Portable shield, usable as cover with its own HP.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema, TS)]
#[ts(export)]
pub struct Shield {
    pub id: String,
    pub name_key: String,
    /// `None` for a body used as a shield: its HP equal the BODY stat the
    /// person had while alive.
    pub hp: Option<u32>,
    pub source: Source,
}

/// A versioned set of entries of one type, the unit of sync with the future API.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct Collection<T> {
    /// Collection name, e.g. `critical_injuries`.
    pub name: String,
    /// Bumped by hand whenever the content changes.
    pub version: u32,
    pub entries: Vec<T>,
}

/// Small description of a collection used to compare local and remote data.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema, TS)]
#[ts(export)]
pub struct CollectionVersion {
    pub name: String,
    pub version: u32,
    /// SHA-256 of the serialized entries, lowercase hex.
    pub hash: String,
}

impl<T: Serialize> Collection<T> {
    pub fn content_hash(&self) -> String {
        let json = serde_json::to_vec(&self.entries).expect("entries are serializable");
        Sha256::digest(&json).iter().map(|b| format!("{b:02x}")).collect()
    }

    pub fn version_info(&self) -> CollectionVersion {
        CollectionVersion {
            name: self.name.clone(),
            version: self.version,
            hash: self.content_hash(),
        }
    }
}

/// An update is offered when the remote version is newer, or the same version
/// carries different content (a forgotten version bump should not go unnoticed).
pub fn needs_update(local: &CollectionVersion, remote: &CollectionVersion) -> bool {
    remote.version > local.version || (remote.version == local.version && remote.hash != local.hash)
}

#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum CatalogError {
    #[error("{0}: duplicate id")]
    DuplicateId(String),
    #[error("{0}: missing id, name key or source page")]
    MissingField(String),
    #[error("{0}: value out of range")]
    OutOfRange(String),
    #[error("critical injuries: expected exactly one row per roll 2..=12 for {0:?}")]
    IncompleteInjuryTable(BodyArea),
}

fn check_ids<'a>(items: impl Iterator<Item = (&'a str, &'a str, u32)>) -> Result<(), CatalogError> {
    let mut seen = std::collections::HashSet::new();
    for (id, name_key, page) in items {
        if id.is_empty() || name_key.is_empty() || page == 0 {
            return Err(CatalogError::MissingField(id.to_string()));
        }
        if !seen.insert(id) {
            return Err(CatalogError::DuplicateId(id.to_string()));
        }
    }
    Ok(())
}

/// Invariants for seed data, checked in tests (see CLAUDE.md "Данные из книги").
pub fn validate_critical_injuries(c: &Collection<CriticalInjury>) -> Result<(), CatalogError> {
    check_ids(c.entries.iter().map(|e| (e.id.as_str(), e.name_key.as_str(), e.source.page)))?;
    for e in &c.entries {
        if e.effect_key.is_empty() {
            return Err(CatalogError::MissingField(e.id.clone()));
        }
        if !(2..=12).contains(&e.roll) {
            return Err(CatalogError::OutOfRange(e.id.clone()));
        }
        if e.first_aid.iter().chain(&e.treatment).any(|o| !(1..=30).contains(&o.dv)) {
            return Err(CatalogError::OutOfRange(e.id.clone()));
        }
        // Every injury must be curable one way or another.
        if e.treatment.is_empty() && !e.quick_fix_removes_effect {
            return Err(CatalogError::OutOfRange(e.id.clone()));
        }
    }
    for area in [BodyArea::Body, BodyArea::Head] {
        let mut rolls: Vec<u8> =
            c.entries.iter().filter(|e| e.area == area).map(|e| e.roll).collect();
        rolls.sort_unstable();
        if rolls != (2..=12).collect::<Vec<u8>>() {
            return Err(CatalogError::IncompleteInjuryTable(area));
        }
    }
    Ok(())
}

pub fn validate_cover_materials(c: &Collection<CoverMaterial>) -> Result<(), CatalogError> {
    check_ids(c.entries.iter().map(|e| (e.id.as_str(), e.name_key.as_str(), e.source.page)))?;
    match c.entries.iter().find(|e| e.hp == 0) {
        Some(e) => Err(CatalogError::OutOfRange(e.id.clone())),
        None => Ok(()),
    }
}

pub fn validate_armor(c: &Collection<Armor>) -> Result<(), CatalogError> {
    check_ids(c.entries.iter().map(|e| (e.id.as_str(), e.name_key.as_str(), e.source.page)))?;
    match c.entries.iter().find(|e| e.sp == 0 || e.sp > 30 || e.penalty > 4) {
        Some(e) => Err(CatalogError::OutOfRange(e.id.clone())),
        None => Ok(()),
    }
}

pub fn validate_shields(c: &Collection<Shield>) -> Result<(), CatalogError> {
    check_ids(c.entries.iter().map(|e| (e.id.as_str(), e.name_key.as_str(), e.source.page)))?;
    match c.entries.iter().find(|e| e.hp == Some(0)) {
        Some(e) => Err(CatalogError::OutOfRange(e.id.clone())),
        None => Ok(()),
    }
}

/// JSON Schemas of all reference collections, by file name.
pub fn json_schemas() -> Vec<(&'static str, schemars::Schema)> {
    use schemars::schema_for;
    vec![
        ("critical_injuries", schema_for!(Collection<CriticalInjury>)),
        ("cover_materials", schema_for!(Collection<CoverMaterial>)),
        ("armor", schema_for!(Collection<Armor>)),
        ("shields", schema_for!(Collection<Shield>)),
        ("collection_version", schema_for!(CollectionVersion)),
    ]
}

#[cfg(test)]
mod tests {
    use super::*;

    fn injury(id: &str, area: BodyArea, roll: u8) -> CriticalInjury {
        CriticalInjury {
            id: id.into(),
            area,
            roll,
            name_key: format!("injury.{id}"),
            effect_key: format!("injury.{id}.effect"),
            first_aid: vec![TreatmentOption { skill: TreatmentSkill::Paramedic, dv: 13 }],
            treatment: vec![TreatmentOption { skill: TreatmentSkill::Surgery, dv: 15 }],
            quick_fix_removes_effect: false,
            death_save_penalty: false,
            source: Source { page: 187 },
        }
    }

    fn full_table() -> Collection<CriticalInjury> {
        let entries = [BodyArea::Body, BodyArea::Head]
            .into_iter()
            .flat_map(|a| (2..=12).map(move |r| injury(&format!("{a:?}{r}"), a, r)))
            .collect();
        Collection { name: "critical_injuries".into(), version: 1, entries }
    }

    #[test]
    fn complete_injury_table_is_valid() {
        assert_eq!(validate_critical_injuries(&full_table()), Ok(()));
    }

    #[test]
    fn injury_table_invariants_are_enforced() {
        let mut t = full_table();
        t.entries.pop();
        assert!(matches!(
            validate_critical_injuries(&t),
            Err(CatalogError::IncompleteInjuryTable(BodyArea::Head))
        ));

        let mut t = full_table();
        t.entries[1].id = t.entries[0].id.clone();
        assert!(matches!(validate_critical_injuries(&t), Err(CatalogError::DuplicateId(_))));

        let mut t = full_table();
        t.entries[0].source.page = 0;
        assert!(matches!(validate_critical_injuries(&t), Err(CatalogError::MissingField(_))));

        let mut t = full_table();
        t.entries[0].roll = 13;
        assert!(matches!(validate_critical_injuries(&t), Err(CatalogError::OutOfRange(_))));
    }

    #[test]
    fn hash_follows_content_and_update_detection_works() {
        let a = full_table();
        let mut b = full_table();
        assert_eq!(a.content_hash(), b.content_hash());
        assert_eq!(a.content_hash().len(), 64);

        b.entries[0].first_aid[0].dv = 15;
        assert_ne!(a.content_hash(), b.content_hash());

        let (va, vb) = (a.version_info(), b.version_info());
        assert!(needs_update(&va, &vb), "same version, different content");
        assert!(!needs_update(&va, &va));

        let newer = CollectionVersion { version: 2, ..va.clone() };
        assert!(needs_update(&va, &newer));
        assert!(!needs_update(&newer, &va));
    }

    #[test]
    fn cover_and_armor_validation() {
        let src = Source { page: 182 };
        let cover = Collection {
            name: "cover_materials".into(),
            version: 1,
            entries: vec![CoverMaterial {
                id: "concrete".into(),
                name_key: "cover.concrete".into(),
                thickness: CoverThickness::Thick,
                hp: 0,
                source: src.clone(),
            }],
        };
        assert!(validate_cover_materials(&cover).is_err());

        let armor = Collection {
            name: "armor".into(),
            version: 1,
            entries: vec![Armor {
                id: "kevlar".into(),
                name_key: "armor.kevlar".into(),
                sp: 7,
                penalty: 0,
                source: src,
            }],
        };
        assert_eq!(validate_armor(&armor), Ok(()));
    }

    /// Writes JSON Schemas to `<workspace>/schemas/` so they can be reviewed in git.
    #[test]
    fn export_json_schemas() {
        let dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../schemas");
        std::fs::create_dir_all(&dir).unwrap();
        for (name, schema) in json_schemas() {
            let json = serde_json::to_string_pretty(&schema).unwrap();
            std::fs::write(dir.join(format!("{name}.schema.json")), json + "\n").unwrap();
        }
    }
}
