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

/// Pack of the main rulebook; entries without an explicit `pack` belong to it.
pub const CORE_PACK: &str = "core";

fn default_pack() -> String {
    CORE_PACK.to_string()
}

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
    /// Content pack the entry belongs to: `core` (the main book) or `dlc:<code>`.
    #[serde(default = "default_pack")]
    pub pack: String,
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
    /// Content pack the entry belongs to: `core` (the main book) or `dlc:<code>`.
    #[serde(default = "default_pack")]
    pub pack: String,
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
    /// Price in eurobucks (p. 97 and p. 350 print the same table).
    pub price: u32,
    /// Content pack the entry belongs to: `core` (the main book) or `dlc:<code>`.
    #[serde(default = "default_pack")]
    pub pack: String,
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
    /// Price in eurobucks; `None` for a body or a living shield, which nobody sells.
    pub price: Option<u32>,
    /// Content pack the entry belongs to: `core` (the main book) or `dlc:<code>`.
    #[serde(default = "default_pack")]
    pub pack: String,
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
    /// Content pack the entry belongs to: `core` (the main book) or `dlc:<code>`.
    #[serde(default = "default_pack")]
    pub pack: String,
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
    /// Content pack the entry belongs to: `core` (the main book) or `dlc:<code>`.
    #[serde(default = "default_pack")]
    pub pack: String,
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
    /// Content pack the entry belongs to: `core` (the main book) or `dlc:<code>`.
    #[serde(default = "default_pack")]
    pub pack: String,
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
    /// What the role does, a few sentences of our own.
    #[serde(default)]
    pub description: String,
    /// What each rank gives, our own words; ranks 1..=10 in order, or empty until filled in.
    #[serde(default)]
    pub ranks: Vec<RankNote>,
    /// Rank points the pool gains per rank (Tech takes two specialty ranks per rank, p. 147).
    #[serde(default = "one_point_per_rank")]
    pub points_per_rank: u8,
    /// Parts of the ability that rank points are spent on; empty when the role has no point pool.
    pub subabilities: Vec<SubAbility>,
    /// Content pack the entry belongs to: `core` (the main book) or `dlc:<code>`.
    #[serde(default = "default_pack")]
    pub pack: String,
    pub source: Source,
}

fn one_point_per_rank() -> u8 {
    1
}

/// What one rank of a role ability gives.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema, TS)]
#[ts(export)]
pub struct RankNote {
    pub rank: u8,
    pub summary: String,
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

/// How a martial art plays: the general techniques every style may use, or a soft or hard style.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema, TS)]
#[ts(export)]
#[serde(rename_all = "snake_case")]
pub enum MartialArtKind {
    /// Techniques open to every style; not a style to pick on the sheet.
    General,
    Soft,
    Hard,
}

/// A stat the technique asks for ("ВОЛЯ 8 или больше").
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema, TS)]
#[ts(export)]
pub struct StatRequirement {
    pub stat: StatKey,
    pub min: u8,
}

/// A special move of a martial art (p. 179-180). The app only shows it: the GM decides when the
/// condition is met and rolls the check.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema, TS)]
#[ts(export)]
pub struct Technique {
    /// Stable id inside the style (e.g. `disarm`).
    pub code: String,
    pub name: String,
    /// What has to be true first, as printed, in the content language; `None` when there is none.
    pub condition: Option<String>,
    /// Stat threshold of the condition, when it has one.
    pub requires: Option<StatRequirement>,
    /// DV of the check the move asks for, when it has one.
    pub dv: Option<u8>,
    /// What the move does, in the content language.
    pub effect: String,
    /// Critical injuries the move inflicts: ids from the critical injury table.
    #[serde(default)]
    pub injuries: Vec<String>,
}

/// A martial art with its techniques; `kind: general` holds the techniques of every style.
/// Server content: texts are in the language of the server the data came from.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema, TS)]
#[ts(export)]
pub struct MartialArt {
    pub uuid: String,
    pub code: String,
    pub name: String,
    pub kind: MartialArtKind,
    /// One phrase: what the style is about, shown in hints.
    pub summary: String,
    pub techniques: Vec<Technique>,
    /// Content pack the entry belongs to: `core` (the main book) or `dlc:<code>`.
    #[serde(default = "default_pack")]
    pub pack: String,
    pub source: Source,
}

