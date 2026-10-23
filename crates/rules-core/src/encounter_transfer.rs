//! Export and import of the GM's own NPC templates and encounters as a versioned JSON file.
//!
//! The file has its own format and type in its name (`name.encounters.redcompanion.json`), apart
//! from the characters files. An encounter that uses one of the GM's own templates travels together with it: the
//! template is stored in the same file under a file-local `key`, and the encounter line points
//! at that key. Book templates are referenced by catalog `code` (every install has them).
//! Import validates everything before anything is stored.

use std::collections::{HashMap, HashSet};

use serde::{Deserialize, Serialize};
use thiserror::Error;
use ts_rs::TS;

use crate::catalog::NpcTier;
use crate::character::{skill_base_id, Character, CharacterError};
use crate::data_source::{DataError, DataSource};
use crate::encounter::{Encounter, EncounterError, EncounterRecord, TemplateSource, UserTemplateRecord};

/// Value of the `format` field.
pub const FORMAT: &str = "red-companion/encounters";
/// Current version of the file.
pub const FILE_VERSION: u32 = 1;
/// Most templates and encounters one file may hold (a sanity limit against runaway files).
pub const ITEMS_MAX: usize = 500;

/// An own template as it sits in the file; `key` is only meaningful inside this file.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct FileTemplate {
    pub key: String,
    pub tier: NpcTier,
    pub character: Character,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct EncountersFile {
    pub format: String,
    pub schema_version: u32,
    #[serde(default)]
    pub templates: Vec<FileTemplate>,
    #[serde(default)]
    pub encounters: Vec<Encounter>,
}

#[derive(Debug, Error)]
pub enum EncounterTransferError {
    #[error("not a valid JSON file: {0}")]
    Json(#[from] serde_json::Error),
    #[error("this is not a Red Companion encounters file")]
    WrongFormat,
    #[error("unsupported file version {0}")]
    UnsupportedVersion(u32),
    #[error("the file contains no templates and no encounters")]
    Empty,
    #[error("the file holds more than {ITEMS_MAX} templates or encounters")]
    TooMany,
    #[error("template key {0:?} appears twice")]
    DuplicateKey(String),
    #[error("template {name:?}: {source}")]
    InvalidTemplate { name: String, source: CharacterError },
    #[error("template {name:?}: unknown skill {skill:?}")]
    UnknownSkill { name: String, skill: String },
    #[error("template {name:?}: unknown injury {injury:?}")]
    UnknownInjury { name: String, injury: String },
    #[error("encounter {name:?}: {source}")]
    InvalidEncounter { name: String, source: EncounterError },
    #[error("encounter {name:?}: own template {key:?} is not in the file")]
    MissingTemplate { name: String, key: String },
    #[error("encounter {name:?}: no book NPC with code {code:?}")]
    UnknownBookTemplate { name: String, code: String },
    #[error(transparent)]
    Data(#[from] DataError),
}

/// Where an export went.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct EncountersSaved {
    pub name: String,
    /// Full path of the new file, for the message.
    pub path: String,
    pub encounters: u32,
    pub templates: u32,
}

/// What an import added.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct EncountersImported {
    pub templates: Vec<UserTemplateRecord>,
    pub encounters: Vec<EncounterRecord>,
}

/// Builds the file. `templates` are `(id, tier, sheet)` of own templates; the id becomes the key.
/// `encounters` have their own-template lines pointing at those ids (`Custom`, id as text).
pub fn export(
    templates: Vec<(i64, NpcTier, Character)>,
    encounters: Vec<Encounter>,
) -> Result<String, EncounterTransferError> {
    let file = EncountersFile {
        format: FORMAT.to_string(),
        schema_version: FILE_VERSION,
        templates: templates
            .into_iter()
            .map(|(id, tier, character)| FileTemplate { key: id.to_string(), tier, character })
            .collect(),
        encounters,
    };
    Ok(serde_json::to_string_pretty(&file)?)
}

/// Own templates an encounter list refers to (ids, in first-use order, no repeats).
pub fn used_template_ids(encounters: &[Encounter]) -> Vec<i64> {
    let mut seen = HashSet::new();
    let mut ids = Vec::new();
    for e in encounters.iter().flat_map(|e| &e.entries) {
        if e.source == TemplateSource::Custom {
            if let Ok(id) = e.template.parse::<i64>() {
                if seen.insert(id) {
                    ids.push(id);
                }
            }
        }
    }
    ids
}

