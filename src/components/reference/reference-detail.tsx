import type { ReactNode } from "react";
import { useTranslation } from "react-i18next";
import type { CriticalInjury } from "@/bindings/CriticalInjury";
import type { Stats } from "@/bindings/Stats";
import type { TreatmentOption } from "@/bindings/TreatmentOption";
import { weaponSkillName, type RefEntry } from "@/lib/reference-entries";

const STAT_ORDER: { key: keyof Stats; label: string }[] = [
  { key: "int", label: "INT" },
  { key: "reflex", label: "REF" },
  { key: "dex", label: "DEX" },
  { key: "tech", label: "TECH" },
  { key: "cool", label: "COOL" },
  { key: "will", label: "WILL" },
  { key: "luck", label: "LUCK" },
  { key: "movement", label: "MOVE" },
  { key: "body", label: "BODY" },
  { key: "emp", label: "EMP" },
];

type FieldProps = { label: string; children: ReactNode };

/** A labelled value: the label is small and muted, the value is what the eye looks for. */
function Field({ label, children }: FieldProps) {
  return (
    <div className="min-w-0">
      <div className="text-xs text-muted-foreground uppercase">{label}</div>
      <div className="num text-sm">{children}</div>
    </div>
  );
}

/** A big number with a caption: damage, SP, HP, roll. */
function Big({ label, children }: FieldProps) {
  return (
    <div className="cut shrink-0 px-4 py-2 text-center [--cut:6px]">
      <div className="num font-display text-3xl leading-none font-bold text-primary">{children}</div>
      <div className="mt-1 text-xs text-muted-foreground uppercase">{label}</div>
    </div>
  );
}

function Chip({ children }: { children: ReactNode }) {
  return <span className="border px-2 py-0.5 text-xs">{children}</span>;
}

function Block({ title, children }: { title: string; children: ReactNode }) {
  return (
    <section className="space-y-1">
      <h4 className="text-xs text-muted-foreground">{title}</h4>
      {children}
    </section>
  );
}

function Header({ name, subtitle, big }: { name: string; subtitle?: string; big?: ReactNode }) {
  return (
    <div className="flex items-start gap-4">
      <div className="min-w-0 flex-1">
        <h3 className="text-xl font-bold">{name}</h3>
        {subtitle && <p className="text-sm text-muted-foreground">{subtitle}</p>}
      </div>
      {big}
    </div>
  );
}

function TreatmentList({ options }: { options: TreatmentOption[] }) {
  const { t } = useTranslation();
  return (
    <ul className="space-y-0.5 text-sm">
      {options.map((o) => (
        <li key={o.skill}>
          {o.skill === "surgery" ? t("heal.skill_surgery") : t(`skill.items.${o.skill}`)} —{" "}
          {t("reference.detail.dv")} <span className="num font-bold">{o.dv}</span>
        </li>
      ))}
    </ul>
  );
}

function InjuryCard({ injury }: { injury: CriticalInjury }) {
  const { t } = useTranslation();
  return (
    <>
      <Header
        name={t(injury.name_key)}
        subtitle={t(`reference.area.${injury.area}`)}
        big={<Big label={t("reference.detail.roll")}>{injury.roll}</Big>}
      />
      <p className="text-sm">{t(injury.effect_key)}</p>
      <div className="flex flex-wrap gap-2">
        {injury.death_save_penalty && <Chip>{t("reference.detail.death_save_penalty")}</Chip>}
        {injury.quick_fix_removes_effect && <Chip>{t("reference.detail.quick_fix")}</Chip>}
      </div>
      <Block title={t("reference.detail.first_aid")}>
        {injury.first_aid.length > 0 ? (
          <TreatmentList options={injury.first_aid} />
        ) : (
          <p className="text-sm text-muted-foreground">{t("reference.detail.impossible")}</p>
        )}
      </Block>
      {injury.treatment.length > 0 && (
        <Block title={t("reference.detail.treatment")}>
          <TreatmentList options={injury.treatment} />
        </Block>
      )}
    </>
  );
}

