import type { TFunction } from "i18next";
import type { Ammo } from "@/bindings/Ammo";
import type { Armor } from "@/bindings/Armor";
import type { ClothingStyle } from "@/bindings/ClothingStyle";
import type { GearItem } from "@/bindings/GearItem";
import type { StreetDrug } from "@/bindings/StreetDrug";
import type { WeaponMod } from "@/bindings/WeaponMod";
import type { Article } from "@/bindings/Article";
import type { MartialArt } from "@/bindings/MartialArt";
import type { CoverPreset } from "@/bindings/CoverPreset";
import type { CriticalInjury } from "@/bindings/CriticalInjury";
import type { Role } from "@/bindings/Role";
import type { Skill } from "@/bindings/Skill";
import type { Weapon } from "@/bindings/Weapon";
import type { ReferenceData } from "@/hooks/use-reference-data";
import { priceCategoryName } from "@/lib/price";
import { STAT_ABBR } from "@/lib/stat-abbr";
import type { ReferenceSection } from "@/store/ui-store";

export const REFERENCE_SECTIONS: ReferenceSection[] = [
  "articles",
  "injuries",
  "cover",
  "armor",
  "weapons",
  "gear",
  "ammo",
  "weapon_mods",
  "clothing",
  "street_drugs",
  "skills",
  "martial_arts",
  "roles",
  "reputation",
];

/** Reputation has no catalog: a few rules and the ten levels, texts come from the locale. */
export type ReputationItem = { kind: "rule"; key: string } | { kind: "level"; level: number };
export const REPUTATION_RULES = ["how", "negative", "intimidation"];

type Base = {
  id: string;
  name: string;
  /** One short line under the name in lists. */
  subtitle: string;
  /** Short number on the right of a list row (roll, HP, SP, damage). */
  value: string;
  /** Folded text the search looks in. */
  search: string;
};

/** A reference record with its typed data; the detail card is drawn per `section`. */
export type RefEntry =
  | (Base & { section: "articles"; item: Article })
  | (Base & { section: "injuries"; item: CriticalInjury })
  | (Base & { section: "cover"; item: CoverPreset })
  | (Base & { section: "armor"; item: Armor })
  | (Base & { section: "weapons"; item: Weapon })
  | (Base & { section: "gear"; item: GearItem })
  | (Base & { section: "ammo"; item: Ammo })
  | (Base & { section: "weapon_mods"; item: WeaponMod })
  | (Base & { section: "clothing"; item: ClothingStyle })
  | (Base & { section: "street_drugs"; item: StreetDrug })
  | (Base & { section: "skills"; item: Skill })
  | (Base & { section: "martial_arts"; item: MartialArt })
  | (Base & { section: "roles"; item: Role })
  | (Base & { section: "reputation"; item: ReputationItem });

/** Case-insensitive search that treats "ё" as "е". */
export const fold = (text: string) => text.toLowerCase().replace(/ё/g, "е");

const join = (parts: (string | null | undefined | false)[], sep = " ") => parts.filter(Boolean).join(sep);

/** Name of a weapon's skill, or "exotic" for weapons without one. */
export const weaponSkillName = (w: Weapon, t: TFunction) =>
  w.skill ? t(`skill.items.${w.skill}`) : t("reference.weapon.kind.exotic");

function roleSearch(r: Role): string {
  return join(
    [
      r.name,
      r.ability_name,
      r.description || r.summary,
      ...r.ranks.map((n) => n.summary),
      ...r.subabilities.flatMap((sub) => [
        sub.name,
        sub.summary,
        ...sub.options.map((o) => `${o.name} ${o.summary}`),
        ...sub.levels.map((l) => l.summary),
      ]),
    ],
    "\n",
  );
}