/// Parses and fully validates an encounters file. Names are trimmed; skill and injury ids and
/// book codes must exist in the reference data; every own template an encounter uses must be
/// in the file.
pub fn parse_import(
    json: &str,
    data: &impl DataSource,
) -> Result<EncountersFile, EncounterTransferError> {
    // Loose check first: a file from a newer version may have another shape.
    let value: serde_json::Value = serde_json::from_str(json)?;
    if value.get("format").and_then(|f| f.as_str()) != Some(FORMAT) {
        return Err(EncounterTransferError::WrongFormat);
    }
    if let Some(version) = value.get("schema_version").and_then(|v| v.as_u64()) {
        if version != u64::from(FILE_VERSION) {
            return Err(EncounterTransferError::UnsupportedVersion(
                version.try_into().unwrap_or(u32::MAX),
            ));
        }
    }
    let mut file: EncountersFile = serde_json::from_value(value)?;
    if file.templates.is_empty() && file.encounters.is_empty() {
        return Err(EncounterTransferError::Empty);
    }
    if file.templates.len() > ITEMS_MAX || file.encounters.len() > ITEMS_MAX {
        return Err(EncounterTransferError::TooMany);
    }

    let skills: HashSet<String> = data.skills()?.entries.into_iter().map(|s| s.id).collect();
    let injuries: HashSet<String> =
        data.critical_injuries()?.entries.into_iter().map(|i| i.id).collect();
    let roles = data.roles()?.entries;
    let book: HashSet<String> = data.npc_templates()?.entries.into_iter().map(|n| n.code).collect();

    let mut keys = HashSet::new();
    for t in &mut file.templates {
        if !keys.insert(t.key.clone()) {
            return Err(EncounterTransferError::DuplicateKey(t.key.clone()));
        }
        let c = &mut t.character;
        c.name = c.name.trim().to_string();
        let name = c.name.clone();
        c.validate()
            .map_err(|source| EncounterTransferError::InvalidTemplate { name: name.clone(), source })?;
        if let Some(skill) = c.skills.keys().find(|k| !skills.contains(skill_base_id(k))) {
            return Err(EncounterTransferError::UnknownSkill { name, skill: skill.clone() });
        }
        if let Some(injury) = c.injuries.iter().find(|k| !injuries.contains(*k)) {
            return Err(EncounterTransferError::UnknownInjury { name, injury: injury.clone() });
        }
        crate::roles::validate_against(&c.roles, &roles).map_err(|e| {
            EncounterTransferError::InvalidTemplate { name, source: e.into() }
        })?;
    }

    for e in &mut file.encounters {
        // `normalize` wants own-template ids to be numbers; in the file they are plain keys,
        // so the numeric check is the file writer's promise, not the author's.
        e.normalize().map_err(|source| EncounterTransferError::InvalidEncounter {
            name: e.name.clone(),
            source,
        })?;
        for line in &e.entries {
            match line.source {
                TemplateSource::Custom if !keys.contains(&line.template) => {
                    return Err(EncounterTransferError::MissingTemplate {
                        name: e.name.clone(),
                        key: line.template.clone(),
                    });
                }
                TemplateSource::Book if !book.contains(&line.template) => {
                    return Err(EncounterTransferError::UnknownBookTemplate {
                        name: e.name.clone(),
                        code: line.template.clone(),
                    });
                }
                _ => {}
            }
        }
    }
    Ok(file)
}

