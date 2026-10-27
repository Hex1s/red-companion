//! A "damage taken" event applied to a stored character and scene, and its undo.
//!
//! `damage::resolve_damage` is pure; this module wires it to the sheet and the
//! scene's cover objects. The apply step never trusts the preview: it recomputes
//! everything from the stored state (cover HP is read from the scene by id).

use serde::{Deserialize, Serialize};
use thiserror::Error;
use ts_rs::TS;

use crate::catalog::{BodyArea, CriticalInjury};
use crate::character::{ArmorSlot, Character, CharacterRecord};
use crate::damage::{
    AimedTarget, ArmorMode, DamageInput, DamageOutcome, DamageRoll, DamageStep, resolve_damage,
    roll_critical_injury,
};
use crate::dice::{DiceError, DiceNotation, DiceRng, RollResult, roll_manual};
use crate::scene::{Scene, SceneRecord};

/// What the GM enters in the "damage taken" window.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct DamageRequest {
    pub damage: DamageRoll,
    pub area: BodyArea,
    pub aimed: Option<AimedTarget>,
    pub armor_mode: ArmorMode,
    pub explosive: bool,
    pub manual_critical: bool,
    /// Cover object the target stands behind for this hit only (it does not
    /// change the character's link in the scene).
    pub cover_id: Option<u32>,
    /// Apply Solo's Damage Deflection to this hit (the UI sets it for the first hit of a round).
    #[serde(default)]
    pub deflect: bool,
}

/// Cover HP before the hit, for undo.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct CoverUndo {
    pub id: u32,
    pub hp_before: u32,
}

/// An applied hit. Keeps only the fields it touched, so undo does not
/// overwrite unrelated edits made to the sheet afterwards.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct DamageEvent {
    #[ts(type = "number")]
    pub character_id: i64,
    pub character_name: String,
    pub request: DamageRequest,
    pub hp_before: u32,
    pub sp_before: u8,
    pub death_save_penalty_before: u8,
    /// Injuries this hit added (aimed-leg one first, then the critical one).
    pub added_injuries: Vec<String>,
    pub cover: Option<CoverUndo>,
    /// This hit killed a filler NPC (0 HP); undo brings them back.
    #[serde(default)]
    pub died: bool,
    /// The hit used up the fighter's Damage Deflection of this round; this is what the mark was
    /// before, so undo can put it back.
    #[serde(default)]
    pub deflected_before: Option<Option<u32>>,
    pub outcome: DamageOutcome,
}

/// Answer to apply/undo: the stored target (if it still exists) and the scene,
/// whose `last_damage` holds the event to show or undo.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct DamageApplied {
    pub character: Option<CharacterRecord>,
    pub scene: SceneRecord,
}

