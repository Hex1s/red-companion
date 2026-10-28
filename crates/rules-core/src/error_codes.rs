//! `ErrorInfo` for every error enum of the crate: the code the UI translates.
//!
//! Every `code` is an exhaustive `match` (no `_` arm). Keep each arm as `Pattern => "code"`
//! on one line: `src-tauri/src/error.rs` reads the codes out of this file to check that both
//! locale files translate them.

use crate::catalog::CatalogError;
use crate::character::CharacterError;
use crate::damage_event::EventError;
use crate::data_source::DataError;
use crate::dice::DiceError;
use crate::encounter::EncounterError;
use crate::encounter_transfer::EncounterTransferError;
use crate::error::{ErrorInfo, Params};
use crate::healing::HealingError;
use crate::humanity::HumanityError;
use crate::initiative::InitiativeError;
use crate::manifest::ManifestError;
use crate::params;
use crate::roles::{DrugError, RoleError};
use crate::scene::SceneError;
use crate::skill_check::SkillCheckError;
use crate::transfer::TransferError;
use crate::vitals::VitalsError;

impl ErrorInfo for DiceError {
    fn code(&self) -> &'static str {
        match self {
            Self::InvalidNotation(_) => "dice.invalid_notation",
            Self::OutOfRange => "dice.out_of_range",
            Self::WrongManualCount { .. } => "dice.wrong_manual_count",
            Self::ManualValueOutOfRange { .. } => "dice.manual_value_out_of_range",
            Self::CheckTotalOutOfRange(_) => "dice.check_total_out_of_range",
        }
    }

    fn params(&self) -> Params {
        match self {
            Self::InvalidNotation(text) => params! { "text" => text },
            Self::OutOfRange => Params::new(),
            Self::WrongManualCount { expected, got } => params! { "expected" => expected, "got" => got },
            Self::ManualValueOutOfRange { value, sides } => params! { "value" => value, "sides" => sides },
            Self::CheckTotalOutOfRange(total) => params! { "total" => total },
        }
    }
}

impl ErrorInfo for ManifestError {
    fn code(&self) -> &'static str {
        match self {
            Self::MissingSignature => "manifest.missing_signature",
            Self::MalformedSignature => "manifest.malformed_signature",
            Self::UntrustedSignature => "manifest.untrusted_signature",
            Self::BadKey => "manifest.bad_key",
            Self::HashMismatch { .. } => "manifest.hash_mismatch",
        }
    }

    fn params(&self) -> Params {
        match self {
            Self::HashMismatch { name } => params! { "name" => name },
            Self::MissingSignature | Self::MalformedSignature | Self::UntrustedSignature | Self::BadKey => Params::new(),
        }
    }
}

impl ErrorInfo for DataError {
    fn code(&self) -> &'static str {
        match self {
            Self::Malformed { .. } => "data.malformed",
            Self::Cache { .. } => "data.cache_failed",
        }
    }

    fn params(&self) -> Params {
        match self {
            Self::Malformed { name, .. } => params! { "name" => name },
            Self::Cache { name, .. } => params! { "name" => name },
        }
    }
}

impl ErrorInfo for CatalogError {
    fn code(&self) -> &'static str {
        match self {
            Self::DuplicateId(_) => "catalog.duplicate_id",
            Self::MissingField(_) => "catalog.missing_field",
            Self::OutOfRange(_) => "catalog.out_of_range",
            Self::IncompleteInjuryTable(_) => "catalog.incomplete_injury_table",
        }
    }

    fn params(&self) -> Params {
        match self {
            Self::DuplicateId(id) | Self::MissingField(id) | Self::OutOfRange(id) => params! { "id" => id },
            Self::IncompleteInjuryTable(area) => params! { "area" => format!("{area:?}") },
        }
    }
}