/// Rewrites own-template lines from file keys to the ids the templates got on import.
pub fn remap_templates(encounter: &mut Encounter, new_ids: &HashMap<String, i64>) {
    for line in &mut encounter.entries {
        if line.source == TemplateSource::Custom {
            if let Some(id) = new_ids.get(&line.template) {
                line.template = id.to_string();
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::data_source::SeedDataSource;
    use crate::encounter::EncounterEntry;

    fn line(source: TemplateSource, template: &str, count: u32) -> EncounterEntry {
        EncounterEntry { source, template: template.to_string(), count }
    }

    fn encounter(name: &str, entries: Vec<EncounterEntry>) -> Encounter {
        Encounter { name: name.to_string(), notes: "n".into(), entries }
    }

    fn book_code() -> String {
        SeedDataSource::default().npc_templates().unwrap().entries[0].code.clone()
    }

    fn sample() -> (Vec<(i64, NpcTier, Character)>, Vec<Encounter>) {
        let mut thug = Character::quick("Thug");
        thug.skills.insert("handgun".into(), 4);
        (
            vec![(7, NpcTier::Lieutenant, thug)],
            vec![encounter(
                "Ambush",
                vec![
                    line(TemplateSource::Book, &book_code(), 2),
                    line(TemplateSource::Custom, "7", 3),
                ],
            )],
        )
    }

    #[test]
    fn export_then_import_roundtrips() {
        let (templates, encounters) = sample();
        let json = export(templates.clone(), encounters.clone()).unwrap();
        let file = parse_import(&json, &SeedDataSource::default()).unwrap();
        assert_eq!(file.encounters, encounters);
        assert_eq!(file.templates.len(), 1);
        assert_eq!(file.templates[0].key, "7");
        assert_eq!(file.templates[0].tier, NpcTier::Lieutenant);
        assert_eq!(file.templates[0].character, templates[0].2);
    }

    #[test]
    fn templates_only_and_encounters_only_files_are_fine() {
        let d = &SeedDataSource::default();
        let (templates, _) = sample();
        let only_templates = export(templates, vec![]).unwrap();
        assert_eq!(parse_import(&only_templates, d).unwrap().encounters.len(), 0);

        let book_only = export(
            vec![],
            vec![encounter("Book", vec![line(TemplateSource::Book, &book_code(), 1)])],
        )
        .unwrap();
        assert_eq!(parse_import(&book_only, d).unwrap().templates.len(), 0);
    }

    #[test]
    fn rejects_foreign_empty_and_newer_files() {
        let d = &SeedDataSource::default();
        assert!(matches!(parse_import("nope", d), Err(EncounterTransferError::Json(_))));
        assert!(matches!(parse_import("{}", d), Err(EncounterTransferError::WrongFormat)));
        // A characters file is not an encounters file.
        let characters = r#"{"format":"red-companion/characters","schema_version":1,"characters":[]}"#;
        assert!(matches!(parse_import(characters, d), Err(EncounterTransferError::WrongFormat)));

        let empty = export(vec![], vec![]).unwrap();
        assert!(matches!(parse_import(&empty, d), Err(EncounterTransferError::Empty)));

        let (t, e) = sample();
        let newer = export(t, e).unwrap().replacen("\"schema_version\": 1", "\"schema_version\": 2", 1);
        assert!(matches!(parse_import(&newer, d), Err(EncounterTransferError::UnsupportedVersion(2))));
    }

    #[test]
    fn rejects_an_encounter_whose_own_template_is_missing_from_the_file() {
        let (_, encounters) = sample();
        let json = export(vec![], encounters).unwrap();
        assert!(matches!(
            parse_import(&json, &SeedDataSource::default()),
            Err(EncounterTransferError::MissingTemplate { key, .. }) if key == "7"
        ));
    }

    #[test]
    fn rejects_unknown_book_code_bad_counts_and_bad_sheets() {
        let d = &SeedDataSource::default();
        let json = export(vec![], vec![encounter("X", vec![line(TemplateSource::Book, "no_such_npc", 1)])]).unwrap();
        assert!(matches!(parse_import(&json, d), Err(EncounterTransferError::UnknownBookTemplate { .. })));

        let json = export(vec![], vec![encounter("X", vec![line(TemplateSource::Book, &book_code(), 0)])]).unwrap();
        assert!(matches!(parse_import(&json, d), Err(EncounterTransferError::InvalidEncounter { .. })));

        let (mut t, e) = sample();
        t[0].2.stats.body = 100;
        let json = export(t, e).unwrap();
        assert!(matches!(parse_import(&json, d), Err(EncounterTransferError::InvalidTemplate { .. })));

        let (mut t, e) = sample();
        t[0].2.skills.insert("no_such_skill".into(), 1);
        let json = export(t, e).unwrap();
        assert!(matches!(parse_import(&json, d), Err(EncounterTransferError::UnknownSkill { .. })));

        let (mut t, e) = sample();
        t[0].2.injuries.push("no_such_injury".into());
        let json = export(t, e).unwrap();
        assert!(matches!(parse_import(&json, d), Err(EncounterTransferError::UnknownInjury { .. })));
    }

    #[test]
    fn rejects_duplicate_keys() {
        let (mut t, e) = sample();
        t.push(t[0].clone());
        let json = export(t, e).unwrap();
        assert!(matches!(
            parse_import(&json, &SeedDataSource::default()),
            Err(EncounterTransferError::DuplicateKey(_))
        ));
    }

    #[test]
    fn remap_rewrites_only_own_lines() {
        let (_, mut e) = sample();
        let new_ids = HashMap::from([("7".to_string(), 42)]);
        remap_templates(&mut e[0], &new_ids);
        assert_eq!(e[0].entries[0].template, book_code());
        assert_eq!(e[0].entries[1].template, "42");
    }

    #[test]
    fn used_templates_are_listed_once_in_order() {
        let a = encounter(
            "A",
            vec![
                line(TemplateSource::Custom, "9", 1),
                line(TemplateSource::Book, "x", 1),
                line(TemplateSource::Custom, "3", 1),
            ],
        );
        let b = encounter("B", vec![line(TemplateSource::Custom, "9", 2)]);
        assert_eq!(used_template_ids(&[a, b]), vec![9, 3]);
    }
}