#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum EventError {
    #[error("cover {0} not found")]
    CoverNotFound(u32),
    #[error("aimed shot at {0} does not match the hit area")]
    AimMismatch(&'static str),
    #[error(transparent)]
    Dice(#[from] DiceError),
    #[error("injury {0:?} not found")]
    UnknownInjury(String),
    #[error("injury {0:?} belongs to another body area")]
    WrongInjuryArea(String),
    #[error("the target already has injury {0:?}")]
    InjuryOwned(String),
    #[error("this hit causes no critical injury")]
    NoCritical,
    #[error("no free injury left in this table")]
    NoFreeInjury,
}

/// Dice of a rolled damage must be consistent: rebuild the total from the
/// faces (keeping where they came from).
fn normalized(damage: &DamageRoll) -> Result<DamageRoll, EventError> {
    Ok(match damage {
        DamageRoll::Total { value } => DamageRoll::Total { value: *value },
        DamageRoll::Rolled { roll } => {
            let rebuilt = roll_manual(roll.notation, &roll.dice)?;
            DamageRoll::Rolled { roll: RollResult { source: roll.source, ..rebuilt } }
        }
    })
}

fn check_aim(req: &DamageRequest) -> Result<(), EventError> {
    match (req.aimed, req.area) {
        (None, _)
        | (Some(AimedTarget::Head), BodyArea::Head)
        | (Some(AimedTarget::HeldItem | AimedTarget::Leg), BodyArea::Body) => Ok(()),
        (Some(AimedTarget::Head), _) => Err(EventError::AimMismatch("head")),
        (Some(AimedTarget::HeldItem), _) => Err(EventError::AimMismatch("held_item")),
        (Some(AimedTarget::Leg), _) => Err(EventError::AimMismatch("leg")),
    }
}

/// Calculates a hit on the stored state without changing anything.
pub fn evaluate(
    character: &Character,
    scene: &Scene,
    req: &DamageRequest,
) -> Result<DamageOutcome, EventError> {
    check_aim(req)?;
    let cover_hp = match req.cover_id {
        Some(id) => Some(scene.cover(id).ok_or(EventError::CoverNotFound(id))?.hp),
        None => None,
    };
    let input = DamageInput {
        damage: normalized(&req.damage)?,
        area: req.area,
        aimed: req.aimed,
        armor_mode: req.armor_mode,
        explosive: req.explosive,
        manual_critical: req.manual_critical,
        cover_hp,
        deflection: if req.deflect { crate::roles::damage_deflection(character) } else { 0 },
    };
    Ok(resolve_damage(&input, &character.target_state()))
}

/// What the "damage taken" window needs to offer Solo's Damage Deflection.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct DeflectionInfo {
    /// How much the first damage of a round is reduced; 0 = the character has no deflection.
    pub amount: u32,
    /// The fight is on and this fighter has not had a damage reduced this round: the window
    /// ticks the box by itself. Outside a fight the GM ticks it by hand.
    pub first_this_round: bool,
}

pub fn deflection_info(character: &Character, character_id: i64, scene: &Scene) -> DeflectionInfo {
    DeflectionInfo {
        amount: crate::roles::damage_deflection(character),
        first_this_round: scene.combat.first_damage_this_round(character_id),
    }
}

fn slot_mut(character: &mut Character, area: BodyArea) -> &mut ArmorSlot {
    match area {
        BodyArea::Head => &mut character.armor_head,
        BodyArea::Body => &mut character.armor_body,
    }
}

/// Applies a hit to the character and scene and records it as the scene's last
/// event. `injury_id` is the critical injury picked or rolled by the GM; it is
/// allowed only when the recomputed outcome is critical.
pub fn apply(
    character: &mut Character,
    character_id: i64,
    scene: &mut Scene,
    req: &DamageRequest,
    injury_id: Option<&str>,
    table: &[CriticalInjury],
) -> Result<DamageEvent, EventError> {
    let outcome = evaluate(character, scene, req)?;

    let chosen = match injury_id {
        None => None,
        Some(id) => {
            if !outcome.critical {
                return Err(EventError::NoCritical);
            }
            let injury = table
                .iter()
                .find(|i| i.id == id)
                .ok_or_else(|| EventError::UnknownInjury(id.to_string()))?;
            if injury.area != req.area {
                return Err(EventError::WrongInjuryArea(id.to_string()));
            }
            // A chosen injury may repeat one the target already has (a second leg, a third arm);
            // only the random roll avoids those. The forced one (aimed leg...) is not chosen twice.
            if outcome.forced_injury.as_deref() == Some(id) {
                return Err(EventError::InjuryOwned(id.to_string()));
            }
            Some(injury)
        }
    };

    let area = req.area;
    let hp_before = character.hp;
    let sp_before = slot_mut(character, area).sp;
    let death_save_penalty_before = character.death_save_penalty;
    let cover = req
        .cover_id
        .and_then(|id| scene.cover(id))
        .map(|c| CoverUndo { id: c.id, hp_before: c.hp });

    character.hp = outcome.new_hp;
    slot_mut(character, area).sp = outcome.sp_after;

    // Two sources of death-save penalty: a hit on a mortal wound, and injuries
    // whose table row raises it.
    let mut penalty = u32::from(outcome.death_save_penalty_increase);
    let mut added_injuries = Vec::new();
    let forced = outcome.forced_injury.as_deref();
    for id in forced.into_iter().chain(chosen.map(|i| i.id.as_str())) {
        if table.iter().any(|i| i.id == id && i.death_save_penalty) {
            penalty += 1;
        }
        character.injuries.push(id.to_string());
        added_injuries.push(id.to_string());
    }
    character.death_save_penalty =
        u8::try_from(u32::from(character.death_save_penalty) + penalty).unwrap_or(u8::MAX);

    if let (Some(undo), Some(hp)) = (cover, outcome.cover_hp_after) {
        scene.set_hp(undo.id, hp).map_err(|_| EventError::CoverNotFound(undo.id))?;
    }

    // A hit that really was reduced uses up this round's deflection of a fighter.
    let mut deflected_before = None;
    let reduced = outcome.steps.iter().any(|s| matches!(s, DamageStep::Deflection { .. }));
    if reduced && scene.combat.round > 0 && scene.combat.is_in_fight(character_id) {
        deflected_before = Some(scene.combat.deflected.get(&character_id).copied());
        let round = scene.combat.round;
        scene.combat.deflected.insert(character_id, round);
    }

    let event = DamageEvent {
        character_id,
        character_name: character.name.clone(),
        request: req.clone(),
        hp_before,
        sp_before,
        death_save_penalty_before,
        added_injuries,
        cover,
        died: false,
        deflected_before,
        outcome,
    };
    scene.last_damage = Some(event.clone());
    Ok(event)
}

