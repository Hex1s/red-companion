import { SkillHint } from "@/components/character/skill-hint";
import { useMemo, useState } from "react";
import { ChevronDown, Search } from "@/components/ui/icons";
import type { Path, UseFormReturn } from "react-hook-form";
import { useTranslation } from "react-i18next";
import type { Character } from "@/bindings/Character";
import type { MartialArt } from "@/bindings/MartialArt";
import type { Skill } from "@/bindings/Skill";
import { SkillCheckButton } from "@/components/character/skill-check-button";
import { Button } from "@/components/ui/button";
import { Checkbox } from "@/components/ui/checkbox";
import { Collapsible, CollapsibleContent, CollapsibleTrigger } from "@/components/ui/collapsible";
import { FieldBadge } from "@/components/ui/field-badge";
import { Hint } from "@/components/ui/hint";
import { Input } from "@/components/ui/input";
import { NumberInput } from "@/components/ui/number-input";
import { Label } from "@/components/ui/label";
import { useMartialArts } from "@/hooks/use-martial-arts";
import { MissingReference } from "@/components/content/missing-reference";
import { useSpecialties } from "@/hooks/use-specialties";
import { SKILL_MAX } from "@/lib/character-schema";
import { STAT_ABBR } from "@/lib/stat-abbr";
import { cn } from "@/lib/utils";

type SkillsSectionProps = {
  /** Saved id of the sheet, for quick checks. */
  characterId: number;
  form: UseFormReturn<Character>;
  skills: Skill[];
  /** Called after a change the form does not report by itself (a removed specialty). */
  onChange: () => void;
};

/** Level of a skill key on the sheet (0 when absent). */
const levelOf = (values: Record<string, number>, key: string) => values[key] ?? 0;

