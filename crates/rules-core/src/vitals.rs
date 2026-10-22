//! Staying alive: the death save at the start of a mortally wounded character's
//! turn and stabilisation (pp. 186, 222).
//!
//! Both change the stored sheet. The pure formulas live in `damage`
//! (`death_save`, `WoundState::stabilization_dv`); this module applies them.

use serde::{Deserialize, Serialize};
use thiserror::Error;
use ts_rs::TS;

use crate::catalog::CriticalInjury;
use crate::character::{Character, CharacterRecord};
use crate::damage::{DeathSave, WoundState, death_save};
use crate::dice::{CheckRoll, RollResult};
use crate::scene::SceneRecord;

/// Skills that may be used to stabilise (p. 222).
pub const STABILIZE_SKILLS: [&str; 2] = ["first_aid", "paramedic"];

#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum VitalsError {
    #[error("only a mortally wounded character makes death saves")]
    NotMortal,
    #[error("a healthy or dead character cannot be stabilised")]
    CannotStabilize,
    #[error("{0:?} cannot be used to stabilise")]
    BadSkill(String),
    #[error("only a dead character can be brought back")]
    NotDead,
}

/// Death save penalty a character falls back to after stabilisation: +1 for
/// every active critical injury that raises it (p. 222).
pub fn base_death_save_penalty(injuries: &[String], table: &[CriticalInjury]) -> u8 {
    let count = injuries
        .iter()
        .filter(|id| table.iter().any(|i| &i.id == *id && i.death_save_penalty))
        .count();
    u8::try_from(count).unwrap_or(u8::MAX)
}

/// Makes the death save with `roll` (a 1d10, random or manual). A fail kills the
/// character; a pass raises the penalty for the next save.
pub fn apply_death_save(
    character: &mut Character,
    roll: RollResult,
) -> Result<DeathSave, VitalsError> {
    if character.wound_state() != WoundState::Mortal {
        return Err(VitalsError::NotMortal);
    }
    let save = death_save(character.stats.body, character.death_save_penalty, roll);
    if save.passed {
        character.death_save_penalty = save.next_penalty;
    } else {
        character.dead = true;
    }
    Ok(save)
}

/// Brings a dead character back: 1 HP, no death save penalty, no critical injuries. A GM
/// decision (a miracle, a fix of a mistake), not a rule of the book.
pub fn revive(character: &mut Character) -> Result<(), VitalsError> {
    if !character.dead {
        return Err(VitalsError::NotDead);
    }
    character.dead = false;
    character.hp = 1;
    character.death_save_penalty = 0;
    character.injuries.clear();
    character.first_aided.clear();
    Ok(())
}

/// Result of a stabilisation attempt.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct Stabilization {
    pub dv: u8,
    /// `None` when the GM declared the target stabilised without a roll.
    pub roll: Option<CheckRoll>,
    /// TECH + skill level of the one who helps.
    pub bonus: i32,
    /// Bonus plus the die; for a declared success, the DV itself.
    pub total: i32,
    pub success: bool,
    /// A mortally wounded target came back to 1 HP and the death save penalty
    /// dropped to its base value.
    pub revived: bool,
}

/// TECH + first aid / paramedic + d10 against the DV of the target's wound
/// state (p. 222). `bonus` is TECH + skill level of the helper.
///
/// Success on a mortally wounded target restores 1 HP (p. 186) and resets the
/// death save penalty to `base_penalty`; for lighter wounds it only reports
/// the result, the book gives no HP effect.
pub fn stabilize(
    target: &mut Character,
    bonus: i32,
    roll: CheckRoll,
    base_penalty: u8,
) -> Result<Stabilization, VitalsError> {
    let dv = target.wound_state().stabilization_dv().ok_or(VitalsError::CannotStabilize)?;
    let total = bonus + roll.total;
    let success = total >= i32::from(dv);
    let revived = success && settle(target, success, base_penalty);
    Ok(Stabilization { dv, roll: Some(roll), bonus, total, success, revived })
}

/// The GM says the target is stabilised: the same effects as a successful check, no roll.
pub fn stabilize_granted(
    target: &mut Character,
    base_penalty: u8,
) -> Result<Stabilization, VitalsError> {
    let dv = target.wound_state().stabilization_dv().ok_or(VitalsError::CannotStabilize)?;
    let revived = settle(target, true, base_penalty);
    Ok(Stabilization { dv, roll: None, bonus: 0, total: i32::from(dv), success: true, revived })
}

/// Applies a successful stabilisation; `true` when a mortally wounded target came back.
fn settle(target: &mut Character, success: bool, base_penalty: u8) -> bool {
    if success && target.wound_state() == WoundState::Mortal {
        target.hp = 1;
        target.death_save_penalty = base_penalty;
        return true;
    }
    false
}

/// What the UI gets back after a death save.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct DeathSaveApplied {
    pub character: CharacterRecord,
    pub save: DeathSave,
    pub scene: SceneRecord,
}