/// A rules summary (combat turn, cover, wound states, ...): a short line for hints and an optional
/// longer card for the reference. Core rules text lives in the locale files under
/// `article.<id>.{name,summary,details}`; the keys are fixed by that convention, see
/// [`validate_articles`].
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema, TS)]
#[ts(export)]
pub struct Article {
    /// Stable id, never translated (e.g. `combat_turn`).
    pub id: String,
    pub name_key: String,
    /// One phrase: what it is, shown in hints.
    pub summary_key: String,
    /// Full description in own words; `None` when the summary says it all.
    #[serde(default)]
    pub details_key: Option<String>,
    /// Filters in the reference (`combat`, `medicine`, ...).
    #[serde(default)]
    pub tags: Vec<String>,
    /// Pack the entry belongs to.
    #[serde(default = "default_pack")]
    pub pack: String,
    pub source: Source,
}

/// What a range table describes.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema, TS)]
#[ts(export)]
#[serde(rename_all = "snake_case")]
pub enum RangeTableKind {
    /// One aimed shot (p. 173).
    Single,
    /// Autofire (p. 173).
    Autofire,
}

/// A distance band in metres, inclusive.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema, TS)]
#[ts(export)]
pub struct RangeBand {
    pub from: u32,
    pub to: u32,
}

/// One weapon class of a range table; `dv` has one cell per band, `None` = the shot is impossible.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema, TS)]
#[ts(export)]
pub struct RangeRow {
    pub id: String,
    /// `range_row.<id>`.
    pub name_key: String,
    /// Weapon `code`s of the catalog this row applies to.
    pub weapon_codes: Vec<String>,
    pub dv: Vec<Option<u8>>,
}

/// DV by distance for weapon classes. Reference only: the app never rolls to hit (see ROADMAP, 0).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema, TS)]
#[ts(export)]
pub struct RangeTable {
    pub id: String,
    /// `range_table.<id>`.
    pub name_key: String,
    pub kind: RangeTableKind,
    pub bands: Vec<RangeBand>,
    pub rows: Vec<RangeRow>,
    #[serde(default = "default_pack")]
    pub pack: String,
    pub source: Source,
}

/// One line of the gear list (p. 351) with its description (p. 352-355). The text is a short
/// summary in the content language, not the book's wording.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema, TS)]
#[ts(export)]
pub struct GearItem {
    pub uuid: String,
    pub code: String,
    pub name: String,
    /// Price in eurobucks; the price category is derived from it (see `price_category_of`).
    pub price: u32,
    pub summary: String,
    #[serde(default = "default_pack")]
    pub pack: String,
    /// Page of the description.
    pub source: Source,
}

/// An ammunition type (p. 345-347). Ammo is sold by 10; grenades and rockets one by one (p. 344).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema, TS)]
#[ts(export)]
pub struct Ammo {
    pub uuid: String,
    pub code: String,
    pub name: String,
    pub price: u32,
    /// Which weapons take it, as printed, in the content language.
    pub usable_with: String,
    pub summary: String,
    #[serde(default = "default_pack")]
    pub pack: String,
    pub source: Source,
}

/// A weapon modification (p. 343-344). Every non-exotic ranged weapon has `MOD_SLOTS` slots.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema, TS)]
#[ts(export)]
pub struct WeaponMod {
    pub uuid: String,
    pub code: String,
    pub name: String,
    pub price: u32,
    /// Slots it takes: 1, or 2 where the book says so.
    pub slots: u8,
    /// Skill ids of the weapons it fits (ranged, never exotic).
    pub skills: Vec<String>,
    pub summary: String,
    #[serde(default = "default_pack")]
    pub pack: String,
    pub source: Source,
}

/// Magazine sizes of a weapon class (p. 344): standard, extended and drum.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema, TS)]
#[ts(export)]
pub struct MagazineRow {
    /// `code` of the weapon in the catalog.
    pub id: String,
    pub standard: u8,
    pub extended: u8,
    pub drum: u8,
    #[serde(default = "default_pack")]
    pub pack: String,
    pub source: Source,
}

/// A price category (p. 385): the lowest market price that belongs to it. The name is the locale
/// key `price.category.<id>`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema, TS)]
#[ts(export)]
pub struct PriceCategory {
    pub id: String,
    pub min_price: u32,
    #[serde(default = "default_pack")]
    pub pack: String,
    pub source: Source,
}

