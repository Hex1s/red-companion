import { useEffect, useMemo, useState } from "react";
import { useTranslation } from "react-i18next";
import type { Armor } from "@/bindings/Armor";
import type { CoverPreset } from "@/bindings/CoverPreset";
import type { CriticalInjury } from "@/bindings/CriticalInjury";
import type { NpcTemplate } from "@/bindings/NpcTemplate";
import type { Skill } from "@/bindings/Skill";
import { Input } from "@/components/ui/input";
import { Tabs, TabsList, TabsTrigger } from "@/components/ui/tabs";
import { api, hasBackend } from "@/lib/api";
import { useUiStore, type ReferenceSection } from "@/store/ui-store";

const SECTIONS: ReferenceSection[] = ["injuries", "cover", "armor", "skills", "npc"];

type Entry = {
  id: string;
  name: string;
  /** Secondary line: effect, stat, tier; also searched. */
  detail: string;
  /** Short number shown on the right (roll, HP, SP). */
  value: string;
};

type Data = {
  injuries: CriticalInjury[];
  cover: CoverPreset[];
  armor: Armor[];
  skills: Skill[];
  npc: NpcTemplate[];
};

const EMPTY: Data = { injuries: [], cover: [], armor: [], skills: [], npc: [] };

/** Case-insensitive search that treats "ё" as "е". */
const fold = (text: string) => text.toLowerCase().replace(/ё/g, "е");

/** Reference pages: one section at a time, filtered by the search box over translated texts. */
export function ReferencePage() {
  const { t, i18n } = useTranslation();
  const { referenceSection, setReferenceSection, referenceQuery, setReferenceQuery } = useUiStore();
  const [data, setData] = useState<Data>(EMPTY);

  useEffect(() => {
    if (!hasBackend) return;
    Promise.all([
      api.listInjuries(),
      api.listCoverPresets(i18n.language),
      api.listArmor(i18n.language),
      api.listSkills(),
      api.listNpcTemplates(i18n.language),
    ]).then(
      ([injuries, cover, armor, skills, npc]) => setData({ injuries, cover, armor, skills, npc }),
      () => setData(EMPTY),
    );
  }, [i18n.language]);

  const entries: Entry[] = useMemo(() => {
    switch (referenceSection) {
      case "injuries":
        return data.injuries.map((i) => ({
          id: i.id,
          name: t(i.name_key),
          detail: `${t(`reference.area.${i.area}`)}. ${t(i.effect_key)}`,
          value: String(i.roll),
        }));
      case "cover":
        return data.cover.map((c) => ({
          id: c.id,
          name: c.name,
          detail: c.thickness ? t(`cover.thickness.${c.thickness}`) : "",
          value: c.hp === null ? "—" : String(c.hp),
        }));
      case "armor":
        return data.armor.map((a) => ({
          id: a.code,
          name: a.name,
          detail: a.penalty > 0 ? t("reference.penalty", { value: a.penalty }) : "",
          value: String(a.sp),
        }));
      case "skills":
        return data.skills.map((s) => ({
          id: s.id,
          name: t(s.name_key),
          detail: `${t(`stats.${s.stat}`)} · ${t(`skill.category.${s.category}`)}`,
          value: s.double_cost ? "×2" : "",
        }));
      case "npc":
        return data.npc.map((n) => ({
          id: n.code,
          name: n.name,
          detail: t(`npc.tier.${n.tier}`),
          value: `${n.armor_head_sp}/${n.armor_body_sp}`,
        }));
    }
  }, [data, referenceSection, t]);

  const query = fold(referenceQuery.trim());
  const shown = query
    ? entries.filter((e) => fold(`${e.name} ${e.detail}`).includes(query))
    : entries;

  return (
    <div className="space-y-4">
      <h2 className="text-2xl font-bold">{t("reference.title")}</h2>
      <Tabs
        value={referenceSection}
        onValueChange={(v) => setReferenceSection(v as ReferenceSection)}
      >
        <TabsList>
          {SECTIONS.map((s) => (
            <TabsTrigger key={s} value={s}>
              {t(`reference.section.${s}`)}
            </TabsTrigger>
          ))}
        </TabsList>
      </Tabs>
      <Input
        value={referenceQuery}
        onChange={(e) => setReferenceQuery(e.target.value)}
        placeholder={t("reference.search")}
        aria-label={t("reference.search")}
      />
      {!hasBackend ? (
        <p className="text-muted-foreground">{t("characters.no_backend")}</p>
      ) : shown.length === 0 ? (
        <p className="text-muted-foreground">{t("reference.nothing_found")}</p>
      ) : (
        <ul className="divide-y border">
          {shown.map((e) => (
            <li key={e.id} className="flex items-baseline gap-3 px-3 py-2">
              <div className="min-w-0 flex-1">
                <div className="font-medium">{e.name}</div>
                {e.detail && <div className="text-sm text-muted-foreground">{e.detail}</div>}
              </div>
              <span className="num text-muted-foreground">{e.value}</span>
            </li>
          ))}
        </ul>
      )}
    </div>
  );
}