/// What the UI gets back after a stabilisation attempt.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct StabilizeApplied {
    pub character: CharacterRecord,
    pub result: Stabilization,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::dice::{check_manual, roll_manual};

    fn d10(face: u32) -> RollResult {
        roll_manual("1d10".parse().unwrap(), &[face]).unwrap()
    }

    fn mortal(body: u8) -> Character {
        let mut c = Character::quick("T");
        c.stats.body = body;
        c.hp = 0;
        c
    }

    #[test]
    fn passed_save_raises_the_penalty_and_keeps_alive() {
        let mut c = mortal(6);
        let save = apply_death_save(&mut c, d10(6)).unwrap();
        assert!(save.passed && !c.dead);
        assert_eq!(c.death_save_penalty, 1);
        // With the penalty of 1 the same 6 now fails (6 + 1 > 6).
        apply_death_save(&mut c, d10(6)).unwrap();
        assert!(c.dead);
    }

    #[test]
    fn ten_always_kills() {
        let mut c = mortal(10);
        assert!(!apply_death_save(&mut c, d10(10)).unwrap().passed);
        assert!(c.dead);
    }

    #[test]
    fn only_mortally_wounded_make_saves() {
        let mut healthy = Character::quick("T");
        assert_eq!(apply_death_save(&mut healthy, d10(1)), Err(VitalsError::NotMortal));
        let mut dead = mortal(5);
        dead.dead = true;
        assert_eq!(apply_death_save(&mut dead, d10(1)), Err(VitalsError::NotMortal));
    }

    #[test]
    fn stabilizing_a_mortal_wound_needs_dv15_and_gives_one_hp() {
        let mut c = mortal(5);
        c.death_save_penalty = 3;
        // TECH 5 + First Aid 4 + 5 = 14 < 15
        let miss = stabilize(&mut c, 9, check_manual(&[5]).unwrap(), 1).unwrap();
        assert_eq!((miss.dv, miss.total, miss.success), (15, 14, false));
        assert_eq!((c.hp, c.death_save_penalty), (0, 3));
        let hit = stabilize(&mut c, 9, check_manual(&[6]).unwrap(), 1).unwrap();
        assert!(hit.success && hit.revived);
        assert_eq!((c.hp, c.death_save_penalty), (1, 1));
        assert_eq!(c.wound_state(), WoundState::Serious);
    }

    #[test]
    fn the_check_explodes() {
        let mut c = mortal(5);
        // 10 then 4 = 14, + 3 = 17 >= 15
        let r = stabilize(&mut c, 3, check_manual(&[10, 4]).unwrap(), 0).unwrap();
        assert!(r.success && r.total == 17);
        // 1 then 9 = -8
        let mut c = mortal(5);
        let r = stabilize(&mut c, 20, check_manual(&[1, 9]).unwrap(), 0).unwrap();
        assert_eq!(r.total, 12);
        assert!(!r.success);
    }

    #[test]
    fn lighter_wounds_use_their_own_dv_and_keep_hp() {
        let mut c = Character::quick("T");
        let max = c.max_hp();
        c.hp = max - 1; // light
        let r = stabilize(&mut c, 0, check_manual(&[10, 1]).unwrap(), 0).unwrap();
        assert_eq!((r.dv, r.success, r.revived), (10, true, false));
        assert_eq!(c.hp, max - 1);
        c.hp = 1; // serious for BODY 5 / WILL 5
        assert_eq!(stabilize(&mut c, 0, check_manual(&[2]).unwrap(), 0).unwrap().dv, 13);
    }

    #[test]
    fn healthy_and_dead_cannot_be_stabilised() {
        let mut c = Character::quick("T");
        let roll = check_manual(&[5]).unwrap();
        assert_eq!(stabilize(&mut c, 0, roll.clone(), 0), Err(VitalsError::CannotStabilize));
        let mut dead = mortal(5);
        dead.dead = true;
        assert_eq!(stabilize(&mut dead, 0, roll, 0), Err(VitalsError::CannotStabilize));
    }

    #[test]
    fn base_penalty_counts_penalising_injuries_only() {
        use crate::data_source::{DataSource, SeedDataSource};
        let table = SeedDataSource::default().critical_injuries().unwrap().entries;
        let with: Vec<&CriticalInjury> = table.iter().filter(|i| i.death_save_penalty).collect();
        let without = table.iter().find(|i| !i.death_save_penalty).unwrap();
        let ids = |list: &[&CriticalInjury]| list.iter().map(|i| i.id.clone()).collect::<Vec<_>>();
        assert!(with.len() >= 2);
        assert_eq!(base_death_save_penalty(&ids(&with[..2]), &table), 2);
        let mut owned = ids(&with[..1]);
        owned.push(without.id.clone());
        owned.push("unknown".into());
        assert_eq!(base_death_save_penalty(&owned, &table), 1);
    }

    #[test]
    fn a_declared_stabilisation_has_the_effects_of_a_success() {
        let mut c = mortal(5);
        c.death_save_penalty = 3;
        let r = stabilize_granted(&mut c, 1).unwrap();
        assert!(r.success && r.revived && r.roll.is_none());
        assert_eq!((r.dv, c.hp, c.death_save_penalty), (15, 1, 1));

        // Lighter wounds: nothing changes but the report.
        let mut c = Character::quick("T");
        c.hp = c.max_hp() - 1;
        let hp = c.hp;
        let r = stabilize_granted(&mut c, 0).unwrap();
        assert_eq!((r.dv, r.success, r.revived, c.hp), (10, true, false, hp));

        let mut healthy = Character::quick("T");
        assert_eq!(stabilize_granted(&mut healthy, 0), Err(VitalsError::CannotStabilize));
    }

    #[test]
    fn reviving_gives_one_hp_and_clears_penalty_and_injuries() {
        let mut c = mortal(5);
        c.dead = true;
        c.death_save_penalty = 4;
        c.injuries = vec!["broken_leg".into(), "lost_eye".into()];
        assert_eq!(revive(&mut c), Ok(()));
        assert_eq!((c.hp, c.dead, c.death_save_penalty), (1, false, 0));
        assert!(c.injuries.is_empty());
        assert_eq!(c.wound_state(), WoundState::Serious);

        // Only the dead can be brought back.
        assert_eq!(revive(&mut c), Err(VitalsError::NotDead));
    }
}
