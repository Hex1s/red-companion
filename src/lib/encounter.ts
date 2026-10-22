import type { TFunction } from "i18next";
import type { NpcTemplate } from "@/bindings/NpcTemplate";
import type { TemplateSource } from "@/bindings/TemplateSource";
import type { UserTemplateRecord } from "@/bindings/UserTemplateRecord";

/** One string per template, so a select can hold book and own templates together. */
export const templateKey = (source: TemplateSource, template: string) => `${source}:${template}`;

/** Display name of a template key, or `null` when the template no longer exists. */
export function templateLabel(
  key: string,
  book: NpcTemplate[],
  own: UserTemplateRecord[],
): string | null {
  if (key.startsWith("book:")) return book.find((x) => templateKey("book", x.code) === key)?.name ?? null;
  return own.find((x) => templateKey("custom", String(x.id)) === key)?.character.name ?? null;
}

/** "Booster" for the first one alone, otherwise "Booster 2", "Booster 3"... after the existing ones. */
export function cloneNames(base: string, count: number, existing: string[]): string[] {
  const escaped = base.replace(/[.*+?^${}()|[\]\\]/g, "\\$&");
  const same = new RegExp(`^${escaped}( \\d+)?$`);
  const taken = existing.filter((n) => same.test(n)).length;
  if (taken === 0 && count === 1) return [base];
  return Array.from({ length: count }, (_, i) => `${base} ${taken + i + 1}`);
}

/** Weapons, gear and notes of a book template as text for the clone's notes field. */
export function npcNotes(t: TFunction, template: NpcTemplate): string {
  return [
    `${t("npc.notes.weapons")}: ${template.weapons.map((w) => `${w.name} ${w.damage}`).join(", ")}`,
    `${t("npc.notes.gear")}: ${template.gear}`,
    template.notes,
  ]
    .filter(Boolean)
    .join("\n");
}