/** The sheet's skills: collapsible groups, a search box and a "trained only" filter. */
export function SkillsSection({ characterId, form, skills, onChange }: SkillsSectionProps) {
  const { t } = useTranslation();
  const {
    register,
    watch,
    setValue,
    unregister,
    formState: { errors },
  } = form;
  const values = (watch("skills") ?? {}) as Record<string, number>;
  const martialArts = useMartialArts();
  const styles = martialArts.filter((a) => a.kind !== "general");
  const specialties = useSpecialties();
  /** What the list offers for a skill: martial arts styles, languages, instruments. The field also takes any typed text. */
  const suggestionsOf = (skillId: string) =>
    skillId === "martial_arts"
      ? styles.map((a) => ({ code: a.code, name: a.name }))
      : specialties.filter((x) => x.skill === skillId).map((x) => ({ code: x.code, name: x.name }));

  const [query, setQuery] = useState("");
  const [onlyTrained, setOnlyTrained] = useState(false);
  const [openGroups, setOpenGroups] = useState<Set<string>>(new Set());
  const [specialtyDraft, setSpecialtyDraft] = useState<Record<string, string>>({});

  const specialtyName = (key: string) => key.slice(key.indexOf(":") + 1);
  const styleOf = (key: string) =>
    key.startsWith("martial_arts:") ? styles.find((a) => a.code === specialtyName(key)) : undefined;
  const specialtyLabel = (key: string) => {
    const name = specialtyName(key);
    // A code from the list shows the name in the interface language; typed text (a style or a
    // language that is not in the list, which the GM may allow) shows as typed.
    const known = suggestionsOf(key.slice(0, key.indexOf(":"))).find((x) => x.code === name);
    if (known) return known.name;
    // A martial art comes only from the downloaded list: once that is loaded, a code missing from it
    // (another pack, a removed entry) says so instead of showing the bare code.
    if (key.startsWith("martial_arts:") && styles.length > 0) return t("content.not_in_references", { name });
    return t(`skill.specialties.${name}`, { defaultValue: name });
  };
  const rowsOf = (skill: Skill) =>
    Object.keys(values)
      .filter((key) => key.startsWith(`${skill.id}:`))
      .sort((a, b) => specialtyLabel(a).localeCompare(specialtyLabel(b)));

  const q = query.trim().toLowerCase();
  const filtering = q !== "" || onlyTrained;

  const rowVisible = (key: string, skillMatches: boolean) =>
    (!onlyTrained || levelOf(values, key) > 0) &&
    (q === "" || skillMatches || specialtyLabel(key).toLowerCase().includes(q));

  const skillVisible = (skill: Skill) => {
    const nameMatches = q === "" || t(skill.name_key).toLowerCase().includes(q);
    if (!skill.specialty) return nameMatches && (!onlyTrained || levelOf(values, skill.id) > 0);
    // A skill with specialties stays visible while one of its rows matches; with no
    // filter it is always there, it holds the "add" field.
    if (!filtering) return true;
    return nameMatches ? !onlyTrained || rowsOf(skill).some((k) => levelOf(values, k) > 0) : rowsOf(skill).some((k) => rowVisible(k, false));
  };

  const groups = useMemo(() => {
    const byCategory = new Map<Skill["category"], Skill[]>();
    for (const skill of skills) byCategory.set(skill.category, [...(byCategory.get(skill.category) ?? []), skill]);
    return [...byCategory];
  }, [skills]);

  const trainedIn = (group: Skill[]) =>
    group.reduce(
      (n, skill) =>
        n +
        (skill.specialty
          ? rowsOf(skill).filter((k) => levelOf(values, k) > 0).length
          : levelOf(values, skill.id) > 0
            ? 1
            : 0),
      0,
    );

  const addSpecialty = (skillId: string) => {
    // Dots and brackets would break the form's field paths, the colon the skill key.
    const typed = (specialtyDraft[skillId] ?? "").trim().replace(/[.:[\]]/g, "");
    setSpecialtyDraft((d) => ({ ...d, [skillId]: "" }));
    // Text that names a suggestion is stored as its code (so it follows the interface language).
    const picked = suggestionsOf(skillId).find((x) => x.name.toLowerCase() === typed.toLowerCase() || x.code === typed);
    const name = picked ? picked.code : typed;
    if (!name || `${skillId}:${name}` in values) return;
    setValue(`skills.${skillId}:${name}` as Path<Character>, 1, { shouldDirty: true });
  };

  const styleHint = (style: MartialArt) => (
    <span className="block max-w-72 space-y-1">
      <span className="block">{style.summary}</span>
      {style.techniques.map((x) => (
        <span key={x.code} className="block text-muted-foreground">
          {x.name}
        </span>
      ))}
    </span>
  );

  const field = (key: string) => register(`skills.${key}` as Path<Character>, { valueAsNumber: true });

  const levelInput = (key: string) => {
    const message = (errors.skills?.[key] as { message?: string } | undefined)?.message;
    return (
      <FieldBadge
        className="shrink-0"
        error={message ? t("characters.validation.range", { min: 0, max: SKILL_MAX }) : null}
      >
        {(invalid) => (
          <NumberInput
            id={`skills.${key}`}
            min={0}
            max={SKILL_MAX}
            className="num h-7 w-14"
            aria-invalid={invalid}
            {...field(key)}
          />
        )}
      </FieldBadge>
    );
  };

  const checkButton = (key: string, label: string) => {
    // The check runs on the saved sheet; a level outside 0..10 would only fail there.
    const level = values[key];
    const invalid = key in values && !(Number.isInteger(level) && level >= 0 && level <= SKILL_MAX);
    return (
      <SkillCheckButton
        characterId={characterId}
        skillKey={key}
        label={label}
        disabledReason={invalid ? t("skill_check.invalid_level") : null}
      />
    );
  };

  const skillRow = (skill: Skill) => {
    const stat = <span className="text-xs text-muted-foreground">{t(`stats.${STAT_ABBR[skill.stat]}`)}</span>;
    const title = (
      <>
        <SkillHint id={skill.id}>
          <span>{t(skill.name_key)}</span>
        </SkillHint>
        {skill.double_cost && <span className="num text-muted-foreground"> ×2</span>}
      </>
    );
    if (!skill.specialty) {
      return (
        <li key={skill.id} className="flex items-center gap-2 text-sm">
          <Label htmlFor={`skills.${skill.id}`} className="flex-1 font-normal">
            {title}
          </Label>
          {stat}
          {levelInput(skill.id)}
          {checkButton(skill.id, t(skill.name_key))}
        </li>
      );
    }
    const nameMatches = q === "" || t(skill.name_key).toLowerCase().includes(q);
    return (
      <li key={skill.id} className="space-y-1 text-sm">
        <div className="flex items-center gap-2">
          <span className="flex-1">{title}</span>
          {stat}
          <Input
            aria-label={t("characters.skill_specialty")}
            placeholder={t("characters.skill_specialty")}
            className="h-7 w-44"
            list={`specialties-${skill.id}`}
            value={specialtyDraft[skill.id] ?? ""}
            onChange={(e) => setSpecialtyDraft((d) => ({ ...d, [skill.id]: e.target.value }))}
            onKeyDown={(e) => {
              if (e.key === "Enter") {
                e.preventDefault();
                addSpecialty(skill.id);
              }
            }}
          />
          {/* The list suggests, the field still takes any text. */}
          <datalist id={`specialties-${skill.id}`}>
            {suggestionsOf(skill.id)
              .filter((x) => !(`${skill.id}:${x.code}` in values))
              .map((x) => (
                <option key={x.code} value={x.name} />
              ))}
          </datalist>
          <Button type="button" size="xs" variant="outline" onClick={() => addSpecialty(skill.id)}>
            {t("characters.skill_specialty_add")}
          </Button>
        </div>
        {rowsOf(skill)
          .filter((key) => rowVisible(key, nameMatches))
          .map((key) => (
            <div key={key} className="space-y-1">
            <div className="flex items-center gap-2 pl-4">
              <Label htmlFor={`skills.${key}`} className="flex-1 font-normal">
                {styleOf(key) ? (
                  <Hint label={styleHint(styleOf(key)!)}>
                    <span>{specialtyLabel(key)}</span>
                  </Hint>
                ) : (
                  specialtyLabel(key)
                )}
              </Label>
              {levelInput(key)}
              {checkButton(key, `${t(skill.name_key)}: ${specialtyLabel(key)}`)}
              <Button
                type="button"
                size="xs"
                variant="outline"
                aria-label={t("characters.skill_specialty_remove")}
                onClick={() => {
                  unregister(`skills.${key}` as Path<Character>);
                  onChange();
                }}
              >
                ×
              </Button>
            </div>
            {key.startsWith("martial_arts:") && !styleOf(key) && (
              <MissingReference className="pl-4" collection="martial_arts" listLoaded={styles.length > 0} />
            )}
            </div>
          ))}
      </li>
    );
  };

  if (skills.length === 0) return null;

  return (
    <div className="space-y-3">
      <div className="flex items-center gap-4">
        <div className="relative flex-1">
          <Search className="pointer-events-none absolute top-1/2 left-2.5 size-4 -translate-y-1/2 text-muted-foreground" />
          <Input
            value={query}
            placeholder={t("characters.skills_search")}
            aria-label={t("characters.skills_search")}
            onChange={(e) => setQuery(e.target.value)}
            className="pl-8"
          />
        </div>
        <label className="flex items-center gap-2 text-sm whitespace-nowrap">
          <Checkbox checked={onlyTrained} onCheckedChange={(v) => setOnlyTrained(v === true)} />
          {t("characters.skills_trained_only")}
        </label>
      </div>

      {/* Two fixed columns (even / odd groups): opening a group never moves a group to the other
          column. Below `xl` the column wrappers vanish and `order` restores the plain list. */}
      <div className="flex flex-col gap-3 xl:flex-row xl:items-start">
      {[0, 1].map((column) => (
      <div key={column} className="contents xl:flex xl:min-w-0 xl:flex-1 xl:flex-col xl:gap-3">
      {groups.map(([category, group], index) => {
        if (index % 2 !== column) return null;
        const visible = group.filter(skillVisible);
        if (filtering && visible.length === 0) return null;
        const open = filtering || openGroups.has(category);
        const trained = trainedIn(group);
        return (
          <Collapsible
            key={category}
            open={open}
            onOpenChange={(next) =>
              setOpenGroups((prev) => {
                const copy = new Set(prev);
                if (next) copy.add(category);
                else copy.delete(category);
                return copy;
              })
            }
            style={{ order: index }}
            className="cut p-0 [--cut:6px] xl:order-none!"
          >
            <CollapsibleTrigger asChild>
              <button
                type="button"
                className="flex w-full items-center gap-2 p-3 text-left transition-colors hover:bg-accent/40"
              >
                <span className="flex-1 text-xs text-muted-foreground">{t(`skill.category.${category}`)}</span>
                {trained > 0 && <span className="num text-xs text-emphasis">{trained}</span>}
                <ChevronDown className={cn("size-4 text-muted-foreground transition-transform", open && "rotate-180")} />
              </button>
            </CollapsibleTrigger>
            <CollapsibleContent>
              <ul className="space-y-1 p-3">{visible.map(skillRow)}</ul>
            </CollapsibleContent>
          </Collapsible>
        );
      })}
      </div>
      ))}
      </div>
    </div>
  );
}
