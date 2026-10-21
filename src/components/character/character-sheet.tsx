import { zodResolver } from "@hookform/resolvers/zod";
import { useMemo, useState, type ReactNode } from "react";
import { useForm, type FieldErrors, type Path } from "react-hook-form";
import { useTranslation } from "react-i18next";
import type { Character } from "@/bindings/Character";
import type { Skill } from "@/bindings/Skill";
import type { Stats } from "@/bindings/Stats";
import { HpBar } from "@/components/character/hp-bar";
import { WoundBadge } from "@/components/character/wound-badge";
import { Button } from "@/components/ui/button";
import {
  Dialog,
  DialogContent,
  DialogDescription,
  DialogFooter,
  DialogHeader,
  DialogTitle,
} from "@/components/ui/dialog";
import { Input } from "@/components/ui/input";
import { Label } from "@/components/ui/label";
import { errorMessage, type CharacterEntry } from "@/hooks/use-characters";
import { useSkills } from "@/hooks/use-skills";
import { characterSchema, SKILL_MAX, SP_MAX, STAT_MAX, STAT_MIN } from "@/lib/character-schema";

const STAT_KEYS: { key: keyof Stats; label: string }[] = [
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

const STAT_LABELS = Object.fromEntries(STAT_KEYS.map(({ key, label }) => [key, label])) as Record<
  keyof Stats,
  string
>;

type CharacterSheetProps = {
  entry: CharacterEntry;
  onSave: (character: Character) => Promise<unknown>;
  onDelete: () => Promise<unknown>;
};

type SectionProps = { title: string; children: ReactNode };

function Section({ title, children }: SectionProps) {
  return (
    <section className="space-y-2">
      <h3 className="text-sm text-muted-foreground">{title}</h3>
      {children}
    </section>
  );
}

type StatTileProps = { label: string; value: string | number };

function StatTile({ label, value }: StatTileProps) {
  return (
    <div className="cut [--cut:6px] p-2">
      <div className="text-xs text-muted-foreground">{label}</div>
      <div className="num text-lg font-bold">{value}</div>
    </div>
  );
}

/** Editable character sheet. Derived numbers come from Rust (saved record). */
export function CharacterSheet({ entry, onSave, onDelete }: CharacterSheetProps) {
  const { t } = useTranslation();
  const { derived } = entry;
  const [status, setStatus] = useState<{ kind: "saved" | "error"; text: string } | null>(null);
  const [confirmDelete, setConfirmDelete] = useState(false);
  const skills = useSkills();

  // Every catalog skill gets an input; missing levels start at 0.
  const values = useMemo<Character>(
    () => ({
      ...entry.character,
      skills: {
        ...Object.fromEntries(skills.map((s) => [s.id, 0])),
        ...entry.character.skills,
      },
    }),
    [entry.character, skills],
  );
  const skillGroups = useMemo(() => {
    const groups = new Map<Skill["category"], Skill[]>();
    for (const skill of skills) groups.set(skill.category, [...(groups.get(skill.category) ?? []), skill]);
    return [...groups];
  }, [skills]);

  const {
    register,
    handleSubmit,
    formState: { errors, isDirty, isSubmitting },
  } = useForm<Character>({
    resolver: zodResolver(characterSchema),
    values,
  });

  const submit = handleSubmit(
    async (values) => {
      try {
        // Level 0 is the default, so only trained skills are stored.
        const trained = Object.entries(values.skills).filter(([, level]) => level > 0);
        await onSave({ ...values, skills: Object.fromEntries(trained) });
        setStatus({ kind: "saved", text: t("characters.saved") });
      } catch (e) {
        setStatus({ kind: "error", text: t("characters.save_error", { error: errorMessage(e) }) });
      }
    },
    () => setStatus(null),
  );

  const num = (name: Path<Character>) => register(name, { valueAsNumber: true });

  const fieldError = (e: FieldErrors<Character>, ...path: string[]): string | null => {
    let node: unknown = e;
    for (const p of path) node = (node as Record<string, unknown> | undefined)?.[p];
    const message = (node as { message?: string } | undefined)?.message;
    if (!message) return null;
    return message === "name" || message === "sp_over_max"
      ? t(`characters.validation.${message}`)
      : t("characters.validation.range", { min: STAT_MIN, max: STAT_MAX });
  };

  const armorField = (area: "armor_head" | "armor_body", label: string) => {
    const error = fieldError(errors, area, "sp");
    return (
      <div className="cut [--cut:6px] space-y-2 p-3">
        <div className="text-xs text-muted-foreground">{label}</div>
        <div className="grid grid-cols-2 gap-2">
          <div className="space-y-1">
            <Label htmlFor={`${area}.sp`}>{t("characters.sp")}</Label>
            <Input
              id={`${area}.sp`}
              type="number"
              min={0}
              max={SP_MAX}
              className="num"
              {...num(`${area}.sp`)}
            />
          </div>
          <div className="space-y-1">
            <Label htmlFor={`${area}.max_sp`}>{t("characters.sp_max")}</Label>
            <Input
              id={`${area}.max_sp`}
              type="number"
              min={0}
              max={SP_MAX}
              className="num"
              {...num(`${area}.max_sp`)}
            />
          </div>
        </div>
        {error && <p className="text-xs text-destructive">{error}</p>}
      </div>
    );
  };

  const nameError = fieldError(errors, "name");

  return (
    <form onSubmit={submit} className="max-w-3xl space-y-6" noValidate>
      <div className="flex items-end gap-3">
        <div className="flex-1 space-y-1">
          <Label htmlFor="name">{t("characters.name")}</Label>
          <Input id="name" aria-invalid={!!errors.name} {...register("name")} />
          {nameError && <p className="text-xs text-destructive">{nameError}</p>}
        </div>
        <span className="pb-2 text-xs text-muted-foreground uppercase">
          {t(`characters.kind.${entry.kind}`)}
        </span>
        <Button type="submit" disabled={!isDirty || isSubmitting}>
          {t("characters.save")}
        </Button>
        <Button type="button" variant="outline" onClick={() => setConfirmDelete(true)}>
          {t("characters.delete")}
        </Button>
      </div>
      {status && (
        <p className={status.kind === "error" ? "text-sm text-destructive" : "text-sm text-state-ok"}>
          {status.text}
        </p>
      )}

      <Section title={t("characters.section.stats")}>
        <div className="grid grid-cols-5 gap-2">
          {STAT_KEYS.map(({ key, label }) => (
            <div key={key} className="cut [--cut:6px] space-y-1 p-2">
              <Label htmlFor={`stats.${key}`} className="text-xs text-muted-foreground">
                {t(`stats.${label}`)}
              </Label>
              <Input
                id={`stats.${key}`}
                type="number"
                min={key === "emp" ? 0 : STAT_MIN}
                max={STAT_MAX}
                className="num text-lg font-bold"
                aria-invalid={!!errors.stats?.[key]}
                {...num(`stats.${key}`)}
              />
            </div>
          ))}
        </div>
      </Section>

      <Section title={t("characters.section.derived")}>
        <div className="cut [--cut:6px] space-y-3 p-3">
          <div className="flex items-end gap-3">
            <div className="w-28 space-y-1">
              <Label htmlFor="hp">{t("characters.hp")}</Label>
              <Input id="hp" type="number" min={0} className="num" {...num("hp")} />
            </div>
            <HpBar current={entry.character.hp} max={derived.max_hp} className="flex-1 pb-3" />
            <WoundBadge state={derived.wound_state} className="pb-2" />
          </div>
          <div className="grid grid-cols-4 gap-2">
            <StatTile label={t("characters.max_hp")} value={derived.max_hp} />
            <StatTile label={t("characters.serious_threshold")} value={derived.serious_threshold} />
            <StatTile label={t("characters.death_save")} value={`≤ ${derived.death_save}`} />
            <StatTile label={t("characters.humanity")} value={derived.max_humanity} />
          </div>
          <p className="text-xs text-muted-foreground">{t("characters.derived_hint")}</p>
        </div>
      </Section>

      <Section title={t("characters.section.armor")}>
        <div className="grid grid-cols-2 gap-2">
          {armorField("armor_head", t("armor.head"))}
          {armorField("armor_body", t("armor.body"))}
        </div>
      </Section>

      <Section title={t("characters.section.injuries")}>
        {entry.character.injuries.length === 0 ? (
          <p className="text-sm text-muted-foreground">{t("characters.no_injuries")}</p>
        ) : (
          <ul className="space-y-1 text-sm">
            {entry.character.injuries.map((id) => (
              <li key={id} className="cut [--cut:4px] px-3 py-1.5">
                {t(`injury.${id}.name`, { defaultValue: id })}
              </li>
            ))}
          </ul>
        )}
      </Section>

      {skillGroups.length > 0 && (
        <Section title={t("characters.section.skills")}>
          <div className="space-y-3">
            {skillGroups.map(([category, group]) => (
              <div key={category} className="cut [--cut:6px] space-y-1 p-3">
                <div className="text-xs text-muted-foreground">{t(`skill.category.${category}`)}</div>
                <ul className="grid grid-cols-2 gap-x-6 gap-y-1">
                  {group.map((skill) => (
                    <li key={skill.id} className="flex items-center gap-2 text-sm">
                      <Label htmlFor={`skills.${skill.id}`} className="flex-1 font-normal">
                        {t(skill.name_key)}
                        {skill.double_cost && <span className="num text-muted-foreground"> ×2</span>}
                      </Label>
                      <span className="text-xs text-muted-foreground">
                        {t(`stats.${STAT_LABELS[skill.stat]}`)}
                      </span>
                      <Input
                        id={`skills.${skill.id}`}
                        type="number"
                        min={0}
                        max={SKILL_MAX}
                        className="num h-7 w-14"
                        aria-invalid={!!errors.skills?.[skill.id]}
                        {...num(`skills.${skill.id}`)}
                      />
                    </li>
                  ))}
                </ul>
              </div>
            ))}
          </div>
        </Section>
      )}

      <Section title={t("characters.notes")}>
        <textarea
          rows={4}
          className="w-full border border-input bg-transparent px-3 py-2 text-sm outline-none focus-visible:border-ring"
          {...register("notes")}
        />
      </Section>

      <Dialog open={confirmDelete} onOpenChange={setConfirmDelete}>
        <DialogContent>
          <DialogHeader>
            <DialogTitle>{t("characters.delete_title")}</DialogTitle>
            <DialogDescription>
              {t("characters.delete_body", { name: entry.character.name })}
            </DialogDescription>
          </DialogHeader>
          <DialogFooter>
            <Button type="button" variant="outline" onClick={() => setConfirmDelete(false)}>
              {t("characters.cancel")}
            </Button>
            <Button
              type="button"
              onClick={async () => {
                setConfirmDelete(false);
                await onDelete();
              }}
            >
              {t("characters.delete")}
            </Button>
          </DialogFooter>
        </DialogContent>
      </Dialog>
    </form>
  );
}
