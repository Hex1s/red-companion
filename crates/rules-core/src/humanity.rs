//! Humanity: loss, therapy and cyberpsychosis (pp. 80, 108–109, 229–230).
//!
//! HUM is EMP × 10; every full 10 HUM lost takes a point off EMP, and below zero the character
//! is a cyberpsycho. The book gives no mechanic for the breakdown itself, so it is a status the
//! GM plays out, not a penalty the app applies.

use serde::{Deserialize, Serialize};
use thiserror::Error;
use ts_rs::TS;

use crate::character::{Character, CharacterRecord, HUMANITY_PER_EMP};
use crate::dice::{CheckRoll, RollResult};

#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum HumanityError {
    #[error("a dead character has no humanity to change")]
    Dead,
    #[error("humanity loss must be above zero")]
    NoLoss,
    #[error("humanity is already at its maximum")]
    AlreadyFull,
    #[error("the restored roll is missing")]
    MissingRestore,
}

/// Current HUM: unset means the sheet is untouched and HUM is full. Never above the maximum, so
/// lowering EMP on the sheet lowers it too.
pub fn current_humanity(c: &Character) -> i32 {
    let max = max_humanity(c);
    c.humanity.map_or(max, |h| h.min(max))
}

pub fn max_humanity(c: &Character) -> i32 {
    i32::from(c.stats.emp) * HUMANITY_PER_EMP as i32
}

/// EMP the character has now: a point is lost per full 10 HUM lost (44 HUM -> 4, 39 -> 3); never
/// above the base EMP on the sheet, and 0 once HUM is below zero.
pub fn effective_emp(c: &Character) -> u8 {
    let hum = current_humanity(c);
    if hum <= 0 {
        return 0;
    }
    // Whole tens only: 44 -> 4, 40 -> 4, 39 -> 3.
    u8::try_from(hum / HUMANITY_PER_EMP as i32).unwrap_or(u8::MAX).min(c.stats.emp)
}

/// Below zero HUM is cyberpsychosis (p. 80).
pub fn is_cyberpsycho(c: &Character) -> bool {
    current_humanity(c) < 0
}

/// What a humanity loss did.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct HumanityLoss {
    pub from: i32,
    pub to: i32,
    pub emp_from: u8,
    pub emp_to: u8,
    /// The character has just crossed below zero.
    pub became_cyberpsycho: bool,
}

/// Takes `loss` HUM off. Nothing stops it going below zero: that is cyberpsychosis.
pub fn lose_humanity(c: &mut Character, loss: u32) -> Result<HumanityLoss, HumanityError> {
    if c.dead {
        return Err(HumanityError::Dead);
    }
    if loss == 0 {
        return Err(HumanityError::NoLoss);
    }
    let from = current_humanity(c);
    let emp_from = effective_emp(c);
    let was_psycho = is_cyberpsycho(c);
    let to = from.saturating_sub(i32::try_from(loss).unwrap_or(i32::MAX));
    c.humanity = Some(to);
    Ok(HumanityLoss {
        from,
        to,
        emp_from,
        emp_to: effective_emp(c),
        became_cyberpsycho: !was_psycho && is_cyberpsycho(c),
    })
}

/// What the UI gets back after a humanity loss.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct HumanityLossApplied {
    pub character: CharacterRecord,
    pub loss: HumanityLoss,
}

/// What the UI gets back after a week of therapy.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct TherapyApplied {
    pub character: CharacterRecord,
    pub result: TherapyResult,
}

/// The therapies that give humanity back (p. 230).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, TS)]
#[ts(export)]
#[serde(rename_all = "snake_case")]
pub enum TherapyKind {
    /// "Standard humanity loss": 2d6 back, DV 15.
    Standard,
    /// "Extreme humanity loss": 4d6 back, DV 17.
    Extreme,
}

impl TherapyKind {
    pub const ALL: [Self; 2] = [Self::Standard, Self::Extreme];

    /// DV of the Medtech's check.
    pub fn dv(self) -> u8 {
        match self {
            Self::Standard => 15,
            Self::Extreme => 17,
        }
    }