impl ErrorInfo for CharacterError {
    fn code(&self) -> &'static str {
        match self {
            Self::EmptyName => "character.empty_name",
            Self::StatOutOfRange { .. } => "character.stat_out_of_range",
            Self::BadArmor { .. } => "character.bad_armor",
            Self::HpAboveMax { .. } => "character.hp_above_max",
            Self::SkillOutOfRange { .. } => "character.skill_out_of_range",
            Self::FirstAidWithoutInjury(_) => "character.first_aid_without_injury",
            Self::ReputationOutOfRange(_) => "character.reputation_out_of_range",
            Self::BadColor(_) => "character.bad_color",
            Self::BadPortrait => "character.bad_portrait",
            Self::Role(e) => e.code(),
            Self::Dose(e) => e.code(),
            Self::UnsupportedVersion(_) => "character.unsupported_version",
        }
    }

    fn params(&self) -> Params {
        match self {
            Self::EmptyName => Params::new(),
            Self::StatOutOfRange { stat, value } => params! { "stat" => stat, "value" => value },
            Self::BadArmor { area, sp, max_sp } => params! { "area" => area, "sp" => sp, "max_sp" => max_sp },
            Self::HpAboveMax { hp, max_hp } => params! { "hp" => hp, "max_hp" => max_hp },
            Self::SkillOutOfRange { skill, level } => params! { "skill" => skill, "level" => level },
            Self::FirstAidWithoutInjury(injury) => params! { "injury" => injury },
            Self::ReputationOutOfRange(value) => params! { "value" => value },
            Self::BadColor(value) => params! { "value" => value },
            Self::BadPortrait => Params::new(),
            Self::Role(e) => e.params(),
            Self::Dose(e) => e.params(),
            Self::UnsupportedVersion(version) => params! { "version" => version },
        }
    }
}

impl ErrorInfo for RoleError {
    fn code(&self) -> &'static str {
        match self {
            Self::Duplicate(_) => "role.duplicate",
            Self::RankOutOfRange { .. } => "role.rank_out_of_range",
            Self::Overspent { .. } => "role.overspent",
            Self::SubAboveRank { .. } => "role.sub_above_rank",
            Self::UnknownRole(_) => "role.unknown_role",
            Self::UnknownSubAbility { .. } => "role.unknown_sub_ability",
            Self::TooManyPoints { .. } => "role.too_many_points",
            Self::UnknownOption { .. } => "role.unknown_option",
            Self::DuplicatePick { .. } => "role.duplicate_pick",
            Self::TooManyPicks { .. } => "role.too_many_picks",
            Self::BadCustomField { .. } => "role.bad_custom_field",
        }
    }

    fn params(&self) -> Params {
        match self {
            Self::Duplicate(role) | Self::UnknownRole(role) => params! { "role" => role },
            Self::RankOutOfRange { role, rank } => params! { "role" => role, "rank" => rank },
            Self::Overspent { role, spent, allowed } => {
                params! { "role" => role, "spent" => spent, "allowed" => allowed }
            }
            Self::SubAboveRank { role, sub, points, rank } => {
                params! { "role" => role, "sub" => sub, "points" => points, "rank" => rank }
            }
            Self::UnknownSubAbility { role, sub } => params! { "role" => role, "sub" => sub },
            Self::TooManyPoints { role, sub, max } => params! { "role" => role, "sub" => sub, "max" => max },
            Self::UnknownOption { role, option } | Self::DuplicatePick { role, option } => {
                params! { "role" => role, "option" => option }
            }
            Self::TooManyPicks { role, sub, picks, points } => {
                params! { "role" => role, "sub" => sub, "picks" => picks, "points" => points }
            }
            Self::BadCustomField { role, what } => params! { "role" => role, "what" => what },
        }
    }
}

impl ErrorInfo for DrugError {
    fn code(&self) -> &'static str {
        match self {
            Self::NotAvailable => "drug.not_available",
            Self::Dead => "drug.dead",
            Self::AlreadyDosed => "drug.already_dosed",
            Self::MortallyWounded => "drug.mortally_wounded",
            Self::Unknown(_) => "drug.unknown",
        }
    }

    fn params(&self) -> Params {
        match self {
            Self::Unknown(drug) => params! { "drug" => drug },
            Self::NotAvailable | Self::Dead | Self::AlreadyDosed | Self::MortallyWounded => Params::new(),
        }
    }
}

