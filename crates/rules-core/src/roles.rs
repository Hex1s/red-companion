//! Roles and role abilities (pp. 27–42, 143–166).
//!
//! A character may hold several roles (multiclass, p. 143), each with a rank 0..=10 and the
//! points of that rank spent on parts of the role ability. Catalog content (`catalog::Role`)
//! only names the parts; what a part does lives here, keyed by code.

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};
use thiserror::Error;
use ts_rs::TS;

use crate::catalog::Role;
use crate::character::{Character, CharacterRecord};

/// Highest rank of a role ability (p. 143).
pub const RANK_MAX: u8 = 10;
/// A new role may be taken once every role held so far is at this rank (p. 143).
pub const MULTICLASS_MIN_RANK: u8 = 4;
/// Rank a starting character has in their role (p. 142).
pub const STARTING_RANK: u8 = 4;

/// Medtech: Medicine (pp. 149–150).
pub const MEDTECH: &str = "medtech";
pub const SURGERY: &str = "surgery";
pub const PHARMACEUTICS: &str = "pharmaceutics";
pub const CRYOSYSTEMS: &str = "cryosystems";
/// Pharmaceutics drugs (p. 150).
pub const ANTIBIOTICS: &str = "antibiotics";
pub const RAPID_DETOX: &str = "rapid_detox";
pub const SPEEDHEAL: &str = "speedheal";
pub const STIM: &str = "stim";
pub const SURGE: &str = "surge";
/// Drugs whose benefit one person gets only once per period (day or week), so the sheet keeps a
/// dose flag. Rapid Detox has no limit.
pub const LIMITED_DRUGS: [&str; 4] = [ANTIBIOTICS, SPEEDHEAL, STIM, SURGE];
/// Dose flags a day of rest clears (the other limits run for a week; the GM clears those).
pub const DAILY_DRUGS: [&str; 2] = [SPEEDHEAL, STIM];
/// A point in Surgery is two points of the Surgery skill (p. 149).
pub const SURGERY_SKILL_PER_POINT: u8 = 2;
/// Skill level cap for the skills Medicine grants.
pub const MEDICINE_SKILL_MAX: u8 = 10;

/// One role held by a character.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct RoleEntry {
    /// `catalog::Role::code`.
    pub role: String,
    pub rank: u8,
    /// Points spent per sub-ability code; their sum is at most the rank.
    #[serde(default)]
    #[ts(type = "Record<string, number>")]
    pub points: BTreeMap<String, u8>,
    /// Option codes the player picked (e.g. drugs), at most one per point of their sub-ability.
    #[serde(default)]
    pub picks: Vec<String>,
}

impl RoleEntry {
    pub fn new(role: &str, rank: u8) -> Self {
        Self { role: role.to_string(), rank, points: BTreeMap::new(), picks: Vec::new() }
    }

    pub fn spent(&self) -> u32 {
        self.points.values().map(|&p| u32::from(p)).sum()
    }

