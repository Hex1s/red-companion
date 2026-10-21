//! Export and import of characters as a versioned JSON file.
//!
//! The file is an envelope: `format` tells it apart from other JSON,
//! `schema_version` is the version of the envelope (each character carries its
//! own `schema_version` too). Import validates everything before anything is
//! stored, so a bad file never leaves half of its characters behind.

use std::collections::HashSet;

use serde::{Deserialize, Serialize};
use thiserror::Error;

use crate::character::{skill_base_id, Character, CharacterError, CharacterKind};
use crate::data_source::{DataError, DataSource};

/// Value of the `format` field.
pub const FORMAT: &str = "red-companion/characters";
/// Current version of the envelope.
pub const FILE_VERSION: u32 = 1;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ExportedCharacter {
    pub kind: CharacterKind,
    pub character: Character,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CharacterFile {
    pub format: String,
    pub schema_version: u32,
    pub characters: Vec<ExportedCharacter>,
}

#[derive(Debug, Error)]
pub enum TransferError {
    #[error("not a valid JSON file: {0}")]
    Json(#[from] serde_json::Error),
    #[error("this is not a Red Companion characters file")]
    WrongFormat,
    #[error("unsupported file version {0}")]
    UnsupportedVersion(u32),
    #[error("the file contains no characters")]
    Empty,
    #[error("character #{index} ({name:?}): {source}")]
    Invalid { index: usize, name: String, source: CharacterError },
    #[error("character #{index} ({name:?}): unknown skill {skill:?}")]
    UnknownSkill { index: usize, name: String, skill: String },
    #[error("character #{index} ({name:?}): unknown injury {injury:?}")]
    UnknownInjury { index: usize, name: String, injury: String },
    #[error(transparent)]
    Data(#[from] DataError),
}

/// Serialises characters into the export file (pretty-printed, for humans too).
pub fn export(items: Vec<ExportedCharacter>) -> Result<String, TransferError> {
    let file = CharacterFile { format: FORMAT.to_string(), schema_version: FILE_VERSION, characters: items };
    Ok(serde_json::to_string_pretty(&file)?)
}

/// Parses and fully validates an export file. Names are trimmed; skill and
/// injury ids must exist in the reference data.
pub fn parse_import(
    json: &str,
    data: &impl DataSource,
) -> Result<Vec<ExportedCharacter>, TransferError> {
    // Check the envelope on a loose value first: a file from a newer version
    // may have a different shape, and "unsupported version" is the useful error.
    let value: serde_json::Value = serde_json::from_str(json)?;
    if value.get("format").and_then(|f| f.as_str()) != Some(FORMAT) {
        return Err(TransferError::WrongFormat);
    }
    if let Some(version) = value.get("schema_version").and_then(|v| v.as_u64()) {
        if version != u64::from(FILE_VERSION) {
            return Err(TransferError::UnsupportedVersion(version.try_into().unwrap_or(u32::MAX)));
        }
    }
    let mut file: CharacterFile = serde_json::from_value(value)?;
    if file.characters.is_empty() {
        return Err(TransferError::Empty);
    }

    let skills: HashSet<String> = data.skills()?.entries.into_iter().map(|s| s.id).collect();
    let injuries: HashSet<String> =
        data.critical_injuries()?.entries.into_iter().map(|i| i.id).collect();

    for (i, item) in file.characters.iter_mut().enumerate() {
        let index = i + 1;
        let c = &mut item.character;
        c.name = c.name.trim().to_string();
        let name = c.name.clone();
        c.validate().map_err(|source| TransferError::Invalid { index, name: name.clone(), source })?;
        if let Some(skill) = c.skills.keys().find(|k| !skills.contains(skill_base_id(k))) {
            return Err(TransferError::UnknownSkill { index, name, skill: skill.clone() });
        }
        if let Some(injury) = c.injuries.iter().find(|k| !injuries.contains(*k)) {
            return Err(TransferError::UnknownInjury { index, name, injury: injury.clone() });
        }
    }
    Ok(file.characters)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::data_source::SeedDataSource;

    fn sample() -> Vec<ExportedCharacter> {
        let mut v = Character::quick("V");
        v.skills.insert("handgun".into(), 4);
        v.injuries.push("broken_leg".into());
        vec![
            ExportedCharacter { kind: CharacterKind::Pc, character: v },
            ExportedCharacter { kind: CharacterKind::Npc, character: Character::quick("Boosterganger") },
        ]
    }

    #[test]
    fn export_then_import_roundtrips() {
        let items = sample();
        let json = export(items.clone()).unwrap();
        assert_eq!(parse_import(&json, &SeedDataSource).unwrap(), items);
    }

    #[test]
    fn rejects_foreign_and_broken_files() {
        let d = &SeedDataSource;
        assert!(matches!(parse_import("not json", d), Err(TransferError::Json(_))));
        assert!(matches!(parse_import("{}", d), Err(TransferError::WrongFormat)));
        assert!(matches!(parse_import("[]", d), Err(TransferError::WrongFormat)));

        let empty = export(vec![]).unwrap();
        assert!(matches!(parse_import(&empty, d), Err(TransferError::Empty)));

        let newer = export(sample()).unwrap().replace("\"schema_version\": 1,\n  \"characters\"", "\"schema_version\": 2,\n  \"characters\"");
        assert!(matches!(parse_import(&newer, d), Err(TransferError::UnsupportedVersion(2))));
    }

    #[test]
    fn rejects_invalid_characters_with_their_position() {
        let mut items = sample();
        items[1].character.stats.body = 11;
        let json = export(items).unwrap();
        match parse_import(&json, &SeedDataSource) {
            Err(TransferError::Invalid { index: 2, name, .. }) => assert_eq!(name, "Boosterganger"),
            other => panic!("unexpected: {other:?}"),
        }

        let mut items = sample();
        items[0].character.schema_version = 99;
        let json = export(items).unwrap();
        assert!(matches!(
            parse_import(&json, &SeedDataSource),
            Err(TransferError::Invalid { index: 1, source: CharacterError::UnsupportedVersion(99), .. })
        ));
    }

    #[test]
    fn rejects_unknown_skill_and_injury_ids() {
        let mut items = sample();
        items[0].character.skills.insert("no_such_skill".into(), 1);
        let json = export(items).unwrap();
        assert!(matches!(parse_import(&json, &SeedDataSource), Err(TransferError::UnknownSkill { .. })));

        let mut items = sample();
        items[0].character.injuries.push("no_such_injury".into());
        let json = export(items).unwrap();
        assert!(matches!(parse_import(&json, &SeedDataSource), Err(TransferError::UnknownInjury { .. })));
    }

    #[test]
    fn import_trims_names() {
        let mut items = sample();
        items[0].character.name = "  V  ".into();
        let json = export(items).unwrap();
        assert_eq!(parse_import(&json, &SeedDataSource).unwrap()[0].character.name, "V");
    }

    #[test]
    fn specialty_skill_keys_are_checked_by_their_base_id() {
        let mut items = sample();
        items[0].character.skills.insert("language:native".into(), 6);
        items[0].character.skills.insert("language:street_slang".into(), 3);
        let json = export(items.clone()).unwrap();
        assert_eq!(parse_import(&json, &SeedDataSource).unwrap(), items);

        items[0].character.skills.insert("no_such_skill:x".into(), 1);
        let json = export(items).unwrap();
        assert!(matches!(parse_import(&json, &SeedDataSource), Err(TransferError::UnknownSkill { .. })));
    }
}
