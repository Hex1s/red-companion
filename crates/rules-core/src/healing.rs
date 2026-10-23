//! Healing: treating critical injuries and resting (pp. 222–223).
//!
//! First aid lifts an injury's effect until the end of the day, treatment (and a quick fix)
//! removes it for good; resting restores BODY HP per full day.

use serde::{Deserialize, Serialize};
use thiserror::Error;
use ts_rs::TS;

use crate::catalog::{CriticalInjury, TreatmentSkill};
use crate::character::{Character, CharacterRecord};
use crate::damage::WoundState;
use crate::dice::CheckRoll;

#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum HealingError {
    #[error("a dead character cannot be healed")]
    Dead,
    #[error("the character does not have this injury")]
    NotInjured,
    #[error("the injury has no such way of healing")]
    NoSuchOption,
    #[error("first aid has already lifted this injury's effect today")]
    AlreadyFirstAided,
    #[error("a mortally wounded character must be stabilised before resting")]
    NeedsStabilization,
    #[error("target HP must be above the current HP and at most the maximum")]
    BadTarget,
}

/// The two ways to heal an injury (p. 223).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, TS)]
#[ts(export)]
#[serde(rename_all = "snake_case")]
pub enum HealKind {
    /// Lifts the effect until the end of the day (or removes it for a quick-fix injury).
    FirstAid,
    /// Removes the injury for good.
    Treatment,
}

/// Result of an attempt to heal one injury.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct HealResult {
    pub injury_id: String,
    pub kind: HealKind,
    pub skill: TreatmentSkill,
    pub dv: u8,
    /// `None` when the GM declared the attempt successful without a roll.
    pub roll: Option<CheckRoll>,
    /// Stat + skill level of the one who heals.
    pub bonus: i32,
    /// Bonus plus the die; for a declared success, the DV itself.
    pub total: i32,
    pub success: bool,
    /// The injury is gone from the sheet.
    pub removed: bool,
}

/// What the UI gets back after an attempt to heal an injury.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct HealApplied {
    pub character: CharacterRecord,
    pub result: HealResult,
}

/// One attempt to heal `injury` on `target` with `skill` against the DV the table gives for it.
/// `bonus` is stat + skill level of the healer, `roll` the d10 check (`None` = declared success).
///
/// A successful first aid on a quick-fix injury removes it; on any other it marks the injury
/// as lifted for the day. A successful treatment removes the injury. The death save penalty
/// already accumulated is not touched.
pub fn heal_injury(
    target: &mut Character,
    injury: &CriticalInjury,
    kind: HealKind,
    skill: TreatmentSkill,
    bonus: i32,
    roll: Option<CheckRoll>,
) -> Result<HealResult, HealingError> {
    if target.dead {
        return Err(HealingError::Dead);
    }
    if !target.injuries.contains(&injury.id) {
        return Err(HealingError::NotInjured);
    }
    let options = match kind {
        HealKind::FirstAid => &injury.first_aid,
        HealKind::Treatment => &injury.treatment,
    };
    let dv = options.iter().find(|o| o.skill == skill).ok_or(HealingError::NoSuchOption)?.dv;
    if kind == HealKind::FirstAid
        && !injury.quick_fix_removes_effect
        && target.first_aided.contains(&injury.id)
    {
        return Err(HealingError::AlreadyFirstAided);
    }

    let total = roll.as_ref().map_or(i32::from(dv), |r| bonus + r.total);
    let success = total >= i32::from(dv);
    let mut removed = false;
    if success {
        if kind == HealKind::Treatment || injury.quick_fix_removes_effect {
            target.injuries.retain(|i| i != &injury.id);
            target.first_aided.retain(|i| i != &injury.id);
            removed = true;
        } else {
            target.first_aided.push(injury.id.clone());
        }
    }
    Ok(HealResult { injury_id: injury.id.clone(), kind, skill, dv, roll, bonus, total, success, removed })
}