    pub fn points_in(&self, sub: &str) -> u8 {
        self.points.get(sub).copied().unwrap_or(0)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum RoleError {
    #[error("role {0:?} is listed twice")]
    Duplicate(String),
    #[error("role {role:?}: rank {rank} is out of range")]
    RankOutOfRange { role: String, rank: u8 },
    #[error("role {role:?}: {spent} points spent, rank is {rank}")]
    Overspent { role: String, spent: u32, rank: u8 },
    #[error("unknown role {0:?}")]
    UnknownRole(String),
    #[error("role {role:?}: unknown sub-ability {sub:?}")]
    UnknownSubAbility { role: String, sub: String },
    #[error("role {role:?}: {sub:?} takes at most {max} points")]
    TooManyPoints { role: String, sub: String, max: u8 },
    #[error("role {role:?}: unknown option {option:?}")]
    UnknownOption { role: String, option: String },
    #[error("role {role:?}: option {option:?} is picked twice")]
    DuplicatePick { role: String, option: String },
    #[error("role {role:?}: {sub:?} has {picks} picks for {points} points")]
    TooManyPicks { role: String, sub: String, picks: usize, points: u8 },
}

/// Checks that need no catalog: distinct roles, ranks, points within the rank.
pub fn validate_entries(entries: &[RoleEntry]) -> Result<(), RoleError> {
    for (i, e) in entries.iter().enumerate() {
        if entries[..i].iter().any(|o| o.role == e.role) {
            return Err(RoleError::Duplicate(e.role.clone()));
        }
        if e.rank > RANK_MAX {
            return Err(RoleError::RankOutOfRange { role: e.role.clone(), rank: e.rank });
        }
        let spent = e.spent();
        if spent > u32::from(e.rank) {
            return Err(RoleError::Overspent { role: e.role.clone(), spent, rank: e.rank });
        }
    }
    Ok(())
}

/// Checks entries against the catalog: known roles, sub-abilities, options, caps.
pub fn validate_against(entries: &[RoleEntry], catalog: &[Role]) -> Result<(), RoleError> {
    validate_entries(entries)?;
    for e in entries {
        let role = catalog
            .iter()
            .find(|r| r.code == e.role)
            .ok_or_else(|| RoleError::UnknownRole(e.role.clone()))?;
        for (sub, &points) in &e.points {
            let def = role
                .subabilities
                .iter()
                .find(|s| &s.code == sub)
                .ok_or_else(|| RoleError::UnknownSubAbility { role: e.role.clone(), sub: sub.clone() })?;
            if let Some(max) = def.max_points.filter(|&m| points > m) {
                return Err(RoleError::TooManyPoints { role: e.role.clone(), sub: sub.clone(), max });
            }
        }
        for (i, option) in e.picks.iter().enumerate() {
            if e.picks[..i].contains(option) {
                return Err(RoleError::DuplicatePick { role: e.role.clone(), option: option.clone() });
            }
            if !role.subabilities.iter().any(|s| s.options.iter().any(|o| &o.code == option)) {
                return Err(RoleError::UnknownOption { role: e.role.clone(), option: option.clone() });
            }
        }
        for sub in &role.subabilities {
            let picks = e.picks.iter().filter(|p| sub.options.iter().any(|o| &o.code == *p)).count();
            let points = e.points_in(&sub.code);
            if picks > usize::from(points) {
                return Err(RoleError::TooManyPicks { role: e.role.clone(), sub: sub.code.clone(), picks, points });
            }
        }
    }
    Ok(())
}

#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum DrugError {
    #[error("the helper is not a Medtech who has picked this drug")]
    NotAvailable,
    #[error("a dead character cannot be given drugs")]
    Dead,
    #[error("the target already got this drug benefit this period")]
    AlreadyDosed,
    #[error("this drug does not work on a mortally wounded target")]
    MortallyWounded,
    #[error("unknown drug {0:?}")]
    Unknown(String),
}

/// What giving a dose did.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct DoseResult {
    pub drug: String,
    /// HP restored at once (Speedheal), 0 for the others.
    pub restored_hp: u32,
}

/// What the UI gets back after a dose: the target as stored, and what the dose did.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct DrugApplied {
    pub character: CharacterRecord,
    pub result: DoseResult,
}

/// A Medtech gives `drug` to `target` (p. 150). The helper must have picked it. Speedheal heals
/// BODY + WILL HP at once (not a mortally wounded target); the limited drugs leave a dose flag
/// so a second dose in the same period gives nothing; the other effects (stim, surge,
/// antibiotics) are for the GM to apply at the table.
pub fn administer_drug(helper: &Character, target: &mut Character, drug: &str) -> Result<DoseResult, DrugError> {
    use crate::damage::WoundState;
    if ![ANTIBIOTICS, RAPID_DETOX, SPEEDHEAL, STIM, SURGE].contains(&drug) {
        return Err(DrugError::Unknown(drug.to_string()));
    }
    if !helper.role(MEDTECH).is_some_and(|e| e.picks.iter().any(|p| p == drug)) {
        return Err(DrugError::NotAvailable);
    }
    if target.dead {
        return Err(DrugError::Dead);
    }
    if LIMITED_DRUGS.contains(&drug) && target.doses.iter().any(|d| d == drug) {
        return Err(DrugError::AlreadyDosed);
    }
    let mut restored_hp = 0;
    if drug == SPEEDHEAL {
        if target.wound_state() == WoundState::Mortal {
            return Err(DrugError::MortallyWounded);
        }
        let max = target.max_hp();
        let healed = (target.hp + u32::from(target.stats.body) + u32::from(target.stats.will)).min(max);
        restored_hp = healed - target.hp;
        target.hp = healed;
    }
    if LIMITED_DRUGS.contains(&drug) {
        target.doses.push(drug.to_string());
    }
    Ok(DoseResult { drug: drug.to_string(), restored_hp })
}

/// A day of rest ends the day: daily drug limits reset.
pub fn end_of_day(c: &mut Character) {
    c.doses.retain(|d| !DAILY_DRUGS.contains(&d.as_str()));
}

/// Dose flags must be known limited drugs, each once.
pub fn validate_doses(doses: &[String]) -> Result<(), DrugError> {
    for (i, d) in doses.iter().enumerate() {
        if !LIMITED_DRUGS.contains(&d.as_str()) {
            return Err(DrugError::Unknown(d.clone()));
        }
        if doses[..i].contains(d) {
            return Err(DrugError::AlreadyDosed);
        }
    }
    Ok(())
}

