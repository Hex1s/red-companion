import { useEffect, useMemo, useState } from "react";
import { useTranslation } from "react-i18next";
import type { Armor } from "@/bindings/Armor";
import type { CoverPreset } from "@/bindings/CoverPreset";
import type { CriticalInjury } from "@/bindings/CriticalInjury";
import type { Role } from "@/bindings/Role";
import type { NpcTemplate } from "@/bindings/NpcTemplate";
import type { Skill } from "@/bindings/Skill";
import type { Weapon } from "@/bindings/Weapon";
import type { WeaponKind } from "@/bindings/WeaponKind";
import { Button } from "@/components/ui/button";
import { Checkbox } from "@/components/ui/checkbox";
import { Input } from "@/components/ui/input";
import { Tabs, TabsList, TabsTrigger } from "@/components/ui/tabs";
import { api, hasBackend } from "@/lib/api";
import { useUiStore, type ReferenceSection } from "@/store/ui-store";

const SECTIONS: ReferenceSection[] = ["injuries", "cover", "armor", "weapons", "skills", "npc", "roles", "reputation"];

type Entry = {
  id: string;
  name: string;
  /** Secondary line: effect, stat, tier; also searched. */
  detail: string;
  /** Short number shown on the right (roll, HP, SP). */
  value: string;
  /** Fields the section filters work on. */
  weaponKind?: WeaponKind;
  concealable?: boolean;
  penalty?: number;
  sp?: number;
};

const WEAPON_KINDS: WeaponKind[] = ["melee", "ranged", "exotic"];

type Data = {
  injuries: CriticalInjury[];
  cover: CoverPreset[];
  armor: Armor[];
  weapons: Weapon[];
  skills: Skill[];
  npc: NpcTemplate[];
  roles: Role[];
};

const EMPTY: Data = { injuries: [], cover: [], armor: [], weapons: [], skills: [], npc: [], roles: [] };

/** Case-insensitive search that treats "ё" as "е". */
const fold = (text: string) => text.toLowerCase().replace(/ё/g, "е");

/** Reference pages: one section at a time, filtered by the search box over translated texts. */
export function ReferencePage() {
  const { t, i18n } = useTranslation();
  const { referenceSection, setReferenceSection, referenceQuery, setReferenceQuery } = useUiStore();
  const [data, setData] = useState<Data>(EMPTY);
  // Section filters; they are plain view state and reset with the page.
  const [weaponKind, setWeaponKind] = useState<WeaponKind | null>(null);
  const [onlyConcealable, setOnlyConcealable] = useState(false);
  const [noPenalty, setNoPenalty] = useState(false);
  const [minSp, setMinSp] = useState("");

  useEffect(() => {
    if (!hasBackend) return;
    Promise.all([
      api.listInjuries(),
      api.listCoverPresets(i18n.language),
      api.listArmor(i18n.language),
      api.listWeapons(i18n.language),
      api.listSkills(),
      api.listNpcTemplates(i18n.language),
      api.listRoles(i18n.language),
    ]).then(
      ([injuries, cover, armor, weapons, skills, npc, roles]) =>
        setData({ injuries, cover, armor, weapons, skills, npc, roles }),
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
          penalty: a.penalty,
          sp: a.sp,
        }));
      case "weapons":
        return data.weapons.map((w) => ({
          id: w.code,
          name: w.name,
          detail: [
            w.skill ? t(`skill.items.${w.skill}`) : t("reference.weapon.kind.exotic"),
            w.examples,
            w.ammo,
            ...w.features,
            w.description,
          ]
            .filter(Boolean)
            .join(" · "),
          value: w.damage ?? `${w.price} ${t("reference.eb")}`,
          weaponKind: w.kind,
          concealable: w.concealable ?? undefined,
        }));
      case "skills":
        return data.skills.map((s) => ({
          id: s.id,
          name: t(s.name_key),
          detail: `${t(`stats.${s.stat}`)} · ${t(`skill.category.${s.category}`)}`,
          value: s.double_cost ? "×2" : "",
        }));
      case "reputation":
        return [
          ...["how", "negative", "intimidation"].map((rule) => ({
            id: `rule_${rule}`,
            name: t(`reputation.rule.${rule}.name`),
            detail: t(`reputation.rule.${rule}.detail`),
            value: "",
          })),
          ...Array.from({ length: 10 }, (_, i) => ({
            id: `level_${i + 1}`,
            name: t("reputation.level", { level: i + 1 }),
            detail: t(`reputation.levels.${i + 1}`),
            value: String(i + 1),
          })),
        ];
      case "roles":
        return data.roles.map((r) => ({
          id: r.code,
          name: `${r.name} — ${r.ability_name}`,
          detail: [
            r.description || r.summary,
            ...r.ranks.map((n) => `${n.rank}: ${n.summary}`),
            ...r.subabilities.flatMap((sub) => [
              `${sub.name}: ${sub.summary}`,
              ...sub.options.map((o) => `${o.name}: ${o.summary}`),
              ...sub.levels.map((l) => `${sub.name} ${l.points}: ${l.summary}`),
            ]),
          ].join("\n"),
          value: "",
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
  const spFrom = Number(minSp) || 0;
  const shown = entries.filter((e) => {
    if (query && !fold(`${e.name} ${e.detail}`).includes(query)) return false;
    if (referenceSection === "weapons") {
      if (weaponKind && e.weaponKind !== weaponKind) return false;
      if (onlyConcealable && !e.concealable) return false;
    }
    if (referenceSection === "armor") {
      if (noPenalty && (e.penalty ?? 0) > 0) return false;
      if ((e.sp ?? 0) < spFrom) return false;
    }
    return true;
  });

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
      {referenceSection === "weapons" && (
        <div className="flex flex-wrap items-center gap-2">
          {[null, ...WEAPON_KINDS].map((k) => (
            <Button
              key={k ?? "all"}
              size="sm"
              variant={weaponKind === k ? "default" : "outline"}
              onClick={() => setWeaponKind(k)}
            >
              {t(k ? `reference.weapon.kind.${k}` : "reference.filter.all")}
            </Button>
          ))}
          <label className="flex items-center gap-2 text-sm whitespace-nowrap">
            <Checkbox
              checked={onlyConcealable}
              onCheckedChange={(v) => setOnlyConcealable(v === true)}
            />
            {t("reference.filter.concealable")}
          </label>
        </div>
      )}
      {referenceSection === "armor" && (
        <div className="flex flex-wrap items-center gap-4">
          <label className="flex items-center gap-2 text-sm whitespace-nowrap">
            <Checkbox checked={noPenalty} onCheckedChange={(v) => setNoPenalty(v === true)} />
            {t("reference.filter.no_penalty")}
          </label>
          <label className="flex items-center gap-2 text-sm whitespace-nowrap">
            {t("reference.filter.sp_from")}
            <Input
              className="num w-16"
              inputMode="numeric"
              value={minSp}
              onChange={(e) => setMinSp(e.target.value.replace(/\D/g, ""))}
            />
          </label>
        </div>
      )}
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
                {e.detail && <div className="whitespace-pre-line text-sm text-muted-foreground">{e.detail}</div>}
              </div>
              <span className="num text-muted-foreground">{e.value}</span>
            </li>
          ))}
        </ul>
      )}
    </div>
  );
}
