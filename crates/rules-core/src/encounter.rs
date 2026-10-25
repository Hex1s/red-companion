//! Encounters: a named list of NPC templates with counts, launched into the session in one go.
//! Templates are either ready-made ones from the book (`Book`, by catalog code) or the GM's own
//! (`Custom`, by id in the user database).

use serde::{Deserialize, Serialize};
use thiserror::Error;
use ts_rs::TS;

use crate::catalog::{NpcTier, Skill};
use crate::character::{skill_base_id, Character};

/// Most clones of one template in an encounter.
pub const COUNT_MAX: u32 = 20;
/// Most lines in one encounter.
pub const ENTRIES_MAX: usize = 30;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, TS)]
#[ts(export)]
#[serde(rename_all = "snake_case")]
pub enum TemplateSource {
    /// A ready-made NPC from the catalog; `template` is its `code`.
    Book,
    /// The GM's own template; `template` is its id in the user database.
    Custom,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct EncounterEntry {
    pub source: TemplateSource,
    pub template: String,
    pub count: u32,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct Encounter {
    pub name: String,
    #[serde(default)]
    pub notes: String,
    pub entries: Vec<EncounterEntry>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct EncounterRecord {
    #[ts(type = "number")]
    pub id: i64,
    pub encounter: Encounter,
}

/// The GM's own NPC template as stored: a fresh sheet plus the tier the GM files it under
/// (the book gives tiers only to its own NPCs).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct UserTemplateDoc {
    #[serde(default = "default_tier")]
    pub tier: NpcTier,
    pub character: Character,
    /// `code` of the book template this one replaces: the GM edited a book NPC, the copy takes its
    /// place in lists and in new fights, and deleting it brings the original back.
    #[serde(default)]
    pub based_on: Option<String>,
}

fn default_tier() -> NpcTier {
    NpcTier::Mook
}

/// The GM's own NPC template: a fresh sheet that is cloned into the session.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct UserTemplateRecord {
    #[ts(type = "number")]
    pub id: i64,
    pub tier: NpcTier,
    pub character: Character,
    /// `code` of the book template this one replaces (see `UserTemplateDoc::based_on`).
    #[serde(default)]
    #[ts(type = "string | null")]
    pub based_on: Option<String>,
}

/// How many edgerunners an NPC of the tier stands for. The book (p. 413-415) says: mooks fight
/// an equal number of edgerunners, one lieutenant per two, one mini-boss per three. The boss
/// has no number in the book; `WEIGHT_BOSS` is our estimate (about twice a mini-boss by
/// toughness x damage of the book's NPCs), not a printed rule.
pub const WEIGHT_MOOK: u32 = 1;
pub const WEIGHT_LIEUTENANT: u32 = 2;
pub const WEIGHT_MINI_BOSS: u32 = 3;
pub const WEIGHT_BOSS: u32 = 6;
/// Largest group the assessment accepts.
pub const PLAYERS_MAX: u32 = 12;

pub fn tier_weight(tier: NpcTier) -> u32 {
    match tier {
        NpcTier::Mook => WEIGHT_MOOK,
        NpcTier::Lieutenant => WEIGHT_LIEUTENANT,
        NpcTier::MiniBoss => WEIGHT_MINI_BOSS,
        NpcTier::Boss => WEIGHT_BOSS,
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, TS)]
#[ts(export)]
#[serde(rename_all = "snake_case")]
pub enum Verdict {
    Weak,
    Fair,
    Strong,
    VeryStrong,
}

/// Information only: the GM decides. Weight against the number of players.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct Assessment {
    pub weight: u32,
    pub players: u32,
    pub verdict: Verdict,
    /// NPCs left out because their template no longer exists.
    pub skipped: u32,
}

/// Weight / players up to 0.7 is weak, up to 1.3 fair, up to 1.8 strong, above very strong.
pub fn verdict(weight: u32, players: u32) -> Verdict {
    let (w, p) = (weight * 100, players * 100);
    if w < p * 7 / 10 {
        Verdict::Weak
    } else if w <= p * 13 / 10 {
        Verdict::Fair
    } else if w <= p * 18 / 10 {
        Verdict::Strong
    } else {
        Verdict::VeryStrong
    }
}

