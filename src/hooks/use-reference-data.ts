import { useEffect, useState } from "react";
import { useTranslation } from "react-i18next";
import type { Ammo } from "@/bindings/Ammo";
import type { Armor } from "@/bindings/Armor";
import type { ClothingStyle } from "@/bindings/ClothingStyle";
import type { GearItem } from "@/bindings/GearItem";
import type { StreetDrug } from "@/bindings/StreetDrug";
import type { MagazineRow } from "@/bindings/MagazineRow";
import type { PriceCategory } from "@/bindings/PriceCategory";
import type { WeaponMod } from "@/bindings/WeaponMod";
import type { Article } from "@/bindings/Article";
import type { MartialArt } from "@/bindings/MartialArt";
import type { CoverPreset } from "@/bindings/CoverPreset";
import type { CriticalInjury } from "@/bindings/CriticalInjury";
import type { Role } from "@/bindings/Role";
import type { Skill } from "@/bindings/Skill";
import type { Weapon } from "@/bindings/Weapon";
import { api, hasBackend } from "@/lib/api";
import { useContentStore } from "@/store/content-store";

export type ReferenceData = {
  articles: Article[];
  martialArts: MartialArt[];
  injuries: CriticalInjury[];
  cover: CoverPreset[];
  armor: Armor[];
  weapons: Weapon[];
  skills: Skill[];
  roles: Role[];
  gear: GearItem[];
  ammo: Ammo[];
  weaponMods: WeaponMod[];
  magazines: MagazineRow[];
  priceCategories: PriceCategory[];
  clothing: ClothingStyle[];
  streetDrugs: StreetDrug[];
};

const EMPTY: ReferenceData = { articles: [], martialArts: [], injuries: [], cover: [], armor: [], weapons: [], skills: [], roles: [],
  gear: [], ammo: [], weaponMods: [], magazines: [], priceCategories: [],
  clothing: [], streetDrugs: [],
};

/** Loads every reference collection (in the interface language) once `enabled` becomes true. */
export function useReferenceData(enabled = true): ReferenceData {
  const { i18n } = useTranslation();
  const [data, setData] = useState<ReferenceData>(EMPTY);
  const contentVersion = useContentStore((s) => s.version);

  useEffect(() => {
    if (!hasBackend || !enabled) return;
    let cancelled = false;
    Promise.all([
      api.listArticles(),
      api.listInjuries(),
      api.listCoverPresets(i18n.language),
      api.listArmor(i18n.language),
      api.listWeapons(i18n.language),
      api.listSkills(),
      api.listMartialArts(i18n.language),
      api.listRoles(i18n.language),
      api.listGear(i18n.language),
      api.listAmmo(i18n.language),
      api.listWeaponMods(i18n.language),
      api.listMagazines(),
      api.listPriceCategories(),
      api.listClothing(i18n.language),
      api.listStreetDrugs(i18n.language),
    ]).then(
      ([articles, injuries, cover, armor, weapons, skills, martialArts, roles, gear, ammo, weaponMods, magazines, priceCategories, clothing, streetDrugs]) => {
        if (!cancelled)
          setData({ articles, injuries, cover, armor, weapons, skills, martialArts, roles, gear, ammo, weaponMods, magazines, priceCategories, clothing, streetDrugs });
      },
      () => {
        if (!cancelled) setData(EMPTY);
      },
    );
    return () => {
      cancelled = true;
    };
  }, [i18n.language, enabled, contentVersion]);

  return data;
}