/// A new role can be added once every role held is at rank 4 or more (p. 143). The sheet does
/// not forbid breaking this (the GM may allow it); the UI warns.
pub fn can_add_role(entries: &[RoleEntry]) -> bool {
    entries.iter().all(|e| e.rank >= MULTICLASS_MIN_RANK)
}

/// Points of a rank not yet put into any sub-ability.
pub fn free_points(entry: &RoleEntry) -> u8 {
    u8::try_from(u32::from(entry.rank).saturating_sub(entry.spent())).unwrap_or(0)
}

/// The Surgery skill level a Medtech's Medicine gives: two per point, at most 10 (p. 149).
/// Not a catalog skill: it exists only through the role ability.
pub fn surgery_skill(c: &Character) -> u8 {
    let points = c.role(MEDTECH).map_or(0, |e| e.points_in(SURGERY));
    points.saturating_mul(SURGERY_SKILL_PER_POINT).min(MEDICINE_SKILL_MAX)
}

/// The Medical Technology skill level: a point from Pharmaceutics or Cryosystems is one point
/// of it, at most 10 (pp. 149–150).
pub fn medical_tech_skill(c: &Character) -> u8 {
    let Some(e) = c.role(MEDTECH) else { return 0 };
    e.points_in(PHARMACEUTICS).saturating_add(e.points_in(CRYOSYSTEMS)).min(MEDICINE_SKILL_MAX)
}

