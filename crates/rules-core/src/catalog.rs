//! Reference-data schemas (collections of game entries) and dataset versioning.
//!
//! Core rules data (critical injuries, skills) keeps names as i18n keys of the
//! app. Server content (cover, shields, armor, NPCs and later weapons, lore)
//! is one table per type: `uuid`, a stable `code`, numeric columns of the
//! contract and plain text columns in the language of the server the data came
//! from; everything but the texts is identical in every language. Every entry
//! records where in the book it came from. The same
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
    /// Identity of the record, the same in every language.
    pub uuid: String,
    /// Stable readable code, unique per material and thickness (e.g.
    /// `metal_thick`); scenes refer to presets by it.
    pub code: String,
    /// Material name in the content language.
    pub name: String,
    pub thickness: CoverThickness,
    pub hp: u32,
    pub source: Source,
}

/// Armor type (book p. 184-185). Any type can be worn on the head or on the
    /// body; armor does not stack: only the best SP of an area counts.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema, TS)]
#[ts(export)]
pub struct Armor {
    pub uuid: String,
    pub code: String,
    pub name: String,
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
    pub uuid: String,
    pub code: String,
    pub name: String,
    /// `None` for a body used as a shield: its HP equal the BODY stat the
    /// person had while alive.
    pub hp: Option<u32>,
    pub source: Source,
}

/// Kind of weapon entry (book p. 91-95).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema, TS)]
#[ts(export)]
#[serde(rename_all = "snake_case")]
pub enum WeaponKind {
    Melee,
    Ranged,
    /// Specialised weapon listed with a description only (p. 95).
    Exotic,
}

/// Weapon type from the tables on p. 92 (melee), p. 94 (ranged) and p. 95
/// (exotic). The book lists classes of weapons, not models. Hit, range and DV
/// are not calculated by the app, so only what the damage window and the
/// catalog need is kept.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema, TS)]
#[ts(export)]
pub struct Weapon {
    pub uuid: String,
    pub code: String,
    pub name: String,
    pub kind: WeaponKind,
    /// Example weapons of a melee type, as printed.
    pub examples: Option<String>,
    /// Skill `id` of the weapon; `None` for exotic weapons.
    pub skill: Option<String>,
    /// Damage dice of one hit or shot as printed, e.g. `3d6`; `None` for exotic weapons.
    pub damage: Option<String>,
    /// Standard magazine; `None` for melee, exotic and bows.
    pub magazine: Option<u32>,
    /// Ammunition type in the content language.
    pub ammo: Option<String>,
    /// Attacks per action (ROF, "СКА"); `None` for exotic weapons.
    pub rof: Option<u8>,
    /// Hands needed; `None` when it depends on the weapon (melee) or is unlisted.
    pub hands: Option<u8>,
    /// Can be hidden under clothes; `None` for exotic weapons.
    pub concealable: Option<bool>,
    /// Alternative fire modes and features as printed, in the content language.
    pub features: Vec<String>,
    /// What an exotic weapon is, in the content language.
    pub description: Option<String>,
    /// Price in eurobucks.
    pub price: u32,
    pub source: Source,
}

/// Stat a skill is linked to (same names as the fields of `character::Stats`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema, TS)]
#[ts(export)]
#[serde(rename_all = "snake_case")]
pub enum StatKey {
    Int,
    Reflex,
    Dex,
    Tech,
    Cool,
    Will,
    Luck,
    Movement,
    Body,
    Emp,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema, TS)]
#[ts(export)]
#[serde(rename_all = "snake_case")]
pub enum SkillCategory {
    Perception,
    Body,
    Control,
    Education,
    Fighting,
    Performance,
    RangedWeapon,
    Social,
    Technique,
}

/// Skill from the list on p. 131-142. Descriptions are not stored (book text).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema, TS)]
#[ts(export)]
pub struct Skill {
    pub id: String,
    pub name_key: String,
    pub stat: StatKey,
    pub category: SkillCategory,
    /// Marked "(x2)" in the book: costs twice as much to learn and improve.
    pub double_cost: bool,
    /// Taken separately per specialty (a language, an area, a field of science,
    /// a martial art, an instrument): the sheet keeps one level per
    /// `id:specialty` key, see `character::skill_base_id`.
    #[serde(default)]
    pub specialty: bool,
    pub source: Source,
}