/// Filler NPCs have no death save: when a hit leaves one at 0 HP, they die on the spot. Marks the
/// scene's last event so that undo revives them. Call it for NPCs only.
pub fn kill_downed_npc(character: &mut Character, scene: &mut Scene) {
    if character.hp == 0 && !character.dead {
        character.dead = true;
        if let Some(event) = scene.last_damage.as_mut() {
            event.died = true;
        }
    }
}

/// Rolls back what `event` changed on the character. HP and SP are clamped to
/// the sheet's current maximums in case it was edited since.
pub fn undo_character(character: &mut Character, event: &DamageEvent) {
    if event.died {
        character.dead = false;
    }
    character.hp = event.hp_before.min(character.max_hp());
    let slot = slot_mut(character, event.request.area);
    slot.sp = event.sp_before.min(slot.max_sp);
    // One instance per injury the hit added; instances the target already had stay.
    for id in &event.added_injuries {
        character.take_injury(id);
    }
    character.death_save_penalty = event.death_save_penalty_before;
}

/// Restores the cover HP and clears the scene's last event. A cover removed in
/// the meantime is skipped.
pub fn undo_scene(scene: &mut Scene, event: &DamageEvent) {
    if let Some(before) = event.deflected_before {
        match before {
            Some(round) => {
                scene.combat.deflected.insert(event.character_id, round);
            }
            None => {
                scene.combat.deflected.remove(&event.character_id);
            }
        }
    }
    if let Some(cover) = event.cover {
        let _ = scene.set_hp(cover.id, cover.hp_before);
    }
    scene.last_damage = None;
}

/// Result of the 2d6 injury roll.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct InjuryRoll {
    pub injury_id: String,
    pub roll: RollResult,
}