/// A suggested specialty of a skill taken per specialty (a language, an instrument). Only a
/// suggestion: the sheet also accepts any text the GM types (p. 45 allows a language that is not
/// in the list). The sheet stores `skill:code`, or `skill:text` for a typed one.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema, TS)]
#[ts(export)]
pub struct Specialty {
    pub uuid: String,
    pub code: String,
    /// Skill `id` it belongs to (a skill with `specialty: true`).
    pub skill: String,
    pub name: String,
    #[serde(default = "default_pack")]
    pub pack: String,
    pub source: Source,
}

/// Columns of the clothing price matrix in book order (p. 102, p. 356); the locale keys
/// `clothing.item.<id>` name them.
pub const CLOTHING_ITEMS: [&str; 9] =
    ["pants", "top", "jacket", "shoes", "jewelry", "mirrorshades", "glasses", "contacts", "hats"];

/// A clothing style with the price of each piece (p. 356; the same table is printed on p. 102).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema, TS)]
#[ts(export)]
pub struct ClothingStyle {
    pub uuid: String,
    pub code: String,
    /// The style's name as the book prints it (English even in the Russian edition).
    pub name: String,
    /// The three or four words that describe the style, in the content language.
    pub tags: String,
    /// Prices in eurobucks in `CLOTHING_ITEMS` order.
    pub prices: Vec<u32>,
    #[serde(default = "default_pack")]
    pub pack: String,
    pub source: Source,
}

/// A street drug (p. 357-358): what one dose does, and what the addiction does.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema, TS)]
#[ts(export)]
pub struct StreetDrug {
    pub uuid: String,
    pub code: String,
    pub name: String,
    /// Price of one dose in eurobucks.
    pub price: u32,
    /// How long the primary effect lasts, hours.
    pub duration_hours: u32,
    /// DV of the check against the secondary effect (addiction).
    pub secondary_dv: u8,
    /// The primary effect, a short summary in the content language.
    pub primary: String,
    /// The secondary effect (addiction), a short summary.
    pub secondary: String,
    #[serde(default = "default_pack")]
    pub pack: String,
    pub source: Source,
}

/// Slots for modifications on a ranged weapon (p. 342).
pub const MOD_SLOTS: u8 = 3;