impl ErrorInfo for EventError {
    fn code(&self) -> &'static str {
        match self {
            Self::CoverNotFound(_) => "damage.cover_not_found",
            Self::AimMismatch(_) => "damage.aim_mismatch",
            Self::Dice(e) => e.code(),
            Self::UnknownInjury(_) => "damage.unknown_injury",
            Self::WrongInjuryArea(_) => "damage.wrong_injury_area",
            Self::InjuryOwned(_) => "damage.injury_owned",
            Self::NoCritical => "damage.no_critical",
            Self::NoFreeInjury => "damage.no_free_injury",
        }
    }

    fn params(&self) -> Params {
        match self {
            Self::CoverNotFound(id) => params! { "id" => id },
            Self::AimMismatch(aim) => params! { "aim" => aim },
            Self::Dice(e) => e.params(),
            Self::UnknownInjury(injury) | Self::WrongInjuryArea(injury) | Self::InjuryOwned(injury) => {
                params! { "injury" => injury }
            }
            Self::NoCritical | Self::NoFreeInjury => Params::new(),
        }
    }
}

impl ErrorInfo for VitalsError {
    fn code(&self) -> &'static str {
        match self {
            Self::NotMortal => "vitals.not_mortal",
            Self::CannotStabilize => "vitals.cannot_stabilize",
            Self::BadSkill(_) => "vitals.bad_skill",
            Self::NotDead => "vitals.not_dead",
        }
    }

    fn params(&self) -> Params {
        match self {
            Self::BadSkill(skill) => params! { "skill" => skill },
            Self::NotMortal | Self::CannotStabilize | Self::NotDead => Params::new(),
        }
    }
}

impl ErrorInfo for HealingError {
    fn code(&self) -> &'static str {
        match self {
            Self::Dead => "healing.dead",
            Self::NotInjured => "healing.not_injured",
            Self::NoSuchOption => "healing.no_such_option",
            Self::AlreadyFirstAided => "healing.already_first_aided",
            Self::NeedsStabilization => "healing.needs_stabilization",
            Self::BadTarget => "healing.bad_target",
        }
    }
}

impl ErrorInfo for HumanityError {
    fn code(&self) -> &'static str {
        match self {
            Self::Dead => "humanity.dead",
            Self::NoLoss => "humanity.no_loss",
            Self::AlreadyFull => "humanity.already_full",
            Self::MissingRestore => "humanity.missing_restore",
        }
    }
}

impl ErrorInfo for InitiativeError {
    fn code(&self) -> &'static str {
        match self {
            Self::NotInFight(_) => "initiative.not_in_fight",
            Self::AlreadyInFight(_) => "initiative.already_in_fight",
            Self::OutOfRange => "initiative.out_of_range",
            Self::NotStarted => "initiative.not_started",
            Self::NobodyCanAct => "initiative.nobody_can_act",
        }
    }
}

impl ErrorInfo for SceneError {
    fn code(&self) -> &'static str {
        match self {
            Self::CoverNotFound(_) => "scene.cover_not_found",
            Self::BadHp => "scene.bad_hp",
            Self::HpRequired(_) => "scene.hp_required",
            Self::UnsupportedVersion(_) => "scene.unsupported_version",
        }
    }

    fn params(&self) -> Params {
        match self {
            Self::CoverNotFound(id) => params! { "id" => id },
            Self::BadHp => params! { "max" => crate::scene::COVER_HP_MAX },
            Self::HpRequired(preset) => params! { "preset" => preset },
            Self::UnsupportedVersion(version) => params! { "version" => version },
        }
    }
}

impl ErrorInfo for SkillCheckError {
    fn code(&self) -> &'static str {
        match self {
            Self::UnknownSkill(_) => "skill_check.unknown_skill",
        }
    }

    fn params(&self) -> Params {
        match self {
            Self::UnknownSkill(skill) => params! { "skill" => skill },
        }
    }
}

impl ErrorInfo for EncounterError {
    fn code(&self) -> &'static str {
        match self {
            Self::EmptyName => "encounter.empty_name",
            Self::NoEntries => "encounter.no_entries",
            Self::TooManyEntries => "encounter.too_many_entries",
            Self::BadCount(_) => "encounter.bad_count",
            Self::BadPlayers(_) => "encounter.bad_players",
            Self::BadTemplateId(_) => "encounter.bad_template_id",
        }
    }

    fn params(&self) -> Params {
        match self {
            Self::EmptyName | Self::NoEntries => Params::new(),
            Self::TooManyEntries => params! { "max" => crate::encounter::ENTRIES_MAX },
            Self::BadCount(count) => params! { "count" => count, "max" => crate::encounter::COUNT_MAX },
            Self::BadPlayers(players) => {
                params! { "players" => players, "max" => crate::encounter::PLAYERS_MAX }
            }
            Self::BadTemplateId(id) => params! { "id" => id },
        }
    }
}

