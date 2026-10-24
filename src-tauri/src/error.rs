//! The error type of every Tauri command: `rules_core::error::AppError` on the wire, so the UI
//! translates the `code` instead of showing English developer text.

use rules_core::error::{AppError, Params};
use rules_core::params;
use serde::{Serialize, Serializer};

#[derive(Debug)]
pub struct CmdError(pub AppError);

impl Serialize for CmdError {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        self.0.serialize(serializer)
    }
}

impl CmdError {
    fn new(code: &str, params: Params, message: String) -> Self {
        Self(AppError::new(code, params, message))
    }

    pub fn not_found(what: &str, id: impl std::fmt::Display) -> Self {
        let message = format!("{what} {id} not found");
        Self::new(&format!("not_found.{what}"), params! { "id" => id }, message)
    }

    pub fn name_length(max: usize) -> Self {
        Self::new("session.name_length", params! { "max" => max }, format!("session name must be 1..{max} characters"))
    }

    pub fn notes_too_long(max: usize) -> Self {
        Self::new("session.notes_too_long", params! { "max" => max }, format!("notes are longer than {max} characters"))
    }

    pub fn nothing_to_export() -> Self {
        Self::new("transfer.nothing_to_export", Params::new(), "nothing to export".to_string())
    }

    pub fn wrong_file_kind(name: &str) -> Self {
        Self::new("transfer.wrong_file_kind", params! { "name" => name }, format!("{name:?} is not a file of this type"))
    }

    pub fn injury_not_on_sheet(id: &str) -> Self {
        Self::new("injury.not_on_sheet", params! { "id" => id }, format!("{id} is not on the sheet"))
    }

    pub fn surgery_bonus_missing() -> Self {
        Self::new("healing.surgery_bonus_missing", Params::new(), "surgery needs a bonus".to_string())
    }

    pub fn nothing_to_undo() -> Self {
        Self::new("scene.nothing_to_undo", Params::new(), "nothing to undo".to_string())
    }

    pub fn unknown_preset(id: &str) -> Self {
        Self::new("not_found.cover_preset", params! { "id" => id }, format!("unknown cover preset {id:?}"))
    }

    /// A poisoned lock: an earlier command panicked while holding it.
    pub fn locked(what: &str) -> Self {
        Self::new("internal.locked", params! { "what" => what }, format!("{what} lock poisoned"))
    }
}

/// `?` on any rules error turns it into a coded `CmdError`.
macro_rules! from_rules_error {
    ($($ty:ty),* $(,)?) => {$(
        impl From<$ty> for CmdError {
            fn from(error: $ty) -> Self {
                Self(AppError::from_info(&error))
            }
        }
    )*};
}

from_rules_error!(
    rules_core::catalog::CatalogError,
    rules_core::character::CharacterError,
    rules_core::damage_event::EventError,
    rules_core::data_source::DataError,
    rules_core::dice::DiceError,
    rules_core::encounter::EncounterError,
    rules_core::encounter_transfer::EncounterTransferError,
    rules_core::healing::HealingError,
    rules_core::humanity::HumanityError,
    rules_core::initiative::InitiativeError,
    rules_core::roles::DrugError,
    rules_core::roles::RoleError,
    rules_core::scene::SceneError,
    rules_core::skill_check::SkillCheckError,
    rules_core::transfer::TransferError,
    rules_core::vitals::VitalsError,
);

impl From<storage::StorageError> for CmdError {
    fn from(error: storage::StorageError) -> Self {
        let code = match &error {
            storage::StorageError::Sqlite(_) => "storage.database",
            storage::StorageError::Migration(_) => "storage.migration",
        };
        Self::new(code, Params::new(), error.to_string())
    }
}

impl From<serde_json::Error> for CmdError {
    /// Stored data that no longer parses; a bad import file is a `TransferError` instead.
    fn from(error: serde_json::Error) -> Self {
        Self::new("storage.corrupt_data", Params::new(), error.to_string())
    }
}

impl From<std::io::Error> for CmdError {
    fn from(error: std::io::Error) -> Self {
        let code = match error.kind() {
            std::io::ErrorKind::NotFound => "file.not_found",
            std::io::ErrorKind::PermissionDenied => "file.permission_denied",
            _ => "file.io",
        };
        Self::new(code, Params::new(), error.to_string())
    }
}

impl From<tauri_plugin_opener::Error> for CmdError {
    fn from(error: tauri_plugin_opener::Error) -> Self {
        Self::new("file.open_folder", Params::new(), error.to_string())
    }
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeSet;

    use super::*;

    /// Codes written in the sources: `=> "domain.name"` arms of `rules-core` and the literals of
    /// this crate. A code that is not translated in both languages fails the test, so a missing
    /// Russian text can never silently fall back to English.
    fn codes_in(src: &str) -> BTreeSet<String> {
        let mut out = BTreeSet::new();
        for line in src.lines() {
            let mut rest = line;
            while let Some(i) = rest.find("=> \"") {
                rest = &rest[i + 4..];
                if let Some(end) = rest.find('"') {
                    let lit = &rest[..end];
                    if lit.contains('.') && lit.chars().all(|c| c.is_ascii_lowercase() || c == '.' || c == '_') {
                        out.insert(lit.to_string());
                    }
                }
            }
        }
        out
    }

    fn has_key(locale: &serde_json::Value, code: &str) -> bool {
        let mut node = &locale["errors"];
        for part in code.split('.') {
            node = &node[part];
        }
        node.as_str().is_some_and(|s| !s.is_empty())
    }

    #[test]
    fn every_error_code_is_translated_in_both_languages() {
        let mut codes = codes_in(include_str!("../../crates/rules-core/src/error_codes.rs"));
        // Codes made in this crate (see `CmdError` above).
        codes.extend(
            [
                "session.name_length",
                "session.notes_too_long",
                "transfer.nothing_to_export",
                "transfer.wrong_file_kind",
                "internal.unknown",
                "internal.locked",
                "storage.database",
                "storage.migration",
                "storage.corrupt_data",
                "file.not_found",
                "file.permission_denied",
                "file.io",
                "file.open_folder",
                "not_found.character",
                "not_found.template",
                "not_found.encounter",
                "not_found.session",
                "not_found.npc_template",
                "not_found.injury",
                "not_found.cover_preset",
                "scene.nothing_to_undo",
                "injury.not_on_sheet",
                "healing.surgery_bonus_missing",
            ]
            .map(String::from),
        );
        assert!(codes.len() > 60, "codes were not read from the sources");

        for (lang, text) in [
            ("ru", include_str!("../../src/i18n/locales/ru.json")),
            ("en", include_str!("../../src/i18n/locales/en.json")),
        ] {
            let locale: serde_json::Value = serde_json::from_str(text).unwrap();
            let missing: Vec<_> = codes.iter().filter(|c| !has_key(&locale, c)).collect();
            assert!(missing.is_empty(), "errors.* keys missing in {lang}.json: {missing:?}");
        }
    }

    #[test]
    fn not_found_code_names_the_thing() {
        let e = CmdError::not_found("character", 5);
        assert_eq!(e.0.code, "not_found.character");
        assert_eq!(e.0.params["id"], "5");
    }
}
