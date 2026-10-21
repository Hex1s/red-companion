//! Single access layer for reference data. The UI and the rules never know
//! whether entries come from the built-in snapshot, a local cache or the API.

use thiserror::Error;

use crate::catalog::{
    Armor, Collection, CollectionVersion, CoverMaterial, CriticalInjury, NpcTemplate, Shield, Skill,
};

#[derive(Debug, Error)]
pub enum DataError {
    #[error("collection {name:?} is malformed: {source}")]
    Malformed { name: &'static str, source: serde_json::Error },
}

pub type DataResult<T> = Result<T, DataError>;

pub trait DataSource {
    fn critical_injuries(&self) -> DataResult<Collection<CriticalInjury>>;
    fn cover_materials(&self) -> DataResult<Collection<CoverMaterial>>;
    fn armor(&self) -> DataResult<Collection<Armor>>;
    fn shields(&self) -> DataResult<Collection<Shield>>;
    fn skills(&self) -> DataResult<Collection<Skill>>;
    fn npc_templates(&self) -> DataResult<Collection<NpcTemplate>>;

    /// Versions of every collection, the "manifest" compared during sync.
    fn versions(&self) -> DataResult<Vec<CollectionVersion>> {
        Ok(vec![
            self.critical_injuries()?.version_info(),
            self.cover_materials()?.version_info(),
            self.armor()?.version_info(),
            self.shields()?.version_info(),
            self.skills()?.version_info(),
            self.npc_templates()?.version_info(),
        ])
    }
}

/// Snapshot compiled into the binary: works offline, no server needed.
#[derive(Debug, Default, Clone, Copy)]
pub struct SeedDataSource;

fn parse<T: serde::de::DeserializeOwned>(
    name: &'static str,
    json: &str,
) -> DataResult<Collection<T>> {
    serde_json::from_str(json).map_err(|source| DataError::Malformed { name, source })
}

impl DataSource for SeedDataSource {
    fn critical_injuries(&self) -> DataResult<Collection<CriticalInjury>> {
        parse("critical_injuries", include_str!("../data/seed/critical_injuries.json"))
    }
    fn cover_materials(&self) -> DataResult<Collection<CoverMaterial>> {
        parse("cover_materials", include_str!("../data/seed/cover_materials.json"))
    }
    fn armor(&self) -> DataResult<Collection<Armor>> {
        parse("armor", include_str!("../data/seed/armor.json"))
    }
    fn shields(&self) -> DataResult<Collection<Shield>> {
        parse("shields", include_str!("../data/seed/shields.json"))
    }
    fn skills(&self) -> DataResult<Collection<Skill>> {
        parse("skills", include_str!("../data/seed/skills.json"))
    }
    fn npc_templates(&self) -> DataResult<Collection<NpcTemplate>> {
        parse("npc_templates", include_str!("../data/seed/npc_templates.json"))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn seed_files_parse_and_names_match() {
        let ds = SeedDataSource;
        assert_eq!(ds.critical_injuries().unwrap().name, "critical_injuries");
        assert_eq!(ds.cover_materials().unwrap().name, "cover_materials");
        assert_eq!(ds.armor().unwrap().name, "armor");
        assert_eq!(ds.shields().unwrap().name, "shields");
        assert_eq!(ds.skills().unwrap().name, "skills");
        assert_eq!(ds.npc_templates().unwrap().name, "npc_templates");
    }

    #[test]
    fn manifest_lists_all_collections() {
        let names: Vec<_> = SeedDataSource.versions().unwrap().into_iter().map(|v| v.name).collect();
        assert_eq!(names, ["critical_injuries", "cover_materials", "armor", "shields", "skills", "npc_templates"]);
    }

    #[test]
    fn works_through_trait_object() {
        let ds: Box<dyn DataSource> = Box::new(SeedDataSource);
        assert!(ds.versions().is_ok());
    }

    #[test]
    fn malformed_json_is_reported() {
        let err = parse::<Armor>("armor", "{").unwrap_err();
        assert!(err.to_string().contains("armor"));
    }

    /// Every i18n key referenced by seed data exists in both locale files.
    #[test]
    fn seed_keys_exist_in_both_locales() {
        let ds = SeedDataSource;
        let mut keys = vec![];
        for e in ds.critical_injuries().unwrap().entries {
            keys.push(e.name_key);
            keys.push(e.effect_key);
        }
        keys.extend(ds.cover_materials().unwrap().entries.into_iter().map(|e| e.name_key));
        keys.extend(ds.armor().unwrap().entries.into_iter().map(|e| e.name_key));
        keys.extend(ds.shields().unwrap().entries.into_iter().map(|e| e.name_key));
        keys.extend(ds.skills().unwrap().entries.into_iter().map(|e| e.name_key));
        for e in ds.npc_templates().unwrap().entries {
            keys.push(e.name_key);
            keys.push(e.armor_name_key);
            keys.push(e.gear_key);
            keys.extend(e.notes_key);
            keys.extend(e.weapons.into_iter().map(|w| w.name_key));
        }

        for lang in ["ru", "en"] {
            let path = format!("{}/../../src/i18n/locales/{lang}.json", env!("CARGO_MANIFEST_DIR"));
            let root: serde_json::Value =
                serde_json::from_str(&std::fs::read_to_string(path).unwrap()).unwrap();
            for key in &keys {
                let found = key.split('.').try_fold(&root, |v, part| v.get(part));
                assert!(found.is_some_and(|v| v.is_string()), "{lang}: missing {key}");
            }
        }
    }

    #[test]
    fn seed_data_passes_invariants() {
        use crate::catalog::*;
        let ds = SeedDataSource;
        assert_eq!(validate_critical_injuries(&ds.critical_injuries().unwrap()), Ok(()));
        assert_eq!(validate_cover_materials(&ds.cover_materials().unwrap()), Ok(()));
        assert_eq!(validate_armor(&ds.armor().unwrap()), Ok(()));
        assert_eq!(validate_shields(&ds.shields().unwrap()), Ok(()));
        assert_eq!(validate_skills(&ds.skills().unwrap()), Ok(()));
        assert_eq!(
            validate_npc_templates(&ds.npc_templates().unwrap(), &ds.skills().unwrap()),
            Ok(())
        );
    }

    /// Skill categories have a name in both locales, like the entries do.
    #[test]
    fn skill_categories_exist_in_both_locales() {
        for lang in ["ru", "en"] {
            let path = format!("{}/../../src/i18n/locales/{lang}.json", env!("CARGO_MANIFEST_DIR"));
            let root: serde_json::Value =
                serde_json::from_str(&std::fs::read_to_string(path).unwrap()).unwrap();
            for cat in ["perception", "body", "control", "education", "fighting", "performance", "ranged_weapon", "social", "technique"] {
                assert!(root["skill"]["category"][cat].is_string(), "{lang}: {cat}");
            }
        }
    }

    /// Spot checks of skills against the book (p. 131-142), per category counts
    /// and the (x2) marks.
    #[test]
    fn skills_match_book() {
        use crate::catalog::*;
        let skills = SeedDataSource.skills().unwrap();
        let count = |c: SkillCategory| skills.entries.iter().filter(|e| e.category == c).count();
        assert_eq!(
            [
                count(SkillCategory::Perception),
                count(SkillCategory::Body),
                count(SkillCategory::Control),
                count(SkillCategory::Education),
                count(SkillCategory::Fighting),
                count(SkillCategory::Performance),
                count(SkillCategory::RangedWeapon),
                count(SkillCategory::Social),
                count(SkillCategory::Technique),
            ],
            [5, 6, 4, 16, 4, 2, 5, 9, 15]
        );
        let mut doubled: Vec<_> =
            skills.entries.iter().filter(|e| e.double_cost).map(|e| e.id.as_str()).collect();
        doubled.sort_unstable();
        assert_eq!(
            doubled,
            ["autofire", "demolitions", "electronics_security_tech", "heavy_weapons", "martial_arts", "paramedic", "pilot_air_vehicle"]
        );
        let mut with_specialty: Vec<_> =
            skills.entries.iter().filter(|e| e.specialty).map(|e| e.id.as_str()).collect();
        with_specialty.sort_unstable();
        assert_eq!(with_specialty, ["language", "local_expert", "martial_arts", "play_instrument", "science"]);
        let stat = |id: &str| skills.entries.iter().find(|e| e.id == id).unwrap().stat;
        assert_eq!(stat("concentration"), StatKey::Will);
        assert_eq!(stat("first_aid"), StatKey::Tech);
        assert_eq!(stat("handgun"), StatKey::Reflex);
        // The Russian edition links pick pocket to TECH (p. 142), unlike the original.
        assert_eq!(stat("pick_pocket"), StatKey::Tech);
    }

    /// Spot checks against the book tables (p. 182, 184-185, 187-188).
    #[test]
    fn seed_values_match_book() {
        use crate::catalog::*;
        let ds = SeedDataSource;
        let cover = ds.cover_materials().unwrap();
        let hp = |id: &str| cover.entries.iter().find(|e| e.id == id).unwrap().hp;
        assert_eq!((hp("concrete_thick"), hp("wood_thin"), hp("metal_thick")), (25, 5, 50));
        assert!(cover.entries.iter().all(|e| e.id != "drywall_thin"));

        let armor = ds.armor().unwrap();
        let a = |id: &str| armor.entries.iter().find(|e| e.id == id).unwrap().clone();
        assert_eq!((a("kevlar").sp, a("kevlar").penalty), (7, 0));
        assert_eq!((a("metalgear").sp, a("metalgear").penalty), (18, 4));

        let shields = ds.shields().unwrap();
        assert_eq!(shields.entries[0].hp, Some(10));
        assert_eq!(shields.entries[1].hp, None);

        let inj = ds.critical_injuries().unwrap();
        let broken_leg = inj.entries.iter().find(|e| e.id == "broken_leg").unwrap();
        assert_eq!((broken_leg.area, broken_leg.roll), (BodyArea::Body, 8));
        assert_eq!(broken_leg.first_aid, [TreatmentOption { skill: TreatmentSkill::Paramedic, dv: 13 }]);
        assert_eq!(broken_leg.treatment.len(), 2);
        let dsp = inj.entries.iter().filter(|e| e.death_save_penalty).count();
        assert_eq!(dsp, 11); // 5 body + 6 head rows
    }

    /// NPC templates against the book (p. 412-416): derived values printed in
    /// the stat blocks, tiers, and the cloned sheet.
    #[test]
    fn npc_templates_match_book() {
        use crate::catalog::*;
        use crate::damage::max_hp;
        let ds = SeedDataSource;
        let npcs = ds.npc_templates().unwrap();
        let skills = ds.skills().unwrap();
        assert_eq!(npcs.entries.len(), 10);

        // (id, HP, serious wound threshold, death save) as printed.
        let printed = [
            ("bodyguard", 35, 18, 6),
            ("booster", 20, 10, 2),
            ("road_bandit", 25, 13, 3),
            ("security_operative", 30, 15, 5),
            ("netrunner", 30, 15, 3),
            ("chief_restorer", 40, 20, 6),
            ("security_officer", 40, 20, 7),
            ("patrol", 40, 20, 6),
            ("pyrotechnician", 35, 18, 5),
            ("cyberpsychopath", 55, 28, 10),
        ];
        for (id, hp, serious, save) in printed {
            let t = npcs.entries.iter().find(|e| e.id == id).unwrap();
            let derived = max_hp(t.stats.body, t.stats.will);
            assert_eq!((derived, derived.div_ceil(2), u32::from(t.stats.body)), (hp, serious, save), "{id}");
            assert_eq!(t.stats.luck, 0, "{id}: LUCK is a dash in the book");
            let c = t.to_character("x", "", &skills.entries);
            assert_eq!((c.validate(), c.hp), (Ok(()), hp), "{id}");
        }

        let tier = |id: &str| npcs.entries.iter().find(|e| e.id == id).unwrap().tier;
        assert_eq!(tier("bodyguard"), NpcTier::Mook);
        assert_eq!(tier("netrunner"), NpcTier::Lieutenant);
        assert_eq!(tier("pyrotechnician"), NpcTier::MiniBoss);
        assert_eq!(tier("cyberpsychopath"), NpcTier::Boss);

        // Armor that is a catalog item has the catalog's SP.
        let armor = ds.armor().unwrap();
        for t in &npcs.entries {
            if let Some(a) = armor.entries.iter().find(|a| a.name_key == t.armor_name_key) {
                assert_eq!((t.armor_head_sp, t.armor_body_sp), (a.sp, a.sp), "{}", t.id);
            }
        }

        // Level = printed sum - linked stat: stat + level gives the printed sum
        // again, except for the one printed penalty (cyberpsychopath concentration).
        let mut below = vec![];
        for t in &npcs.entries {
            let levels = t.skill_levels(&skills.entries);
            for (key, &total) in &t.skill_totals {
                let id = crate::character::skill_base_id(key);
                let stat = skills.entries.iter().find(|s| s.id == id).unwrap().stat.value_in(&t.stats);
                if stat + levels[key] != total {
                    below.push(format!("{}:{key}", t.id));
                }
            }
        }
        assert_eq!(below, ["cyberpsychopath:concentration"]);

        // Spot checks against the images.
        let bodyguard = npcs.entries.iter().find(|e| e.id == "bodyguard").unwrap();
        assert_eq!(bodyguard.skill_totals["handgun"], 10);
        assert_eq!(bodyguard.skill_levels(&skills.entries)["handgun"], 4); // REF 6
        let officer = npcs.entries.iter().find(|e| e.id == "security_officer").unwrap();
        assert_eq!((officer.stats.reflex, officer.armor_body_sp), (8, 12));
    }
}
