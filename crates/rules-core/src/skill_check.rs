//! Skill check: STAT + skill level + a d10 check (p. 130).
//!
//! The app only adds the bonus up; the DV and the verdict stay with the GM.

use serde::{Deserialize, Serialize};
use thiserror::Error;
use ts_rs::TS;

use crate::catalog::{Skill, StatKey};
use crate::character::{Character, Stats, skill_base_id};
use crate::dice::CheckRoll;

impl Stats {
    /// Value of the stat a skill is linked to.
    pub fn get(&self, key: StatKey) -> u8 {
        match key {
            StatKey::Int => self.int,
            StatKey::Reflex => self.reflex,
            StatKey::Dex => self.dex,
            StatKey::Tech => self.tech,
            StatKey::Cool => self.cool,
            StatKey::Will => self.will,
            StatKey::Luck => self.luck,
            StatKey::Movement => self.movement,
            StatKey::Body => self.body,
            StatKey::Emp => self.emp,
        }
    }
}

#[derive(Debug, Error, PartialEq, Eq)]
pub enum SkillCheckError {
    #[error("unknown skill: {0}")]
    UnknownSkill(String),
}

/// A finished check: what was added up and the die behind it.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct SkillCheck {
    /// Sheet key of the skill, `id` or `id:specialty`.
    pub skill: String,
    pub stat: StatKey,
    pub stat_value: u8,
    /// Level on the sheet, 0 when the skill is not trained.
    pub level: u8,
    pub die: CheckRoll,
    /// STAT + level + die.
    pub total: i32,
}

/// Bonus of a skill on a sheet: STAT + level (0 for an untrained skill).
pub fn skill_bonus(
    character: &Character,
    key: &str,
    catalog: &[Skill],
) -> Result<(StatKey, u8, u8), SkillCheckError> {
    let skill = catalog
        .iter()
        .find(|s| s.id == skill_base_id(key))
        .ok_or_else(|| SkillCheckError::UnknownSkill(key.to_string()))?;
    let level = character.skills.get(key).copied().unwrap_or(0);
    Ok((skill.stat, character.stats.get(skill.stat), level))
}

pub fn skill_check(
    character: &Character,
    key: &str,
    catalog: &[Skill],
    die: CheckRoll,
) -> Result<SkillCheck, SkillCheckError> {
    let (stat, stat_value, level) = skill_bonus(character, key, catalog)?;
    let total = i32::from(stat_value) + i32::from(level) + die.total;
    Ok(SkillCheck { skill: key.to_string(), stat, stat_value, level, die, total })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::data_source::{DataSource, SeedDataSource};
    use crate::dice::{check_manual, check_total};

    fn catalog() -> Vec<Skill> {
        SeedDataSource::default().skills().unwrap().entries
    }

    fn sheet() -> Character {
        let mut c = Character::quick("Test");
        c.stats.dex = 7;
        c.stats.int = 6;
        c.skills.insert("handgun".into(), 4);
        c.skills.insert("language:streetslang".into(), 3);
        c
    }

    #[test]
    fn adds_stat_level_and_die() {
        let check = skill_check(&sheet(), "handgun", &catalog(), check_total(6).unwrap()).unwrap();
        assert_eq!((check.stat, check.stat_value, check.level), (StatKey::Reflex, 5, 4));
        assert_eq!(check.total, 5 + 4 + 6);
    }

    #[test]
    fn untrained_skill_counts_as_level_zero() {
        let check = skill_check(&sheet(), "athletics", &catalog(), check_total(3).unwrap()).unwrap();
        assert_eq!(check.level, 0);
        assert_eq!(check.total, i32::from(check.stat_value) + 3);
    }

    #[test]
    fn specialty_key_uses_the_catalog_skill_and_its_own_level() {
        let check =
            skill_check(&sheet(), "language:streetslang", &catalog(), check_total(5).unwrap()).unwrap();
        assert_eq!((check.stat, check.level), (StatKey::Int, 3));
        assert_eq!(check.total, 6 + 3 + 5);
    }

    #[test]
    fn exploding_die_counts_in_total() {
        // Natural 10 then 4: the die contributes 14.
        let die = check_manual(&[10, 4]).unwrap();
        let check = skill_check(&sheet(), "handgun", &catalog(), die).unwrap();
        assert_eq!(check.total, 5 + 4 + 14);
        // Natural 1 then 6: the die contributes -5.
        let die = check_manual(&[1, 6]).unwrap();
        assert_eq!(skill_check(&sheet(), "handgun", &catalog(), die).unwrap().total, 5 + 4 - 5);
    }

    #[test]
    fn unknown_skill_is_rejected() {
        let err = skill_check(&sheet(), "telekinesis", &catalog(), check_total(1).unwrap()).unwrap_err();
        assert_eq!(err, SkillCheckError::UnknownSkill("telekinesis".into()));
    }
}
