//! Initiative tracker: who acts in which order, rounds and the active turn.
//!
//! Initiative is REF + 1d10, rolled once per fight (or typed in from physical
//! dice). The d10 explodes like any check: a natural 10 adds another d10, a
//! natural 1 subtracts one (p. 130), so the total can even be negative. The order is stored, not recomputed: ties are broken by higher REF and
//! then by the order of adding, and the GM can nudge anyone up or down by hand
//! (decision of the user, ROADMAP 1.6). Dead characters stay in the list but
//! their turn is skipped; the caller passes their ids as `skip`.

use serde::{Deserialize, Serialize};
use thiserror::Error;
use ts_rs::TS;

use crate::dice::CheckRoll;

/// Initiative range accepted from the UI (REF 10 + two d10 = 30 at best,
/// REF 1 + 1 - 10 = -8 at worst, with room for bonuses).
pub const INITIATIVE_MAX: i32 = 99;
pub const INITIATIVE_MIN: i32 = -99;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct Combatant {
    #[ts(type = "number")]
    pub character_id: i64,
    /// REF at the moment of rolling; only used to break ties.
    pub reflex: u8,
    /// The d10 check; `None` when the total was typed in by hand.
    pub roll: Option<CheckRoll>,
    /// What a role added to the roll (Solo's Personal Initiative); part of `initiative`.
    #[serde(default)]
    pub bonus: i32,
    pub initiative: i32,
}

/// The fight in progress. `round == 0` means it has not started yet (the GM is
/// still gathering the participants).
#[derive(Debug, Default, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct Combat {
    /// In acting order.
    #[serde(default)]
    pub combatants: Vec<Combatant>,
    #[serde(default)]
    pub round: u32,
    /// Whose turn it is; `None` before the start.
    #[serde(default)]
    #[ts(type = "number | null")]
    pub active: Option<i64>,
    /// The active fighter is mortally wounded and has not made this turn's
    /// death save yet (set by the app when a turn begins).
    #[serde(default)]
    pub death_save_due: bool,
    /// The round in which a fighter last had their first damage reduced (Solo's Damage
    /// Deflection works once per round); cleared when the fight ends.
    #[serde(default)]
    #[ts(type = "Record<string, number>")]
    pub deflected: std::collections::BTreeMap<i64, u32>,
}

#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum InitiativeError {
    #[error("character {0} is not in the fight")]
    NotInFight(i64),
    #[error("character {0} is already in the fight")]
    AlreadyInFight(i64),
    #[error("initiative must be between {INITIATIVE_MIN} and {INITIATIVE_MAX}")]
    OutOfRange,
    #[error("the fight has not started")]
    NotStarted,
    #[error("nobody is able to act")]
    NobodyCanAct,
}

impl Combat {
    pub fn is_started(&self) -> bool {
        self.round > 0
    }

    fn position(&self, character_id: i64) -> Result<usize, InitiativeError> {
        self.combatants
            .iter()
            .position(|c| c.character_id == character_id)
            .ok_or(InitiativeError::NotInFight(character_id))
    }

    /// Inserts after everyone who acts before or together with `entry`
    /// (higher initiative, then higher REF; full ties keep the adding order).
    fn insert_sorted(&mut self, entry: Combatant) {
        let key = (entry.initiative, entry.reflex);
        let at = self
            .combatants
            .iter()
            .position(|c| (c.initiative, c.reflex) < key)
            .unwrap_or(self.combatants.len());
        self.combatants.insert(at, entry);
    }

    /// Adds a participant with a rolled initiative die.
    pub fn add(
        &mut self,
        character_id: i64,
        reflex: u8,
        roll: CheckRoll,
    ) -> Result<(), InitiativeError> {
        self.add_with_bonus(character_id, reflex, 0, roll)
    }

