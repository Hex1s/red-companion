//! Dice engine: `NdM` notation parsing, injectable RNG, roll log and manual input.
//!
//! Every roll goes through [`DiceRng`], so the app can use real randomness while
//! tests (and manual entry from physical dice) supply exact values.

use std::str::FromStr;

use serde::{Deserialize, Serialize};
use thiserror::Error;
use ts_rs::TS;

/// Upper bounds keep a typo like `99999d99999` from hanging the app.
pub const MAX_DICE: u32 = 100;
pub const MAX_SIDES: u32 = 1000;

#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum DiceError {
    #[error("invalid dice notation: {0:?}")]
    InvalidNotation(String),
    #[error("dice count must be 1..={MAX_DICE} and sides 2..={MAX_SIDES}")]
    OutOfRange,
    #[error("expected {expected} manual values, got {got}")]
    WrongManualCount { expected: u32, got: usize },
    #[error("manual value {value} is outside 1..={sides}")]
    ManualValueOutOfRange { value: u32, sides: u32 },
}

/// Parsed `NdM[+K|-K]` expression.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct DiceNotation {
    pub count: u32,
    pub sides: u32,
    pub modifier: i32,
}

impl FromStr for DiceNotation {
    type Err = DiceError;

    fn from_str(input: &str) -> Result<Self, DiceError> {
        let err = || DiceError::InvalidNotation(input.to_string());
        let s: String = input.chars().filter(|c| !c.is_whitespace()).collect();
        let s = s.to_lowercase();

        let (count_str, rest) = s.split_once('d').ok_or_else(err)?;
        // A bare `d6` means one die.
        let count: u32 =
            if count_str.is_empty() { 1 } else { count_str.parse().map_err(|_| err())? };

        let (sides_str, modifier) = match rest.find(['+', '-']) {
            Some(i) => {
                let m: i32 = rest[i..].parse().map_err(|_| err())?;
                (&rest[..i], m)
            }
            None => (rest, 0),
        };
        let sides: u32 = sides_str.parse().map_err(|_| err())?;

        if !(1..=MAX_DICE).contains(&count) || !(2..=MAX_SIDES).contains(&sides) {
            return Err(DiceError::OutOfRange);
        }
        Ok(Self { count, sides, modifier })
    }
}

/// Source of die faces. Implement for real randomness, scripted values, etc.
pub trait DiceRng {
    /// Returns a value in `1..=sides`.
    fn roll(&mut self, sides: u32) -> u32;
}

/// OS-seeded random source used by the app.
pub struct ThreadDice(rand::rngs::ThreadRng);

impl ThreadDice {
    pub fn new() -> Self {
        Self(rand::rng())
    }
}

impl Default for ThreadDice {
    fn default() -> Self {
        Self::new()
    }
}

impl DiceRng for ThreadDice {
    fn roll(&mut self, sides: u32) -> u32 {
        use rand::RngExt;
        self.0.random_range(1..=sides)
    }
}

/// Returns preset values in order; panics when exhausted. For tests.
pub struct ScriptedDice {
    values: std::vec::IntoIter<u32>,
}

impl ScriptedDice {
    pub fn new(values: impl IntoIterator<Item = u32>) -> Self {
        Self { values: values.into_iter().collect::<Vec<_>>().into_iter() }
    }
}

impl DiceRng for ScriptedDice {
    fn roll(&mut self, _sides: u32) -> u32 {
        self.values.next().expect("ScriptedDice ran out of values")
    }
}

/// Where the faces of a roll came from.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, TS)]
#[ts(export)]
#[serde(rename_all = "snake_case")]
pub enum RollSource {
    Random,
    Manual,
}

/// One logged roll: enough to explain "why the result is X".
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct RollResult {
    pub notation: DiceNotation,
    pub dice: Vec<u32>,
    pub total: i32,
    pub source: RollSource,
}

impl RollResult {
    fn new(notation: DiceNotation, dice: Vec<u32>, source: RollSource) -> Self {
        let total = dice.iter().sum::<u32>() as i32 + notation.modifier;
        Self { notation, dice, total, source }
    }

    /// Number of dice showing their maximum face.
    pub fn max_faces(&self) -> usize {
        self.dice.iter().filter(|&&d| d == self.notation.sides).count()
    }