/// Power tier of a filler NPC (p. 413-416): how many to put against a team.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema, TS)]
#[ts(export)]
#[serde(rename_all = "snake_case")]
pub enum NpcTier {
    /// "Шестёрка": low-level enemy.
    Mook,
    /// "Лейтенант": mid-level, one per two edgerunners.
    Lieutenant,
    /// "Мини-босс": high-level, one per three edgerunners.
    MiniBoss,
    /// Made for boss fights.
    Boss,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema, TS)]
#[ts(export)]
pub struct NpcWeapon {
    pub name: String,
    /// Damage dice as printed, e.g. `5d6`.
    pub damage: String,
}

/// Ready-made NPC from "Шестёрки и Засранцы" (p. 412-416). Cloning one makes a
/// new NPC character; HP, serious-wound threshold and death save are derived
/// from the stats, the book's printed values are checked against them in tests.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema, TS)]
#[ts(export)]
pub struct NpcTemplate {
    pub uuid: String,
    pub code: String,
    pub name: String,
    pub tier: NpcTier,
    pub stats: crate::character::Stats,
    /// Armor as printed, in the content language.
    pub armor_name: String,
    /// `code` of the armor catalog entry when the armor is one (its SP is
    /// then the catalog's); `None` for armor outside the catalog (cyberware).
    pub armor_code: Option<String>,
    pub armor_head_sp: u8,
    pub armor_body_sp: u8,
    pub weapons: Vec<NpcWeapon>,
    /// Skill sums as printed in the book (p. 412): linked STAT + skill level +
    /// modifiers of gear and cyberware, by sheet skill key (`id` or
    /// `id:specialty`). The sheet's level is the sum minus the stat, see
    /// `NpcTemplate::to_character`.
    #[ts(type = "Record<string, number>")]
    pub skill_totals: std::collections::BTreeMap<String, u8>,
    /// Cyberware and gear line, one text in the content language.
    pub gear: String,
    /// Printed details the sheet has no field for (role abilities, bracketed
    /// alternative values, cyberdeck programs).
    pub notes: Option<String>,
    pub source: Source,
}

/// A player role (book p. 27–42) with its role ability (p. 143–166). Server content: names and
/// short summaries only, the rules themselves live in `roles`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema, TS)]
#[ts(export)]
pub struct Role {
    pub uuid: String,
    pub code: String,
    pub name: String,
    pub ability_code: String,
    pub ability_name: String,
    /// What the role ability is about, one or two sentences of our own.
    pub summary: String,
    /// Parts of the ability that rank points are spent on; empty until the role is filled in.
    pub subabilities: Vec<SubAbility>,
    pub source: Source,
}

/// A part of a role ability that gets points (e.g. Medicine: Surgery, Pharmaceuticals).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema, TS)]
#[ts(export)]
pub struct SubAbility {
    pub code: String,
    pub name: String,
    pub summary: String,
    /// Cap on the points that may be put here; `None` = only the rank limits it.
    pub max_points: Option<u8>,
    /// Things the player picks, at most one per point (e.g. a drug).
    pub options: Vec<AbilityOption>,
    /// Benefits that arrive by themselves when the points reach a threshold.
    pub levels: Vec<AbilityLevel>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema, TS)]
#[ts(export)]
pub struct AbilityOption {
    pub code: String,
    pub name: String,
    pub summary: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema, TS)]
#[ts(export)]
pub struct AbilityLevel {
    /// Points in the sub-ability at which the benefit is gained.
    pub points: u8,
    pub summary: String,
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
    check_ids(c.entries.iter().map(|e| (e.code.as_str(), e.name.as_str(), e.source.page)))?;
    match c.entries.iter().find(|e| e.hp == 0) {
        Some(e) => Err(CatalogError::OutOfRange(e.code.clone())),
        None => Ok(()),
    }
}

pub fn validate_armor(c: &Collection<Armor>) -> Result<(), CatalogError> {
    check_ids(c.entries.iter().map(|e| (e.code.as_str(), e.name.as_str(), e.source.page)))?;
    match c.entries.iter().find(|e| e.sp == 0 || e.sp > 30 || e.penalty > 4) {
        Some(e) => Err(CatalogError::OutOfRange(e.code.clone())),
        None => Ok(()),
    }
}

pub fn validate_shields(c: &Collection<Shield>) -> Result<(), CatalogError> {
    check_ids(c.entries.iter().map(|e| (e.code.as_str(), e.name.as_str(), e.source.page)))?;
    match c.entries.iter().find(|e| e.hp == Some(0)) {
        Some(e) => Err(CatalogError::OutOfRange(e.code.clone())),
        None => Ok(()),
    }
}

