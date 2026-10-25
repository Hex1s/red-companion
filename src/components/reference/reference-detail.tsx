import { useSkillText } from "@/components/character/skill-hint";
import type { ReactNode } from "react";
import { useTranslation } from "react-i18next";
import type { CriticalInjury } from "@/bindings/CriticalInjury";
import type { TreatmentOption } from "@/bindings/TreatmentOption";
import type { ReferenceData } from "@/hooks/use-reference-data";
import { weaponSkillName, type RefEntry } from "@/lib/reference-entries";
import { priceCategoryName } from "@/lib/price";
import { cn } from "@/lib/utils";

/** Pieces of clothing in the order of the price columns (`catalog::CLOTHING_ITEMS`). */
const CLOTHING_ITEMS = ["pants", "top", "jacket", "shoes", "jewelry", "mirrorshades", "glasses", "contacts", "hats"];
import { STAT_ABBR } from "@/lib/stat-abbr";

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
      <div className="num font-display text-3xl leading-none font-bold text-emphasis">{children}</div>
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

/** Magazine sizes by weapon class (p. 344); the column of the chosen modification is marked. */
function MagazineTable({ which, data }: { which: "extended" | "drum"; data: ReferenceData }) {
  const { t } = useTranslation();
  const { magazines, weapons } = data;
  if (magazines.length === 0) return null;
  const name = (id: string) => weapons.find((w) => w.code === id)?.name ?? id;
  return (
    <Block title={t("reference.detail.magazines")}>
      <table className="w-full text-sm">
        <thead>
          <tr className="text-xs text-muted-foreground uppercase">
            <th className="py-1 text-left font-normal" />
            <th className="py-1 text-right font-normal">{t("reference.detail.mag_standard")}</th>
            <th className="py-1 text-right font-normal">{t("reference.detail.mag_extended")}</th>
            <th className="py-1 text-right font-normal">{t("reference.detail.mag_drum")}</th>
          </tr>
        </thead>
        <tbody>
          {magazines.map((r) => (
            <tr key={r.id} className="border-t">
              <td className="py-1">{name(r.id)}</td>
              <td className="num py-1 text-right text-muted-foreground">{r.standard}</td>
              <td className={cn("num py-1 text-right", which === "extended" ? "font-bold" : "text-muted-foreground")}>{r.extended}</td>
              <td className={cn("num py-1 text-right", which === "drum" ? "font-bold" : "text-muted-foreground")}>{r.drum}</td>
            </tr>
          ))}
        </tbody>
      </table>
    </Block>
  );
}

function SkillText({ id }: { id: string }) {
  const { summary, details } = useSkillText(id);
  return (
    <>
      {summary && <p className="text-sm">{summary}</p>}
      {details && <p className="text-sm text-muted-foreground">{details}</p>}
    </>
  );
}

