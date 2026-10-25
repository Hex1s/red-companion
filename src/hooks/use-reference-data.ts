import { useEffect, useState } from "react";
import { useTranslation } from "react-i18next";
import type { Armor } from "@/bindings/Armor";
import type { Article } from "@/bindings/Article";
import type { MartialArt } from "@/bindings/MartialArt";
import type { CoverPreset } from "@/bindings/CoverPreset";
import type { CriticalInjury } from "@/bindings/CriticalInjury";
import type { Role } from "@/bindings/Role";
import type { Skill } from "@/bindings/Skill";
import type { Weapon } from "@/bindings/Weapon";
import { api, hasBackend } from "@/lib/api";

export type ReferenceData = {
  articles: Article[];
  martialArts: MartialArt[];
  injuries: CriticalInjury[];
  cover: CoverPreset[];
  armor: Armor[];
  weapons: Weapon[];
  skills: Skill[];
  roles: Role[];
};

const EMPTY: ReferenceData = { articles: [], martialArts: [], injuries: [], cover: [], armor: [], weapons: [], skills: [], roles: [] };

/** Loads every reference collection (in the interface language) once `enabled` becomes true. */
export function useReferenceData(enabled = true): ReferenceData {
  const { i18n } = useTranslation();
  const [data, setData] = useState<ReferenceData>(EMPTY);

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
    ]).then(
      ([articles, injuries, cover, armor, weapons, skills, martialArts, roles]) => {
        if (!cancelled) setData({ articles, injuries, cover, armor, weapons, skills, martialArts, roles });
      },
      () => {
        if (!cancelled) setData(EMPTY);
      },
    );
    return () => {
      cancelled = true;
    };
  }, [i18n.language, enabled]);

  return data;
}