pub fn validate_roles(c: &Collection<Role>) -> Result<(), CatalogError> {
    check_ids(c.entries.iter().map(|e| (e.code.as_str(), e.name.as_str(), e.source.page)))?;
    for role in &c.entries {
        if role.ability_code.is_empty() || role.ability_name.is_empty() {
            return Err(CatalogError::MissingField(role.code.clone()));
        }
        check_ids(role.subabilities.iter().map(|s| (s.code.as_str(), s.name.as_str(), 1)))?;
        for sub in &role.subabilities {
            if sub.max_points == Some(0) || sub.max_points.is_some_and(|m| m > 10) {
                return Err(CatalogError::OutOfRange(format!("{}.{}", role.code, sub.code)));
            }
            check_ids(sub.options.iter().map(|o| (o.code.as_str(), o.name.as_str(), 1)))?;
            let cap = sub.max_points.unwrap_or(10);
            let mut last = 0;
            for level in &sub.levels {
                if level.points <= last || level.points > cap {
                    return Err(CatalogError::OutOfRange(format!("{}.{}", role.code, sub.code)));
                }
                last = level.points;
            }
        }
    }
    Ok(())
}

/// Printed pages of the weapon tables.
pub const WEAPON_PAGES: std::ops::RangeInclusive<u32> = 92..=95;

pub fn validate_weapons(c: &Collection<Weapon>, skills: &Collection<Skill>) -> Result<(), CatalogError> {
    check_ids(c.entries.iter().map(|e| (e.code.as_str(), e.name.as_str(), e.source.page)))?;
    for e in &c.entries {
        let bad = |why: &str| CatalogError::OutOfRange(format!("{} ({why})", e.code));
        if !WEAPON_PAGES.contains(&e.source.page) || e.price == 0 {
            return Err(bad("page or price"));
        }
        if e.kind == WeaponKind::Exotic {
            let described = e.description.as_deref().is_some_and(|d| !d.is_empty());
            if !described || e.damage.is_some() || e.skill.is_some() || e.rof.is_some() {
                return Err(bad("exotic fields"));
            }
            continue;
        }
        let dice: crate::dice::DiceNotation =
            e.damage.as_deref().ok_or_else(|| bad("damage"))?.parse().map_err(|_| bad("damage dice"))?;
        if dice.sides != 6 || dice.modifier != 0 || !(1..=8).contains(&dice.count) {
            return Err(bad("damage dice"));
        }
        let skill = e.skill.as_deref().ok_or_else(|| bad("skill"))?;
        if !skills.entries.iter().any(|s| s.id == skill) {
            return Err(bad("unknown skill"));
        }
        if !e.rof.is_some_and(|r| (1..=2).contains(&r)) || e.concealable.is_none() {
            return Err(bad("rof or concealable"));
        }
        if e.hands.is_some_and(|h| !(1..=2).contains(&h)) || e.magazine == Some(0) {
            return Err(bad("hands or magazine"));
        }
        if (e.kind == WeaponKind::Melee) != (e.skill.as_deref() == Some("melee_weapon")) {
            return Err(bad("kind and skill"));
        }
    }
    Ok(())
}

/// The Russian edition lists 66 skills (p. 131-142).
pub const SKILL_COUNT: usize = 66;

pub fn validate_skills(c: &Collection<Skill>) -> Result<(), CatalogError> {
    check_ids(c.entries.iter().map(|e| (e.id.as_str(), e.name_key.as_str(), e.source.page)))?;
    if c.entries.len() != SKILL_COUNT {
        return Err(CatalogError::OutOfRange(format!("skills: {} entries", c.entries.len())));
    }
    match c.entries.iter().find(|e| !(131..=142).contains(&e.source.page)) {
        Some(e) => Err(CatalogError::OutOfRange(e.id.clone())),
        None => Ok(()),
    }
}

impl StatKey {
    pub fn value_in(self, stats: &crate::character::Stats) -> u8 {
        match self {
            StatKey::Int => stats.int,
            StatKey::Reflex => stats.reflex,
            StatKey::Dex => stats.dex,
            StatKey::Tech => stats.tech,
            StatKey::Cool => stats.cool,
            StatKey::Will => stats.will,
            StatKey::Luck => stats.luck,
            StatKey::Movement => stats.movement,
            StatKey::Body => stats.body,
            StatKey::Emp => stats.emp,
        }
    }
}