pub fn assess(weight: u32, players: u32, skipped: u32) -> Result<Assessment, EncounterError> {
    if !(1..=PLAYERS_MAX).contains(&players) {
        return Err(EncounterError::BadPlayers(players));
    }
    Ok(Assessment { weight, players, verdict: verdict(weight, players), skipped })
}

/// Skills that count as "combat" for the tier hint: shooting, melee, hand-to-hand and dodging.
pub const COMBAT_SKILLS: [&str; 9] = [
    "handgun",
    "shoulder_arms",
    "heavy_weapons",
    "autofire",
    "archery",
    "melee_weapon",
    "brawling",
    "martial_arts",
    "evasion",
];

/// Thresholds of the tier hint. The book gives no rule for a tier by stats, so they are fitted
/// to its ten NPCs (p. 412-416) once and fixed here; the NPCs themselves are not consulted.
/// Each sits in the gap between the neighbouring tiers of the book:
/// - armor (body SP): mooks 4-7, everyone above them 11-12 -> lieutenant from 10;
/// - best combat skill (STAT + level): mooks and lieutenants 10-12, mini-bosses 14, boss 17
///   -> mini-boss from 13, boss from 16;
/// - HP: 20-40 for all but the boss (55) -> boss from 50.
/// Only one boss is printed, so the boss thresholds are the least certain.
pub const HINT_LIEUTENANT_SP: u8 = 10;
pub const HINT_MINI_BOSS_SKILL: u8 = 13;
pub const HINT_BOSS_SKILL: u8 = 16;
pub const HINT_BOSS_HP: u32 = 50;

/// What pushed the hint to its tier.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, TS)]
#[ts(export)]
#[serde(rename_all = "snake_case")]
pub enum HintReason {
    /// Nothing reaches a higher tier: a mook.
    Baseline,
    Armor,
    Skill,
    Hp,
}

/// "By its stats the sheet is closest to this tier": information only, the GM picks the tier.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct TierHint {
    pub tier: NpcTier,
    pub reason: HintReason,
    pub hp: u32,
    /// Body armor SP (the better of the two slots when they differ).
    pub sp: u8,
    /// Best combat skill, STAT + level.
    pub skill: u8,
}

/// Best STAT + level over the combat skills on the sheet (0 when it has none).
pub fn best_combat_skill(character: &Character, catalog: &[Skill]) -> u8 {
    character
        .skills
        .iter()
        .filter(|(key, _)| COMBAT_SKILLS.contains(&skill_base_id(key)))
        .filter_map(|(key, level)| {
            let skill = catalog.iter().find(|s| s.id == skill_base_id(key))?;
            Some(character.stats.get(skill.stat).saturating_add(*level))
        })
        .max()
        .unwrap_or(0)
}

/// Picks the highest tier whose threshold the sheet reaches (see the `HINT_*` constants).
pub fn tier_hint(character: &Character, catalog: &[Skill]) -> TierHint {
    let hp = character.max_hp();
    let sp = character.armor_body.max_sp.max(character.armor_head.max_sp);
    let skill = best_combat_skill(character, catalog);
    let (tier, reason) = if hp >= HINT_BOSS_HP {
        (NpcTier::Boss, HintReason::Hp)
    } else if skill >= HINT_BOSS_SKILL {
        (NpcTier::Boss, HintReason::Skill)
    } else if skill >= HINT_MINI_BOSS_SKILL {
        (NpcTier::MiniBoss, HintReason::Skill)
    } else if sp >= HINT_LIEUTENANT_SP {
        (NpcTier::Lieutenant, HintReason::Armor)
    } else {
        (NpcTier::Mook, HintReason::Baseline)
    };
    TierHint { tier, reason, hp, sp, skill }
}

#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum EncounterError {
    #[error("encounter name must not be empty")]
    EmptyName,
    #[error("encounter has no NPCs")]
    NoEntries,
    #[error("encounter has more than {ENTRIES_MAX} lines")]
    TooManyEntries,
    #[error("count {0} must be 1..={COUNT_MAX}")]
    BadCount(u32),
    #[error("players {0} must be 1..={PLAYERS_MAX}")]
    BadPlayers(u32),
    #[error("custom template id {0:?} is not a number")]
    BadTemplateId(String),
}

