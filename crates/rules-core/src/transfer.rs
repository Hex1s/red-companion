//! Export and import of characters as a versioned JSON file.
//!
//! The file is an envelope: `format` tells it apart from other JSON,
//! `schema_version` is the version of the envelope (each character carries its
//! own `schema_version` too). Import validates everything before anything is
//! stored, so a bad file never leaves half of its characters behind.

use std::collections::HashSet;

use serde::{Deserialize, Serialize};
use ts_rs::TS;
use thiserror::Error;

use crate::character::{skill_base_id, Character, CharacterError, CharacterKind};
use crate::data_source::{DataError, DataSource};

/// Value of the `format` field.
pub const FORMAT: &str = "red-companion/characters";
/// The `format` value of files written under the name Red Companion; still accepted.
const LEGACY_FORMAT: &str = "favoterm/characters";
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

/// An export file found in the exports folder.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct ExportFile {
    pub name: String,
    /// Last modification, milliseconds since the Unix epoch.
    #[ts(type = "number")]
    pub modified_ms: u64,
    #[ts(type = "number")]
    pub size: u64,
}

/// Where an export went.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct ExportSaved {
    pub name: String,
    /// Full path of the new file, for the message.
    pub path: String,
    pub count: u32,
}

/// Every export file ends in this; the type of the file stands before it (`ivan.character.…`).
pub const FILE_EXT: &str = ".redcompanion.json";
/// The extension of files written under the name Red Companion; such files can still be imported
/// (`ivan.character.favoterm.json`). Files from before types are `name.redcompanion.json` (the
/// current extension without a type) or `name.favoterm.json` in the exports folder itself.
const LEGACY_FILE_EXT: &str = ".favoterm.json";
/// What the app wrote for encounters before files carried their type.
const LEGACY_ENCOUNTERS_SUFFIX: &str = ".redencounters.json";

/// What an export file holds. The type is part of the file name and of the folder it lives in,
/// so an import window offers only the files that belong to it.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "snake_case")]
#[ts(export)]
pub enum ExportKind {
    /// One character (or several picked by hand).
    Character,
    /// The participants of a session.
    Session,
    /// Encounters with the own NPC templates they use.
    Encounters,
}

impl ExportKind {
    pub const ALL: [ExportKind; 3] = [Self::Character, Self::Session, Self::Encounters];

    /// The word before the extension: `ivan.character.redcompanion.json`.
    pub fn infix(self) -> &'static str {
        match self {
            Self::Character => "character",
            Self::Session => "session",
            Self::Encounters => "encounters",
        }
    }

    /// Sub-folder of the exports folder.
    pub fn folder(self) -> &'static str {
        match self {
            Self::Character => "characters",
            Self::Session => "sessions",
            Self::Encounters => "encounters",
        }
    }

    /// Name base when there is no name to take (an empty name, only dots...).
    fn fallback(self) -> &'static str {
        self.folder()
    }

    /// The end of every file name of this type: `.character.redcompanion.json`.
    pub fn suffix(self) -> String {
        format!(".{}{FILE_EXT}", self.infix())
    }

    /// The same for a file written under the name Favoterm: `.character.favoterm.json`.
    fn legacy_suffix(self) -> String {
        format!(".{}{LEGACY_FILE_EXT}", self.infix())
    }
}

/// Longest slug of a name in a file name.
const SLUG_MAX: usize = 60;

/// `Джонни Сильверхенд` becomes `dzhonni-sil-verkhend`: lower case, ASCII, dashes, safe on any file
/// system. `None` when nothing is left of the name.
pub fn slug_of(name: &str) -> Option<String> {
    let slug = slug::slugify(name);
    let slug: String = slug.chars().take(SLUG_MAX).collect();
    let slug = slug.trim_matches('-').to_string();
    (!slug.is_empty()).then_some(slug)
}

/// File name of an export: the slug of `base` (or a fixed word for an empty one), the type, the
/// extension.
pub fn typed_file_name(kind: ExportKind, base: &str) -> String {
    let slug = slug_of(base).unwrap_or_else(|| kind.fallback().to_string());
    format!("{slug}{}", kind.suffix())
}

/// A bare name: no folders, no `..`, no reserved characters, no leading dot.
fn is_bare(name: &str) -> bool {
    !name.is_empty()
        && !name.chars().any(|c| c == '\0' || c.is_control() || "\\/:*?\"<>|".contains(c))
        && !name.starts_with('.')
        && !name.contains("..")
}