    /// Like `add`, with a bonus from a role on top of REF and the die.
    pub fn add_with_bonus(
        &mut self,
        character_id: i64,
        reflex: u8,
        bonus: i32,
        roll: CheckRoll,
    ) -> Result<(), InitiativeError> {
        if self.combatants.iter().any(|c| c.character_id == character_id) {
            return Err(InitiativeError::AlreadyInFight(character_id));
        }
        self.insert_sorted(Combatant {
            character_id,
            reflex,
            bonus,
            initiative: i32::from(reflex) + bonus + roll.total,
            roll: Some(roll),
        });
        Ok(())
    }

    /// Re-rolls someone's initiative (a new die) and puts them in place.
    pub fn reroll(
        &mut self,
        character_id: i64,
        reflex: u8,
        roll: CheckRoll,
    ) -> Result<(), InitiativeError> {
        self.reroll_with_bonus(character_id, reflex, 0, roll)
    }

    pub fn reroll_with_bonus(
        &mut self,
        character_id: i64,
        reflex: u8,
        bonus: i32,
        roll: CheckRoll,
    ) -> Result<(), InitiativeError> {
        let at = self.position(character_id)?;
        self.combatants.remove(at);
        self.insert_sorted(Combatant {
            character_id,
            reflex,
            bonus,
            initiative: i32::from(reflex) + bonus + roll.total,
            roll: Some(roll),
        });
        self.follow_turn(character_id, at);
        Ok(())
    }

    /// Sets the total by hand (a bonus, a house rule); the die face is dropped.
    pub fn set_initiative(
        &mut self,
        character_id: i64,
        initiative: i32,
    ) -> Result<(), InitiativeError> {
        if !(INITIATIVE_MIN..=INITIATIVE_MAX).contains(&initiative) {
            return Err(InitiativeError::OutOfRange);
        }
        let at = self.position(character_id)?;
        let mut entry = self.combatants.remove(at);
        entry.roll = None;
        entry.bonus = 0;
        entry.initiative = initiative;
        self.insert_sorted(entry);
        self.follow_turn(character_id, at);
        Ok(())
    }

    /// Rolls someone's initiative: a new entry, or a fresh die for an existing
    /// one. The card's dice button works the same before and during the fight.
    pub fn roll(
        &mut self,
        character_id: i64,
        reflex: u8,
        roll: CheckRoll,
    ) -> Result<(), InitiativeError> {
        self.roll_with_bonus(character_id, reflex, 0, roll)
    }

    pub fn roll_with_bonus(
        &mut self,
        character_id: i64,
        reflex: u8,
        bonus: i32,
        roll: CheckRoll,
    ) -> Result<(), InitiativeError> {
        if self.is_in_fight(character_id) {
            self.reroll_with_bonus(character_id, reflex, bonus, roll)
        } else {
            self.add_with_bonus(character_id, reflex, bonus, roll)
        }
    }

    /// Sets the total by hand for someone who may not have one yet: the
    /// initiative field on a card.
    pub fn set_or_add(
        &mut self,
        character_id: i64,
        reflex: u8,
        initiative: i32,
    ) -> Result<(), InitiativeError> {
        if !(INITIATIVE_MIN..=INITIATIVE_MAX).contains(&initiative) {
            return Err(InitiativeError::OutOfRange);
        }
        if !self.is_in_fight(character_id) {
            self.insert_sorted(Combatant { character_id, reflex, bonus: 0, roll: None, initiative });
            return Ok(());
        }
        self.set_initiative(character_id, initiative)
    }

    pub fn is_in_fight(&self, character_id: i64) -> bool {
        self.combatants.iter().any(|c| c.character_id == character_id)
    }

    /// Moves one place earlier (`delta = -1`) or later (`delta = 1`) in the order.
    pub fn shift(&mut self, character_id: i64, delta: i32) -> Result<(), InitiativeError> {
        let at = self.position(character_id)?;
        let to = at as i64 + i64::from(delta.signum());
        if (0..self.combatants.len() as i64).contains(&to) {
            self.combatants.swap(at, to as usize);
        }
        Ok(())
    }