/** The detail card of one reference entry: a layout per section, not a generic text blob. */
export function ReferenceDetail({ entry, data }: { entry: RefEntry; data: ReferenceData }) {
  const { t } = useTranslation();

  const body = (() => {
    switch (entry.section) {
      case "articles": {
        const a = entry.item;
        return (
          <>
            <Header name={t(a.name_key)} subtitle={t(a.summary_key)} />
            {a.details_key && <p className="text-sm whitespace-pre-line">{t(a.details_key)}</p>}
          </>
        );
      }
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
            <div className="grid grid-cols-2 gap-3">
              <Field label={t("reference.detail.penalty")}>
                {a.penalty > 0 ? t("reference.penalty", { value: a.penalty }) : t("reference.detail.none")}
              </Field>
              <Field label={t("reference.detail.price")}>
                {a.price} {t("reference.eb")}
              </Field>
            </div>
          </>
        );
      }
      case "clothing": {
        const c = entry.item;
        return (
          <>
            <Header name={c.name} subtitle={c.tags} />
            <ul className="divide-y border">
              {c.prices.map((price, i) => (
                <li key={CLOTHING_ITEMS[i]} className="flex items-baseline justify-between gap-3 px-3 py-1.5 text-sm">
                  <span>{t(`clothing.item.${CLOTHING_ITEMS[i]}`)}</span>
                  <span className="num">
                    {price} {t("reference.eb")}
                    <span className="ml-2 text-xs text-muted-foreground">
                      {priceCategoryName(data.priceCategories, price, t)}
                    </span>
                  </span>
                </li>
              ))}
            </ul>
          </>
        );
      }
      case "street_drugs": {
        const d = entry.item;
        return (
          <>
            <Header
              name={d.name}
              subtitle={entry.subtitle}
              big={
                <Big label={t("reference.detail.price")}>
                  {d.price} {t("reference.eb")}
                </Big>
              }
            />
            <div className="grid grid-cols-2 gap-3">
              <Field label={t("reference.detail.duration")}>{t("reference.hours", { count: d.duration_hours })}</Field>
              <Field label={t("reference.detail.secondary_dv")}>{d.secondary_dv}</Field>
            </div>
            <Block title={t("reference.detail.primary")}>
              <p className="text-sm">{d.primary}</p>
            </Block>
            <Block title={t("reference.detail.secondary")}>
              <p className="text-sm">{d.secondary}</p>
            </Block>
          </>
        );
      }
      case "gear": {
        const g = entry.item;
        return (
          <>
            <Header
              name={g.name}
              subtitle={entry.subtitle}
              big={
                <Big label={t("reference.detail.price")}>
                  {g.price} {t("reference.eb")}
                </Big>
              }
            />
            <p className="text-sm">{g.summary}</p>
          </>
        );
      }
      case "ammo": {
        const a = entry.item;
        return (
          <>
            <Header
              name={a.name}
              subtitle={entry.subtitle}
              big={
                <Big label={t("reference.detail.price")}>
                  {a.price} {t("reference.eb")}
                </Big>
              }
            />
            <p className="text-sm">{a.summary}</p>
            <Field label={t("reference.detail.usable")}>{a.usable_with}</Field>
          </>
        );
      }
      case "weapon_mods": {
        const m = entry.item;
        return (
          <>
            <Header
              name={m.name}
              subtitle={entry.subtitle}
              big={
                <Big label={t("reference.detail.price")}>
                  {m.price} {t("reference.eb")}
                </Big>
              }
            />
            <p className="text-sm">{m.summary}</p>
            <div className="grid grid-cols-2 gap-3">
              <Field label={t("reference.detail.slots")}>{m.slots}</Field>
              <Field label={t("reference.detail.fits")}>
                {m.skills.map((s) => t(`skill.items.${s}`)).join(", ")}
              </Field>
            </div>
            {(m.code === "extended_magazine" || m.code === "drum_magazine") && (
              <MagazineTable which={m.code === "drum_magazine" ? "drum" : "extended"} data={data} />
            )}
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
            <SkillText id={s.id} />
            <div className="grid grid-cols-2 gap-3 sm:grid-cols-3">
              <Field label={t("reference.detail.stat")}>{t(`stats.${STAT_ABBR[s.stat]}`)}</Field>
              <Field label={t("reference.detail.cost")}>{s.double_cost ? "×2" : "×1"}</Field>
            </div>
            {s.specialty && <p className="text-sm text-muted-foreground">{t("reference.detail.specialty")}</p>}
          </>
        );
      }
      case "martial_arts": {
        const m = entry.item;
        return (
          <>
            <Header name={m.name} subtitle={t(`reference.martial_kind.${m.kind}`)} />
            <p className="text-sm">{m.summary}</p>
            <div className="space-y-3">
              {m.techniques.map((x) => (
                <section key={x.code} className="cut space-y-2 p-3 [--cut:6px]">
                  <div className="flex flex-wrap items-center gap-2">
                    <h4 className="text-sm">{x.name}</h4>
                    {x.dv !== null && (
                      <Chip>
                        {t("reference.detail.dv")} <span className="num font-bold">{x.dv}</span>
                      </Chip>
                    )}
                    {x.injuries.map((id) => (
                      <Chip key={id}>{t(`injury.${id}.name`)}</Chip>
                    ))}
                  </div>
                  {x.condition && (
                    <p className="text-sm text-muted-foreground">
                      {t("reference.detail.condition")}: {x.condition}
                    </p>
                  )}
                  <p className="text-sm">{x.effect}</p>
                </section>
              ))}
            </div>
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
                      <span className="num w-5 shrink-0 font-bold text-emphasis">{n.rank}</span>
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
                        <span className="num font-bold text-emphasis">{l.points}</span>{" "}
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
      case "reputation":
        // Reputation is one page of its own (ReputationReference), not a card.
        return null;
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