impl ErrorInfo for TransferError {
    fn code(&self) -> &'static str {
        match self {
            Self::Json(_) => "transfer.json",
            Self::WrongFormat => "transfer.wrong_format",
            Self::UnsupportedVersion(_) => "transfer.unsupported_version",
            Self::Empty => "transfer.empty",
            Self::Invalid { .. } => "transfer.invalid",
            Self::UnknownSkill { .. } => "transfer.unknown_skill",
            Self::UnknownInjury { .. } => "transfer.unknown_injury",
            Self::Data(e) => e.code(),
        }
    }

    fn params(&self) -> Params {
        match self {
            Self::Json(_) | Self::WrongFormat | Self::Empty => Params::new(),
            Self::UnsupportedVersion(version) => params! { "version" => version },
            // `index` is 0-based in the code, 1-based for the person reading the file.
            Self::Invalid { index, name, .. } => params! { "index" => index + 1, "name" => name },
            Self::UnknownSkill { index, name, skill } => {
                params! { "index" => index + 1, "name" => name, "skill" => skill }
            }
            Self::UnknownInjury { index, name, injury } => {
                params! { "index" => index + 1, "name" => name, "injury" => injury }
            }
            Self::Data(e) => e.params(),
        }
    }

    fn cause_info(&self) -> Option<&dyn ErrorInfo> {
        match self {
            Self::Invalid { source, .. } => Some(source),
            Self::Data(e) => e.cause_info(),
            Self::Json(_)
            | Self::WrongFormat
            | Self::UnsupportedVersion(_)
            | Self::Empty
            | Self::UnknownSkill { .. }
            | Self::UnknownInjury { .. } => None,
        }
    }
}

impl ErrorInfo for EncounterTransferError {
    fn code(&self) -> &'static str {
        match self {
            Self::Json(_) => "encounter_transfer.json",
            Self::WrongFormat => "encounter_transfer.wrong_format",
            Self::UnsupportedVersion(_) => "encounter_transfer.unsupported_version",
            Self::Empty => "encounter_transfer.empty",
            Self::TooMany => "encounter_transfer.too_many",
            Self::DuplicateKey(_) => "encounter_transfer.duplicate_key",
            Self::InvalidTemplate { .. } => "encounter_transfer.invalid_template",
            Self::UnknownSkill { .. } => "encounter_transfer.unknown_skill",
            Self::UnknownInjury { .. } => "encounter_transfer.unknown_injury",
            Self::InvalidEncounter { .. } => "encounter_transfer.invalid_encounter",
            Self::MissingTemplate { .. } => "encounter_transfer.missing_template",
            Self::UnknownBookTemplate { .. } => "encounter_transfer.unknown_book_template",
            Self::Data(e) => e.code(),
        }
    }

    fn params(&self) -> Params {
        match self {
            Self::Json(_) | Self::WrongFormat | Self::Empty => Params::new(),
            Self::UnsupportedVersion(version) => params! { "version" => version },
            Self::TooMany => params! { "max" => crate::encounter_transfer::ITEMS_MAX },
            Self::DuplicateKey(key) => params! { "key" => key },
            Self::InvalidTemplate { name, .. } | Self::InvalidEncounter { name, .. } => params! { "name" => name },
            Self::UnknownSkill { name, skill } => params! { "name" => name, "skill" => skill },
            Self::UnknownInjury { name, injury } => params! { "name" => name, "injury" => injury },
            Self::MissingTemplate { name, key } => params! { "name" => name, "key" => key },
            Self::UnknownBookTemplate { name, code } => params! { "name" => name, "template" => code },
            Self::Data(e) => e.params(),
        }
    }

    fn cause_info(&self) -> Option<&dyn ErrorInfo> {
        match self {
            Self::InvalidTemplate { source, .. } => Some(source),
            Self::InvalidEncounter { source, .. } => Some(source),
            Self::Data(e) => e.cause_info(),
            Self::Json(_)
            | Self::WrongFormat
            | Self::UnsupportedVersion(_)
            | Self::Empty
            | Self::TooMany
            | Self::DuplicateKey(_)
            | Self::UnknownSkill { .. }
            | Self::UnknownInjury { .. }
            | Self::MissingTemplate { .. }
            | Self::UnknownBookTemplate { .. } => None,
        }
    }
}
