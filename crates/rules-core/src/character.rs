//! Character sheet: stats, armor, injuries and everything derived from them.
//!
//! One document type for PCs and NPCs; storage keeps it as a JSON string, so
//! `schema_version` lets old documents be migrated on load. Skills are kept as
//! levels by skill id (see `catalog::Skill`).

use std::collections::BTreeMap;

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use thiserror::Error;
use ts_rs::TS;

use crate::humanity;
use crate::roles::{self, RoleEntry, RoleError};
use crate::damage::{max_hp, wound_state, TargetState, WoundState};

/// Current version of the character document.
pub const SCHEMA_VERSION: u32 = 1;
/// Stat range on the sheet. The book's 1..=10 is the usual scale, but stats may
/// legitimately go above 10 (gear, cyberware, other effects), so the upper bound
/// only catches typos. See p. 79; EMP may drop to 0 from cyberware, and
/// LUCK is 0 for the book's filler NPCs ("—" in their stat blocks, p. 412).
pub const STAT_MIN: u8 = 1;
pub const STAT_MAX: u8 = 99;
/// Reputation runs 1..=10 (p. 193); negative values are the same scale after cowardice.
pub const REPUTATION_MAX: i8 = 10;

/// Start of a portrait data URI.
pub const PORTRAIT_PREFIX: &str = "data:image/webp;base64,";
/// Longest portrait data URI, chars (the app's own are ~10-15 thousand; this guards imports).
pub const PORTRAIT_MAX_LEN: usize = 60_000;

/// A WebP data URI of sane size with a pure base64 body.
pub fn is_portrait(s: &str) -> bool {
    s.len() <= PORTRAIT_MAX_LEN
        && s.strip_prefix(PORTRAIT_PREFIX).is_some_and(|b| {
            !b.is_empty() && b.bytes().all(|c| c.is_ascii_alphanumeric() || matches!(c, b'+' | b'/' | b'='))
        })
}

/// `#rrggbb` with lower-case hex digits: the one form the colour label is stored in.
pub fn is_hex_color(s: &str) -> bool {
    s.len() == 7 && s.starts_with('#') && s[1..].bytes().all(|b| matches!(b, b'0'..=b'9' | b'a'..=b'f'))
}
/// Armor SP range accepted on the sheet (best armor in the book is 18).
pub const SP_MAX: u8 = 20;
/// Humanity per point of EMP (p. 79).
pub const HUMANITY_PER_EMP: u32 = 10;
/// Skill level range on the sheet.
pub const SKILL_MAX: u8 = 10;
/// Stat value used by "quick create"; the GM tunes it afterwards.
pub const QUICK_STAT: u8 = 5;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema, TS)]
#[ts(export)]
pub struct Stats {
    pub int: u8,
    pub reflex: u8,
    pub dex: u8,
    pub tech: u8,
    pub cool: u8,
    pub will: u8,
    pub luck: u8,
    pub movement: u8,
    pub body: u8,
    pub emp: u8,
}

impl Stats {
    fn all_equal(value: u8) -> Self {
        Self {
            int: value,
            reflex: value,
            dex: value,
            tech: value,
            cool: value,
            will: value,
            luck: value,
            movement: value,
            body: value,
            emp: value,
        }
    }
}

/// Separator between a skill id and its specialty in a sheet's skill key.
pub const SKILL_SPECIALTY_SEP: char = ':';

/// Catalog id of a skill key: `language:native` -> `language`.
pub fn skill_base_id(key: &str) -> &str {
    key.split_once(SKILL_SPECIALTY_SEP).map_or(key, |(id, _)| id)
}

/// Specialty part of a skill key, if any: `language:native` -> `native`.
pub fn skill_specialty(key: &str) -> Option<&str> {
    key.split_once(SKILL_SPECIALTY_SEP).map(|(_, spec)| spec)
}

/// Worn armor of one area: current SP drops with ablation, max is what a
/// repair restores.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema, TS)]
#[ts(export)]
pub struct ArmorSlot {
    pub sp: u8,
    pub max_sp: u8,
}