impl NpcTemplate {
    /// Sheet skill levels: the printed sum minus the linked stat, so that
    /// `stat + level` gives the printed number again. Skills unknown to the
    /// catalog are skipped (validation rejects such templates).
    pub fn skill_levels(&self, skills: &[Skill]) -> std::collections::BTreeMap<String, u8> {
        use crate::character::skill_base_id;
        self.skill_totals
            .iter()
            .filter_map(|(key, &total)| {
                let skill = skills.iter().find(|s| s.id == skill_base_id(key))?;
                Some((key.clone(), total.saturating_sub(skill.stat.value_in(&self.stats))))
            })
            .collect()
    }

    /// A fresh NPC sheet: full HP, unworn armor at full SP. `notes` is the
    /// localized weapons and gear text the caller composes (the sheet has no
    /// weapon or gear fields).
    pub fn to_character(&self, name: &str, notes: &str, skills: &[Skill]) -> crate::character::Character {
        use crate::character::{ArmorSlot, Character};
        let mut c = Character::quick(name);
        c.stats = self.stats.clone();
        c.hp = c.max_hp();
        c.armor_head = ArmorSlot { sp: self.armor_head_sp, max_sp: self.armor_head_sp };
        c.armor_body = ArmorSlot { sp: self.armor_body_sp, max_sp: self.armor_body_sp };
        c.skills = self.skill_levels(skills);
        c.notes = notes.to_string();
        c
    }
}

/// Printed range of filler NPC templates (p. 412-416).
pub const NPC_PAGES: std::ops::RangeInclusive<u32> = 412..=416;

pub fn validate_npc_templates(
    c: &Collection<NpcTemplate>,
    skills: &Collection<Skill>,
) -> Result<(), CatalogError> {
    use crate::character::{skill_base_id, skill_specialty, SKILL_MAX};
    check_ids(c.entries.iter().map(|e| (e.code.as_str(), e.name.as_str(), e.source.page)))?;
    for e in &c.entries {
        let bad = |why: &str| CatalogError::OutOfRange(format!("{} ({why})", e.code));
        if !NPC_PAGES.contains(&e.source.page) || e.gear.is_empty() || e.armor_name.is_empty() {
            return Err(bad("page or texts"));
        }
        let st = &e.stats;
        for v in [st.int, st.reflex, st.dex, st.tech, st.cool, st.will, st.movement, st.body] {
            if !(crate::character::STAT_MIN..=crate::character::STAT_MAX).contains(&v) {
                return Err(bad("stat"));
            }
        }
        if st.emp > crate::character::STAT_MAX || st.luck > crate::character::STAT_MAX || e.armor_head_sp > 20 || e.armor_body_sp > 20 {
            return Err(bad("emp, luck or armor"));
        }
        for w in &e.weapons {
            let dice: crate::dice::DiceNotation = w.damage.parse().map_err(|_| bad("damage dice"))?;
            if dice.sides != 6 || dice.modifier != 0 || w.name.is_empty() {
                return Err(bad("weapon"));
            }
        }
        for (key, &total) in &e.skill_totals {
            let known = skills.entries.iter().find(|s| s.id == skill_base_id(key));
            let Some(skill) = known else { return Err(bad(&format!("unknown skill {key}"))) };
            // A sum is stat + level (+ modifiers), so at most stat + 10. A sum below
            // the stat (a printed penalty) is allowed and gives level 0.
            let stat = skill.stat.value_in(&e.stats);
            if total.saturating_sub(stat) > SKILL_MAX {
                return Err(bad(&format!("sum of {key}")));
            }
            // A specialty is given exactly for skills that take one.
            if skill.specialty != skill_specialty(key).is_some_and(|s| !s.is_empty()) {
                return Err(bad(&format!("specialty of {key}")));
            }
        }
    }
    Ok(())
}

/// JSON Schemas of all reference collections, by file name.
pub fn json_schemas() -> Vec<(&'static str, schemars::Schema)> {
    use schemars::schema_for;
    vec![
        ("critical_injuries", schema_for!(Collection<CriticalInjury>)),
        ("cover_materials", schema_for!(Collection<CoverMaterial>)),
        ("armor", schema_for!(Collection<Armor>)),
        ("shields", schema_for!(Collection<Shield>)),
        ("weapons", schema_for!(Collection<Weapon>)),
        ("skills", schema_for!(Collection<Skill>)),
        ("npc_templates", schema_for!(Collection<NpcTemplate>)),
        ("roles", schema_for!(Collection<Role>)),
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
                uuid: "u1".into(),
                code: "concrete".into(),
                name: "Concrete".into(),
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
                uuid: "u2".into(),
                code: "kevlar".into(),
                name: "Kevlar".into(),
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