/// Rolls the injury table of `area`. With `manual` faces (two d6 from physical
/// dice) the row is taken as is: if the target already has it, the book says
/// to roll again, so that is an error the GM sees. Random rolls re-roll on
/// their own.
pub fn roll_injury(
    table: &[CriticalInjury],
    area: BodyArea,
    owned: &[String],
    manual: Option<&[u32]>,
    rng: &mut impl DiceRng,
) -> Result<InjuryRoll, EventError> {
    match manual {
        Some(faces) => {
            let notation = DiceNotation { count: 2, sides: 6, modifier: 0 };
            let result = roll_manual(notation, faces)?;
            let row = table
                .iter()
                .find(|i| i.area == area && i32::from(i.roll) == result.total)
                .ok_or(EventError::NoFreeInjury)?;
            if owned.contains(&row.id) {
                return Err(EventError::InjuryOwned(row.id.clone()));
            }
            Ok(InjuryRoll { injury_id: row.id.clone(), roll: result })
        }
        None => {
            let (injury, result) =
                roll_critical_injury(table, area, owned, rng).ok_or(EventError::NoFreeInjury)?;
            Ok(InjuryRoll { injury_id: injury.id.clone(), roll: result })
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::character::ArmorSlot;
    use crate::data_source::{DataSource, SeedDataSource};
    use crate::dice::{RollSource, ScriptedDice};
    use crate::scene::cover_presets;

    fn table() -> Vec<CriticalInjury> {
        SeedDataSource::default().critical_injuries().unwrap().entries
    }

    fn character() -> Character {
        let mut c = Character::quick("Test");
        c.armor_body = ArmorSlot { sp: 11, max_sp: 11 };
        c.armor_head = ArmorSlot { sp: 7, max_sp: 7 };
        c
    }

    fn request(value: u32) -> DamageRequest {
        DamageRequest {
            damage: DamageRoll::Total { value },
            area: BodyArea::Body,
            aimed: None,
            armor_mode: ArmorMode::Full,
            explosive: false,
            manual_critical: false,
            cover_id: None,
            deflect: false,
        }
    }

    fn scene_with_concrete() -> Scene {
        let mut scene = Scene::default();
        let presets = cover_presets(&SeedDataSource::default()).unwrap();
        let concrete = presets.iter().find(|p| p.hp == Some(25)).expect("25 HP preset");
        scene.add_cover(Some(concrete), "", None).unwrap();
        scene
    }

    #[test]
    fn apply_then_undo_restores_the_exact_state() {
        let original = character();
        let mut c = original.clone();
        let mut scene = scene_with_concrete();
        let mut req = request(20);
        req.manual_critical = true;
        req.cover_id = Some(1);
        // Cover (25 HP) swallows everything: nothing else changes but the cover.
        apply(&mut c, 1, &mut scene, &req, None, &table()).unwrap();
        assert_eq!(scene.covers[0].hp, 5);
        assert_eq!(c, original);

        req.cover_id = None;
        req.manual_critical = true;
        let injury = table().into_iter().find(|i| i.area == BodyArea::Body).unwrap();
        let event2 = apply(&mut c, 1, &mut scene, &req, Some(&injury.id), &table()).unwrap();
        assert_eq!(c.hp, original.hp - (20 - 11) - 5);
        assert_eq!(c.armor_body.sp, 10);
        assert!(c.injuries.contains(&injury.id));

        undo_character(&mut c, &event2);
        undo_scene(&mut scene, &event2);
        assert_eq!(c, original);
        assert!(scene.last_damage.is_none());
    }

    #[test]
    fn undo_restores_cover_hp() {
        let mut c = character();
        let mut scene = scene_with_concrete();
        let mut req = request(17);
        req.cover_id = Some(1);
        let event = apply(&mut c, 1, &mut scene, &req, None, &table()).unwrap();
        assert_eq!(scene.covers[0].hp, 8);
        undo_character(&mut c, &event);
        undo_scene(&mut scene, &event);
        assert_eq!(scene.covers[0].hp, 25);
    }

    #[test]
    fn a_chosen_injury_may_repeat_one_the_target_already_has() {
        let mut c = character();
        let injury = table().into_iter().find(|i| i.area == BodyArea::Body).unwrap();
        c.injuries.push(injury.id.clone());
        let mut scene = Scene::default();
        let mut req = request(20);
        req.manual_critical = true;
        let event = apply(&mut c, 1, &mut scene, &req, Some(&injury.id), &table()).unwrap();
        assert_eq!(c.injury_count(&injury.id), 2);
        // Undo takes back only what the hit added.
        undo_character(&mut c, &event);
        assert_eq!(c.injury_count(&injury.id), 1);
    }

    #[test]
    fn injury_without_critical_or_in_wrong_area_is_rejected() {
        let mut c = character();
        let mut scene = Scene::default();
        let body = table().into_iter().find(|i| i.area == BodyArea::Body).unwrap();
        let head = table().into_iter().find(|i| i.area == BodyArea::Head).unwrap();
        let err = apply(&mut c, 1, &mut scene, &request(20), Some(&body.id), &table()).unwrap_err();
        assert_eq!(err, EventError::NoCritical);
        let mut req = request(20);
        req.manual_critical = true;
        let err = apply(&mut c, 1, &mut scene, &req, Some(&head.id), &table()).unwrap_err();
        assert_eq!(err, EventError::WrongInjuryArea(head.id));
        // Nothing was changed by the failed attempts.
        assert_eq!(c, character());
    }

    #[test]
    fn aimed_leg_adds_broken_leg_and_undo_removes_it() {
        let original = character();
        let mut c = original.clone();
        let mut scene = Scene::default();
        let mut req = request(12);
        req.aimed = Some(AimedTarget::Leg);
        let event = apply(&mut c, 1, &mut scene, &req, None, &table()).unwrap();
        assert_eq!(c.injuries, vec!["broken_leg".to_string()]);
        undo_character(&mut c, &event);
        assert_eq!(c, original);
    }

    #[test]
    fn mortal_hit_and_penalty_injury_both_raise_death_save_penalty() {
        let mut c = character();
        c.hp = 0;
        c.armor_body = ArmorSlot::NONE;
        let mut scene = Scene::default();
        let penalty_injury =
            table().into_iter().find(|i| i.area == BodyArea::Body && i.death_save_penalty);
        let Some(injury) = penalty_injury else { return };
        let event = apply(&mut c, 1, &mut scene, &request(3), Some(&injury.id), &table()).unwrap();
        assert_eq!(c.death_save_penalty, 2);
        undo_character(&mut c, &event);
        assert_eq!(c.death_save_penalty, 0);
    }

    #[test]
    fn apply_ignores_client_cover_hp_and_checks_dice() {
        let mut c = character();
        let mut scene = scene_with_concrete();
        let mut req = request(1);
        req.cover_id = Some(9);
        assert_eq!(
            apply(&mut c, 1, &mut scene, &req, None, &table()).unwrap_err(),
            EventError::CoverNotFound(9)
        );
        // A forged total is recomputed from the faces and the source is kept.
        let mut forged = roll_manual("2d6".parse().unwrap(), &[3, 4]).unwrap();
        forged.total = 99;
        let req = DamageRequest { damage: DamageRoll::Rolled { roll: forged }, ..request(0) };
        let out = evaluate(&c, &scene, &req).unwrap();
        assert_eq!(out.hp_lost, 0); // 7 vs SP 11
        let event = apply(&mut c, 1, &mut scene, &req, None, &table()).unwrap();
        match event.request.damage {
            DamageRoll::Rolled { roll } => assert_eq!(roll.source, RollSource::Manual),
            DamageRoll::Total { .. } => unreachable!(),
        }
    }

    #[test]
    fn manual_injury_roll_on_owned_row_is_an_error() {
        let t = table();
        let row = t.iter().find(|i| i.area == BodyArea::Body && i.roll == 7).unwrap();
        let mut rng = ScriptedDice::new([]);
        let ok = roll_injury(&t, BodyArea::Body, &[], Some(&[3, 4]), &mut rng).unwrap();
        assert_eq!(ok.injury_id, row.id);
        let err = roll_injury(&t, BodyArea::Body, std::slice::from_ref(&row.id), Some(&[3, 4]), &mut rng)
            .unwrap_err();
        assert_eq!(err, EventError::InjuryOwned(row.id.clone()));
        assert!(roll_injury(&t, BodyArea::Body, &[], Some(&[3]), &mut rng).is_err());
    }

    #[test]
    fn a_downed_npc_dies_and_undo_brings_them_back() {
        let mut c = Character::quick("Boosterganger");
        c.hp = 5;
        let mut scene = Scene::default();
        let event = apply(&mut c, 1, &mut scene, &request(30), None, &table()).unwrap();
        assert_eq!(c.hp, 0);
        kill_downed_npc(&mut c, &mut scene);
        assert!(c.dead);
        assert!(scene.last_damage.as_ref().unwrap().died);

        undo_character(&mut c, scene.last_damage.as_ref().unwrap());
        assert_eq!((c.hp, c.dead), (event.hp_before, false));

        // A hit that leaves HP does not kill, and does not mark the event.
        let mut c = Character::quick("Boosterganger");
        let mut scene = Scene::default();
        apply(&mut c, 1, &mut scene, &request(8), None, &table()).unwrap();
        kill_downed_npc(&mut c, &mut scene);
        assert!(!c.dead && !scene.last_damage.unwrap().died);
    }

    fn soloist(deflection_points: u8) -> Character {
        let mut c = character();
        let mut e = crate::roles::RoleEntry::new(crate::roles::SOLO, 6);
        e.points.insert(crate::roles::DAMAGE_DEFLECTION.into(), deflection_points);
        c.roles.push(e);
        c
    }

    fn fight_with(id: i64) -> Scene {
        let mut scene = Scene::default();
        let die = crate::dice::check_manual(&[5]).unwrap();
        scene.combat.add(id, 6, die).unwrap();
        scene.combat.start(&[]).unwrap();
        scene
    }

    #[test]
    fn deflection_takes_off_what_passed_the_armor_and_is_used_once_per_round() {
        // SP 11 body (see `character()`), 20 damage: 9 pass, 4 points of deflection = -2.
        let mut c = soloist(4);
        let mut scene = fight_with(1);
        let req = DamageRequest { deflect: true, ..request(20) };
        let outcome = evaluate(&c, &scene, &req).unwrap();
        assert_eq!(outcome.hp_lost, 7);
        let event = apply(&mut c, 1, &mut scene, &req, None, &table()).unwrap();
        assert_eq!(event.outcome.hp_lost, 7);
        assert_eq!(scene.combat.deflected.get(&1), Some(&1));
        assert!(!scene.combat.first_damage_this_round(1), "this round's deflection is spent");
        // A new round makes it available again.
        scene.combat.next_turn(&[]).unwrap();
        assert!(scene.combat.first_damage_this_round(1));
        // Undo puts the mark back: the round counts as unused.
        let mut scene = fight_with(1);
        let mut c = soloist(4);
        let event = apply(&mut c, 1, &mut scene, &req, None, &table()).unwrap();
        undo_scene(&mut scene, &event);
        assert!(scene.combat.first_damage_this_round(1));
    }

    #[test]
    fn deflection_never_goes_below_zero_does_not_touch_armor_wear_and_skips_the_critical_bonus() {
        let c = soloist(10); // -5
        let scene = fight_with(1);
        // 13 vs SP 11: 2 pass, -5 -> 0; the armor still wore (damage did get through).
        let outcome = evaluate(&c, &scene, &DamageRequest { deflect: true, ..request(13) }).unwrap();
        assert_eq!((outcome.hp_lost, outcome.sp_after), (0, 10));
        // With a critical, the +5 is not reduced.
        let crit = DamageRequest { deflect: true, manual_critical: true, ..request(13) };
        assert_eq!(evaluate(&c, &scene, &crit).unwrap().hp_lost, 5);
        // Switched off in the request: nothing is reduced.
        assert_eq!(evaluate(&c, &scene, &request(13)).unwrap().hp_lost, 2);
    }

    #[test]
    fn a_hit_that_deflection_did_not_reduce_does_not_spend_it() {
        let mut c = soloist(4);
        let mut scene = fight_with(1);
        // The armor stops everything: nothing to reduce, the round's deflection stays.
        let req = DamageRequest { deflect: true, ..request(5) };
        apply(&mut c, 1, &mut scene, &req, None, &table()).unwrap();
        assert!(scene.combat.first_damage_this_round(1));
    }
}