impl ArmorSlot {
    pub const NONE: Self = Self { sp: 0, max_sp: 0 };
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct Character {
    pub schema_version: u32,
    pub name: String,
    pub stats: Stats,
    pub hp: u32,
    pub armor_head: ArmorSlot,
    pub armor_body: ArmorSlot,
    /// Ids of active critical injuries (see `catalog::CriticalInjury`).
    #[serde(default)]
    pub injuries: Vec<String>,
    /// Injuries whose effect first aid has lifted until the end of the day (p. 223); a subset of
    /// `injuries`. A day of rest ends the day and clears it.
    #[serde(default)]
    pub first_aided: Vec<String>,
    /// Accumulated death save penalty (p. 186).
    #[serde(default)]
    pub death_save_penalty: u8,
    #[serde(default)]
    pub dead: bool,
    /// Current humanity (p. 80); `None` while untouched, which means full (EMP × 10). Can go
    /// below zero: that is cyberpsychosis. See `humanity`.
    #[serde(default)]
    #[ts(type = "number | null")]
    pub humanity: Option<i32>,
    /// Reputation (p. 193): 0 for most characters, 1..=10 for a known one, negative when it
    /// comes from cowardice or other shameful deeds (the sign is kept: p. 194).
    #[serde(default)]
    pub reputation: i8,
    /// GM's colour label, `#rrggbb` in lower case; `None` = neutral. User data, not a theme
    /// colour: the GM picks it (usually by a player's clothes) to tell characters apart.
    #[serde(default)]
    #[ts(type = "string | null")]
    pub color: Option<String>,
    /// Portrait: a small square WebP as a data URI (`PORTRAIT_PREFIX` + base64), made by the app
    /// from the GM's own picture; `None` = initials instead.
    #[serde(default)]
    #[ts(type = "string | null")]
    pub portrait: Option<String>,
    #[serde(default)]
    pub notes: String,
    /// Drugs whose once-per-period benefit this character already got (p. 150); see `roles`.
    #[serde(default)]
    pub doses: Vec<String>,
    /// Roles held (multiclass, p. 143) with their ranks; see `roles`.
    #[serde(default)]
    pub roles: Vec<RoleEntry>,
    /// Skill levels by skill key; skills that are absent are at level 0. The key
    /// is a catalog skill id, or `id:specialty` for skills taken several times
    /// with a specialty (see `skill_base_id`).
    #[serde(default)]
    #[ts(type = "Record<string, number>")]
    pub skills: BTreeMap<String, u8>,
}

/// Numbers computed from the sheet; never stored.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct Derived {
    pub max_hp: u32,
    /// Below this HP the character is seriously wounded (half HP, rounded up).
    pub serious_threshold: u32,
    pub wound_state: WoundState,
    /// Death save succeeds on d10 + penalty <= this (= BODY).
    pub death_save: u8,
    pub max_humanity: u32,
    /// Current humanity (can be negative).
    pub humanity: i32,
    /// EMP after humanity loss: one point per full 10 HUM lost.
    pub effective_emp: u8,
    /// HUM is below zero (p. 80).
    pub cyberpsychosis: bool,
    /// DV of stabilising the character in their current wound state (p. 186); `None` when
    /// there is nothing to stabilise (healthy, dead).
    pub stabilization_dv: Option<u8>,
    /// Surgery skill from a Medtech's Medicine (0 without it).
    pub surgery_skill: u8,
    /// Medical Technology skill from a Medtech's Medicine (0 without it).
    pub medical_tech_skill: u8,
    /// Solo's Combat Awareness, while the role's effects are on: initiative roll bonus...
    pub solo_initiative: i32,
    /// ...how much the first damage of a round is reduced...
    pub solo_deflection: u32,
    /// ...bonus to Perception checks...
    pub solo_perception: i32,
    /// ...and the two attack bonuses shown as hints (to any attack; to the first hit's damage).
    pub solo_precision: u8,
    pub solo_spot_weakness: u8,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, TS)]