export function entriesOf(section: ReferenceSection, data: ReferenceData, t: TFunction): RefEntry[] {
  const make = (e: Omit<RefEntry, "search"> & { search: string }): RefEntry => ({ ...e, search: fold(e.search) }) as RefEntry;
  switch (section) {
    case "articles":
      return data.articles.map((item) => {
        const name = t(item.name_key);
        const summary = t(item.summary_key);
        return make({
          section,
          item,
          id: item.id,
          name,
          subtitle: summary,
          value: "",
          search: join([name, summary, item.details_key && t(item.details_key)], "\n"),
        });
      });
    case "injuries":
      return data.injuries.map((item) => {
        const name = t(item.name_key);
        const area = t(`reference.area.${item.area}`);
        return make({
          section,
          item,
          id: item.id,
          name,
          subtitle: area,
          value: String(item.roll),
          search: join([name, area, t(item.effect_key)], "\n"),
        });
      });
    case "cover":
      return data.cover.map((item) => {
        const thickness = item.thickness ? t(`cover.thickness.${item.thickness}`) : "";
        return make({
          section,
          item,
          id: item.id,
          name: item.name,
          subtitle: thickness,
          value: item.hp === null ? "—" : String(item.hp),
          search: join([item.name, thickness]),
        });
      });
    case "armor":
      return data.armor.map((item) =>
        make({
          section,
          item,
          id: item.code,
          name: item.name,
          subtitle: item.penalty > 0 ? t("reference.penalty", { value: item.penalty }) : "",
          value: String(item.sp),
          search: join([item.name, priceCategoryName(data.priceCategories, item.price, t)]),
        }),
      );
    case "weapons":
      return data.weapons.map((item) => {
        const skill = weaponSkillName(item, t);
        return make({
          section,
          item,
          id: item.code,
          name: item.name,
          subtitle: skill,
          value: item.damage ?? `${item.price} ${t("reference.eb")}`,
          search: join([item.name, skill, item.examples, item.ammo, ...item.features, item.description], "\n"),
        });
      });
    case "gear":
      return data.gear.map((item) => {
        const category = priceCategoryName(data.priceCategories, item.price, t);
        return make({
          section,
          item,
          id: item.code,
          name: item.name,
          subtitle: category,
          value: `${item.price} ${t("reference.eb")}`,
          search: join([item.name, item.summary, category], "\n"),
        });
      });
    case "ammo":
      return data.ammo.map((item) => {
        const category = priceCategoryName(data.priceCategories, item.price, t);
        return make({
          section,
          item,
          id: item.code,
          name: item.name,
          subtitle: category,
          value: `${item.price} ${t("reference.eb")}`,
          search: join([item.name, item.usable_with, item.summary, category], "\n"),
        });
      });
    case "weapon_mods":
      return data.weaponMods.map((item) => {
        const category = priceCategoryName(data.priceCategories, item.price, t);
        return make({
          section,
          item,
          id: item.code,
          name: item.name,
          subtitle: category,
          value: `${item.price} ${t("reference.eb")}`,
          search: join([item.name, item.summary, category], "\n"),
        });
      });
    case "clothing":
      return data.clothing.map((item) =>
        make({
          section,
          item,
          id: item.code,
          name: item.name,
          subtitle: item.tags,
          value: `${Math.min(...item.prices)}–${Math.max(...item.prices)} ${t("reference.eb")}`,
          search: join([item.name, item.tags]),
        }),
      );
    case "street_drugs":
      return data.streetDrugs.map((item) => {
        const category = priceCategoryName(data.priceCategories, item.price, t);
        return make({
          section,
          item,
          id: item.code,
          name: item.name,
          subtitle: category,
          value: `${item.price} ${t("reference.eb")}`,
          search: join([item.name, item.primary, item.secondary, category], "\n"),
        });
      });
    case "skills":
      return data.skills.map((item) => {
        const name = t(item.name_key);
        const stat = t(`stats.${STAT_ABBR[item.stat]}`);
        const category = t(`skill.category.${item.category}`);
        return make({
          section,
          item,
          id: item.id,
          name,
          subtitle: `${stat} · ${category}`,
          value: item.double_cost ? "×2" : "",
          search: join([name, stat, category, t(`skill.summary.${item.id}`, { defaultValue: "" })], "\n"),
        });
      });
    case "martial_arts":
      return data.martialArts.map((item) => {
        const kind = t(`reference.martial_kind.${item.kind}`);
        return make({
          section,
          item,
          id: item.code,
          name: item.name,
          subtitle: kind,
          value: item.kind === "general" ? "" : String(item.techniques.length),
          search: join([item.name, kind, item.summary, ...item.techniques.map((x) => `${x.name} ${x.condition ?? ""} ${x.effect}`)], "\n"),
        });
      });
    case "roles":
      return data.roles.map((item) =>
        make({
          section,
          item,
          id: item.code,
          name: item.name,
          subtitle: item.ability_name,
          value: "",
          search: roleSearch(item),
        }),
      );
    case "reputation":
      return [
        ...REPUTATION_RULES.map((key) => {
          const name = t(`reputation.rule.${key}.name`);
          return make({
            section,
            item: { kind: "rule", key } as ReputationItem,
            id: `rule_${key}`,
            name,
            subtitle: "",
            value: "",
            search: join([name, t(`reputation.rule.${key}.detail`)], "\n"),
          });
        }),
        ...Array.from({ length: 10 }, (_, i) => {
          const level = i + 1;
          const name = t("reputation.level", { level });
          return make({
            section,
            item: { kind: "level", level } as ReputationItem,
            id: `level_${level}`,
            name,
            subtitle: "",
            value: String(level),
            search: join([name, t(`reputation.levels.${level}`)], "\n"),
          });
        }),
      ];
  }
}

/** Entries of every section, for the global search. */
export function allEntries(data: ReferenceData, t: TFunction): RefEntry[] {
  return REFERENCE_SECTIONS.flatMap((section) => entriesOf(section, data, t));
}