    /// Cyberpunk RED critical injury trigger: at least two 6s on d6 damage dice.
    pub fn is_critical(&self) -> bool {
        self.notation.sides == 6 && self.max_faces() >= 2
    }
}

pub fn roll(notation: DiceNotation, rng: &mut impl DiceRng) -> RollResult {
    let dice = (0..notation.count).map(|_| rng.roll(notation.sides)).collect();
    RollResult::new(notation, dice, RollSource::Random)
}

/// Build a result from faces entered by hand from physical dice.
pub fn roll_manual(notation: DiceNotation, values: &[u32]) -> Result<RollResult, DiceError> {
    if values.len() != notation.count as usize {
        return Err(DiceError::WrongManualCount { expected: notation.count, got: values.len() });
    }
    if let Some(&value) = values.iter().find(|&&v| v == 0 || v > notation.sides) {
        return Err(DiceError::ManualValueOutOfRange { value, sides: notation.sides });
    }
    Ok(RollResult::new(notation, values.to_vec(), RollSource::Manual))
}

/// How a d10 check turned out (p. 130).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, TS)]
#[ts(export)]
#[serde(rename_all = "snake_case")]
pub enum CheckKind {
    Normal,
    /// Natural 10: one more d10 is added.
    CriticalSuccess,
    /// Natural 1: one more d10 is subtracted.
    CriticalFailure,
}

/// A single d10 of a check (initiative, skill check) with the "exploding" rule
/// of the book: a natural 10 adds another d10, a natural 1 subtracts one. The
/// extra die never explodes again (p. 130). Not for d10 saves or d6 damage.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct CheckRoll {
    /// The first die, then the extra one after a natural 10 or 1.
    pub dice: Vec<u32>,
    pub kind: CheckKind,
    /// First die, plus or minus the extra one.
    pub total: i32,
    pub source: RollSource,
}

impl CheckRoll {
    fn new(dice: Vec<u32>, source: RollSource) -> Self {
        let first = dice[0] as i32;
        let (kind, total) = match (first, dice.get(1)) {
            (10, Some(&extra)) => (CheckKind::CriticalSuccess, first + extra as i32),
            (1, Some(&extra)) => (CheckKind::CriticalFailure, first - extra as i32),
            _ => (CheckKind::Normal, first),
        };
        Self { dice, kind, total, source }
    }
}

const CHECK_SIDES: u32 = 10;

pub fn roll_check(rng: &mut impl DiceRng) -> CheckRoll {
    let mut dice = vec![rng.roll(CHECK_SIDES)];
    if matches!(dice[0], 1 | 10) {
        dice.push(rng.roll(CHECK_SIDES));
    }
    CheckRoll::new(dice, RollSource::Random)
}

/// Check from faces read off physical dice: the first die, and a second one
/// exactly when the first shows 10 or 1.
pub fn check_manual(values: &[u32]) -> Result<CheckRoll, DiceError> {
    let expected = match values.first() {
        Some(1 | 10) => 2,
        _ => 1,
    };
    if values.len() != expected as usize {
        return Err(DiceError::WrongManualCount { expected, got: values.len() });
    }
    if let Some(&value) = values.iter().find(|&&v| v == 0 || v > CHECK_SIDES) {
        return Err(DiceError::ManualValueOutOfRange { value, sides: CHECK_SIDES });
    }
    Ok(CheckRoll::new(values.to_vec(), RollSource::Manual))
}

/// Chronological log of rolls for a session or a single resolution.
#[derive(Debug, Default, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct RollLog {
    pub entries: Vec<RollResult>,
}