/// The category a price belongs to: the highest one whose minimum it reaches; below the first, the first.
pub fn price_category_of(categories: &[PriceCategory], price: u32) -> Option<&PriceCategory> {
    categories.iter().rev().find(|c| price >= c.min_price).or(categories.first())
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
    /// The entries as canonical JSON: object keys sorted, entries ordered by `code` (else `id`,
    /// else `uuid`), no spaces. The server computes the same bytes, so equal data gives an equal
    /// hash whatever the field or file order is. (Keys are ASCII, so sorting by bytes and by
    /// UTF-16 units agree and the form is compatible with RFC 8785.)
    pub fn canonical_json(&self) -> Vec<u8> {
        let mut items: Vec<serde_json::Value> =
            self.entries.iter().map(|e| serde_json::to_value(e).expect("entries are serializable")).collect();
        let key = |v: &serde_json::Value| {
            ["code", "id", "uuid"]
                .iter()
                .find_map(|k| v.get(*k).and_then(|x| x.as_str()))
                .unwrap_or_default()
                .to_string()
        };
        items.sort_by_key(key);
        // `serde_json::Value` objects are sorted maps (no `preserve_order` feature).
        serde_json::to_vec(&items).expect("values are serializable")
    }

    /// SHA-256 of the canonical JSON, lowercase hex.
    pub fn content_hash(&self) -> String {
        Sha256::digest(self.canonical_json()).iter().map(|b| format!("{b:02x}")).collect()
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
    match c.entries.iter().find(|e| e.sp == 0 || e.sp > 30 || e.penalty > 4 || e.price == 0) {
        Some(e) => Err(CatalogError::OutOfRange(e.code.clone())),
        None => Ok(()),
    }
}

pub fn validate_shields(c: &Collection<Shield>) -> Result<(), CatalogError> {
    check_ids(c.entries.iter().map(|e| (e.code.as_str(), e.name.as_str(), e.source.page)))?;
    match c.entries.iter().find(|e| e.hp == Some(0) || e.price == Some(0)) {
        Some(e) => Err(CatalogError::OutOfRange(e.code.clone())),
        None => Ok(()),
    }
}

/// Gear list descriptions are on p. 352-355.
pub const GEAR_PAGES: std::ops::RangeInclusive<u32> = 352..=355;
/// Ammunition descriptions are on p. 345-347.
pub const AMMO_PAGES: std::ops::RangeInclusive<u32> = 345..=347;
/// Weapon modification descriptions are on p. 343-344.
pub const MOD_PAGES: std::ops::RangeInclusive<u32> = 343..=344;

pub fn validate_gear(c: &Collection<GearItem>) -> Result<(), CatalogError> {
    check_ids(c.entries.iter().map(|e| (e.code.as_str(), e.name.as_str(), e.source.page)))?;
    match c.entries.iter().find(|e| e.price == 0 || e.summary.is_empty() || !GEAR_PAGES.contains(&e.source.page)) {
        Some(e) => Err(CatalogError::OutOfRange(e.code.clone())),
        None => Ok(()),
    }
}

pub fn validate_ammo(c: &Collection<Ammo>) -> Result<(), CatalogError> {
    check_ids(c.entries.iter().map(|e| (e.code.as_str(), e.name.as_str(), e.source.page)))?;
    let bad = |e: &&Ammo| {
        e.price == 0 || e.summary.is_empty() || e.usable_with.is_empty() || !AMMO_PAGES.contains(&e.source.page)
    };
    match c.entries.iter().find(bad) {
        Some(e) => Err(CatalogError::OutOfRange(e.code.clone())),
        None => Ok(()),
    }
}

/// Modifications: slots 1..=3, only ranged non-exotic skills that exist.
pub fn validate_weapon_mods(c: &Collection<WeaponMod>, skills: &Collection<Skill>, weapons: &Collection<Weapon>) -> Result<(), CatalogError> {
    check_ids(c.entries.iter().map(|e| (e.code.as_str(), e.name.as_str(), e.source.page)))?;
    for e in &c.entries {
        let bad = |why: &str| CatalogError::OutOfRange(format!("{} ({why})", e.code));
        if e.price == 0 || e.summary.is_empty() || !MOD_PAGES.contains(&e.source.page) {
            return Err(bad("price, summary or page"));
        }
        if !(1..=MOD_SLOTS).contains(&e.slots) {
            return Err(bad("slots"));
        }
        if e.skills.is_empty() {
            return Err(bad("skills"));
        }
        for skill in &e.skills {
            let ranged = weapons.entries.iter().any(|w| w.kind == WeaponKind::Ranged && w.skill.as_deref() == Some(skill));
            if !skills.entries.iter().any(|s| &s.id == skill) || !ranged {
                return Err(bad("skill"));
            }
        }
    }
    Ok(())
}

/// Magazines: one row per ranged weapon class that has a magazine, growing sizes, and the standard
/// size is the one the weapon catalog prints (p. 94).
pub fn validate_magazines(c: &Collection<MagazineRow>, weapons: &Collection<Weapon>) -> Result<(), CatalogError> {
    check_ids(c.entries.iter().map(|e| (e.id.as_str(), e.id.as_str(), e.source.page)))?;
    for e in &c.entries {
        let bad = |why: &str| CatalogError::OutOfRange(format!("{} ({why})", e.id));
        let w = weapons.entries.iter().find(|w| w.code == e.id).ok_or_else(|| bad("unknown weapon"))?;
        if w.magazine != Some(u32::from(e.standard)) {
            return Err(bad("standard size differs from the weapon catalog"));
        }
        if !(e.standard < e.extended && e.extended < e.drum) {
            return Err(bad("sizes must grow"));
        }
    }
    Ok(())
}

/// Clothing: one price per column of the matrix, all positive; the book prints the table on p. 102 and p. 356.
pub fn validate_clothing(c: &Collection<ClothingStyle>) -> Result<(), CatalogError> {
    check_ids(c.entries.iter().map(|e| (e.code.as_str(), e.name.as_str(), e.source.page)))?;
    let bad = |e: &&ClothingStyle| {
        e.prices.len() != CLOTHING_ITEMS.len()
            || e.prices.contains(&0)
            || e.tags.is_empty()
            || ![102, 356].contains(&e.source.page)
    };
    match c.entries.iter().find(bad) {
        Some(e) => Err(CatalogError::OutOfRange(e.code.clone())),
        None => Ok(()),
    }
}

/// Street drugs: description on p. 357-358, a sane DV and duration, both effects described.
pub fn validate_street_drugs(c: &Collection<StreetDrug>) -> Result<(), CatalogError> {
    check_ids(c.entries.iter().map(|e| (e.code.as_str(), e.name.as_str(), e.source.page)))?;
    let bad = |e: &&StreetDrug| {
        e.price == 0
            || e.duration_hours == 0
            || !(1..=30).contains(&e.secondary_dv)
            || e.primary.is_empty()
            || e.secondary.is_empty()
            || !(357..=358).contains(&e.source.page)
    };
    match c.entries.iter().find(bad) {
        Some(e) => Err(CatalogError::OutOfRange(e.code.clone())),
        None => Ok(()),
    }
}

/// Specialties: unique per skill, only for skills that are taken per specialty, never for martial
/// arts (they have their own collection), and a code must not contain the key separator.
pub fn validate_specialties(c: &Collection<Specialty>, skills: &Collection<Skill>) -> Result<(), CatalogError> {
    check_ids(c.entries.iter().map(|e| (e.uuid.as_str(), e.name.as_str(), e.source.page)))?;
    let mut seen = std::collections::HashSet::new();
    for e in &c.entries {
        let bad = |why: &str| CatalogError::OutOfRange(format!("{}:{} ({why})", e.skill, e.code));
        let skill = skills.entries.iter().find(|s| s.id == e.skill).ok_or_else(|| bad("unknown skill"))?;
        if !skill.specialty || skill.id == "martial_arts" {
            return Err(bad("skill is not taken per specialty or has its own collection"));
        }
        if e.code.is_empty() || e.code.contains([':', '.', '[', ']']) || e.pack.is_empty() {
            return Err(bad("code"));
        }
        if !seen.insert((e.skill.as_str(), e.code.as_str())) {
            return Err(CatalogError::DuplicateId(format!("{}:{}", e.skill, e.code)));
        }
    }
    Ok(())
}

/// Price categories: ascending minimums, one id each.
pub fn validate_price_categories(c: &Collection<PriceCategory>) -> Result<(), CatalogError> {
    check_ids(c.entries.iter().map(|e| (e.id.as_str(), e.id.as_str(), e.source.page)))?;
    let ascending = c.entries.windows(2).all(|w| w[0].min_price < w[1].min_price);
    match c.entries.first() {
        Some(first) if ascending && first.min_price > 0 => Ok(()),
        _ => Err(CatalogError::OutOfRange("price_categories".into())),
    }
}

/// Martial arts: unique codes, techniques with a name and an effect, injuries that exist.
pub fn validate_martial_arts(c: &Collection<MartialArt>, injuries: &Collection<CriticalInjury>) -> Result<(), CatalogError> {
    check_ids(c.entries.iter().map(|e| (e.code.as_str(), e.name.as_str(), e.source.page)))?;
    for art in &c.entries {
        if art.summary.is_empty() || art.techniques.is_empty() || art.pack.is_empty() {
            return Err(CatalogError::MissingField(art.code.clone()));
        }
        check_ids(art.techniques.iter().map(|t| (t.code.as_str(), t.name.as_str(), 1)))?;
        for t in &art.techniques {
            let bad_dv = t.dv.is_some_and(|dv| !(1..=30).contains(&dv));
            let bad_req = t.requires.as_ref().is_some_and(|r| !(1..=10).contains(&r.min));
            let unknown_injury = t.injuries.iter().any(|id| !injuries.entries.iter().any(|i| &i.id == id));
            if t.effect.is_empty() || bad_dv || bad_req || unknown_injury {
                return Err(CatalogError::OutOfRange(format!("{}/{}", art.code, t.code)));
            }
        }
    }
    Ok(())
}

/// Articles: unique ids, locale keys follow `article.<id>.*`, a page of the book for each.
pub fn validate_articles(c: &Collection<Article>) -> Result<(), CatalogError> {
    check_ids(c.entries.iter().map(|e| (e.id.as_str(), e.name_key.as_str(), e.source.page)))?;
    for e in &c.entries {
        let bad_keys = e.name_key != format!("article.{}.name", e.id)
            || e.summary_key != format!("article.{}.summary", e.id)
            || e.details_key.as_deref().is_some_and(|k| k != format!("article.{}.details", e.id));
        if bad_keys || e.pack.is_empty() {
            return Err(CatalogError::MissingField(e.id.clone()));
        }
    }
    Ok(())
}

/// Range tables: bands are ordered and do not overlap, every row has one DV (1..=40) or `None` per band,
/// weapon codes exist in the catalog and belong to one row per table.
pub fn validate_range_tables(c: &Collection<RangeTable>, weapons: &Collection<Weapon>) -> Result<(), CatalogError> {
    check_ids(c.entries.iter().map(|e| (e.id.as_str(), e.name_key.as_str(), e.source.page)))?;
    for t in &c.entries {
        let bad = t.name_key != format!("range_table.{}", t.id)
            || t.bands.is_empty()
            || t.rows.is_empty()
            || t.bands.iter().any(|b| b.from > b.to)
            || t.bands.windows(2).any(|w| w[0].to >= w[1].from);
        if bad {
            return Err(CatalogError::OutOfRange(t.id.clone()));
        }
        check_ids(t.rows.iter().map(|r| (r.id.as_str(), r.name_key.as_str(), t.source.page)))?;
        let mut used = std::collections::HashSet::new();
        for r in &t.rows {
            let bad = r.name_key != format!("range_row.{}", r.id)
                || r.dv.len() != t.bands.len()
                || r.dv.iter().flatten().any(|d| !(1..=40).contains(d))
                || r.weapon_codes.is_empty()
                || r.weapon_codes.iter().any(|w| !weapons.entries.iter().any(|x| &x.code == w) || !used.insert(w.as_str()));
            if bad {
                return Err(CatalogError::OutOfRange(format!("{}/{}", t.id, r.id)));
            }
        }
    }
    Ok(())
}

pub fn validate_roles(c: &Collection<Role>) -> Result<(), CatalogError> {
    check_ids(c.entries.iter().map(|e| (e.code.as_str(), e.name.as_str(), e.source.page)))?;
    for role in &c.entries {
        if role.ability_code.is_empty() || role.ability_name.is_empty() {
            return Err(CatalogError::MissingField(role.code.clone()));
        }
        if !(1..=2).contains(&role.points_per_rank) {
            return Err(CatalogError::OutOfRange(format!("{} points_per_rank", role.code)));
        }
        if !role.ranks.is_empty() {
            let in_order = role.ranks.iter().map(|r| r.rank).eq(1..=10);
            if !in_order || role.ranks.iter().any(|r| r.summary.trim().is_empty()) {
                return Err(CatalogError::OutOfRange(format!("{} ranks", role.code)));
            }
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
        ("rules", schema_for!(Collection<Article>)),
        ("martial_arts", schema_for!(Collection<MartialArt>)),
        ("range_tables", schema_for!(Collection<RangeTable>)),
        ("gear", schema_for!(Collection<GearItem>)),
        ("ammo", schema_for!(Collection<Ammo>)),
        ("weapon_mods", schema_for!(Collection<WeaponMod>)),
        ("magazines", schema_for!(Collection<MagazineRow>)),
        ("price_categories", schema_for!(Collection<PriceCategory>)),
        ("specialties", schema_for!(Collection<Specialty>)),
        ("clothing", schema_for!(Collection<ClothingStyle>)),
        ("street_drugs", schema_for!(Collection<StreetDrug>)),
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
            pack: default_pack(),
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
                pack: default_pack(),
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
                price: 50,
                pack: default_pack(),
                source: src,
            }],
        };
        assert_eq!(validate_armor(&armor), Ok(()));
    }

    fn article(id: &str) -> Article {
        Article {
            id: id.into(),
            name_key: format!("article.{id}.name"),
            summary_key: format!("article.{id}.summary"),
            details_key: None,
            tags: vec![],
            pack: default_pack(),
            source: Source { page: 168 },
        }
    }

    #[test]
    fn article_keys_follow_the_convention() {
        let wrap = |entries| Collection { name: "rules".into(), version: 1, entries };
        assert_eq!(validate_articles(&wrap(vec![article("a"), article("b")])), Ok(()));
        assert_eq!(
            validate_articles(&wrap(vec![article("a"), article("a")])),
            Err(CatalogError::DuplicateId("a".into()))
        );
        let mut wrong = article("a");
        wrong.summary_key = "article.b.summary".into();
        assert_eq!(validate_articles(&wrap(vec![wrong])), Err(CatalogError::MissingField("a".into())));
        let mut details = article("a");
        details.details_key = Some("article.a.details".into());
        assert_eq!(validate_articles(&wrap(vec![details])), Ok(()));
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