    /// Keeps the turn on the right slot when the active fighter moved later in the order (a delay,
    /// a re-rolled total): the turn passes to whoever now stands where they stood, and the mover acts
    /// at the new place. Others who cross the active fighter simply act again or are skipped, as the
    /// order says: that is how a GM gives an extra turn or takes one away.
    fn follow_turn(&mut self, character_id: i64, was_at: usize) {
        if !self.is_started() || self.active != Some(character_id) {
            return;
        }
        if self.position(character_id).is_ok_and(|now| now > was_at) {
            self.active = Some(self.combatants[was_at].character_id);
        }
    }

    /// Puts the combatant at `to` (a place in the order, clamped to the ends): a drag of the card.
    /// The total follows the new place, so the order stays sorted by initiative: it is kept when it
    /// already fits between the new neighbours, else it becomes the nearest bound (a tie with the
    /// neighbour); dropped on top (an ambush) or at the bottom (a delay) it goes one past the end.
    /// A changed total drops the die face. Moving across the active fighter is allowed: someone who
    /// has acted and goes below them acts again, someone who has not and goes above them is skipped.
    pub fn move_to(&mut self, character_id: i64, to: usize) -> Result<(), InitiativeError> {
        let at = self.position(character_id)?;
        let to = to.min(self.combatants.len() - 1);
        if to == at {
            return Ok(());
        }
        let mut entry = self.combatants.remove(at);
        let own = entry.initiative;
        let above = to.checked_sub(1).map(|i| self.combatants[i].initiative);
        let below = self.combatants.get(to).map(|c| c.initiative);
        let total = match (above, below) {
            (Some(a), Some(b)) => own.min(a).max(b),
            (None, Some(b)) => if own >= b { own } else { b + 1 },
            (Some(a), None) => if own <= a { own } else { a - 1 },
            (None, None) => own,
        }
        .clamp(INITIATIVE_MIN, INITIATIVE_MAX);
        if total != own {
            entry.initiative = total;
            entry.roll = None;
            entry.bonus = 0;
        }
        self.combatants.insert(to, entry);
        self.follow_turn(character_id, at);
        Ok(())
    }

    /// Index and wrap flag of the next combatant after `from` who is not in
    /// `skip`. `include_self` allows coming back round to `from` itself.
    fn next_after(&self, from: usize, skip: &[i64], include_self: bool) -> Option<(usize, bool)> {
        let n = self.combatants.len();
        let steps = if include_self { n } else { n.saturating_sub(1) };
        (1..=steps)
            .map(|i| (from + i) % n.max(1))
            .zip(1..=steps)
            .find(|&(idx, _)| !skip.contains(&self.combatants[idx].character_id))
            .map(|(idx, i)| (idx, from + i >= n))
    }

    /// Starts round 1 with the first participant who can act.
    pub fn start(&mut self, skip: &[i64]) -> Result<(), InitiativeError> {
        let first = self
            .combatants
            .iter()
            .find(|c| !skip.contains(&c.character_id))
            .ok_or(InitiativeError::NobodyCanAct)?;
        self.active = Some(first.character_id);
        self.round = 1;
        Ok(())
    }

    /// Passes the turn on; the round counter grows when the order wraps.
    pub fn next_turn(&mut self, skip: &[i64]) -> Result<(), InitiativeError> {
        let active =
            self.active.filter(|_| self.is_started()).ok_or(InitiativeError::NotStarted)?;
        let from = self.position(active)?;
        let (idx, wrapped) =
            self.next_after(from, skip, true).ok_or(InitiativeError::NobodyCanAct)?;
        self.active = Some(self.combatants[idx].character_id);
        if wrapped {
            self.round += 1;
        }
        Ok(())
    }