impl RollLog {
    pub fn push(&mut self, result: RollResult) -> &RollResult {
        self.entries.push(result);
        self.entries.last().expect("just pushed")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn n(s: &str) -> DiceNotation {
        s.parse().unwrap()
    }

    #[test]
    fn parses_notation() {
        assert_eq!(n("5d6"), DiceNotation { count: 5, sides: 6, modifier: 0 });
        assert_eq!(n("d10"), DiceNotation { count: 1, sides: 10, modifier: 0 });
        assert_eq!(n(" 2D6 + 3 "), DiceNotation { count: 2, sides: 6, modifier: 3 });
        assert_eq!(n("1d10-2").modifier, -2);
    }

    #[test]
    fn rejects_bad_notation() {
        for bad in ["", "d", "5", "5d", "xd6", "5d6+", "5d6++1", "0d6", "5d1", "101d6", "5d1001"] {
            assert!(bad.parse::<DiceNotation>().is_err(), "{bad:?} should fail");
        }
    }

    #[test]
    fn rolls_with_injected_rng() {
        let r = roll(n("3d6+1"), &mut ScriptedDice::new([2, 4, 6]));
        assert_eq!(r.dice, [2, 4, 6]);
        assert_eq!(r.total, 13);
        assert_eq!(r.source, RollSource::Random);
    }

    #[test]
    fn random_rolls_stay_in_range() {
        let mut rng = ThreadDice::new();
        let r = roll(n("100d6"), &mut rng);
        assert!(r.dice.iter().all(|d| (1..=6).contains(d)));
    }

    #[test]
    fn manual_roll_validates_input() {
        let r = roll_manual(n("2d6"), &[3, 5]).unwrap();
        assert_eq!((r.total, r.source), (8, RollSource::Manual));
        assert_eq!(
            roll_manual(n("2d6"), &[3]),
            Err(DiceError::WrongManualCount { expected: 2, got: 1 })
        );
        assert_eq!(
            roll_manual(n("2d6"), &[3, 7]),
            Err(DiceError::ManualValueOutOfRange { value: 7, sides: 6 })
        );
        assert!(roll_manual(n("2d6"), &[0, 1]).is_err());
    }

    #[test]
    fn detects_two_sixes() {
        assert!(roll_manual(n("5d6"), &[6, 1, 6, 2, 3]).unwrap().is_critical());
        assert!(!roll_manual(n("5d6"), &[6, 1, 5, 2, 3]).unwrap().is_critical());
        assert!(!roll_manual(n("2d10"), &[10, 10]).unwrap().is_critical());
    }

    #[test]
    fn log_keeps_order() {
        let mut log = RollLog::default();
        log.push(roll_manual(n("1d10"), &[4]).unwrap());
        log.push(roll_manual(n("1d10"), &[9]).unwrap());
        assert_eq!(log.entries.iter().map(|e| e.total).collect::<Vec<_>>(), [4, 9]);
    }
    #[test]
    fn check_explodes_on_ten_and_implodes_on_one() {
        let r = roll_check(&mut ScriptedDice::new([10, 4]));
        assert_eq!(
            (r.total, r.kind, r.dice.clone()),
            (14, CheckKind::CriticalSuccess, vec![10, 4])
        );
        let r = roll_check(&mut ScriptedDice::new([1, 4]));
        assert_eq!((r.total, r.kind), (-3, CheckKind::CriticalFailure));
        let r = roll_check(&mut ScriptedDice::new([7]));
        assert_eq!((r.total, r.kind, r.dice.len()), (7, CheckKind::Normal, 1));
    }

    #[test]
    fn extra_die_does_not_explode_again() {
        let r = roll_check(&mut ScriptedDice::new([10, 10]));
        assert_eq!((r.total, r.dice.len()), (20, 2));
        let r = roll_check(&mut ScriptedDice::new([1, 1]));
        assert_eq!((r.total, r.dice.len()), (0, 2));
    }

    #[test]
    fn manual_check_needs_the_extra_die_exactly_when_the_book_does() {
        assert_eq!(check_manual(&[6]).unwrap().total, 6);
        assert_eq!(check_manual(&[10, 3]).unwrap().total, 13);
        assert_eq!(check_manual(&[1, 3]).unwrap().total, -2);
        assert_eq!(check_manual(&[10]), Err(DiceError::WrongManualCount { expected: 2, got: 1 }));
        assert_eq!(check_manual(&[1]), Err(DiceError::WrongManualCount { expected: 2, got: 1 }));
        assert_eq!(check_manual(&[6, 2]), Err(DiceError::WrongManualCount { expected: 1, got: 2 }));
        assert_eq!(check_manual(&[]), Err(DiceError::WrongManualCount { expected: 1, got: 0 }));
        assert!(check_manual(&[11]).is_err());
        assert!(check_manual(&[10, 0]).is_err());
        assert_eq!(check_manual(&[10, 3]).unwrap().source, RollSource::Manual);
    }
}
