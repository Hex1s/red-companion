import { useEffect, useState } from "react";
import { useTranslation } from "react-i18next";
import type { Armor } from "@/bindings/Armor";
import type { CoverPreset } from "@/bindings/CoverPreset";
import type { CriticalInjury } from "@/bindings/CriticalInjury";
import type { NpcTemplate } from "@/bindings/NpcTemplate";
import type { Role } from "@/bindings/Role";
import type { Skill } from "@/bindings/Skill";
import type { Weapon } from "@/bindings/Weapon";
import { api, hasBackend } from "@/lib/api";

export type ReferenceData = {
  injuries: CriticalInjury[];
  cover: CoverPreset[];
  armor: Armor[];
  weapons: Weapon[];
  skills: Skill[];
  npc: NpcTemplate[];
  roles: Role[];
};

const EMPTY: ReferenceData = { injuries: [], cover: [], armor: [], weapons: [], skills: [], npc: [], roles: [] };

/** Loads every reference collection (in the interface language) once `enabled` becomes true. */
export function useReferenceData(enabled = true): ReferenceData {
  const { i18n } = useTranslation();
  const [data, setData] = useState<ReferenceData>(EMPTY);

  useEffect(() => {
    if (!hasBackend || !enabled) return;
    let cancelled = false;
    Promise.all([
      api.listInjuries(),
      api.listCoverPresets(i18n.language),
      api.listArmor(i18n.language),
      api.listWeapons(i18n.language),
      api.listSkills(),
      api.listNpcTemplates(i18n.language),
      api.listRoles(i18n.language),
    ]).then(
      ([injuries, cover, armor, weapons, skills, npc, roles]) => {
        if (!cancelled) setData({ injuries, cover, armor, weapons, skills, npc, roles });
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