    /// Steps the turn back to the previous fighter who can act; the round counter goes down when
    /// the order wraps backwards. Nothing happens at the very start of round 1. Only the pointer
    /// moves: damage dealt and saves made are not undone.
    pub fn prev_turn(&mut self, skip: &[i64]) -> Result<(), InitiativeError> {
        let active =
            self.active.filter(|_| self.is_started()).ok_or(InitiativeError::NotStarted)?;
        let from = self.position(active)?;
        let n = self.combatants.len();
        let found = (1..=n)
            .map(|i| ((from + n - i) % n, i > from))
            .find(|&(idx, _)| !skip.contains(&self.combatants[idx].character_id));
        let (idx, wrapped) = found.ok_or(InitiativeError::NobodyCanAct)?;
        if wrapped && self.round == 1 {
            return Ok(());
        }
        self.active = Some(self.combatants[idx].character_id);
        if wrapped {
            self.round -= 1;
        }
        Ok(())
    }

    /// Takes someone out of the fight. If it was their turn, it passes on.
    pub fn remove(&mut self, character_id: i64, skip: &[i64]) -> Result<(), InitiativeError> {
        let at = self.position(character_id)?;
        if self.active == Some(character_id) {
            match self.next_after(at, skip, false) {
                Some((idx, wrapped)) => {
                    self.active = Some(self.combatants[idx].character_id);
                    if wrapped {
                        self.round += 1;
                    }
                }
                None => self.active = None,
            }
        }
        self.combatants.remove(at);
        self.deflected.remove(&character_id);
        Ok(())
    }

    /// The fight is on, this fighter is in it and has not had a damage reduced this round yet.
    pub fn first_damage_this_round(&self, character_id: i64) -> bool {
        self.round > 0
            && self.is_in_fight(character_id)
            && self.deflected.get(&character_id) != Some(&self.round)
    }