    /// Dice of the restored humanity.
    pub fn dice(self) -> &'static str {
        match self {
            Self::Standard => "2d6",
            Self::Extreme => "4d6",
        }
    }

    /// Price of the week of therapy, eb.
    pub fn price_eb(self) -> u32 {
        match self {
            Self::Standard => 500,
            Self::Extreme => 1000,
        }
    }

    /// Materials the Medtech spends (lost with a failed check), eb.
    pub fn materials_eb(self) -> u32 {
        match self {
            Self::Standard => 100,
            Self::Extreme => 500,
        }
    }
}

/// A therapy with its numbers, for the window to show.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct TherapyInfo {
    pub kind: TherapyKind,
    pub dv: u8,
    pub dice: String,
    pub price_eb: u32,
    pub materials_eb: u32,
}

pub fn therapies() -> Vec<TherapyInfo> {
    TherapyKind::ALL
        .into_iter()
        .map(|kind| TherapyInfo {
            kind,
            dv: kind.dv(),
            dice: kind.dice().to_string(),
            price_eb: kind.price_eb(),
            materials_eb: kind.materials_eb(),
        })
        .collect()
}

/// Result of a week of therapy.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct TherapyResult {
    pub kind: TherapyKind,
    pub dv: u8,
    /// `None` when the GM declared the check successful without a roll.
    pub check: Option<CheckRoll>,
    /// Medtech's bonus (Medicine ability rank + stat), typed in by the GM.
    pub bonus: i32,
    pub total: i32,
    pub success: bool,
    /// Dice of the restoration; only on success.
    pub restored_roll: Option<RollResult>,
    /// HUM actually gained (the maximum caps it).
    pub restored: i32,
    pub from: i32,
    pub to: i32,
}