/// What resting up to a target HP takes (p. 222: BODY HP per full day of rest).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct RestPlan {
    pub from_hp: u32,
    /// HP the player asked for.
    pub target_hp: u32,
    pub hp_per_day: u32,
    pub days: u32,
    /// HP after the last day: whole days overshoot the target, but never pass the maximum.
    pub final_hp: u32,
    pub max_hp: u32,
}

/// What the UI gets back after a rest.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct RestApplied {
    pub character: CharacterRecord,
    pub plan: RestPlan,
}

/// Days of rest needed to reach `target_hp`.
pub fn rest_plan(character: &Character, target_hp: u32) -> Result<RestPlan, HealingError> {
    if character.dead {
        return Err(HealingError::Dead);
    }
    if character.wound_state() == WoundState::Mortal {
        return Err(HealingError::NeedsStabilization);
    }
    let max_hp = character.max_hp();
    if target_hp <= character.hp || target_hp > max_hp {
        return Err(HealingError::BadTarget);
    }
    let hp_per_day = u32::from(character.stats.body).max(1);
    let days = (target_hp - character.hp).div_ceil(hp_per_day);
    let final_hp = (character.hp + days * hp_per_day).min(max_hp);
    Ok(RestPlan { from_hp: character.hp, target_hp, hp_per_day, days, final_hp, max_hp })
}