#[ts(export)]
#[serde(rename_all = "snake_case")]
pub enum CharacterKind {
    Pc,
    Npc,
}

/// A stored character as the UI sees it: sheet plus computed numbers.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct CharacterRecord {
    #[ts(type = "number")]
    pub id: i64,
    pub character: Character,
    pub derived: Derived,
}

impl CharacterRecord {
    pub fn new(id: i64, character: Character) -> Self {
        let derived = character.derived();
        Self { id, character, derived }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum CharacterError {
    #[error("name must not be empty")]
    EmptyName,
    #[error("stat {stat} = {value} is out of range")]
    StatOutOfRange { stat: &'static str, value: u8 },
    #[error("armor SP {sp}/{max_sp} on {area} is invalid")]
    BadArmor { area: &'static str, sp: u8, max_sp: u8 },
    #[error("HP {hp} exceeds max HP {max_hp}")]
    HpAboveMax { hp: u32, max_hp: u32 },
    #[error("skill {skill} level {level} is out of range")]
    SkillOutOfRange { skill: String, level: u8 },
    #[error("first aid is recorded for {0:?}, which the character does not have")]
    FirstAidWithoutInjury(String),
    #[error("reputation {0} is out of range")]
    ReputationOutOfRange(i8),
    #[error("colour label {0:?} is not #rrggbb")]
    BadColor(String),
    #[error("portrait is not a small WebP data URI")]
    BadPortrait,
    #[error(transparent)]
    Role(#[from] RoleError),
    #[error(transparent)]
    Dose(#[from] roles::DrugError),
    #[error("unsupported schema version {0}")]
    UnsupportedVersion(u32),
}

impl Character {
    /// Quick create: every stat at `QUICK_STAT`, full HP, no armor.
    pub fn quick(name: &str) -> Self {
        let stats = Stats::all_equal(QUICK_STAT);
        let hp = max_hp(stats.body, stats.will);
        Self {
            schema_version: SCHEMA_VERSION,
            name: name.trim().to_string(),
            stats,
            hp,
            armor_head: ArmorSlot::NONE,
            armor_body: ArmorSlot::NONE,
            injuries: Vec::new(),
            first_aided: Vec::new(),
            death_save_penalty: 0,
            dead: false,
            humanity: None,
            reputation: 0,
            color: None,
            portrait: None,
            notes: String::new(),
            doses: Vec::new(),
            roles: Vec::new(),
            skills: BTreeMap::new(),
        }
    }

    /// The entry of one role, if the character holds it.
    pub fn role(&self, code: &str) -> Option<&RoleEntry> {
        self.roles.iter().find(|e| e.role == code)
    }

    pub fn max_hp(&self) -> u32 {
        max_hp(self.stats.body, self.stats.will)
    }

    /// A filler NPC has no mortal wound and no death save: at 0 HP they are simply dead. PCs are
    /// not touched.
    pub fn settle_npc(&mut self) {
        if self.hp == 0 {
            self.dead = true;
        }
    }

    /// How many times the injury is on the sheet (the same injury may repeat: two legs).
    pub fn injury_count(&self, id: &str) -> usize {
        self.injuries.iter().filter(|i| i.as_str() == id).count()
    }

    /// How many of those injuries had their effect lifted by first aid today.
    pub fn aided_count(&self, id: &str) -> usize {
        self.first_aided.iter().filter(|i| i.as_str() == id).count()
    }

    /// Takes one instance of the injury off the sheet; a first-aid mark that no longer has an
    /// injury behind it goes too. `false` if there was none. The death save penalty is not touched.
    pub fn take_injury(&mut self, id: &str) -> bool {
        let Some(at) = self.injuries.iter().position(|i| i == id) else { return false };
        self.injuries.remove(at);
        if self.aided_count(id) > self.injury_count(id) {
            if let Some(mark) = self.first_aided.iter().position(|i| i == id) {
                self.first_aided.remove(mark);
            }
        }
        true
    }

    /// The GM puts an injury on the sheet by hand, no roll and no checks (it may repeat). An injury
    /// whose table row raises the death save penalty raises it, as a hit would.
    pub fn add_injury(&mut self, id: &str, raises_death_save_penalty: bool) {
        self.injuries.push(id.to_string());
        if raises_death_save_penalty {
            self.death_save_penalty = self.death_save_penalty.saturating_add(1);
        }
    }

    /// The GM takes one instance of an injury off by hand; its penalty goes with it.
    pub fn remove_injury(&mut self, id: &str, raises_death_save_penalty: bool) -> bool {
        let removed = self.take_injury(id);
        if removed && raises_death_save_penalty {
            self.death_save_penalty = self.death_save_penalty.saturating_sub(1);
        }
        removed
    }

    /// A fresh copy of the sheet for use as a template: full HP and armor, no wounds, no injuries.
    pub fn as_fresh(&self) -> Self {
        let mut c = self.clone();
        c.hp = c.max_hp();
        c.armor_head.sp = c.armor_head.max_sp;
        c.armor_body.sp = c.armor_body.max_sp;
        c.injuries.clear();
        c.first_aided.clear();
        c.doses.clear();
        c.death_save_penalty = 0;
        c.dead = false;
        c.humanity = None;
        c
    }

    pub fn wound_state(&self) -> WoundState {
        wound_state(self.hp, self.max_hp(), self.dead)
    }

    pub fn derived(&self) -> Derived {
        let max_hp = self.max_hp();
        Derived {
            max_hp,
            serious_threshold: max_hp.div_ceil(2),
            wound_state: self.wound_state(),
            death_save: self.stats.body,
            max_humanity: u32::from(self.stats.emp) * HUMANITY_PER_EMP,
            humanity: humanity::current_humanity(self),
            effective_emp: humanity::effective_emp(self),
            cyberpsychosis: humanity::is_cyberpsycho(self),
            stabilization_dv: self.wound_state().stabilization_dv(),
            surgery_skill: roles::surgery_skill(self),
            medical_tech_skill: roles::medical_tech_skill(self),
            solo_initiative: roles::initiative_bonus(self),
            solo_deflection: roles::damage_deflection(self),
            solo_perception: roles::perception_bonus(self),
            solo_precision: roles::precision_bonus(self),
            solo_spot_weakness: roles::spot_weakness_bonus(self),
        }
    }

    /// Input for `damage::resolve_damage`.
    pub fn target_state(&self) -> TargetState {
        TargetState {
            hp: self.hp,
            max_hp: self.max_hp(),
            sp_head: self.armor_head.sp,
            sp_body: self.armor_body.sp,
            injuries: self.injuries.clone(),
        }
    }

    /// Checks the invariants a stored or imported document must satisfy.
    pub fn validate(&self) -> Result<(), CharacterError> {
        if self.schema_version != SCHEMA_VERSION {
            return Err(CharacterError::UnsupportedVersion(self.schema_version));
        }
        if self.name.trim().is_empty() {
            return Err(CharacterError::EmptyName);
        }
        // Injuries may repeat (two legs, three arms); each first-aid mark needs its own injury.
        if let Some(id) = self.first_aided.iter().find(|id| self.aided_count(id) > self.injury_count(id)) {
            return Err(CharacterError::FirstAidWithoutInjury(id.clone()));
        }
        if !(-REPUTATION_MAX..=REPUTATION_MAX).contains(&self.reputation) {
            return Err(CharacterError::ReputationOutOfRange(self.reputation));
        }
        if let Some(color) = &self.color {
            if !is_hex_color(color) {
                return Err(CharacterError::BadColor(color.clone()));
            }
        }
        if let Some(portrait) = &self.portrait {
            if !is_portrait(portrait) {
                return Err(CharacterError::BadPortrait);
            }
        }
        roles::validate_entries(&self.roles)?;
        roles::validate_doses(&self.doses)?;
        let s = &self.stats;
        for (stat, value) in [
            ("int", s.int),
            ("reflex", s.reflex),
            ("dex", s.dex),
            ("tech", s.tech),
            ("cool", s.cool),
            ("will", s.will),
            ("movement", s.movement),
            ("body", s.body),
        ] {
            if !(STAT_MIN..=STAT_MAX).contains(&value) {
                return Err(CharacterError::StatOutOfRange { stat, value });
            }
        }
        for (stat, value) in [("emp", s.emp), ("luck", s.luck)] {
            if value > STAT_MAX {
                return Err(CharacterError::StatOutOfRange { stat, value });
            }
        }
        for (area, slot) in [("head", self.armor_head), ("body", self.armor_body)] {
            if slot.max_sp > SP_MAX || slot.sp > slot.max_sp {
                return Err(CharacterError::BadArmor { area, sp: slot.sp, max_sp: slot.max_sp });
            }
        }
        if let Some((skill, &level)) = self.skills.iter().find(|(_, l)| **l > SKILL_MAX) {
            return Err(CharacterError::SkillOutOfRange { skill: skill.clone(), level });
        }
        let max_hp = self.max_hp();
        if self.hp > max_hp {
            return Err(CharacterError::HpAboveMax { hp: self.hp, max_hp });
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn quick_character_is_valid_and_healthy() {
        let c = Character::quick("  V ");
        assert_eq!(c.name, "V");
        assert_eq!(c.validate(), Ok(()));
        // BODY 5 + WILL 5 -> 10 + 5 * 5 = 35
        assert_eq!(c.derived().max_hp, 35);
        assert_eq!(c.hp, 35);
        assert_eq!(c.wound_state(), WoundState::Healthy);
    }

    #[test]
    fn derived_values_follow_the_book_formulas() {
        let mut c = Character::quick("Johnny");
        c.stats.body = 7;
        c.stats.will = 6; // ceil(13 / 2) = 7 -> 10 + 35 = 45
        c.stats.emp = 4;
        c.hp = 22;
        let d = c.derived();
        assert_eq!(d.max_hp, 45);
        assert_eq!(d.serious_threshold, 23);
        assert_eq!(d.wound_state, WoundState::Serious);
        assert_eq!(d.death_save, 7);
        assert_eq!(d.max_humanity, 40);
        assert_eq!((d.humanity, d.effective_emp, d.cyberpsychosis), (40, 4, false));
        assert_eq!(d.stabilization_dv, Some(13)); // seriously wounded
    }

    #[test]
    fn an_npc_at_zero_hp_is_dead_and_one_with_hp_is_not() {
        let mut npc = Character::quick("Boosterganger");
        npc.hp = 3;
        npc.settle_npc();
        assert!(!npc.dead);
        npc.hp = 0;
        npc.settle_npc();
        assert!(npc.dead);
        assert_eq!(npc.wound_state(), WoundState::Dead);
    }

    #[test]
    fn dead_flag_and_zero_hp_drive_wound_state() {
        let mut c = Character::quick("Boosterganger");
        c.hp = 0;
        assert_eq!(c.wound_state(), WoundState::Mortal);
        c.dead = true;
        assert_eq!(c.wound_state(), WoundState::Dead);
    }

    #[test]
    fn target_state_carries_armor_and_injuries() {
        let mut c = Character::quick("V");
        c.armor_head = ArmorSlot { sp: 7, max_sp: 11 };
        c.armor_body = ArmorSlot { sp: 11, max_sp: 11 };
        c.injuries.push("broken_leg".into());
        let t = c.target_state();
        assert_eq!((t.sp_head, t.sp_body, t.max_hp), (7, 11, 35));
        assert_eq!(t.injuries, vec!["broken_leg".to_string()]);
    }

    #[test]
    fn validation_rejects_bad_sheets() {
        let mut c = Character::quick("V");
        c.name = "  ".into();
        assert_eq!(c.validate(), Err(CharacterError::EmptyName));

        let mut c = Character::quick("V");
        c.stats.body = 100;
        assert!(matches!(c.validate(), Err(CharacterError::StatOutOfRange { stat: "body", .. })));

        let mut c = Character::quick("V");
        c.stats.body = 14; // above the usual scale, not an error
        assert_eq!(c.validate(), Ok(()));

        let mut c = Character::quick("V");
        c.stats.int = 0;
        assert!(matches!(c.validate(), Err(CharacterError::StatOutOfRange { stat: "int", .. })));

        let mut c = Character::quick("V");
        c.stats.emp = 0; // allowed: cyberware can burn EMP to zero
        assert_eq!(c.validate(), Ok(()));

        let mut c = Character::quick("V");
        c.stats.luck = 0; // allowed: filler NPCs have no luck
        assert_eq!(c.validate(), Ok(()));
        c.stats.luck = 100;
        assert!(matches!(c.validate(), Err(CharacterError::StatOutOfRange { stat: "luck", .. })));

        let mut c = Character::quick("V");
        c.armor_body = ArmorSlot { sp: 12, max_sp: 11 };
        assert!(matches!(c.validate(), Err(CharacterError::BadArmor { area: "body", .. })));

        let mut c = Character::quick("V");
        c.hp = 36;
        assert!(matches!(c.validate(), Err(CharacterError::HpAboveMax { .. })));

        let mut c = Character::quick("V");
        c.skills.insert("handgun".into(), 11);
        assert!(matches!(c.validate(), Err(CharacterError::SkillOutOfRange { level: 11, .. })));
        c.skills.insert("handgun".into(), SKILL_MAX);
        assert_eq!(c.validate(), Ok(()));

        let mut c = Character::quick("V");
        for bad in ["red", "#FFAA00", "#12345", "#12345g", "ff0000"] {
            c.color = Some(bad.into());
            assert_eq!(c.validate(), Err(CharacterError::BadColor(bad.into())));
        }
        c.color = Some("#d6302b".into());
        assert_eq!(c.validate(), Ok(()));
        // A template's portrait survives cloning too.
        c.portrait = Some(format!("{PORTRAIT_PREFIX}AAAA"));
        assert_eq!(c.validate(), Ok(()));
        assert_eq!(c.as_fresh().portrait, c.portrait);
        for bad in ["http://x/y.png", "data:image/png;base64,AAAA", "data:image/webp;base64,", "data:image/webp;base64,<b>"] {
            c.portrait = Some(bad.into());
            assert_eq!(c.validate(), Err(CharacterError::BadPortrait));
        }
        c.portrait = None;
        // A template's label survives cloning.
        assert_eq!(c.as_fresh().color.as_deref(), Some("#d6302b"));

        let mut c = Character::quick("V");
        c.schema_version = 99;
        assert_eq!(c.validate(), Err(CharacterError::UnsupportedVersion(99)));
    }

    #[test]
    fn reputation_is_limited_to_ten_either_way() {
        let mut c = Character::quick("V");
        assert_eq!(c.reputation, 0);
        for ok in [-10, -1, 0, 4, 10] {
            c.reputation = ok;
            assert_eq!(c.validate(), Ok(()));
        }
        for bad in [-11, 11] {
            c.reputation = bad;
            assert_eq!(c.validate(), Err(CharacterError::ReputationOutOfRange(bad)));
        }
    }

    #[test]
    fn skill_keys_split_into_id_and_specialty() {
        assert_eq!(skill_base_id("language:native"), "language");
        assert_eq!(skill_specialty("language:native"), Some("native"));
        assert_eq!(skill_base_id("handgun"), "handgun");
        assert_eq!(skill_specialty("handgun"), None);
    }

    #[test]
    fn json_roundtrip_and_defaults_for_optional_fields() {
        let c = Character::quick("V");
        let json = serde_json::to_string(&c).unwrap();
        assert_eq!(serde_json::from_str::<Character>(&json).unwrap(), c);

        // A document without the optional fields still loads.
        let mut value: serde_json::Value = serde_json::from_str(&json).unwrap();
        for key in ["injuries", "first_aided", "death_save_penalty", "dead", "humanity", "reputation", "notes", "skills"] {
            value.as_object_mut().unwrap().remove(key);
        }
        let loaded: Character = serde_json::from_value(value).unwrap();
        assert_eq!(loaded, c);
    }
}
