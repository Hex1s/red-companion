//! Single access layer for reference data. The UI and the rules never know
//! whether entries come from the built-in snapshot, a local cache or the API.

use serde::{Deserialize, Serialize};
use thiserror::Error;
use ts_rs::TS;

use crate::catalog::{
    Ammo, Armor, Article, ClothingStyle, Collection, CollectionVersion, CoverMaterial, CriticalInjury, GearItem, MagazineRow, MartialArt, NpcTemplate,
    PriceCategory, RangeTable, Role, Shield, Skill, Specialty, StreetDrug, Weapon, WeaponMod,
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
    fn weapons(&self) -> DataResult<Collection<Weapon>>;
    fn skills(&self) -> DataResult<Collection<Skill>>;
    fn npc_templates(&self) -> DataResult<Collection<NpcTemplate>>;
    fn roles(&self) -> DataResult<Collection<Role>>;
    fn martial_arts(&self) -> DataResult<Collection<MartialArt>>;
    /// Rules summaries (collection `rules`).
    fn articles(&self) -> DataResult<Collection<Article>>;
    /// DV by distance and weapon class (collection `range_tables`, core).
    fn range_tables(&self) -> DataResult<Collection<RangeTable>>;
    /// General gear (collection `gear`, p. 351-355).
    fn gear(&self) -> DataResult<Collection<GearItem>>;
    /// Ammunition types (collection `ammo`, p. 345-347).
    fn ammo(&self) -> DataResult<Collection<Ammo>>;
    /// Weapon modifications (collection `weapon_mods`, p. 343-344).
    fn weapon_mods(&self) -> DataResult<Collection<WeaponMod>>;
    /// Magazine sizes (collection `magazines`, core, p. 344).
    fn magazines(&self) -> DataResult<Collection<MagazineRow>>;
    /// Price categories (collection `price_categories`, core, p. 385).
    fn price_categories(&self) -> DataResult<Collection<PriceCategory>>;
    /// Suggested specialties of skills: languages and instruments (collection `specialties`).
    fn specialties(&self) -> DataResult<Collection<Specialty>>;
    /// Clothing styles with prices (collection `clothing`, p. 356).
    fn clothing(&self) -> DataResult<Collection<ClothingStyle>>;
    /// Street drugs (collection `street_drugs`, p. 357-358).
    fn street_drugs(&self) -> DataResult<Collection<StreetDrug>>;

    /// Versions of every collection, the "manifest" compared during sync.
    fn versions(&self) -> DataResult<Vec<CollectionVersion>> {
        Ok(vec![
            self.critical_injuries()?.version_info(),
            self.cover_materials()?.version_info(),
            self.armor()?.version_info(),
            self.shields()?.version_info(),
            self.weapons()?.version_info(),
            self.skills()?.version_info(),
            self.npc_templates()?.version_info(),
            self.roles()?.version_info(),
            self.martial_arts()?.version_info(),
            self.articles()?.version_info(),
            self.range_tables()?.version_info(),
            self.gear()?.version_info(),
            self.ammo()?.version_info(),
            self.weapon_mods()?.version_info(),
            self.magazines()?.version_info(),
            self.price_categories()?.version_info(),
            self.specialties()?.version_info(),
            self.clothing()?.version_info(),
            self.street_drugs()?.version_info(),
        ])
    }
}

/// Language of server content. Each language has its own server (`ru.…`,
/// `en.…`) with identical tables whose texts are in that language.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize, TS)]
#[ts(export)]
#[serde(rename_all = "snake_case")]
pub enum Lang {
    #[default]
    Ru,
    En,
}

impl Lang {
    pub const ALL: [Lang; 2] = [Lang::Ru, Lang::En];

    /// Language code as used by the UI (`i18n.language`); unknown codes fall back to English.
    pub fn from_code(code: &str) -> Lang {
        if code.to_ascii_lowercase().starts_with("ru") { Lang::Ru } else { Lang::En }
    }
}

/// Snapshot compiled into the binary: works offline and stands in for the
/// server until it exists (the "response" of the `ru` or `en` server). Core
/// collections (injuries, skills) are the same for both languages; content
/// collections come in one copy per language.
#[derive(Debug, Default, Clone, Copy)]
pub struct SeedDataSource {
    pub lang: Lang,
}

