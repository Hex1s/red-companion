import type { TFunction } from "i18next";
import { STAT_ABBR } from "@/lib/stat-abbr";

import { weaponSkillName, type RefEntry } from "@/lib/reference-entries";
import type { ReferenceSection } from "@/store/ui-store";

/** A column of the reference table: what it shows and what it sorts by. */
export type RefColumn = {
  id: string;
  label: string;
  /** A number (or a short code): monospace, right-aligned. */
  numeric?: boolean;
  /** Text in the cell. */
  text: (e: RefEntry) => string;
  /** Sort key; defaults to the cell text. Empty cells (`null`) always go last. */
  sort?: (e: RefEntry) => number | string | null;
};

/** Average of `NdM+K` ("3d6", "2d6+2"); `null` for anything else. */
export function averageDamage(dice: string | null): number | null {
  const m = dice ? /^(\d+)d(\d+)(?:\s*([+-])\s*(\d+))?$/i.exec(dice.trim()) : null;
  if (!m) return null;
  const sign = m[3] === "-" ? -1 : 1;
  return Number(m[1]) * (Number(m[2]) + 1) / 2 + sign * Number(m[4] ?? 0);
}

/** Columns of the clothing matrix, as in `catalog::CLOTHING_ITEMS`. */
const CLOTHING_COLUMNS = ["pants", "top", "jacket", "shoes", "jewelry", "mirrorshades", "glasses", "contacts", "hats"];

/** The price in eurobucks, sorted as a number. */
function priceColumn(t: TFunction, price: (e: RefEntry) => number | null): RefColumn {
  return {
    id: "price",
    label: t("reference.col.price"),
    numeric: true,
    text: (e) => {
      const p = price(e);
      return p === null ? "—" : `${p} ${t("reference.eb")}`;
    },
    sort: price,
  };
}

/** The price category (derived from the price) is the row's subtitle; it sorts by the price. */
function categoryColumn(t: TFunction, price: (e: RefEntry) => number | null): RefColumn {
  return { id: "category", label: t("reference.col.category"), text: (e) => e.subtitle, sort: price };
}