    /// Ends the fight and forgets the order.
    pub fn end(&mut self) {
        *self = Self::default();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::dice::{CheckKind, check_manual};

    fn die(face: u32) -> CheckRoll {
        check_manual(&[face]).unwrap()
    }

    fn ids(combat: &Combat) -> Vec<i64> {
        combat.combatants.iter().map(|c| c.character_id).collect()
    }

    fn fight() -> Combat {
        let mut combat = Combat::default();
        combat.add(1, 6, die(4)).unwrap(); // 10
        combat.add(2, 5, die(9)).unwrap(); // 14
        combat.add(3, 8, die(2)).unwrap(); // 10, higher REF than #1
        combat.add(4, 8, die(2)).unwrap(); // 10, full tie with #3: added later
        combat
    }

    #[test]
    fn order_is_initiative_then_reflex_then_adding_order() {
        assert_eq!(ids(&fight()), vec![2, 3, 4, 1]);
    }

    #[test]
    fn initiative_is_reflex_plus_die() {
        let combat = fight();
        assert_eq!(combat.combatants[0].initiative, 14);
        assert_eq!(combat.combatants[0].roll.as_ref().map(|r| r.total), Some(9));
    }

    #[test]
    fn exploding_die_changes_the_total() {
        let mut combat = Combat::default();
        combat.add(1, 6, check_manual(&[10, 5]).unwrap()).unwrap(); // 6 + 15
        combat.add(2, 6, check_manual(&[1, 9]).unwrap()).unwrap(); // 6 - 8
        combat.add(3, 6, die(7)).unwrap(); // 13
        assert_eq!(ids(&combat), vec![1, 3, 2]);
        assert_eq!(combat.combatants[0].initiative, 21);
        assert_eq!(combat.combatants[0].roll.as_ref().unwrap().kind, CheckKind::CriticalSuccess);
        assert_eq!(combat.combatants[2].initiative, -2);
    }

    #[test]
    fn duplicate_and_unknown_characters_are_rejected() {
        let mut combat = fight();
        assert_eq!(combat.add(1, 5, die(5)), Err(InitiativeError::AlreadyInFight(1)));
        assert_eq!(combat.reroll(9, 5, die(5)), Err(InitiativeError::NotInFight(9)));
        assert_eq!(combat.shift(9, 1), Err(InitiativeError::NotInFight(9)));
    }

    #[test]
    fn a_role_bonus_is_part_of_the_total_and_kept_for_the_tooltip() {
        let mut combat = Combat::default();
        combat.add_with_bonus(1, 6, 3, die(5)).unwrap();
        assert_eq!((combat.combatants[0].initiative, combat.combatants[0].bonus), (14, 3));
        combat.roll_with_bonus(1, 6, 2, die(9)).unwrap();
        assert_eq!((combat.combatants[0].initiative, combat.combatants[0].bonus), (17, 2));
        // A total typed in by hand carries no bonus: the table already did the sum.
        combat.set_initiative(1, 11).unwrap();
        assert_eq!(combat.combatants[0].bonus, 0);
    }

    #[test]
    fn reroll_and_manual_total_reposition_the_combatant() {
        let mut combat = fight();
        combat.reroll(1, 6, die(9)).unwrap(); // 15: now first
        assert_eq!(ids(&combat), vec![1, 2, 3, 4]);
        combat.set_initiative(1, 3).unwrap();
        assert_eq!(ids(&combat), vec![2, 3, 4, 1]);
        assert_eq!(combat.combatants[3].roll, None);
        assert_eq!(combat.set_initiative(1, 100), Err(InitiativeError::OutOfRange));
        assert_eq!(combat.set_initiative(1, -100), Err(InitiativeError::OutOfRange));
    }

    #[test]
    fn manual_shift_breaks_a_full_tie() {
        let mut combat = fight();
        combat.shift(4, -1).unwrap();
        assert_eq!(ids(&combat), vec![2, 4, 3, 1]);
        combat.shift(2, -1).unwrap(); // already first: stays
        assert_eq!(ids(&combat), vec![2, 4, 3, 1]);
        combat.shift(1, 1).unwrap(); // already last: stays
        assert_eq!(ids(&combat), vec![2, 4, 3, 1]);
    }

    fn initiatives(combat: &Combat) -> Vec<i32> {
        combat.combatants.iter().map(|c| c.initiative).collect()
    }

    #[test]
    fn move_to_keeps_the_total_when_it_fits() {
        let mut combat = fight(); // 2:14, 3:10, 4:10, 1:10
        combat.move_to(1, 1).unwrap(); // a tie broken by hand
        assert_eq!(ids(&combat), vec![2, 1, 3, 4]);
        assert_eq!(initiatives(&combat), vec![14, 10, 10, 10]);
        assert!(combat.combatants[1].roll.is_some());
    }

    #[test]
    fn dropped_on_top_the_total_goes_one_past_the_first() {
        let mut combat = fight();
        combat.move_to(1, 0).unwrap(); // an ambush
        assert_eq!(ids(&combat), vec![1, 2, 3, 4]);
        assert_eq!(initiatives(&combat), vec![15, 14, 10, 10]);
        assert_eq!(combat.combatants[0].roll, None);
    }

    #[test]
    fn dropped_between_neighbours_takes_the_nearest_bound() {
        let mut combat = fight();
        combat.move_to(2, 99).unwrap(); // the leader goes last: one below the rest
        assert_eq!(ids(&combat), vec![3, 4, 1, 2]);
        assert_eq!(initiatives(&combat), vec![10, 10, 10, 9]);
        combat.move_to(2, 1).unwrap(); // back up between 10 and 10: clamped to 10
        assert_eq!(ids(&combat), vec![3, 2, 4, 1]);
        assert_eq!(initiatives(&combat), vec![10, 10, 10, 10]);
    }

    #[test]
    fn move_to_is_checked_and_clamped() {
        let mut combat = fight();
        assert_eq!(combat.move_to(9, 0), Err(InitiativeError::NotInFight(9)));
        combat.move_to(3, 0).unwrap();
        assert_eq!(initiatives(&combat), vec![15, 14, 10, 10]);
    }

    #[test]
    fn someone_who_acted_dragged_below_the_active_one_acts_again() {
        let mut combat = fight(); // 2:14, 3:10, 4:10, 1:10
        combat.start(&[]).unwrap();
        combat.next_turn(&[]).unwrap(); // #3 acts; #2 has acted
        combat.move_to(2, 3).unwrap();
        assert_eq!(ids(&combat), vec![3, 4, 1, 2]);
        assert_eq!(combat.active, Some(3));
        for expected in [4, 1, 2] {
            combat.next_turn(&[]).unwrap();
            assert_eq!((combat.active, combat.round), (Some(expected), 1));
        }
        combat.next_turn(&[]).unwrap(); // the order wraps
        assert_eq!((combat.active, combat.round), (Some(3), 2));
    }

    #[test]
    fn someone_who_has_not_acted_dragged_above_the_active_one_is_skipped() {
        let mut combat = fight();
        combat.start(&[]).unwrap();
        combat.next_turn(&[]).unwrap(); // #3 acts
        combat.move_to(1, 0).unwrap();
        assert_eq!(ids(&combat), vec![1, 2, 3, 4]);
        assert_eq!(combat.active, Some(3));
        combat.next_turn(&[]).unwrap();
        assert_eq!((combat.active, combat.round), (Some(4), 1));
        combat.next_turn(&[]).unwrap(); // #1 would have gone now: it comes first next round
        assert_eq!((combat.active, combat.round), (Some(1), 2));
    }

    #[test]
    fn the_active_fighter_moved_later_hands_the_turn_on() {
        let mut combat = fight();
        combat.start(&[]).unwrap(); // #2 acts
        combat.move_to(2, 3).unwrap(); // a delay
        assert_eq!(ids(&combat), vec![3, 4, 1, 2]);
        assert_eq!(combat.active, Some(3));
        let mut combat = fight();
        combat.start(&[]).unwrap();
        combat.set_initiative(2, 3).unwrap(); // a lower total moves them down the same way
        assert_eq!(ids(&combat), vec![3, 4, 1, 2]);
        assert_eq!(combat.active, Some(3));
    }

    #[test]
    fn prev_turn_walks_back_and_counts_rounds_down() {
        let mut combat = fight(); // 2, 3, 4, 1
        combat.start(&[]).unwrap();
        combat.prev_turn(&[]).unwrap(); // the start of round 1: stays
        assert_eq!((combat.active, combat.round), (Some(2), 1));
        combat.next_turn(&[]).unwrap();
        combat.prev_turn(&[]).unwrap();
        assert_eq!((combat.active, combat.round), (Some(2), 1));
        for _ in 0..4 {
            combat.next_turn(&[]).unwrap();
        }
        assert_eq!((combat.active, combat.round), (Some(2), 2));
        combat.prev_turn(&[]).unwrap(); // wrapped back to the end of round 1
        assert_eq!((combat.active, combat.round), (Some(1), 1));
        combat.prev_turn(&[4]).unwrap(); // the one who cannot act is passed over
        assert_eq!((combat.active, combat.round), (Some(3), 1));
        let mut idle = Combat::default();
        assert_eq!(idle.prev_turn(&[]), Err(InitiativeError::NotStarted));
    }

    #[test]
    fn turns_walk_the_order_and_count_rounds() {
        let mut combat = fight();
        assert_eq!(combat.next_turn(&[]), Err(InitiativeError::NotStarted));
        combat.start(&[]).unwrap();
        assert_eq!((combat.round, combat.active), (1, Some(2)));
        for expected in [3, 4, 1] {
            combat.next_turn(&[]).unwrap();
            assert_eq!((combat.round, combat.active), (1, Some(expected)));
        }
        combat.next_turn(&[]).unwrap();
        assert_eq!((combat.round, combat.active), (2, Some(2)));
    }

    #[test]
    fn dead_combatants_are_skipped() {
        let mut combat = fight();
        combat.start(&[2]).unwrap();
        assert_eq!(combat.active, Some(3), "dead #2 would have been first");
        combat.next_turn(&[2]).unwrap();
        combat.next_turn(&[2]).unwrap();
        assert_eq!(combat.active, Some(1));
        combat.next_turn(&[2]).unwrap();
        assert_eq!((combat.round, combat.active), (2, Some(3)));
    }

    #[test]
    fn a_lone_survivor_keeps_the_turn_and_a_new_round_begins() {
        let mut combat = fight();
        combat.start(&[2, 3, 4]).unwrap();
        assert_eq!(combat.active, Some(1));
        combat.next_turn(&[2, 3, 4]).unwrap();
        assert_eq!((combat.round, combat.active), (2, Some(1)));
        assert_eq!(combat.next_turn(&[1, 2, 3, 4]), Err(InitiativeError::NobodyCanAct));
        assert_eq!(combat.start(&[1, 2, 3, 4]), Err(InitiativeError::NobodyCanAct));
    }

    #[test]
    fn removing_the_active_combatant_passes_the_turn() {
        let mut combat = fight();
        combat.start(&[]).unwrap();
        combat.remove(2, &[]).unwrap();
        assert_eq!((combat.round, combat.active), (1, Some(3)));
        assert_eq!(ids(&combat), vec![3, 4, 1]);
        // Removing the last in order while active wraps into the next round.
        combat.next_turn(&[]).unwrap();
        combat.next_turn(&[]).unwrap();
        assert_eq!(combat.active, Some(1));
        combat.remove(1, &[]).unwrap();
        assert_eq!((combat.round, combat.active), (2, Some(3)));
    }

    #[test]
    fn removing_someone_else_keeps_the_turn() {
        let mut combat = fight();
        combat.start(&[]).unwrap();
        combat.next_turn(&[]).unwrap();
        combat.remove(1, &[]).unwrap();
        assert_eq!(combat.active, Some(3));
    }

    #[test]
    fn removing_the_last_combatant_leaves_nobody_active() {
        let mut combat = Combat::default();
        combat.add(1, 5, die(5)).unwrap();
        combat.start(&[]).unwrap();
        combat.remove(1, &[]).unwrap();
        assert_eq!(combat.active, None);
        assert!(combat.combatants.is_empty());
    }

    #[test]
    fn joining_mid_fight_does_not_steal_the_turn() {
        let mut combat = fight();
        combat.start(&[]).unwrap();
        combat.next_turn(&[]).unwrap();
        combat.add(5, 10, check_manual(&[10, 1]).unwrap()).unwrap(); // 21: top of the order
        assert_eq!(combat.active, Some(3));
        assert_eq!(ids(&combat)[0], 5);
        combat.next_turn(&[]).unwrap();
        assert_eq!(combat.active, Some(4));
    }

    #[test]
    fn end_resets_everything() {
        let mut combat = fight();
        combat.start(&[]).unwrap();
        combat.end();
        assert_eq!(combat, Combat::default());
        assert!(!combat.is_started());
    }

    #[test]
    fn roll_and_set_or_add_work_for_newcomers_and_veterans() {
        let mut c = Combat::default();
        c.roll(1, 6, die(5)).unwrap(); // new: 6 + 5 = 11
        c.set_or_add(2, 4, 13).unwrap(); // new, typed in
        assert_eq!(ids(&c), vec![2, 1]);
        assert!(c.combatants[0].roll.is_none());

        c.roll(1, 6, die(9)).unwrap(); // existing: re-rolled to 15
        assert_eq!(ids(&c), vec![1, 2]);
        c.set_or_add(2, 4, 20).unwrap(); // existing: typed in again
        assert_eq!(ids(&c), vec![2, 1]);
        assert_eq!(c.set_or_add(3, 4, 100), Err(InitiativeError::OutOfRange));
    }
}