/// One week of therapy. A successful check gives back `restore` (the 2d6 / 4d6 of the kind),
/// never above the maximum; a failed one wastes the week and the materials.
pub fn therapy(
    c: &mut Character,
    kind: TherapyKind,
    bonus: i32,
    check: Option<CheckRoll>,
    restore: Option<RollResult>,
) -> Result<TherapyResult, HumanityError> {
    if c.dead {
        return Err(HumanityError::Dead);
    }
    let from = current_humanity(c);
    let max = max_humanity(c);
    if from >= max {
        return Err(HumanityError::AlreadyFull);
    }
    let dv = kind.dv();
    let total = check.as_ref().map_or(i32::from(dv), |r| bonus + r.total);
    let success = total >= i32::from(dv);
    let mut restored_roll = None;
    let mut to = from;
    if success {
        let roll = restore.ok_or(HumanityError::MissingRestore)?;
        to = from.saturating_add(roll.total.max(0)).min(max);
        restored_roll = Some(roll);
        c.humanity = Some(to);
    }
    Ok(TherapyResult {
        kind,
        dv,
        check,
        bonus,
        total,
        success,
        restored_roll,
        restored: to - from,
        from,
        to,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::dice::{check_manual, roll_manual};

    fn character(emp: u8) -> Character {
        let mut c = Character::quick("T");
        c.stats.emp = emp;
        c
    }

    fn die(face: u32) -> Option<CheckRoll> {
        Some(check_manual(&[face]).unwrap())
    }

    fn dice(notation: &str, faces: &[u32]) -> Option<RollResult> {
        Some(roll_manual(notation.parse().unwrap(), faces).unwrap())
    }

    #[test]
    fn humanity_is_ten_per_emp_and_untouched_sheets_are_full() {
        let c = character(5);
        assert_eq!((current_humanity(&c), max_humanity(&c)), (50, 50));
        assert_eq!(effective_emp(&c), 5);
        assert!(!is_cyberpsycho(&c));
    }

    #[test]
    fn every_ten_lost_takes_a_point_of_emp_like_the_book_example() {
        // p. 80: 44 HUM is 4 EMP until HUM drops to 39, then EMP is 3.
        let mut c = character(5);
        c.humanity = Some(44);
        assert_eq!(effective_emp(&c), 4);
        let loss = lose_humanity(&mut c, 5).unwrap();
        assert_eq!((loss.from, loss.to), (44, 39));
        assert_eq!((loss.emp_from, loss.emp_to), (4, 3));
        assert!(!loss.became_cyberpsycho);
        // p. 229: from 50 HUM a loss down to 46 takes EMP from 5 to 4.
        let mut c = character(5);
        let loss = lose_humanity(&mut c, 4).unwrap();
        assert_eq!((loss.to, loss.emp_to), (46, 4));
    }

    #[test]
    fn below_zero_is_cyberpsychosis_and_emp_is_zero() {
        let mut c = character(2);
        let loss = lose_humanity(&mut c, 15).unwrap();
        assert_eq!((loss.from, loss.to), (20, 5));
        assert!(!loss.became_cyberpsycho);
        let loss = lose_humanity(&mut c, 8).unwrap();
        assert_eq!(loss.to, -3);
        assert!(loss.became_cyberpsycho && is_cyberpsycho(&c));
        assert_eq!(effective_emp(&c), 0);
        // Already a cyberpsycho: not "just became".
        assert!(!lose_humanity(&mut c, 1).unwrap().became_cyberpsycho);
        // Exactly zero is not yet cyberpsychosis (p. 80: "below zero").
        let mut c = character(2);
        lose_humanity(&mut c, 20).unwrap();
        assert!(!is_cyberpsycho(&c));
    }

    #[test]
    fn lowering_emp_on_the_sheet_caps_current_humanity() {
        let mut c = character(5);
        c.humanity = Some(48);
        c.stats.emp = 3;
        assert_eq!(current_humanity(&c), 30);
        assert_eq!(effective_emp(&c), 3);
    }

    #[test]
    fn the_dead_and_zero_losses_are_refused() {
        let mut c = character(5);
        assert_eq!(lose_humanity(&mut c, 0), Err(HumanityError::NoLoss));
        c.dead = true;
        assert_eq!(lose_humanity(&mut c, 3), Err(HumanityError::Dead));
    }

    #[test]
    fn therapy_numbers_follow_the_book_table() {
        assert_eq!((TherapyKind::Standard.dv(), TherapyKind::Standard.dice()), (15, "2d6"));
        assert_eq!((TherapyKind::Extreme.dv(), TherapyKind::Extreme.dice()), (17, "4d6"));
        assert_eq!((TherapyKind::Standard.price_eb(), TherapyKind::Standard.materials_eb()), (500, 100));
        assert_eq!((TherapyKind::Extreme.price_eb(), TherapyKind::Extreme.materials_eb()), (1000, 500));
    }

    #[test]
    fn a_successful_therapy_restores_the_roll_up_to_the_maximum() {
        let mut c = character(5);
        c.humanity = Some(20);
        let r = therapy(&mut c, TherapyKind::Standard, 9, die(6), dice("2d6", &[3, 4])).unwrap();
        assert_eq!((r.total, r.success), (15, true));
        assert_eq!((r.from, r.restored, r.to), (20, 7, 27));
        assert_eq!(current_humanity(&c), 27);
        // The maximum caps the gain.
        c.humanity = Some(48);
        let r = therapy(&mut c, TherapyKind::Extreme, 20, die(2), dice("4d6", &[6, 6, 6, 6])).unwrap();
        assert!(r.success);
        assert_eq!((r.restored, r.to), (2, 50));
    }

    #[test]
    fn a_failed_therapy_changes_nothing() {
        let mut c = character(5);
        c.humanity = Some(20);
        let r = therapy(&mut c, TherapyKind::Standard, 9, die(5), None).unwrap();
        assert_eq!((r.total, r.success, r.restored), (14, false, 0));
        assert_eq!(current_humanity(&c), 20);
    }

    #[test]
    fn a_declared_success_needs_the_restoration_roll_and_full_humanity_is_refused() {
        let mut c = character(5);
        c.humanity = Some(10);
        assert_eq!(
            therapy(&mut c, TherapyKind::Standard, 0, None, None),
            Err(HumanityError::MissingRestore)
        );
        let r = therapy(&mut c, TherapyKind::Standard, 0, None, dice("2d6", &[1, 1])).unwrap();
        assert_eq!((r.total, r.success, r.to), (15, true, 12));
        let mut full = character(5);
        assert_eq!(
            therapy(&mut full, TherapyKind::Standard, 0, None, dice("2d6", &[1, 1])),
            Err(HumanityError::AlreadyFull)
        );
    }

    #[test]
    fn a_cyberpsycho_can_be_brought_back_above_zero() {
        let mut c = character(5);
        c.humanity = Some(-6);
        assert!(is_cyberpsycho(&c));
        therapy(&mut c, TherapyKind::Extreme, 20, die(2), dice("4d6", &[5, 5, 5, 5])).unwrap();
        assert_eq!(current_humanity(&c), 14);
        assert!(!is_cyberpsycho(&c));
    }
}