/** Columns per section. The first one is the name; prose sections show a one-line summary. */
export function columnsOf(section: ReferenceSection, t: TFunction): RefColumn[] {
  const name: RefColumn = { id: "name", label: t("reference.col.name"), text: (e) => e.name };
  const sub = (label: string): RefColumn => ({ id: "subtitle", label, text: (e) => e.subtitle });

  switch (section) {
    case "injuries":
      return [
        {
          id: "roll",
          label: t("reference.col.roll"),
          numeric: true,
          text: (e) => e.value,
          sort: (e) => (e.section === "injuries" ? e.item.roll : null),
        },
        name,
        sub(t("reference.col.area")),
      ];
    case "cover":
      return [
        name,
        sub(t("reference.col.thickness")),
        {
          id: "hp",
          label: t("reference.col.hp"),
          numeric: true,
          text: (e) => e.value,
          sort: (e) => (e.section === "cover" ? e.item.hp : null),
        },
      ];
    case "armor":
      return [
        name,
        {
          id: "sp",
          label: t("reference.col.sp"),
          numeric: true,
          text: (e) => e.value,
          sort: (e) => (e.section === "armor" ? e.item.sp : null),
        },
        {
          id: "penalty",
          label: t("reference.col.penalty"),
          numeric: true,
          text: (e) => (e.section === "armor" && e.item.penalty > 0 ? `−${e.item.penalty}` : "—"),
          sort: (e) => (e.section === "armor" ? e.item.penalty : null),
        },
        priceColumn(t, (e) => (e.section === "armor" ? e.item.price : null)),
      ];
    case "clothing":
      return [
        name,
        sub(t("reference.col.tags")),
        ...CLOTHING_COLUMNS.map((item, i) => ({
          id: item,
          label: t(`clothing.item.${item}`),
          numeric: true,
          text: (e: RefEntry) => (e.section === "clothing" ? String(e.item.prices[i]) : ""),
          sort: (e: RefEntry) => (e.section === "clothing" ? e.item.prices[i] : null),
        })),
      ];
    case "street_drugs":
      return [
        name,
        priceColumn(t, (e) => (e.section === "street_drugs" ? e.item.price : null)),
        {
          id: "hours",
          label: t("reference.col.hours"),
          numeric: true,
          text: (e) => (e.section === "street_drugs" ? t("reference.hours", { count: e.item.duration_hours }) : ""),
          sort: (e) => (e.section === "street_drugs" ? e.item.duration_hours : null),
        },
        {
          id: "secondary_dv",
          label: t("reference.col.secondary_dv"),
          numeric: true,
          text: (e) => (e.section === "street_drugs" ? String(e.item.secondary_dv) : ""),
          sort: (e) => (e.section === "street_drugs" ? e.item.secondary_dv : null),
        },
      ];
    case "gear":
      return [name, priceColumn(t, (e) => (e.section === "gear" ? e.item.price : null)), categoryColumn(t, (e) => (e.section === "gear" ? e.item.price : null))];
    case "ammo":
      return [
        name,
        priceColumn(t, (e) => (e.section === "ammo" ? e.item.price : null)),
        categoryColumn(t, (e) => (e.section === "ammo" ? e.item.price : null)),
        { id: "usable", label: t("reference.col.usable"), text: (e) => (e.section === "ammo" ? e.item.usable_with : "") },
      ];
    case "weapon_mods":
      return [
        name,
        priceColumn(t, (e) => (e.section === "weapon_mods" ? e.item.price : null)),
        {
          id: "slots",
          label: t("reference.col.slots"),
          numeric: true,
          text: (e) => (e.section === "weapon_mods" ? String(e.item.slots) : ""),
          sort: (e) => (e.section === "weapon_mods" ? e.item.slots : null),
        },
        categoryColumn(t, (e) => (e.section === "weapon_mods" ? e.item.price : null)),
      ];
    case "weapons":
      return [
        name,
        {
          id: "skill",
          label: t("reference.col.skill"),
          text: (e) => (e.section === "weapons" ? weaponSkillName(e.item, t) : ""),
        },
        {
          id: "damage",
          label: t("reference.col.damage"),
          numeric: true,
          text: (e) => (e.section === "weapons" ? (e.item.damage ?? "—") : ""),
          sort: (e) => (e.section === "weapons" ? averageDamage(e.item.damage) : null),
        },
        {
          id: "rof",
          label: t("reference.col.rof"),
          numeric: true,
          text: (e) => (e.section === "weapons" ? (e.item.rof === null ? "—" : String(e.item.rof)) : ""),
          sort: (e) => (e.section === "weapons" ? e.item.rof : null),
        },
        {
          id: "price",
          label: t("reference.col.price"),
          numeric: true,
          text: (e) => (e.section === "weapons" ? `${e.item.price} ${t("reference.eb")}` : ""),
          sort: (e) => (e.section === "weapons" ? e.item.price : null),
        },
      ];
    case "skills":
      return [
        name,
        {
          id: "stat",
          label: t("reference.col.stat"),
          text: (e) => (e.section === "skills" ? t(`stats.${STAT_ABBR[e.item.stat]}`) : ""),
        },
        {
          id: "category",
          label: t("reference.col.category"),
          text: (e) => (e.section === "skills" ? t(`skill.category.${e.item.category}`) : ""),
        },
        {
          id: "cost",
          label: t("reference.col.cost"),
          numeric: true,
          text: (e) => (e.section === "skills" && e.item.double_cost ? "×2" : "×1"),
          sort: (e) => (e.section === "skills" ? (e.item.double_cost ? 2 : 1) : null),
        },
      ];
    case "martial_arts":
      return [
        name,
        sub(t("reference.col.kind")),
        {
          id: "techniques",
          label: t("reference.col.techniques"),
          numeric: true,
          text: (e) => e.value || "—",
          sort: (e) => (e.value ? Number(e.value) : null),
        },
      ];
    case "roles":
      return [name, sub(t("reference.col.ability"))];
    case "articles":
      return [name, sub(t("reference.col.summary"))];
    case "reputation":
      return [name];
  }
}

/** Sorts a copy of `entries` by a column; empty cells stay at the end in both directions. */
export function sortEntries(entries: RefEntry[], column: RefColumn, dir: "asc" | "desc"): RefEntry[] {
  const key = (e: RefEntry) => (column.sort ? column.sort(e) : column.text(e));
  const sign = dir === "asc" ? 1 : -1;
  return [...entries].sort((a, b) => {
    const x = key(a);
    const y = key(b);
    if (x === null || y === null) return x === y ? 0 : x === null ? 1 : -1;
    if (typeof x === "number" && typeof y === "number") return (x - y) * sign;
    return String(x).localeCompare(String(y), undefined, { numeric: true }) * sign;
  });
}
