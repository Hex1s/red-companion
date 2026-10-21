//! "Damage taken" resolution: cover, armor, ablation, head multiplier, critical
//! injuries, wound states and death saves (book p. 182-188).
//!
//! The calculator knows nothing about hit chance or range: the GM has already
//! decided the attack landed. It returns a step log so the UI can answer
//! "why did I lose X".

use serde::{Deserialize, Serialize};
use ts_rs::TS;

use crate::catalog::{BodyArea, CriticalInjury};
use crate::dice::{roll, DiceNotation, DiceRng, RollResult};

/// Flat damage added by every critical injury, ignores armor (p. 187).
pub const CRITICAL_BONUS_DAMAGE: u32 = 5;
/// Id of the head injury that triples aimed head-shot damage (p. 188).
pub const CRACKED_SKULL_ID: &str = "cracked_skull";

/// How the target's armor treats this damage.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, TS)]
#[ts(export)]
#[serde(rename_all = "snake_case")]
pub enum ArmorMode {
    /// Ranged weapons: full SP.
    Full,
    /// Melee and martial arts: half SP, rounded up (p. 175).
    HalfUp,
    /// Fire, poison, suffocation and the like: armor is skipped (p. 186).
    Ignored,
}

/// Where the damage number comes from.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[ts(export)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum DamageRoll {
    /// A plain number typed by the GM; a critical must be flagged by hand.
    Total { value: u32 },
    /// Dice rolled (or entered from physical dice); two 6s means a critical.
    Rolled { roll: RollResult },
}