impl SeedDataSource {
    pub fn new(lang: Lang) -> Self {
        Self { lang }
    }
}

fn parse<T: serde::de::DeserializeOwned>(
    name: &'static str,
    json: &str,
) -> DataResult<Collection<T>> {
    serde_json::from_str(json).map_err(|source| DataError::Malformed { name, source })
}

/// Picks the compiled-in file of a content collection by language.
macro_rules! content {
    ($lang:expr, $name:literal) => {
        match $lang {
            Lang::Ru => parse($name, include_str!(concat!("../data/seed/ru/", $name, ".json"))),
            Lang::En => parse($name, include_str!(concat!("../data/seed/en/", $name, ".json"))),
        }
    };
}

impl DataSource for SeedDataSource {
    fn critical_injuries(&self) -> DataResult<Collection<CriticalInjury>> {
        parse("critical_injuries", include_str!("../data/seed/critical_injuries.json"))
    }
    fn cover_materials(&self) -> DataResult<Collection<CoverMaterial>> {
        content!(self.lang, "cover_materials")
    }
    fn armor(&self) -> DataResult<Collection<Armor>> {
        content!(self.lang, "armor")
    }
    fn shields(&self) -> DataResult<Collection<Shield>> {
        content!(self.lang, "shields")
    }
    fn weapons(&self) -> DataResult<Collection<Weapon>> {
        content!(self.lang, "weapons")
    }
    fn skills(&self) -> DataResult<Collection<Skill>> {
        parse("skills", include_str!("../data/seed/skills.json"))
    }
    fn npc_templates(&self) -> DataResult<Collection<NpcTemplate>> {
        content!(self.lang, "npc_templates")
    }
    fn roles(&self) -> DataResult<Collection<Role>> {
        content!(self.lang, "roles")
    }
    fn martial_arts(&self) -> DataResult<Collection<MartialArt>> {
        content!(self.lang, "martial_arts")
    }
    fn articles(&self) -> DataResult<Collection<Article>> {
        parse("rules", include_str!("../data/seed/rules.json"))
    }
    fn range_tables(&self) -> DataResult<Collection<RangeTable>> {
        parse("range_tables", include_str!("../data/seed/range_tables.json"))
    }
    fn gear(&self) -> DataResult<Collection<GearItem>> {
        content!(self.lang, "gear")
    }
    fn ammo(&self) -> DataResult<Collection<Ammo>> {
        content!(self.lang, "ammo")
    }
    fn weapon_mods(&self) -> DataResult<Collection<WeaponMod>> {
        content!(self.lang, "weapon_mods")
    }
    fn magazines(&self) -> DataResult<Collection<MagazineRow>> {
        parse("magazines", include_str!("../data/seed/magazines.json"))
    }
    fn price_categories(&self) -> DataResult<Collection<PriceCategory>> {
        parse("price_categories", include_str!("../data/seed/price_categories.json"))
    }
    fn specialties(&self) -> DataResult<Collection<Specialty>> {
        content!(self.lang, "specialties")
    }
    fn clothing(&self) -> DataResult<Collection<ClothingStyle>> {
        content!(self.lang, "clothing")
    }
    fn street_drugs(&self) -> DataResult<Collection<StreetDrug>> {
        content!(self.lang, "street_drugs")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn seed_files_parse_and_names_match() {
        let ds = SeedDataSource::default();
        assert_eq!(ds.critical_injuries().unwrap().name, "critical_injuries");
        assert_eq!(ds.cover_materials().unwrap().name, "cover_materials");
        assert_eq!(ds.armor().unwrap().name, "armor");
        assert_eq!(ds.shields().unwrap().name, "shields");
        assert_eq!(ds.weapons().unwrap().name, "weapons");
        assert_eq!(ds.skills().unwrap().name, "skills");
        assert_eq!(ds.npc_templates().unwrap().name, "npc_templates");
    }

    /// Range tables (p. 173): invariants and the book's worked examples (p. 170 and p. 173).
    #[test]
    fn range_tables_match_the_book() {
        use crate::catalog::validate_range_tables;
        let ds = SeedDataSource::default();
        let tables = ds.range_tables().unwrap();
        assert_eq!(validate_range_tables(&tables, &ds.weapons().unwrap()), Ok(()));
        let dv = |table: &str, row: &str, meters: u32| {
            let t = tables.entries.iter().find(|t| t.id == table).unwrap();
            let band = t.bands.iter().position(|b| (b.from..=b.to).contains(&meters)).unwrap();
            t.rows.iter().find(|r| r.id == row).unwrap().dv[band]
        };
        // p. 170: assault rifle at 24 m, DV 15.
        assert_eq!(dv("single", "assault_rifle", 24), Some(15));
        // p. 173: assault rifle on autofire at 14 m, DV 17.
        assert_eq!(dv("autofire", "assault_rifle", 14), Some(17));
        // Dark "НЕТ" cells.
        assert_eq!(dv("single", "pistol", 300), None);
        assert_eq!(dv("single", "bow_crossbow", 300), None);
        // Whole-table checksum guards against a mistyped cell (sum of all printed DVs).
        let sum: u32 = tables.entries.iter().flat_map(|t| &t.rows).flat_map(|r| &r.dv).flatten().map(|&d| d as u32).sum();
        assert_eq!(sum, 1339, "DV checksum");
    }

    /// Equipment as printed on p. 342-355 and p. 385: counts, price categories (the whole price
    /// column of the gear list agrees with the table of categories) and a few spot checks.
    #[test]
    fn equipment_matches_the_book() {
        use crate::catalog::{price_category_of, MOD_SLOTS};
        let ds = SeedDataSource::default();
        let categories = ds.price_categories().unwrap().entries;
        // p. 385: eight categories with these minimums.
        let table: Vec<_> = categories.iter().map(|c| (c.id.as_str(), c.min_price)).collect();
        assert_eq!(
            table,
            [("cheap", 10), ("everyday", 20), ("costly", 50), ("premium", 100), ("expensive", 500), ("very_expensive", 1000), ("luxury", 5000), ("super_luxury", 10000)]
        );
        let id = |price| price_category_of(&categories, price).unwrap().id.clone();
        assert_eq!((id(10).as_str(), id(100).as_str(), id(1000).as_str(), id(25000).as_str()), ("cheap", "premium", "very_expensive", "super_luxury"));

        let gear = ds.gear().unwrap().entries;
        assert_eq!(gear.len(), 53, "the list on p. 351 has 27 + 26 rows");
        let price = |code: &str| gear.iter().find(|g| g.code == code).unwrap().price;
        assert_eq!((price("agent"), price("cryopump"), price("techscanner"), price("glowstick")), (100, 5000, 1000, 10));
        // Prices only take the values of the categories.
        assert!(gear.iter().all(|g| categories.iter().any(|c| c.min_price == g.price)));
        // Checksum of the whole price column of the list (both readings of p. 351 agree on it).
        assert_eq!(gear.iter().map(|g| g.price).sum::<u32>(), 28_640);

        let ammo = ds.ammo().unwrap().entries;
        assert_eq!(ammo.len(), 13);
        assert_eq!(ammo.iter().find(|a| a.code == "standard").unwrap().price, 10);
        assert_eq!(ammo.iter().find(|a| a.code == "smoke").unwrap().source.page, 347);

        let mods = ds.weapon_mods().unwrap().entries;
        assert_eq!(mods.len(), 8);
        let two_slot: Vec<_> = mods.iter().filter(|m| m.slots == 2).map(|m| m.code.as_str()).collect();
        assert_eq!(two_slot, ["underbarrel_grenade_launcher", "underbarrel_shotgun", "smartlink_connector"]);
        assert!(mods.iter().all(|m| m.slots <= MOD_SLOTS));

        // Armor and shield prices (p. 97 / p. 350): the bodyweight suit is the pricey one at 1000.
        let armor = ds.armor().unwrap().entries;
        let armor_price = |code: &str| armor.iter().find(|a| a.code == code).unwrap().price;
        assert_eq!((armor_price("leather"), armor_price("bodyweight_suit"), armor_price("metalgear")), (20, 1000, 5000));
        let shields = ds.shields().unwrap().entries;
        assert_eq!(shields.iter().find(|s| s.code == "bulletproof_shield").unwrap().price, Some(100));
        assert_eq!(shields.iter().find(|s| s.code == "corpse").unwrap().price, None);
    }

    /// Specialty suggestions: the 52 distinct languages of the 10 culture regions (p. 45) and the five
    /// instruments of p. 137; the tables are the same in both languages.
    #[test]
    fn specialties_match_the_book() {
        let list = |lang| SeedDataSource::new(lang).specialties().unwrap().entries;
        let ru = list(Lang::Ru);
        let of = |skill: &str| ru.iter().filter(|s| s.skill == skill).count();
        assert_eq!((of("language"), of("play_instrument"), ru.len()), (52, 5, 57));
        assert!(ru.iter().filter(|s| s.skill == "language").all(|s| s.source.page == 45));
        assert!(ru.iter().filter(|s| s.skill == "play_instrument").all(|s| s.source.page == 137));
        // The book prints truncated forms ("урд", "йоруб", "наваха"); the real names are stored.
        for name in ["Урду", "Йоруба", "Навахо", "Непальский"] {
            assert!(ru.iter().any(|s| s.name == name), "{name}");
        }
        let en = list(Lang::En);
        assert_eq!(ru.iter().map(|s| &s.code).collect::<Vec<_>>(), en.iter().map(|s| &s.code).collect::<Vec<_>>());
    }

    /// Clothing (p. 356, printed again on p. 102) and street drugs (p. 357): shape, spot checks and a
    /// checksum of the 90 prices (32 500 eb) that both printed copies and both readings agree on.
    #[test]
    fn clothing_and_drugs_match_the_book() {
        use crate::catalog::CLOTHING_ITEMS;
        let ds = SeedDataSource::default();
        let clothing = ds.clothing().unwrap().entries;
        assert_eq!(clothing.len(), 10);
        let price = |style: &str, item: &str| {
            let col = CLOTHING_ITEMS.iter().position(|i| *i == item).unwrap();
            clothing.iter().find(|c| c.code == style).unwrap().prices[col]
        };
        assert_eq!((price("bag_lady_chic", "top"), price("businesswear", "jewelry"), price("high_fashion", "shoes")), (10, 5000, 5000));
        assert_eq!(price("high_fashion", "top"), 500);
        assert_eq!(clothing.iter().flat_map(|c| &c.prices).sum::<u32>(), 32_500);
        // The cheapest and the dearest of every piece never swap order along the style ladder: High Fashion is never cheaper than Bag Lady Chic.
        assert!(CLOTHING_ITEMS.iter().all(|i| price("high_fashion", i) > price("bag_lady_chic", i)));

        let drugs = ds.street_drugs().unwrap().entries;
        let row = |code: &str| drugs.iter().find(|d| d.code == code).unwrap();
        assert_eq!(drugs.len(), 5);
        assert_eq!((row("black_lace").price, row("black_lace").duration_hours, row("black_lace").secondary_dv), (50, 24, 17));
        assert_eq!((row("blue_glass").price, row("blue_glass").duration_hours, row("blue_glass").secondary_dv), (20, 4, 15));
        assert_eq!((row("boost").price, row("boost").duration_hours, row("boost").secondary_dv), (50, 24, 17));
        assert_eq!((row("smash").price, row("smash").duration_hours, row("smash").secondary_dv), (10, 4, 15));
        assert_eq!((row("syntococaine").price, row("syntococaine").duration_hours, row("syntococaine").secondary_dv), (20, 4, 15));
    }

    /// Hashes of every collection in every language, for the server's contract tests (it must compute
    /// the same bytes). Written to `schemas/hash-vectors.json`; the canonical form is described at
    /// `Collection::canonical_json`.
    #[test]
    fn export_hash_vectors() {
        let mut out = serde_json::Map::new();
        for (lang, name) in [(Lang::Ru, "ru"), (Lang::En, "en")] {
            let versions = SeedDataSource::new(lang).versions().unwrap();
            let map: serde_json::Map<String, serde_json::Value> =
                versions.into_iter().map(|v| (v.name, serde_json::json!({ "version": v.version, "hash": v.hash }))).collect();
            out.insert(name.to_string(), serde_json::Value::Object(map));
        }
        // A tiny hand-checkable vector: the keys come out sorted and the entries ordered by code.
        let tiny = Collection {
            name: "tiny".into(),
            version: 1,
            entries: vec![
                serde_json::json!({ "code": "b", "price": 2, "name": "B" }),
                serde_json::json!({ "name": "A", "price": 1, "code": "a" }),
            ],
        };
        assert_eq!(
            String::from_utf8(tiny.canonical_json()).unwrap(),
            r#"[{"code":"a","name":"A","price":1},{"code":"b","name":"B","price":2}]"#
        );
        out.insert("tiny".into(), serde_json::json!({ "canonical": String::from_utf8(tiny.canonical_json()).unwrap(), "hash": tiny.content_hash() }));
        let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../schemas/hash-vectors.json");
        std::fs::write(path, serde_json::to_string_pretty(&out).unwrap() + "
").unwrap();
    }

    #[test]
    fn manifest_lists_all_collections() {
        let names: Vec<_> = SeedDataSource::default().versions().unwrap().into_iter().map(|v| v.name).collect();
        assert_eq!(names, ["critical_injuries", "cover_materials", "armor", "shields", "weapons", "skills", "npc_templates", "roles", "martial_arts", "rules", "range_tables", "gear", "ammo", "weapon_mods", "magazines", "price_categories", "specialties", "clothing", "street_drugs"]);
    }

    #[test]
    fn works_through_trait_object() {
        let ds: Box<dyn DataSource> = Box::new(SeedDataSource::default());
        assert!(ds.versions().is_ok());
    }

    #[test]
    fn malformed_json_is_reported() {
        let err = parse::<Armor>("armor", "{").unwrap_err();
        assert!(err.to_string().contains("armor"));
    }

    /// Core rules data: every i18n key referenced by it exists in both locale files.
    #[test]
    fn core_keys_exist_in_both_locales() {
        let ds = SeedDataSource::default();
        let mut keys = vec![];
        for e in ds.critical_injuries().unwrap().entries {
            keys.push(e.name_key);
            keys.push(e.effect_key);
        }
        for e in ds.skills().unwrap().entries {
            // Hint texts follow the id: `skill.summary.<id>` is required, `skill.details.<id>` is optional.
            keys.push(format!("skill.summary.{}", e.id));
            keys.push(e.name_key);
        }
        for abbr in ["INT", "REF", "DEX", "TECH", "COOL", "WILL", "LUCK", "MOVE", "BODY", "EMP"] {
            keys.push(format!("stat_info.{abbr}.name"));
            keys.push(format!("stat_info.{abbr}.summary"));
        }
        for e in ds.articles().unwrap().entries {
            keys.push(e.name_key);
            keys.push(e.summary_key);
            keys.extend(e.details_key);
        }

        for t in ds.range_tables().unwrap().entries {
            keys.push(t.name_key);
            keys.extend(t.rows.into_iter().map(|r| r.name_key));
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

    /// Server content: the databases of all languages are identical but for
    /// the texts. Serialize with the texts blanked out and compare.
    #[test]
    fn content_differs_between_languages_only_in_texts() {
        fn blank(v: &mut serde_json::Value) {
            match v {
                serde_json::Value::Object(m) => {
                    for (k, x) in m.iter_mut() {
                        if matches!(k.as_str(), "name" | "armor_name" | "gear" | "notes" | "examples" | "ammo" | "description" | "summary" | "ability_name" | "usable_with" | "tags" | "primary" | "secondary") && x.is_string() {
                            assert!(!x.as_str().unwrap().is_empty(), "empty text {k}");
                            *x = "".into();
                        } else if k == "features" {
                            // Texts of a list: same number of items, blanked.
                            if let Some(a) = x.as_array_mut() {
                                a.iter_mut().for_each(|i| *i = "".into());
                            }
                        } else {
                            blank(x);
                        }
                    }
                }
                serde_json::Value::Array(a) => a.iter_mut().for_each(blank),
                _ => {}
            }
        }
        fn shape(ds: SeedDataSource) -> Vec<serde_json::Value> {
            let mut all = vec![
                serde_json::to_value(ds.cover_materials().unwrap()).unwrap(),
                serde_json::to_value(ds.armor().unwrap()).unwrap(),
                serde_json::to_value(ds.shields().unwrap()).unwrap(),
                serde_json::to_value(ds.weapons().unwrap()).unwrap(),
                serde_json::to_value(ds.npc_templates().unwrap()).unwrap(),
                serde_json::to_value(ds.roles().unwrap()).unwrap(),
                serde_json::to_value(ds.gear().unwrap()).unwrap(),
                serde_json::to_value(ds.ammo().unwrap()).unwrap(),
                serde_json::to_value(ds.weapon_mods().unwrap()).unwrap(),
                serde_json::to_value(ds.specialties().unwrap()).unwrap(),
                serde_json::to_value(ds.clothing().unwrap()).unwrap(),
                serde_json::to_value(ds.street_drugs().unwrap()).unwrap(),
            ];
            all.iter_mut().for_each(blank);
            all
        }
        assert_eq!(shape(SeedDataSource::new(Lang::Ru)), shape(SeedDataSource::new(Lang::En)));
        // The texts really are in different languages.
        let name = |l| SeedDataSource::new(l).armor().unwrap().entries[0].name.clone();
        assert_ne!(name(Lang::Ru), name(Lang::En));
    }

    /// uuids are unique across every content table.
    #[test]
    fn content_uuids_are_unique() {
        let ds = SeedDataSource::default();
        let mut ids: Vec<String> = ds.cover_materials().unwrap().entries.into_iter().map(|e| e.uuid).collect();
        ids.extend(ds.armor().unwrap().entries.into_iter().map(|e| e.uuid));
        ids.extend(ds.shields().unwrap().entries.into_iter().map(|e| e.uuid));
        ids.extend(ds.weapons().unwrap().entries.into_iter().map(|e| e.uuid));
        ids.extend(ds.npc_templates().unwrap().entries.into_iter().map(|e| e.uuid));
        ids.extend(ds.roles().unwrap().entries.into_iter().map(|e| e.uuid));
        ids.extend(ds.martial_arts().unwrap().entries.into_iter().map(|e| e.uuid));
        ids.extend(ds.gear().unwrap().entries.into_iter().map(|e| e.uuid));
        ids.extend(ds.ammo().unwrap().entries.into_iter().map(|e| e.uuid));
        ids.extend(ds.weapon_mods().unwrap().entries.into_iter().map(|e| e.uuid));
        ids.extend(ds.specialties().unwrap().entries.into_iter().map(|e| e.uuid));
        ids.extend(ds.clothing().unwrap().entries.into_iter().map(|e| e.uuid));
        ids.extend(ds.street_drugs().unwrap().entries.into_iter().map(|e| e.uuid));
        let n = ids.len();
        ids.sort();
        ids.dedup();
        assert_eq!(ids.len(), n);
    }

    #[test]
    fn seed_data_passes_invariants() {
        use crate::catalog::*;
        let ds = SeedDataSource::default();
        assert_eq!(validate_critical_injuries(&ds.critical_injuries().unwrap()), Ok(()));
        assert_eq!(validate_cover_materials(&ds.cover_materials().unwrap()), Ok(()));
        assert_eq!(validate_armor(&ds.armor().unwrap()), Ok(()));
        assert_eq!(validate_shields(&ds.shields().unwrap()), Ok(()));
        assert_eq!(validate_weapons(&ds.weapons().unwrap(), &ds.skills().unwrap()), Ok(()));
        assert_eq!(validate_skills(&ds.skills().unwrap()), Ok(()));
        assert_eq!(validate_roles(&ds.roles().unwrap()), Ok(()));
        assert_eq!(validate_gear(&ds.gear().unwrap()), Ok(()));
        assert_eq!(validate_ammo(&ds.ammo().unwrap()), Ok(()));
        assert_eq!(validate_weapon_mods(&ds.weapon_mods().unwrap(), &ds.skills().unwrap(), &ds.weapons().unwrap()), Ok(()));
        assert_eq!(validate_magazines(&ds.magazines().unwrap(), &ds.weapons().unwrap()), Ok(()));
        assert_eq!(validate_price_categories(&ds.price_categories().unwrap()), Ok(()));
        assert_eq!(validate_specialties(&ds.specialties().unwrap(), &ds.skills().unwrap()), Ok(()));
        assert_eq!(validate_clothing(&ds.clothing().unwrap()), Ok(()));
        assert_eq!(validate_street_drugs(&ds.street_drugs().unwrap()), Ok(()));
        assert_eq!(validate_articles(&ds.articles().unwrap()), Ok(()));
        assert_eq!(validate_martial_arts(&ds.martial_arts().unwrap(), &ds.critical_injuries().unwrap()), Ok(()));
        assert_eq!(
            validate_npc_templates(&ds.npc_templates().unwrap(), &ds.skills().unwrap()),
            Ok(())
        );
    }

    /// Martial arts as printed on p. 178-180: four styles with two own moves each, plus the general
    /// move; both languages carry the same numbers.
    #[test]
    fn martial_arts_match_the_book() {
        use crate::catalog::MartialArtKind;
        let shape = |lang| {
            SeedDataSource::new(lang)
                .martial_arts()
                .unwrap()
                .entries
                .into_iter()
                .map(|a| {
                    let moves: Vec<_> = a
                        .techniques
                        .iter()
                        .map(|t| (t.code.clone(), t.dv, t.requires.as_ref().map(|r| (r.stat, r.min)), t.injuries.clone()))
                        .collect();
                    (a.code, a.kind, a.source.page, moves)
                })
                .collect::<Vec<_>>()
        };
        assert_eq!(shape(Lang::Ru), shape(Lang::En));
        let arts = SeedDataSource::default().martial_arts().unwrap().entries;
        let codes: Vec<_> = arts.iter().map(|a| a.code.as_str()).collect();
        assert_eq!(codes, ["general", "aikido", "karate", "judo", "taekwondo"]);
        assert_eq!(arts.iter().filter(|a| a.kind == MartialArtKind::General).count(), 1);
        assert_eq!(arts[0].techniques[0].dv, Some(13), "Return is DV 13");
        for style in &arts[1..] {
            assert_eq!(style.techniques.len(), 2, "{}: two own moves", style.code);
        }
        // Every printed DV is 15 except Return; the four stat-gated moves have none.
        let dvs: Vec<_> = arts[1..].iter().flat_map(|a| a.techniques.iter().map(|t| (t.code.as_str(), t.dv))).collect();
        assert_eq!(
            dvs,
            [
                ("disarm", Some(15)),
                ("iron_grip", Some(15)),
                ("break_armor", Some(15)),
                ("bone_crusher", None),
                ("counter_throw", Some(15)),
                ("escape_grab", Some(15)),
                ("pressure_point", None),
                ("flying_kick", None),
            ]
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
        let skills = SeedDataSource::default().skills().unwrap();
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
        let ds = SeedDataSource::default();
        let cover = ds.cover_materials().unwrap();
        let hp = |id: &str| cover.entries.iter().find(|e| e.code == id).unwrap().hp;
        assert_eq!((hp("concrete_thick"), hp("wood_thin"), hp("metal_thick")), (25, 5, 50));
        assert!(cover.entries.iter().all(|e| e.code != "drywall_thin"));

        let armor = ds.armor().unwrap();
        let a = |id: &str| armor.entries.iter().find(|e| e.code == id).unwrap().clone();
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
        let ds = SeedDataSource::default();
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
            let t = npcs.entries.iter().find(|e| e.code == id).unwrap();
            let derived = max_hp(t.stats.body, t.stats.will);
            assert_eq!((derived, derived.div_ceil(2), u32::from(t.stats.body)), (hp, serious, save), "{id}");
            assert_eq!(t.stats.luck, 0, "{id}: LUCK is a dash in the book");
            let c = t.to_character("x", "", &skills.entries);
            assert_eq!((c.validate(), c.hp), (Ok(()), hp), "{id}");
        }

        let tier = |id: &str| npcs.entries.iter().find(|e| e.code == id).unwrap().tier;
        assert_eq!(tier("bodyguard"), NpcTier::Mook);
        assert_eq!(tier("netrunner"), NpcTier::Lieutenant);
        assert_eq!(tier("pyrotechnician"), NpcTier::MiniBoss);
        assert_eq!(tier("cyberpsychopath"), NpcTier::Boss);

        // Armor that is a catalog item has the catalog's SP.
        let armor = ds.armor().unwrap();
        for t in &npcs.entries {
            if let Some(code) = &t.armor_code {
                let a = armor.entries.iter().find(|a| &a.code == code).expect("armor code is in the catalog");
                assert_eq!((t.armor_head_sp, t.armor_body_sp), (a.sp, a.sp), "{}", t.code);
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
                    below.push(format!("{}:{key}", t.code));
                }
            }
        }
        assert_eq!(below, ["cyberpsychopath:concentration"]);

        // Spot checks against the images.
        let bodyguard = npcs.entries.iter().find(|e| e.code == "bodyguard").unwrap();
        assert_eq!(bodyguard.skill_totals["handgun"], 10);
        assert_eq!(bodyguard.skill_levels(&skills.entries)["handgun"], 4); // REF 6
        let officer = npcs.entries.iter().find(|e| e.code == "security_officer").unwrap();
        assert_eq!((officer.stats.reflex, officer.armor_body_sp), (8, 12));
    }

    /// Every critical injury has a glyph in the UI (`injury-icons.tsx`), and no glyph is orphaned.
    #[test]
    fn every_injury_has_an_icon() {
        let path = format!("{}/../../src/components/combat/injury-icons.tsx", env!("CARGO_MANIFEST_DIR"));
        let source = std::fs::read_to_string(path).unwrap();
        let ids: Vec<String> =
            SeedDataSource::default().critical_injuries().unwrap().entries.into_iter().map(|e| e.id).collect();
        for id in &ids {
            assert!(source.contains(&format!("
  {id}: ")), "no icon for {id}");
        }
        let glyphs = source.lines().filter(|l| l.starts_with("  ") && !l.starts_with("   ") && l.contains(": ") && l.trim_end().trim_end_matches(',').ends_with(['(', '>'])).count();
        assert_eq!(glyphs, ids.len(), "icons and injuries differ in number");
    }

    /// Weapon tables of the book (p. 92, 94, 95): counts and a few printed rows.
    #[test]
    fn weapons_match_the_book() {
        use crate::catalog::WeaponKind;
        let w = SeedDataSource::default().weapons().unwrap().entries;
        let count = |k| w.iter().filter(|e| e.kind == k).count();
        assert_eq!((count(WeaponKind::Melee), count(WeaponKind::Ranged), count(WeaponKind::Exotic)), (4, 11, 14));
        let get = |c: &str| w.iter().find(|e| e.code == c).unwrap();
        let ar = get("assault_rifle");
        assert_eq!((ar.damage.as_deref(), ar.magazine, ar.rof, ar.hands, ar.price), (Some("5d6"), Some(25), Some(1), Some(2), 500));
        assert_eq!(ar.features.len(), 2);
        let pistol = get("very_heavy_pistol");
        assert_eq!((pistol.damage.as_deref(), pistol.magazine, pistol.concealable), (Some("4d6"), Some(8), Some(false)));
        assert_eq!(get("rocket_launcher").damage.as_deref(), Some("8d6"));
        assert_eq!(get("light_melee").concealable, Some(true));
        assert_eq!(get("malorian_3516").price, 10000);
    }

    /// Roles and Medicine against pp. 143–161 (numbers read from the page images, two passes).
    #[test]
    fn roles_match_the_book() {
        for lang in Lang::ALL {
            let roles = SeedDataSource::new(lang).roles().unwrap().entries;
            let codes: Vec<_> = roles.iter().map(|r| r.code.as_str()).collect();
            assert_eq!(
                codes,
                ["rockerboy", "solo", "netrunner", "tech", "medtech", "media", "exec", "lawman", "fixer", "nomad"]
            );
            let medtech = roles.iter().find(|r| r.code == "medtech").unwrap();
            assert_eq!(medtech.source.page, 149);
            let subs: Vec<_> = medtech.subabilities.iter().map(|s| (s.code.as_str(), s.max_points)).collect();
            assert_eq!(subs, [("surgery", Some(5)), ("pharmaceutics", Some(5)), ("cryosystems", Some(5))]);
            let drugs: Vec<_> = medtech.subabilities[1].options.iter().map(|o| o.code.as_str()).collect();
            assert_eq!(drugs, ["antibiotics", "rapid_detox", "speedheal", "stim", "surge"]);
            for r in &roles {
                assert!(!r.description.is_empty(), "{}: description", r.code);
                assert!(r.ranks.iter().map(|n| n.rank).eq(1..=10), "{}: ranks", r.code);
            }
            let pools: Vec<_> = roles.iter().filter(|r| !r.subabilities.is_empty()).map(|r| r.code.as_str()).collect();
            assert_eq!(pools, ["solo", "tech", "medtech"]);
            let solo = roles.iter().find(|r| r.code == "solo").unwrap();
            assert_eq!(solo.subabilities.len(), 6);
            let deflection: Vec<_> = solo.subabilities[0].levels.iter().map(|l| l.points).collect();
            assert_eq!(deflection, [2, 4, 6, 8, 10]);
            let levels: Vec<_> = medtech.subabilities[2].levels.iter().map(|l| l.points).collect();
            assert_eq!(levels, [1, 2, 3, 4, 5]);
        }
    }
}