/// A name taken from the UI is only accepted as a bare file name of this type.
pub fn is_typed_file_name(kind: ExportKind, name: &str) -> bool {
    let ends_with_stem = |suffix: String| name.len() > suffix.len() && name.ends_with(&suffix);
    is_bare(name) && (ends_with_stem(kind.suffix()) || ends_with_stem(kind.legacy_suffix()))
}

/// A file from before types: `name.redcompanion.json` or `name.favoterm.json` (characters), `name.redencounters.json`
/// (encounters), lying in the exports folder itself.
pub fn is_legacy_file_name(kind: ExportKind, name: &str) -> bool {
    if !is_bare(name) {
        return false;
    }
    match kind {
        ExportKind::Encounters => name.ends_with(LEGACY_ENCOUNTERS_SUFFIX),
        ExportKind::Character => {
            (name.ends_with(FILE_EXT) || name.ends_with(LEGACY_FILE_EXT))
                && !ExportKind::ALL.iter().any(|k| is_typed_file_name(*k, name))
        }
        ExportKind::Session => false,
    }
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
    if !matches!(value.get("format").and_then(|f| f.as_str()), Some(FORMAT | LEGACY_FORMAT)) {
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
    let roles = data.roles()?.entries;

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
        crate::roles::validate_against(&c.roles, &roles)
            .map_err(|e| TransferError::Invalid { index, name, source: e.into() })?;
    }
    Ok(file.characters)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::data_source::SeedDataSource;

    #[test]
    fn files_written_under_the_old_name_are_still_recognised_and_imported() {
        // The format a file carries and the name it is saved under are the new ones...
        let json = export(sample()).unwrap();
        assert!(json.contains(r#""format": "red-companion/characters""#));
        assert_eq!(typed_file_name(ExportKind::Character, "V"), "v.character.redcompanion.json");

        // ...but a file made by the app before it was renamed still lists and imports.
        assert!(is_typed_file_name(ExportKind::Character, "v.character.favoterm.json"));
        assert!(is_typed_file_name(ExportKind::Session, "night.session.favoterm.json"));
        assert!(!is_typed_file_name(ExportKind::Character, "v.session.favoterm.json"));
        let old = json.replacen("red-companion/characters", "favoterm/characters", 1);
        let items = parse_import(&old, &SeedDataSource::default()).unwrap();
        assert_eq!(items.len(), sample().len());
        let foreign = json.replacen("red-companion/characters", "other-app/characters", 1);
        assert!(matches!(parse_import(&foreign, &SeedDataSource::default()), Err(TransferError::WrongFormat)));
    }

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
        assert_eq!(parse_import(&json, &SeedDataSource::default()).unwrap(), items);
    }

    #[test]
    fn rejects_foreign_and_broken_files() {
        let d = &SeedDataSource::default();
        assert!(matches!(parse_import("not json", d), Err(TransferError::Json(_))));
        assert!(matches!(parse_import("{}", d), Err(TransferError::WrongFormat)));
        assert!(matches!(parse_import("[]", d), Err(TransferError::WrongFormat)));

        let empty = export(vec![]).unwrap();
        assert!(matches!(parse_import(&empty, d), Err(TransferError::Empty)));

        let newer = export(sample()).unwrap().replace("\"schema_version\": 1,\n  \"characters\"", "\"schema_version\": 2,\n  \"characters\"");
        assert!(matches!(parse_import(&newer, d), Err(TransferError::UnsupportedVersion(2))));
    }

    #[test]
    fn rejects_unknown_roles_and_overspent_ranks() {
        let mut items = sample();
        items[0].character.roles.push(crate::roles::RoleEntry::new("plumber", 4));
        let json = export(items).unwrap();
        assert!(matches!(
            parse_import(&json, &SeedDataSource::default()),
            Err(TransferError::Invalid { index: 1, source: CharacterError::Role(_), .. })
        ));

        let mut items = sample();
        let mut medtech = crate::roles::RoleEntry::new("medtech", 2);
        medtech.points.insert("surgery".into(), 3);
        items[0].character.roles.push(medtech);
        let json = export(items).unwrap();
        assert!(parse_import(&json, &SeedDataSource::default()).is_err());
    }

    #[test]
    fn rejects_invalid_characters_with_their_position() {
        let mut items = sample();
        items[1].character.stats.body = 100;
        let json = export(items).unwrap();
        match parse_import(&json, &SeedDataSource::default()) {
            Err(TransferError::Invalid { index: 2, name, .. }) => assert_eq!(name, "Boosterganger"),
            other => panic!("unexpected: {other:?}"),
        }

        let mut items = sample();
        items[0].character.schema_version = 99;
        let json = export(items).unwrap();
        assert!(matches!(
            parse_import(&json, &SeedDataSource::default()),
            Err(TransferError::Invalid { index: 1, source: CharacterError::UnsupportedVersion(99), .. })
        ));
    }

    #[test]
    fn rejects_unknown_skill_and_injury_ids() {
        let mut items = sample();
        items[0].character.skills.insert("no_such_skill".into(), 1);
        let json = export(items).unwrap();
        assert!(matches!(parse_import(&json, &SeedDataSource::default()), Err(TransferError::UnknownSkill { .. })));

        let mut items = sample();
        items[0].character.injuries.push("no_such_injury".into());
        let json = export(items).unwrap();
        assert!(matches!(parse_import(&json, &SeedDataSource::default()), Err(TransferError::UnknownInjury { .. })));
    }

    #[test]
    fn import_trims_names() {
        let mut items = sample();
        items[0].character.name = "  V  ".into();
        let json = export(items).unwrap();
        assert_eq!(parse_import(&json, &SeedDataSource::default()).unwrap()[0].character.name, "V");
    }

    #[test]
    fn specialty_skill_keys_are_checked_by_their_base_id() {
        let mut items = sample();
        items[0].character.skills.insert("language:native".into(), 6);
        items[0].character.skills.insert("language:street_slang".into(), 3);
        let json = export(items.clone()).unwrap();
        assert_eq!(parse_import(&json, &SeedDataSource::default()).unwrap(), items);

        items[0].character.skills.insert("no_such_skill:x".into(), 1);
        let json = export(items).unwrap();
        assert!(matches!(parse_import(&json, &SeedDataSource::default()), Err(TransferError::UnknownSkill { .. })));
    }

    #[test]
    fn export_file_names_carry_a_slug_and_the_type() {
        use ExportKind::*;
        assert_eq!(typed_file_name(Character, "Джонни Сильверхенд"), "dzhonni-sil-verkhend.character.redcompanion.json");
        assert_eq!(typed_file_name(Session, "Сессия 1"), "sessiia-1.session.redcompanion.json");
        assert_eq!(typed_file_name(Encounters, "Бандиты у склада!"), "bandity-u-sklada.encounters.redcompanion.json");
        assert_eq!(typed_file_name(Character, "a/b:c*"), "a-b-c.character.redcompanion.json");
        // Nothing left of the name: a fixed word, never an empty stem.
        assert_eq!(typed_file_name(Character, "  .. "), "characters.character.redcompanion.json");
        assert_eq!(typed_file_name(Session, ""), "sessions.session.redcompanion.json");
        assert_eq!(typed_file_name(Encounters, "???"), "encounters.encounters.redcompanion.json");
        assert!(typed_file_name(Character, &"я".repeat(300)).len() < 100);
    }

    #[test]
    fn imports_accept_only_bare_names_of_their_own_type() {
        use ExportKind::*;
        assert!(is_typed_file_name(Character, "ivan.character.redcompanion.json"));
        assert!(is_typed_file_name(Character, "ivan-2.character.redcompanion.json"));
        assert!(!is_typed_file_name(Character, "ivan.session.redcompanion.json"), "a session file is not a character");
        assert!(!is_typed_file_name(Session, "ivan.character.redcompanion.json"));
        assert!(!is_typed_file_name(Encounters, "ivan.character.redcompanion.json"));
        assert!(!is_typed_file_name(Character, ".character.redcompanion.json"), "no stem");
        for bad in [
            "",
            "x.json",
            "../x.character.redcompanion.json",
            "a/x.character.redcompanion.json",
            "a\\x.character.redcompanion.json",
            "C:x.character.redcompanion.json",
            ".hidden.character.redcompanion.json",
            "a..b.character.redcompanion.json",
        ] {
            assert!(!is_typed_file_name(Character, bad), "{bad}");
            assert!(!is_legacy_file_name(Character, bad), "{bad}");
        }
    }

    #[test]
    fn files_from_before_types_are_still_known() {
        use ExportKind::*;
        assert!(is_legacy_file_name(Character, "Джонни.favoterm.json"));
        assert!(is_legacy_file_name(Character, "Johnny (2).favoterm.json"));
        // The current extension without a type is also a file from before types.
        assert!(is_legacy_file_name(Character, "Johnny.redcompanion.json"));
        assert!(!is_legacy_file_name(Character, "Johnny.character.redcompanion.json"));
        assert!(!is_legacy_file_name(Character, "ivan.session.favoterm.json"), "typed files are not legacy");
        assert!(!is_legacy_file_name(Character, "ivan.character.favoterm.json"));
        assert!(is_legacy_file_name(Encounters, "Засада.redencounters.json"));
        assert!(!is_legacy_file_name(Session, "Johnny.favoterm.json"));
    }
}