/// Rests until `target_hp` is reached: restores the HP and ends the day, so first aid wears off.
pub fn rest(character: &mut Character, target_hp: u32) -> Result<RestPlan, HealingError> {
    let plan = rest_plan(character, target_hp)?;
    character.hp = plan.final_hp;
    character.first_aided.clear();
    crate::roles::end_of_day(character);
    Ok(plan)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::data_source::{DataSource, SeedDataSource};
    use crate::dice::check_manual;

    fn table() -> Vec<CriticalInjury> {
        SeedDataSource::default().critical_injuries().unwrap().entries
    }

    fn injury(id: &str) -> CriticalInjury {
        table().into_iter().find(|i| i.id == id).unwrap()
    }

    fn injured(id: &str) -> Character {
        let mut c = Character::quick("T");
        c.injuries.push(id.into());
        c
    }

    fn die(face: u32) -> Option<CheckRoll> {
        Some(check_manual(&[face]).unwrap())
    }

    #[test]
    fn first_aid_lifts_the_effect_for_the_day_and_keeps_the_injury() {
        let inj = injury("broken_ribs"); // first aid: paramedic DV13
        let mut c = injured("broken_ribs");
        let miss = heal_injury(&mut c, &inj, HealKind::FirstAid, TreatmentSkill::Paramedic, 8, die(4))
            .unwrap();
        assert_eq!((miss.dv, miss.total, miss.success), (13, 12, false));
        assert!(c.first_aided.is_empty());
        let hit = heal_injury(&mut c, &inj, HealKind::FirstAid, TreatmentSkill::Paramedic, 8, die(5))
            .unwrap();
        assert!(hit.success && !hit.removed);
        assert_eq!(c.injuries, vec!["broken_ribs"]);
        assert_eq!(c.first_aided, vec!["broken_ribs"]);
        assert_eq!(
            heal_injury(&mut c, &inj, HealKind::FirstAid, TreatmentSkill::Paramedic, 8, die(5)),
            Err(HealingError::AlreadyFirstAided)
        );
    }

    #[test]
    fn treatment_removes_the_injury_for_good() {
        let inj = injury("broken_ribs");
        let mut c = injured("broken_ribs");
        c.first_aided.push("broken_ribs".into());
        let r = heal_injury(&mut c, &inj, HealKind::Treatment, TreatmentSkill::Paramedic, 0, None)
            .unwrap();
        assert!(r.success && r.removed && r.roll.is_none());
        assert_eq!((r.dv, r.total), (15, 15));
        assert!(c.injuries.is_empty() && c.first_aided.is_empty());
    }

    #[test]
    fn quick_fix_injuries_are_removed_by_first_aid() {
        let inj = injury("concussion");
        assert!(inj.quick_fix_removes_effect);
        let mut c = injured("concussion");
        let r = heal_injury(&mut c, &inj, HealKind::FirstAid, TreatmentSkill::FirstAid, 9, die(4))
            .unwrap();
        assert!(r.success && r.removed);
        assert!(c.injuries.is_empty());
        // No separate treatment exists for them.
        let mut c = injured("concussion");
        assert_eq!(
            heal_injury(&mut c, &inj, HealKind::Treatment, TreatmentSkill::Surgery, 0, None),
            Err(HealingError::NoSuchOption)
        );
    }

    #[test]
    fn impossible_ways_and_wrong_targets_are_refused() {
        // Dismembered arm: no first aid, surgery only.
        let inj = injury("dismembered_arm");
        let mut c = injured("dismembered_arm");
        assert_eq!(
            heal_injury(&mut c, &inj, HealKind::FirstAid, TreatmentSkill::Paramedic, 0, None),
            Err(HealingError::NoSuchOption)
        );
        assert_eq!(
            heal_injury(&mut c, &inj, HealKind::Treatment, TreatmentSkill::Paramedic, 0, None),
            Err(HealingError::NoSuchOption)
        );
        let mut healthy = Character::quick("T");
        assert_eq!(
            heal_injury(&mut healthy, &inj, HealKind::Treatment, TreatmentSkill::Surgery, 0, None),
            Err(HealingError::NotInjured)
        );
        c.dead = true;
        assert_eq!(
            heal_injury(&mut c, &inj, HealKind::Treatment, TreatmentSkill::Surgery, 0, None),
            Err(HealingError::Dead)
        );
    }

    #[test]
    fn rest_takes_body_hp_per_day_and_rounds_days_up() {
        let mut c = Character::quick("T"); // BODY 5, WILL 5: max 35
        let max = c.max_hp();
        assert_eq!(c.stats.body, 5);
        c.hp = 10;
        let plan = rest_plan(&c, 22).unwrap(); // 12 HP at 5 a day = 3 days
        assert_eq!((plan.days, plan.hp_per_day, plan.final_hp), (3, 5, 25));
        // Whole days overshoot the target, never the maximum.
        let plan = rest_plan(&c, max).unwrap(); // 25 HP = exactly 5 days
        assert_eq!((plan.days, plan.final_hp), (5, max));
        c.hp = max - 2;
        assert_eq!(rest_plan(&c, max).unwrap().final_hp, max);
        assert_eq!(rest_plan(&c, max).unwrap().days, 1);
    }

    #[test]
    fn rest_applies_the_plan_and_ends_the_day() {
        let mut c = injured("broken_ribs");
        c.first_aided.push("broken_ribs".into());
        c.hp = 10;
        let plan = rest(&mut c, 20).unwrap();
        assert_eq!((plan.days, c.hp), (2, 20));
        assert!(c.first_aided.is_empty());
        assert_eq!(c.injuries, vec!["broken_ribs"]);
    }

    #[test]
    fn rest_is_refused_for_the_dead_the_mortal_and_bad_targets() {
        let mut c = Character::quick("T");
        let max = c.max_hp();
        c.hp = 10;
        assert_eq!(rest_plan(&c, 10), Err(HealingError::BadTarget));
        assert_eq!(rest_plan(&c, 5), Err(HealingError::BadTarget));
        assert_eq!(rest_plan(&c, max + 1), Err(HealingError::BadTarget));
        c.hp = 0;
        assert_eq!(rest_plan(&c, 10), Err(HealingError::NeedsStabilization));
        c.dead = true;
        assert_eq!(rest_plan(&c, 10), Err(HealingError::Dead));
    }

    #[test]
    fn first_aid_without_the_injury_is_invalid_on_a_sheet() {
        let mut c = Character::quick("T");
        c.first_aided.push("broken_ribs".into());
        assert!(c.validate().is_err());
        c.injuries.push("broken_ribs".into());
        assert!(c.validate().is_ok());
    }
}