impl Encounter {
    /// Trims the name and checks the invariants a stored encounter must satisfy.
    pub fn normalize(&mut self) -> Result<(), EncounterError> {
        self.name = self.name.trim().to_string();
        if self.name.is_empty() {
            return Err(EncounterError::EmptyName);
        }
        if self.entries.is_empty() {
            return Err(EncounterError::NoEntries);
        }
        if self.entries.len() > ENTRIES_MAX {
            return Err(EncounterError::TooManyEntries);
        }
        for e in &self.entries {
            if !(1..=COUNT_MAX).contains(&e.count) {
                return Err(EncounterError::BadCount(e.count));
            }
            if e.source == TemplateSource::Custom && e.template.parse::<i64>().is_err() {
                return Err(EncounterError::BadTemplateId(e.template.clone()));
            }
        }
        Ok(())
    }

    /// Total number of NPCs the encounter puts on the table.
    pub fn total(&self) -> u32 {
        self.entries.iter().map(|e| e.count).sum()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::character::ArmorSlot;

    fn entry(source: TemplateSource, template: &str, count: u32) -> EncounterEntry {
        EncounterEntry { source, template: template.to_string(), count }
    }

    fn encounter(entries: Vec<EncounterEntry>) -> Encounter {
        Encounter { name: "  Ambush ".into(), notes: String::new(), entries }
    }

    #[test]
    fn valid_encounter_is_trimmed_and_counted() {
        let mut e = encounter(vec![
            entry(TemplateSource::Book, "road_bandit", 3),
            entry(TemplateSource::Custom, "7", 1),
        ]);
        e.normalize().unwrap();
        assert_eq!(e.name, "Ambush");
        assert_eq!(e.total(), 4);
    }

    #[test]
    fn rejects_bad_encounters() {
        let mut empty_name = encounter(vec![entry(TemplateSource::Book, "x", 1)]);
        empty_name.name = "  ".into();
        assert_eq!(empty_name.normalize(), Err(EncounterError::EmptyName));

        assert_eq!(encounter(vec![]).normalize(), Err(EncounterError::NoEntries));
        assert_eq!(
            encounter(vec![entry(TemplateSource::Book, "x", 0)]).normalize(),
            Err(EncounterError::BadCount(0))
        );
        assert_eq!(
            encounter(vec![entry(TemplateSource::Book, "x", COUNT_MAX + 1)]).normalize(),
            Err(EncounterError::BadCount(COUNT_MAX + 1))
        );
        assert_eq!(
            encounter(vec![entry(TemplateSource::Custom, "boss", 1)]).normalize(),
            Err(EncounterError::BadTemplateId("boss".into()))
        );
        let many = (0..=ENTRIES_MAX).map(|_| entry(TemplateSource::Book, "x", 1)).collect();
        assert_eq!(encounter(many).normalize(), Err(EncounterError::TooManyEntries));
    }

    #[test]
    fn weights_follow_the_book_ratios() {
        // p. 413-415: equal number of mooks, one lieutenant per two, one mini-boss per three.
        assert_eq!(tier_weight(NpcTier::Mook), 1);
        assert_eq!(tier_weight(NpcTier::Lieutenant), 2);
        assert_eq!(tier_weight(NpcTier::MiniBoss), 3);
        // The book's own proportions are "fair" for four edgerunners.
        assert_eq!(verdict(4, 4), Verdict::Fair); // four mooks
        assert_eq!(verdict(2 * WEIGHT_LIEUTENANT, 4), Verdict::Fair); // two lieutenants
        assert_eq!(verdict(WEIGHT_MINI_BOSS + 1, 4), Verdict::Fair); // one mini-boss and a mook
    }

    #[test]
    fn verdict_thresholds() {
        assert_eq!(verdict(0, 4), Verdict::Weak);
        assert_eq!(verdict(2, 4), Verdict::Weak); // 0.5
        assert_eq!(verdict(3, 4), Verdict::Fair); // 0.75
        assert_eq!(verdict(7, 10), Verdict::Fair); // exactly 0.7 is fair
        assert_eq!(verdict(13, 10), Verdict::Fair); // exactly 1.3 is fair
        assert_eq!(verdict(6, 4), Verdict::Strong); // 1.5
        assert_eq!(verdict(18, 10), Verdict::Strong); // exactly 1.8 is strong
        assert_eq!(verdict(8, 4), Verdict::VeryStrong); // 2.0
        assert_eq!(verdict(WEIGHT_BOSS, 1), Verdict::VeryStrong);
    }

    #[test]
    fn assessment_checks_players() {
        assert_eq!(assess(4, 0, 0), Err(EncounterError::BadPlayers(0)));
        assert_eq!(assess(4, PLAYERS_MAX + 1, 0), Err(EncounterError::BadPlayers(PLAYERS_MAX + 1)));
        let a = assess(4, 4, 1).unwrap();
        assert_eq!((a.weight, a.verdict, a.skipped), (4, Verdict::Fair, 1));
    }

    #[test]
    fn fresh_sheet_has_full_hp_and_armor_and_no_wounds() {
        let mut c = Character::quick("Thug");
        c.hp = 3;
        c.armor_body = ArmorSlot { sp: 2, max_sp: 7 };
        c.injuries.push("broken_leg".into());
        c.death_save_penalty = 2;
        c.dead = true;

        let fresh = c.as_fresh();
        assert_eq!(fresh.hp, fresh.max_hp());
        assert_eq!(fresh.armor_body, ArmorSlot { sp: 7, max_sp: 7 });
        assert!(fresh.injuries.is_empty());
        assert_eq!(fresh.death_save_penalty, 0);
        assert!(!fresh.dead);
        assert_eq!(fresh.name, "Thug");
    }

    #[test]
    fn tier_hint_reproduces_the_ten_book_npcs() {
        use crate::data_source::{DataSource, SeedDataSource};
        let data = SeedDataSource::default();
        let skills = data.skills().unwrap().entries;
        let npcs = data.npc_templates().unwrap().entries;
        assert_eq!(npcs.len(), 10);
        for npc in &npcs {
            let sheet = npc.to_character(&npc.name, "", &skills);
            let hint = tier_hint(&sheet, &skills);
            assert_eq!(hint.tier, npc.tier, "{}: {hint:?}", npc.code);
        }
    }

    #[test]
    fn tier_hint_thresholds_and_reasons() {
        use crate::character::ArmorSlot;
        use crate::data_source::DataSource;
        let skills = crate::data_source::SeedDataSource::default().skills().unwrap().entries;
        let mut c = Character::quick("X");
        // quick(): default stats; no skills -> baseline.
        assert_eq!(tier_hint(&c, &skills).reason, HintReason::Baseline);
        assert_eq!(tier_hint(&c, &skills).skill, 0);

        c.armor_body = ArmorSlot { sp: 10, max_sp: 10 };
        let h = tier_hint(&c, &skills);
        assert_eq!((h.tier, h.reason), (NpcTier::Lieutenant, HintReason::Armor));
        c.armor_body = ArmorSlot { sp: 9, max_sp: 9 };
        assert_eq!(tier_hint(&c, &skills).tier, NpcTier::Mook);

        // Only combat skills count; a non-combat one does not raise the tier.
        c.stats.reflex = 8;
        c.skills.insert("perception".into(), 10);
        assert_eq!(tier_hint(&c, &skills).skill, 0);
        c.skills.insert("handgun".into(), 5); // handgun is REF-based: 8 + 5
        let h = tier_hint(&c, &skills);
        assert_eq!((h.skill, h.tier, h.reason), (13, NpcTier::MiniBoss, HintReason::Skill));
        c.skills.insert("handgun".into(), 8);
        assert_eq!(tier_hint(&c, &skills).tier, NpcTier::Boss);

        // Specialty keys and HP.
        let mut tank = Character::quick("Tank");
        tank.stats.body = 10;
        tank.stats.will = 10;
        let h = tier_hint(&tank, &skills);
        assert_eq!((h.hp, h.tier, h.reason), (60, NpcTier::Boss, HintReason::Hp));
    }
}