impl DamageRoll {
    pub fn value(&self) -> u32 {
        match self {
            Self::Total { value } => *value,
            Self::Rolled { roll } => roll.total.max(0) as u32,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct DamageInput {
    pub damage: DamageRoll,
    pub area: BodyArea,
    /// Aimed shot at the head doubles the damage that gets through armor.
    pub aimed_shot: bool,
    pub armor_mode: ArmorMode,
    /// Explosions: when they destroy the cover, the target takes full damage.
    pub explosive: bool,
    /// Critical flag for typed-in damage (dice detect it on their own).
    pub manual_critical: bool,
    /// Current HP of the cover the target stands behind, if any.
    pub cover_hp: Option<u32>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct TargetState {
    pub hp: u32,
    pub max_hp: u32,
    pub sp_head: u8,
    pub sp_body: u8,
    /// Ids of critical injuries the target already has.
    pub injuries: Vec<String>,
}

/// One line of the "why" log. Numbers only: the UI turns them into text.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[ts(export)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum DamageStep {
    Damage { value: u32, critical_dice: bool },
    CoverAbsorbed { absorbed: u32, hp_before: u32, hp_after: u32, destroyed: bool },
    /// Damage left over after the cover broke; it does no harm (p. 182).
    CoverOverflowDissipated { amount: u32 },
    /// Explosion that destroyed the cover: the target takes it in full (p. 174).
    CoverDestroyedByExplosion { damage: u32 },
    Armor { mode: ArmorMode, sp: u8, effective_sp: u8 },
    Penetrated { damage: u32, effective_sp: u8, through: u32 },
    Ablation { sp_before: u8, sp_after: u8 },
    AimedHeadMultiplier { factor: u32, before: u32, after: u32 },
    /// Hit on a mortally wounded target causes a critical injury (p. 186).
    MortalWoundCritical,
    CriticalBonus { amount: u32 },
    HpLost { before: u32, after: u32 },
    WoundStateChanged { from: WoundState, to: WoundState },
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct DamageOutcome {
    pub hp_lost: u32,
    pub new_hp: u32,
    /// SP of the struck area after ablation.
    pub sp_after: u8,
    pub cover_hp_after: Option<u32>,
    pub wound_before: WoundState,
    pub wound_after: WoundState,
    /// A critical injury must be rolled or picked (its +5 damage is already
    /// included in `hp_lost`).
    pub critical: bool,
    /// Added to the target's death save penalty (hit on a mortal wound).
    pub death_save_penalty_increase: u8,
    pub steps: Vec<DamageStep>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, TS)]
#[ts(export)]
#[serde(rename_all = "snake_case")]
pub enum WoundState {
    Healthy,
    Light,
    Serious,
    Mortal,
    Dead,
}

impl WoundState {
    /// Penalty to all actions (p. 186).
    pub fn action_penalty(self) -> u8 {
        match self {
            Self::Healthy | Self::Light => 0,
            Self::Serious => 2,
            Self::Mortal | Self::Dead => 4,
        }
    }

    /// Extra penalty to MOVE (minimum MOVE is 1).
    pub fn move_penalty(self) -> u8 {
        match self {
            Self::Mortal | Self::Dead => 6,
            _ => 0,
        }
    }

    /// DV of stabilisation (p. 186); `None` when it is impossible.
    pub fn stabilization_dv(self) -> Option<u8> {
        match self {
            Self::Healthy => None,
            Self::Light => Some(10),
            Self::Serious => Some(13),
            Self::Mortal => Some(15),
            Self::Dead => None,
        }
    }
}

/// Wound state by remaining HP. `dead` comes from a failed death save, not HP.
pub fn wound_state(hp: u32, max_hp: u32, dead: bool) -> WoundState {
    if dead {
        WoundState::Dead
    } else if hp == 0 {
        WoundState::Mortal
    } else if hp >= max_hp {
        WoundState::Healthy
    } else if hp < max_hp.div_ceil(2) {
        WoundState::Serious
    } else {
        WoundState::Light
    }
}

/// HP = 10 + 5 x ceil((BODY + WILL) / 2) (p. 79).
pub fn max_hp(body: u8, will: u8) -> u32 {
    10 + 5 * (u32::from(body) + u32::from(will)).div_ceil(2)
}

/// Apply one damage event to a target. Pure: returns the new numbers and the
/// log, the caller stores them (and keeps the old state to undo).
pub fn resolve_damage(input: &DamageInput, target: &TargetState) -> DamageOutcome {
    let mut steps = Vec::new();
    let sp = match input.area {
        BodyArea::Head => target.sp_head,
        BodyArea::Body => target.sp_body,
    };
    let wound_before = wound_state(target.hp, target.max_hp, false);

    let damage = input.damage.value();
    let critical_dice = matches!(&input.damage, DamageRoll::Rolled { roll } if roll.is_critical());
    steps.push(DamageStep::Damage { value: damage, critical_dice });

    // 1. Cover. A destroyed cover (0 HP) is no cover at all.
    let mut cover_hp_after = input.cover_hp;
    let mut to_target = damage;
    if let Some(cover) = input.cover_hp.filter(|&hp| hp > 0) {
        let absorbed = damage.min(cover);
        let hp_after = cover - absorbed;
        let destroyed = hp_after == 0;
        cover_hp_after = Some(hp_after);
        steps.push(DamageStep::CoverAbsorbed { absorbed, hp_before: cover, hp_after, destroyed });
        if input.explosive && destroyed {
            steps.push(DamageStep::CoverDestroyedByExplosion { damage });
        } else {
            if damage > absorbed {
                steps.push(DamageStep::CoverOverflowDissipated { amount: damage - absorbed });
            }
            to_target = 0;
        }
    }

    // Nothing reached the target: no armor wear, no critical.
    if to_target == 0 {
        return DamageOutcome {
            hp_lost: 0,
            new_hp: target.hp,
            sp_after: sp,
            cover_hp_after,
            wound_before,
            wound_after: wound_before,
            critical: false,
            death_save_penalty_increase: 0,
            steps,
        };
    }

    // 2-3. Armor: best SP of the area is already in `sp` (armor never stacks).
    let effective_sp = match input.armor_mode {
        ArmorMode::Full => sp,
        ArmorMode::HalfUp => sp.div_ceil(2),
        ArmorMode::Ignored => 0,
    };
    steps.push(DamageStep::Armor { mode: input.armor_mode, sp, effective_sp });
    let mut through = to_target.saturating_sub(u32::from(effective_sp));
    steps.push(DamageStep::Penetrated { damage: to_target, effective_sp, through });

    // Armor wears only when damage got through and the armor was not skipped.
    let mut sp_after = sp;
    if through > 0 && input.armor_mode != ArmorMode::Ignored && sp > 0 {
        sp_after = sp - 1;
        steps.push(DamageStep::Ablation { sp_before: sp, sp_after });
    }

    // 4. Aimed head shot multiplies only what passed the armor.
    if input.area == BodyArea::Head && input.aimed_shot && through > 0 {
        let factor = if target.injuries.iter().any(|i| i == CRACKED_SKULL_ID) { 3 } else { 2 };
        let after = through * factor;
        steps.push(DamageStep::AimedHeadMultiplier { factor, before: through, after });
        through = after;
    }

    // 5. Critical: two 6s / manual flag / any penetrating hit on a mortal wound.
    let mortal_hit = wound_before == WoundState::Mortal && through > 0;
    if mortal_hit {
        steps.push(DamageStep::MortalWoundCritical);
    }
    let critical = critical_dice || input.manual_critical || mortal_hit;
    let mut hp_lost = through;
    if critical {
        hp_lost += CRITICAL_BONUS_DAMAGE;
        steps.push(DamageStep::CriticalBonus { amount: CRITICAL_BONUS_DAMAGE });
    }

    // 6. HP and wound state.
    let new_hp = target.hp.saturating_sub(hp_lost);
    steps.push(DamageStep::HpLost { before: target.hp, after: new_hp });
    let wound_after = wound_state(new_hp, target.max_hp, false);
    if wound_after != wound_before {
        steps.push(DamageStep::WoundStateChanged { from: wound_before, to: wound_after });
    }

    DamageOutcome {
        hp_lost,
        new_hp,
        sp_after,
        cover_hp_after,
        wound_before,
        wound_after,
        critical,
        death_save_penalty_increase: u8::from(mortal_hit),
        steps,
    }
}

/// Roll 2d6 on the injury table of `area` until it gives an injury the target
/// does not have yet (p. 187). `None` when the target already has them all.
pub fn roll_critical_injury<'a>(
    table: &'a [CriticalInjury],
    area: BodyArea,
    owned: &[String],
    rng: &mut impl DiceRng,
) -> Option<(&'a CriticalInjury, RollResult)> {
    let free = |i: &&CriticalInjury| i.area == area && !owned.contains(&i.id);
    table.iter().find(free)?;
    let notation = DiceNotation { count: 2, sides: 6, modifier: 0 };
    // Bounded so a stuck RNG cannot hang the app.
    for _ in 0..10_000 {
        let result = roll(notation, rng);
        let row = table.iter().find(|i| i.area == area && i.roll as i32 == result.total);
        if let Some(injury) = row.filter(|i| !owned.contains(&i.id)) {
            return Some((injury, result));
        }
    }
    None
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct DeathSave {
    pub roll: RollResult,
    pub penalty: u8,
    pub passed: bool,
    /// Penalty for the next save: grows by 1 after every save (p. 188).
    pub next_penalty: u8,
}

/// Death save: d10 + penalty must not exceed BODY; a 10 always fails. A fail
/// means the character is dead. `roll` is a 1d10 result (random or manual).
pub fn death_save(body: u8, penalty: u8, roll: RollResult) -> DeathSave {
    let face = roll.dice.first().copied().unwrap_or(0);
    let passed = face != 10 && face + u32::from(penalty) <= u32::from(body);
    DeathSave { roll, penalty, passed, next_penalty: penalty.saturating_add(1) }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::data_source::{DataSource, SeedDataSource};
    use crate::dice::{roll_manual, ScriptedDice};

    fn dice(s: &str, faces: &[u32]) -> RollResult {
        roll_manual(s.parse().unwrap(), faces).unwrap()
    }

    fn target(hp: u32, sp_head: u8, sp_body: u8) -> TargetState {
        TargetState { hp, max_hp: 40, sp_head, sp_body, injuries: vec![] }
    }

    fn hit(value: u32, mode: ArmorMode) -> DamageInput {
        DamageInput {
            damage: DamageRoll::Total { value },
            area: BodyArea::Body,
            aimed_shot: false,
            armor_mode: mode,
            explosive: false,
            manual_critical: false,
            cover_hp: None,
        }
    }

    // Melee example, p. 175: light armor SP 11 counts as 6 against melee, a
    // hit for 7 does 1 damage and the armor drops to 10.
    #[test]
    fn melee_halves_armor_rounded_up_and_ablates_by_one_p175() {
        let out = resolve_damage(&hit(7, ArmorMode::HalfUp), &target(40, 0, 11));
        assert_eq!(out.hp_lost, 1);
        assert_eq!(out.new_hp, 39);
        assert_eq!(out.sp_after, 10);
        assert!(out.steps.contains(&DamageStep::Armor {
            mode: ArmorMode::HalfUp,
            sp: 11,
            effective_sp: 6
        }));
        assert!(out.steps.contains(&DamageStep::Penetrated {
            damage: 7,
            effective_sp: 6,
            through: 1
        }));
    }

    // Cover example, p. 182: concrete barricade 25 HP, a 17-damage shot leaves 8.
    // The second shot's damage is not in the book: 9 is our own value.
    #[test]
    fn cover_absorbs_and_is_destroyed_overflow_dissipates_p182() {
        let mut input = hit(17, ArmorMode::Full);
        input.cover_hp = Some(25);
        let out = resolve_damage(&input, &target(40, 0, 7));
        assert_eq!(out.cover_hp_after, Some(8));
        assert_eq!((out.hp_lost, out.sp_after), (0, 7));

        input.damage = DamageRoll::Total { value: 9 };
        input.cover_hp = out.cover_hp_after;
        let out = resolve_damage(&input, &target(40, 0, 7));
        assert_eq!(out.cover_hp_after, Some(0));
        assert_eq!((out.hp_lost, out.sp_after), (0, 7));
        assert!(out.steps.contains(&DamageStep::CoverOverflowDissipated { amount: 1 }));
    }

    #[test]
    fn destroyed_cover_no_longer_protects() {
        let mut input = hit(5, ArmorMode::Full);
        input.cover_hp = Some(0);
        assert_eq!(resolve_damage(&input, &target(40, 0, 0)).hp_lost, 5);
    }

    // Explosions, p. 174: too weak to break the cover = no damage; strong
    // enough = the target takes the full damage.
    #[test]
    fn explosion_hurts_only_when_it_destroys_cover_p174() {
        let mut input = hit(10, ArmorMode::Full);
        input.explosive = true;
        input.cover_hp = Some(15);
        let out = resolve_damage(&input, &target(40, 0, 0));
        assert_eq!((out.hp_lost, out.cover_hp_after), (0, Some(5)));

        input.cover_hp = Some(10);
        let out = resolve_damage(&input, &target(40, 0, 0));
        assert_eq!((out.hp_lost, out.cover_hp_after), (10, Some(0)));
    }

    #[test]
    fn armor_wears_only_when_damage_penetrates_p186() {
        let out = resolve_damage(&hit(7, ArmorMode::Full), &target(40, 0, 7));
        assert_eq!((out.hp_lost, out.sp_after), (0, 7));
        let out = resolve_damage(&hit(8, ArmorMode::Full), &target(40, 0, 7));
        assert_eq!((out.hp_lost, out.sp_after), (1, 6));
    }

    #[test]
    fn ignored_armor_does_full_damage_and_no_wear_p186() {
        let out = resolve_damage(&hit(4, ArmorMode::Ignored), &target(40, 0, 11));
        assert_eq!((out.hp_lost, out.sp_after), (4, 11));
    }

    #[test]
    fn head_uses_head_sp_and_aimed_shot_doubles_what_passed_p186() {
        let mut input = hit(20, ArmorMode::Full);
        input.area = BodyArea::Head;
        input.aimed_shot = true;
        let out = resolve_damage(&input, &target(40, 11, 4));
        assert_eq!(out.hp_lost, (20 - 11) * 2);
        assert_eq!((out.sp_after, out.new_hp), (10, 22));

        input.aimed_shot = false;
        assert_eq!(resolve_damage(&input, &target(40, 11, 4)).hp_lost, 9);
    }

    #[test]
    fn cracked_skull_triples_aimed_head_shots_p188() {
        let mut input = hit(14, ArmorMode::Full);
        input.area = BodyArea::Head;
        input.aimed_shot = true;
        let mut t = target(40, 11, 0);
        t.injuries.push(CRACKED_SKULL_ID.to_string());
        assert_eq!(resolve_damage(&input, &t).hp_lost, 3 * 3);
    }

    // p. 187: two or more 6s = critical; +5 ignores armor and is not doubled.
    #[test]
    fn two_sixes_add_five_damage_even_when_armor_holds_p187() {
        let mut input = hit(0, ArmorMode::Full);
        input.damage = DamageRoll::Rolled { roll: dice("3d6", &[6, 6, 1]) };
        let out = resolve_damage(&input, &target(40, 0, 11));
        assert!(out.critical);
        assert_eq!(out.hp_lost, 2 + 5);
        assert_eq!(out.sp_after, 10);

        let mut input = hit(0, ArmorMode::Full);
        input.damage = DamageRoll::Rolled { roll: dice("2d6", &[6, 2]) };
        assert!(!resolve_damage(&input, &target(40, 0, 11)).critical);

        let mut input = hit(3, ArmorMode::Full);
        input.manual_critical = true;
        let out = resolve_damage(&input, &target(40, 0, 11));
        assert_eq!((out.critical, out.hp_lost, out.sp_after), (true, 5, 11));
    }

    #[test]
    fn critical_bonus_is_not_doubled_on_aimed_head_shot_p187() {
        let mut input = hit(13, ArmorMode::Full);
        input.area = BodyArea::Head;
        input.aimed_shot = true;
        input.manual_critical = true;
        assert_eq!(resolve_damage(&input, &target(40, 11, 0)).hp_lost, 2 * 2 + 5);
    }

    #[test]
    fn critical_is_lost_when_cover_swallowed_the_hit() {
        let mut input = hit(10, ArmorMode::Full);
        input.manual_critical = true;
        input.cover_hp = Some(30);
        let out = resolve_damage(&input, &target(40, 0, 0));
        assert!(!out.critical);
        assert_eq!(out.hp_lost, 0);
    }

    #[test]
    fn hit_on_mortal_wound_causes_critical_and_raises_death_penalty_p186() {
        let out = resolve_damage(&hit(3, ArmorMode::Full), &target(0, 0, 0));
        assert!(out.critical);
        assert_eq!(out.death_save_penalty_increase, 1);
        assert_eq!(out.new_hp, 0);

        // Armor held: nothing gets through, so no new wound.
        let out = resolve_damage(&hit(3, ArmorMode::Full), &target(0, 0, 4));
        assert!(!out.critical);
        assert_eq!(out.death_save_penalty_increase, 0);
    }

    #[test]
    fn wound_states_follow_hp_thresholds_p186() {
        use WoundState::*;
        // max 40: serious below 20
        let st = |hp| wound_state(hp, 40, false);
        assert_eq!([st(40), st(39), st(20), st(19), st(1), st(0)], [Healthy, Light, Light, Serious, Serious, Mortal]);
        // odd max rounds half up: 35 -> 18
        assert_eq!([wound_state(18, 35, false), wound_state(17, 35, false)], [Light, Serious]);
        assert_eq!(wound_state(30, 40, true), Dead);
    }

    #[test]
    fn wound_penalties_p186() {
        assert_eq!(WoundState::Light.action_penalty(), 0);
        assert_eq!(WoundState::Serious.action_penalty(), 2);
        assert_eq!((WoundState::Mortal.action_penalty(), WoundState::Mortal.move_penalty()), (4, 6));
        assert_eq!(
            [WoundState::Light, WoundState::Serious, WoundState::Mortal].map(|w| w.stabilization_dv()),
            [Some(10), Some(13), Some(15)]
        );
    }

    #[test]
    fn max_hp_formula_p79() {
        assert_eq!(max_hp(6, 5), 40); // ceil(11/2) = 6
        assert_eq!(max_hp(8, 8), 50);
        assert_eq!(max_hp(2, 2), 20);
    }

    #[test]
    fn death_save_rules_p188() {
        // d10 + penalty must be <= BODY
        assert!(death_save(6, 0, dice("1d10", &[6])).passed);
        assert!(!death_save(6, 0, dice("1d10", &[7])).passed);
        // penalty shifts the roll and grows after each save
        let s = death_save(6, 1, dice("1d10", &[6]));
        assert!(!s.passed);
        assert_eq!(s.next_penalty, 2);
        // 10 always fails, even for BODY 10 or more
        assert!(!death_save(10, 0, dice("1d10", &[10])).passed);
        assert!(death_save(10, 0, dice("1d10", &[9])).passed);
    }

    #[test]
    fn critical_injury_roll_skips_injuries_already_owned_p187() {
        let seed = SeedDataSource;
        let table = seed.critical_injuries().unwrap().entries;
        let owned = vec!["broken_arm".to_string()]; // body, roll 6
        // First 2d6 = 6 (3+3) is owned: reroll; then 2+3 = 5.
        let mut rng = ScriptedDice::new([3, 3, 2, 3]);
        let (injury, result) =
            roll_critical_injury(&table, BodyArea::Body, &owned, &mut rng).unwrap();
        assert_eq!(injury.id, "broken_ribs");
        assert_eq!(result.total, 5);
    }

    #[test]
    fn critical_injury_roll_gives_none_when_all_owned() {
        let seed = SeedDataSource;
        let table = seed.critical_injuries().unwrap().entries;
        let owned: Vec<String> =
            table.iter().filter(|i| i.area == BodyArea::Head).map(|i| i.id.clone()).collect();
        let mut rng = ScriptedDice::new([]);
        assert!(roll_critical_injury(&table, BodyArea::Head, &owned, &mut rng).is_none());
    }
}