/// Bonus of a surgery check: TECH + Surgery level (surgery is a TECH skill, p. 149).
pub fn surgery_bonus(c: &Character) -> i32 {
    i32::from(c.stats.tech) + i32::from(surgery_skill(c))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::data_source::{DataSource, SeedDataSource};

    fn medtech(rank: u8, surgery: u8, pharma: u8, cryo: u8) -> Character {
        let mut c = Character::quick("Doc");
        let mut e = RoleEntry::new(MEDTECH, rank);
        e.points.insert(SURGERY.into(), surgery);
        e.points.insert(PHARMACEUTICS.into(), pharma);
        e.points.insert(CRYOSYSTEMS.into(), cryo);
        c.roles.push(e);
        c
    }

    #[test]
    fn surgery_gives_two_skill_levels_per_point() {
        let c = medtech(4, 3, 1, 0);
        assert_eq!(surgery_skill(&c), 6);
        assert_eq!(medical_tech_skill(&c), 1);
        // QUICK_STAT 5 + 6.
        assert_eq!(surgery_bonus(&c), 11);
    }

    #[test]
    fn skills_are_capped_at_ten() {
        let c = medtech(10, 7, 5, 5);
        assert_eq!(surgery_skill(&c), 10);
        assert_eq!(medical_tech_skill(&c), 10);
    }

    #[test]
    fn no_medtech_no_skills() {
        let c = Character::quick("Nobody");
        assert_eq!(surgery_skill(&c), 0);
        assert_eq!(medical_tech_skill(&c), 0);
        assert_eq!(surgery_bonus(&c), 5);
    }

    #[test]
    fn points_cannot_exceed_rank_and_roles_are_unique() {
        assert_eq!(validate_entries(&medtech(4, 2, 2, 0).roles), Ok(()));
        assert!(matches!(
            validate_entries(&medtech(4, 3, 2, 0).roles),
            Err(RoleError::Overspent { spent: 5, rank: 4, .. })
        ));
        assert!(matches!(
            validate_entries(&[RoleEntry::new("solo", 11)]),
            Err(RoleError::RankOutOfRange { .. })
        ));
        assert!(matches!(
            validate_entries(&[RoleEntry::new("solo", 4), RoleEntry::new("solo", 1)]),
            Err(RoleError::Duplicate(_))
        ));
    }

    #[test]
    fn multiclass_needs_rank_four_everywhere() {
        assert!(can_add_role(&[]));
        assert!(can_add_role(&[RoleEntry::new("solo", 4)]));
        assert!(!can_add_role(&[RoleEntry::new("solo", 3)]));
        assert!(!can_add_role(&[RoleEntry::new("solo", 6), RoleEntry::new("nomad", 1)]));
    }

    #[test]
    fn free_points_counts_what_is_left() {
        let mut e = RoleEntry::new(MEDTECH, 5);
        e.points.insert(SURGERY.into(), 2);
        assert_eq!(free_points(&e), 3);
    }

    #[test]
    fn catalog_checks_codes_caps_and_picks() {
        let catalog = SeedDataSource::default().roles().unwrap().entries;
        let ok = medtech(5, 2, 2, 1);
        assert_eq!(validate_against(&ok.roles, &catalog), Ok(()));

        let mut unknown = ok.clone();
        unknown.roles[0].role = "plumber".into();
        assert!(matches!(validate_against(&unknown.roles, &catalog), Err(RoleError::UnknownRole(_))));

        let mut capped = medtech(10, 0, 6, 0);
        assert!(matches!(
            validate_against(&capped.roles, &catalog),
            Err(RoleError::TooManyPoints { max: 5, .. })
        ));
        capped.roles[0].points.insert(PHARMACEUTICS.into(), 1);
        capped.roles[0].picks = vec!["antibiotics".into(), "speedheal".into()];
        assert!(matches!(
            validate_against(&capped.roles, &catalog),
            Err(RoleError::TooManyPicks { picks: 2, points: 1, .. })
        ));
        capped.roles[0].picks = vec!["antibiotics".into()];
        assert_eq!(validate_against(&capped.roles, &catalog), Ok(()));
        capped.roles[0].picks = vec!["moonshine".into()];
        assert!(matches!(validate_against(&capped.roles, &catalog), Err(RoleError::UnknownOption { .. })));
    }

    fn medic_with(drugs: &[&str]) -> Character {
        let mut c = medtech(5, 0, drugs.len() as u8, 0);
        c.roles[0].picks = drugs.iter().map(|d| d.to_string()).collect();
        c
    }

    #[test]
    fn speedheal_restores_body_plus_will_once_a_day() {
        let medic = medic_with(&[SPEEDHEAL]);
        let mut patient = Character::quick("P"); // BODY 5, WILL 5, max HP 35
        patient.hp = 12;
        let r = administer_drug(&medic, &mut patient, SPEEDHEAL).unwrap();
        assert_eq!((r.restored_hp, patient.hp), (10, 22));
        assert_eq!(administer_drug(&medic, &mut patient, SPEEDHEAL), Err(DrugError::AlreadyDosed));
        end_of_day(&mut patient);
        let r = administer_drug(&medic, &mut patient, SPEEDHEAL).unwrap();
        assert_eq!(r.restored_hp, 10);
    }

    #[test]
    fn speedheal_stops_at_max_hp_and_refuses_the_mortally_wounded() {
        let medic = medic_with(&[SPEEDHEAL]);
        let mut patient = Character::quick("P");
        patient.hp = 30;
        assert_eq!(administer_drug(&medic, &mut patient, SPEEDHEAL).unwrap().restored_hp, 5);
        assert_eq!(patient.hp, patient.max_hp());

        let mut dying = Character::quick("D");
        dying.hp = 0;
        assert_eq!(administer_drug(&medic, &mut dying, SPEEDHEAL), Err(DrugError::MortallyWounded));
        assert_eq!(dying.hp, 0);
    }

    #[test]
    fn only_a_medtech_with_the_drug_can_give_it() {
        let mut patient = Character::quick("P");
        assert_eq!(
            administer_drug(&Character::quick("Nobody"), &mut patient, STIM),
            Err(DrugError::NotAvailable)
        );
        assert_eq!(administer_drug(&medic_with(&[STIM]), &mut patient, SPEEDHEAL), Err(DrugError::NotAvailable));
        assert!(matches!(
            administer_drug(&medic_with(&[STIM]), &mut patient, "moonshine"),
            Err(DrugError::Unknown(_))
        ));
        patient.dead = true;
        assert_eq!(administer_drug(&medic_with(&[STIM]), &mut patient, STIM), Err(DrugError::Dead));
    }

    #[test]
    fn weekly_flags_survive_a_day_and_detox_has_no_limit() {
        let medic = medic_with(&[SURGE, RAPID_DETOX]);
        let mut patient = Character::quick("P");
        administer_drug(&medic, &mut patient, SURGE).unwrap();
        administer_drug(&medic, &mut patient, RAPID_DETOX).unwrap();
        administer_drug(&medic, &mut patient, RAPID_DETOX).unwrap();
        assert_eq!(patient.doses, vec![SURGE]);
        end_of_day(&mut patient);
        assert_eq!(patient.doses, vec![SURGE]);
        assert_eq!(administer_drug(&medic, &mut patient, SURGE), Err(DrugError::AlreadyDosed));
    }

    #[test]
    fn dose_flags_are_validated() {
        assert_eq!(validate_doses(&[STIM.into(), SURGE.into()]), Ok(()));
        assert!(validate_doses(&[RAPID_DETOX.into()]).is_err());
        assert!(validate_doses(&[STIM.into(), STIM.into()]).is_err());
    }
}