/** The detail card of one reference entry: a layout per section, not a generic text blob. */
export function ReferenceDetail({ entry }: { entry: RefEntry }) {
  const { t } = useTranslation();

  const body = (() => {
    switch (entry.section) {
      case "injuries":
        return <InjuryCard injury={entry.item} />;
      case "cover": {
        const c = entry.item;
        return (
          <>
            <Header
              name={c.name}
              subtitle={c.thickness ? t(`cover.thickness.${c.thickness}`) : undefined}
              big={<Big label={t("reference.detail.hp")}>{c.hp ?? "—"}</Big>}
            />
            {c.hp === null && <p className="text-sm text-muted-foreground">{t("reference.detail.hp_manual")}</p>}
          </>
        );
      }
      case "armor": {
        const a = entry.item;
        return (
          <>
            <Header name={a.name} big={<Big label={t("glossary.sp")}>{a.sp}</Big>} />
            <Field label={t("reference.detail.penalty")}>
              {a.penalty > 0 ? t("reference.penalty", { value: a.penalty }) : t("reference.detail.none")}
            </Field>
          </>
        );
      }
      case "weapons": {
        const w = entry.item;
        return (
          <>
            <Header
              name={w.name}
              subtitle={w.examples ?? undefined}
              big={
                w.damage ? (
                  <Big label={t("reference.detail.damage")}>{w.damage}</Big>
                ) : (
                  <Big label={t("reference.detail.price")}>
                    {w.price} {t("reference.eb")}
                  </Big>
                )
              }
            />
            {w.description && <p className="text-sm">{w.description}</p>}
            <div className="grid grid-cols-2 gap-3 sm:grid-cols-3">
              <Field label={t("reference.detail.skill")}>{weaponSkillName(w, t)}</Field>
              <Field label={t("reference.detail.kind")}>{t(`reference.weapon.kind.${w.kind}`)}</Field>
              {w.rof !== null && <Field label={t("reference.detail.rof")}>{w.rof}</Field>}
              {w.magazine !== null && <Field label={t("reference.detail.magazine")}>{w.magazine}</Field>}
              {w.ammo && <Field label={t("reference.detail.ammo")}>{w.ammo}</Field>}
              {w.hands !== null && <Field label={t("reference.detail.hands")}>{w.hands}</Field>}
              {w.concealable !== null && (
                <Field label={t("reference.detail.concealable")}>
                  {t(w.concealable ? "reference.detail.yes" : "reference.detail.no")}
                </Field>
              )}
              {w.damage && (
                <Field label={t("reference.detail.price")}>
                  {w.price} {t("reference.eb")}
                </Field>
              )}
            </div>
            {w.features.length > 0 && (
              <div className="flex flex-wrap gap-2">
                {w.features.map((f) => (
                  <Chip key={f}>{f}</Chip>
                ))}
              </div>
            )}
          </>
        );
      }
      case "skills": {
        const s = entry.item;
        return (
          <>
            <Header name={t(s.name_key)} subtitle={t(`skill.category.${s.category}`)} />
            <div className="grid grid-cols-2 gap-3 sm:grid-cols-3">
              <Field label={t("reference.detail.stat")}>{t(`stats.${s.stat}`)}</Field>
              <Field label={t("reference.detail.cost")}>{s.double_cost ? "×2" : "×1"}</Field>
            </div>
            {s.specialty && <p className="text-sm text-muted-foreground">{t("reference.detail.specialty")}</p>}
          </>
        );
      }
      case "npc": {
        const n = entry.item;
        const skills = Object.entries(n.skill_totals).sort(([, a], [, b]) => b - a);
        return (
          <>
            <Header
              name={n.name}
              subtitle={t(`npc.tier.${n.tier}`)}
              big={
                <Big label={t("reference.detail.armor_sp")}>
                  {n.armor_head_sp}/{n.armor_body_sp}
                </Big>
              }
            />
            <div className="grid grid-cols-5 gap-2">
              {STAT_ORDER.map(({ key, label }) => (
                <Field key={key} label={t(`stats.${label}`)}>
                  <span className="font-bold">{n.stats[key]}</span>
                </Field>
              ))}
            </div>
            <Block title={t("reference.detail.armor")}>
              <p className="text-sm">{n.armor_name}</p>
            </Block>
            {n.weapons.length > 0 && (
              <Block title={t("reference.detail.weapons")}>
                <ul className="space-y-0.5 text-sm">
                  {n.weapons.map((w) => (
                    <li key={w.name}>
                      {w.name} — <span className="num font-bold">{w.damage}</span>
                    </li>
                  ))}
                </ul>
              </Block>
            )}
            {skills.length > 0 && (
              <Block title={t("reference.detail.skills")}>
                <div className="flex flex-wrap gap-2">
                  {skills.map(([key, total]) => {
                    const [id, specialty] = key.split(":");
                    return (
                      <Chip key={key}>
                        {t(`skill.items.${id}`, { defaultValue: id })}
                        {specialty ? ` (${specialty})` : ""} <span className="num font-bold">{total}</span>
                      </Chip>
                    );
                  })}
                </div>
              </Block>
            )}
            {n.gear && (
              <Block title={t("reference.detail.gear")}>
                <p className="text-sm">{n.gear}</p>
              </Block>
            )}
            {n.notes && (
              <Block title={t("reference.detail.notes")}>
                <p className="text-sm whitespace-pre-line">{n.notes}</p>
              </Block>
            )}
          </>
        );
      }
      case "roles": {
        const r = entry.item;
        return (
          <>
            <Header name={r.name} subtitle={r.ability_name} />
            <p className="text-sm whitespace-pre-line">{r.description || r.summary}</p>
            {r.description && r.summary && (
              <p className="text-sm text-muted-foreground">{r.summary}</p>
            )}
            {r.ranks.length > 0 && (
              <Block title={t("reference.detail.ranks")}>
                <ol className="divide-y border text-sm">
                  {r.ranks.map((n) => (
                    <li key={n.rank} className="flex gap-3 px-3 py-1.5">
                      <span className="num w-5 shrink-0 font-bold text-primary">{n.rank}</span>
                      <span>{n.summary}</span>
                    </li>
                  ))}
                </ol>
              </Block>
            )}
            {r.subabilities.map((sub) => (
              <Block key={sub.code} title={sub.name}>
                <p className="text-sm">{sub.summary}</p>
                {sub.options.length > 0 && (
                  <ul className="space-y-0.5 text-sm">
                    {sub.options.map((o) => (
                      <li key={o.code}>
                        <span className="font-medium">{o.name}</span>
                        {o.summary && <span className="text-muted-foreground"> — {o.summary}</span>}
                      </li>
                    ))}
                  </ul>
                )}
                {sub.levels.length > 0 && (
                  <ul className="space-y-0.5 text-sm">
                    {sub.levels.map((l) => (
                      <li key={l.points}>
                        <span className="num font-bold text-primary">{l.points}</span>{" "}
                        <span>{l.summary}</span>
                      </li>
                    ))}
                  </ul>
                )}
              </Block>
            ))}
          </>
        );
      }
      case "reputation": {
        const r = entry.item;
        return r.kind === "rule" ? (
          <>
            <Header name={entry.name} />
            <p className="text-sm whitespace-pre-line">{t(`reputation.rule.${r.key}.detail`)}</p>
          </>
        ) : (
          <>
            <Header name={entry.name} big={<Big label={t("reference.detail.level")}>{r.level}</Big>} />
            <p className="text-sm">{t(`reputation.levels.${r.level}`)}</p>
          </>
        );
      }
    }
  })();

  const page = "source" in entry.item ? entry.item.source.page : null;
  return (
    <article className="space-y-4">
      {body}
      {page !== null && (
        <p className="num text-xs text-muted-foreground">{t("reference.detail.source", { page })}</p>
      )}
    </article>
  );
}
