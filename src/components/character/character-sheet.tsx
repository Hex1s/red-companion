import { zodResolver } from "@hookform/resolvers/zod";
import { useEffect, useMemo, useRef, useState, type ReactNode } from "react";
import { BrainCircuit, Plus, Stethoscope, X } from "@/components/ui/icons";
import { useForm, type Path } from "react-hook-form";
import { useTranslation } from "react-i18next";
import type { Character } from "@/bindings/Character";
import type { CriticalInjury } from "@/bindings/CriticalInjury";
import type { HealKind } from "@/bindings/HealKind";
import type { Derived } from "@/bindings/Derived";
import type { Stats } from "@/bindings/Stats";
import { InjuryIcon } from "@/components/combat/injury-icons";
import { MarkPicker } from "@/components/character/color-mark";
import { Icon } from "@/components/ui/icons";
import { Chip, ModuleTile } from "@/components/character/hardware";
import { HpBar } from "@/components/character/hp-bar";
import { ReputationMeter } from "@/components/character/reputation-meter";
import { RolesSection } from "@/components/character/roles-section";
import { SkillsSection } from "@/components/character/skills-section";
import { WoundBadge } from "@/components/character/wound-badge";
import { Button } from "@/components/ui/button";
import { FieldBadge } from "@/components/ui/field-badge";
import { Hint } from "@/components/ui/hint";
import { Input } from "@/components/ui/input";
import { NumberInput } from "@/components/ui/number-input";
import { Label } from "@/components/ui/label";
import { Tabs, TabsContent, TabsList, TabsTrigger } from "@/components/ui/tabs";
import { Popover, PopoverContent, PopoverTrigger } from "@/components/ui/popover";
import { errorMessage } from "@/lib/errors";
import type { CharacterEntry } from "@/hooks/use-characters";
import { useRoles } from "@/hooks/use-roles";
import { useUiStore, type SheetTab } from "@/store/ui-store";
import { useSkills } from "@/hooks/use-skills";
import { api } from "@/lib/api";
import { characterSchema, SP_MAX, STAT_MAX, STAT_MIN } from "@/lib/character-schema";
import { status } from "@/store/status-store";

/** Pause after the last edit before the sheet saves itself / asks Rust for the derived numbers. */
const AUTOSAVE_DELAY_MS = 700;
const PREVIEW_DELAY_MS = 120;
/** The "saving" message stays at least this long, so a quick save does not flash. */
const MIN_SAVING_MS = 350;

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

type CharacterSheetProps = {
  entry: CharacterEntry;
  onSave: (character: Character) => Promise<unknown>;
  /** Saves a copy of this NPC as one of the GM's own templates; only NPCs offer it. */
  onSaveAsTemplate: () => Promise<unknown>;
  /** The sheet is an own NPC template: no "save as template", the badge says it is a template. */
  asTemplate?: boolean;
  /** Critical injury table, to offer the ways each injury can be healed. */
  injuryTable: CriticalInjury[];
  /** Opens the healing window for one injury of this character. */
  onHeal: (injury: CriticalInjury, kind: HealKind) => void;
  /** The GM adds an injury by hand (no roll, may repeat) or removes one instance of it. */
  onAddInjury: (injuryId: string) => void;
  onRemoveInjury: (injuryId: string) => void;
  /** Opens the humanity loss window. */
  onLoseHumanity: () => void;
  /** Opens the therapy window (a week with a Medtech, humanity back). */
  onTherapy: () => void;
  /** Opens the dose window: this character (a Medtech) gives a drug they picked. */
  onAdminister: (drug: string) => void;
};

/** Tabs of the sheet; the header with name, HP and armor stays above them. */
const SHEET_TABS: { value: SheetTab; label: string }[] = [
  { value: "general", label: "characters.tab.general" },
  { value: "state", label: "characters.tab.state" },
  { value: "skills", label: "characters.tab.skills" },
  { value: "roles", label: "characters.tab.roles" },
  { value: "gear", label: "characters.tab.gear" },
];

type SectionProps = { title: string; children: ReactNode };

function Section({ title, children }: SectionProps) {
  return (
    <section className="space-y-2">
      <h3 className="text-sm text-muted-foreground">{title}</h3>
      {children}
    </section>
  );
}

type StatTileProps = { label: string; value: string | number; icon?: ReactNode };

function StatTile({ label, value, icon }: StatTileProps) {
  return <ModuleTile label={label} value={value} icon={icon} />;
}

/** Editable character sheet. Derived numbers come from Rust (saved record). */
export function CharacterSheet({
  entry,
  onSave,
  onSaveAsTemplate,
  asTemplate = false,
  injuryTable,
  onHeal,
  onAddInjury,
  onRemoveInjury,
  onLoseHumanity,
  onTherapy,
  onAdminister,
}: CharacterSheetProps) {
  const { t } = useTranslation();
  const sheetTab = useUiStore((st) => st.sheetTab);
  const setSheetTab = useUiStore((st) => st.setSheetTab);
  const [derived, setDerived] = useState<Derived>(entry.derived);
  const skills = useSkills();
  const roles = useRoles();
  const alive = { dead: derived.wound_state === "dead" };

  // Every plain catalog skill gets an input; missing levels start at 0. Skills with
  // specialties (languages, areas...) only have the rows that were added.
  const values = useMemo<Character>(
    () => ({
      ...entry.character,
      skills: {
        ...Object.fromEntries(skills.filter((s) => !s.specialty).map((s) => [s.id, 0])),
        ...entry.character.skills,
      },
    }),
    [entry.character, skills],
  );

  const form = useForm<Character>({
    resolver: zodResolver(characterSchema),
    defaultValues: values,
  });
  const {
    register,
    handleSubmit,
    watch,
    setValue,
    getValues,
    reset,
    formState: { errors },
  } = form;

  // New values from the backend (a save, a hit taken) must not wipe what is being typed right
  // now, and must not look like an edit: `reset` announces every field to `watch`, so the
  // autosave ignores those announcements while the flag is up. Typed values are kept only while an
  // edit waits for its save: "dirty" fields stay dirty after a save, and keeping them forever
  // would freeze a field (HP) against changes that come from outside (damage on the card).
  const resetting = useRef(false);
  const pendingSave = useRef(false);
  useEffect(() => {
    resetting.current = true;
    try {
      reset(values, { keepDirtyValues: pendingSave.current });
    } finally {
      resetting.current = false;
    }
  }, [values, reset]);

  // The sheet saves itself shortly after the last edit; invalid values wait until fixed. The
  // progress shows in the status area under one id, so repeated saves update one message.
  const saveId = `save:${entry.id}`;
  const submit = handleSubmit(async (values) => {
    pendingSave.current = false;
    status.show({ id: saveId, kind: "loading", text: t("status.saving") });
    const started = Date.now();
    try {
      // Level 0 is the default, so only trained skills are stored.
      const trained = Object.entries(values.skills).filter(([, level]) => level > 0);
      // HP typed above the maximum is saved as the maximum (the field itself is clamped on blur).
      await onSave({ ...values, hp: Math.min(values.hp, derived.max_hp), skills: Object.fromEntries(trained) });
      const wait = Math.max(0, MIN_SAVING_MS - (Date.now() - started));
      window.setTimeout(() => status.show({ id: saveId, kind: "success", text: t("status.saved") }), wait);
    } catch (e) {
      status.show({
        id: saveId,
        kind: "error",
        text: t("characters.save_error", { error: errorMessage(e) }),
      });
    }
  });

  const saveTimer = useRef<number>(undefined);
  const previewTimer = useRef<number>(undefined);
  const submitRef = useRef(submit);
  submitRef.current = submit;

  const scheduleSave = () => {
    window.clearTimeout(saveTimer.current);
    pendingSave.current = true;
    saveTimer.current = window.setTimeout(() => void submitRef.current(), AUTOSAVE_DELAY_MS);
  };

  // Derived numbers (max HP, wound threshold, death save, humanity) follow every edit: Rust
  // computes them for the sheet as it is now, the formulas are not repeated here.
  const schedulePreview = () => {
    window.clearTimeout(previewTimer.current);
    previewTimer.current = window.setTimeout(() => {
      const parsed = characterSchema.safeParse(getValues());
      if (parsed.success) api.previewDerived(parsed.data).then(setDerived, () => undefined);
    }, PREVIEW_DELAY_MS);
  };

  useEffect(() => {
    setDerived(entry.derived);
  }, [entry.derived]);

  useEffect(() => {
    // A reset (values arriving from the backend) has no field name: only real edits count.
    const subscription = watch((_, info) => {
      if (resetting.current || !info.name) return;
      scheduleSave();
      schedulePreview();
    });
    return () => {
      subscription.unsubscribe();
      window.clearTimeout(saveTimer.current);
      window.clearTimeout(previewTimer.current);
      // Leaving the sheet (another card, a delete) must not lose an edit still waiting for its save.
      if (pendingSave.current) void submitRef.current();
    };
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [watch]);

  const num = (name: Path<Character>) => register(name, { valueAsNumber: true });

  /** Text of a field error for the badge; `range` is what the field accepts. */
  const errorText = (error: unknown, range: { min: number; max: number }): string | null => {
    const message = (error as { message?: string } | undefined)?.message;
    if (!message) return null;
    return message === "name" || message === "sp_over_max"
      ? t(`characters.validation.${message}`)
      : t("characters.validation.range", range);
  };

  const tabHasError = (tab: SheetTab) => {
    switch (tab) {
      case "general":
        return Boolean(errors.stats || errors.reputation);
      case "skills":
        return Boolean(errors.skills);
      case "roles":
        return Boolean(errors.roles);
      case "gear":
        return Boolean(errors.notes);
      default:
        return false;
    }
  };

  const armorField = (area: "armor_head" | "armor_body", label: string) => {
    const range = { min: 0, max: SP_MAX };
    return (
      <div className="cut [--cut:6px] space-y-2 p-4">
        <div className="text-xs text-muted-foreground">{label}</div>
        <div className="grid grid-cols-2 gap-3">
          <div className="space-y-1">
            <Label htmlFor={`${area}.sp`}>{t("characters.sp")}</Label>
            <FieldBadge error={errorText(errors[area]?.sp ?? errors[area], range)}>
              {(invalid) => (
                <NumberInput
                  id={`${area}.sp`}
                  min={0}
                  max={SP_MAX}
                  className="num"
                  aria-invalid={invalid}
                  {...num(`${area}.sp`)}
                />
              )}
            </FieldBadge>
          </div>
          <div className="space-y-1">
            <Label htmlFor={`${area}.max_sp`}>{t("characters.sp_max")}</Label>
            <FieldBadge error={errorText(errors[area]?.max_sp, range)}>
              {(invalid) => (
                <NumberInput
                  id={`${area}.max_sp`}
                  min={0}
                  max={SP_MAX}
                  className="num"
                  aria-invalid={invalid}
                  {...num(`${area}.max_sp`)}
                />
              )}
            </FieldBadge>
          </div>
        </div>
      </div>
    );
  };

  return (
    <form onSubmit={(e) => e.preventDefault()} className="space-y-4" noValidate>
      <Tabs value={sheetTab} onValueChange={(v) => setSheetTab(v as SheetTab)} className="gap-4">
        <div className="sticky top-0 z-10 -mx-1 space-y-3 bg-background px-1 pb-2">
        <div className="flex items-end gap-3">
          <MarkPicker
            color={watch("color")}
            portrait={watch("portrait")}
            name={watch("name")}
            markClassName="size-10"
            onChange={(change) => {
              if (change.color !== undefined) setValue("color", change.color, { shouldDirty: true, shouldValidate: true });
              if (change.portrait !== undefined) setValue("portrait", change.portrait, { shouldDirty: true, shouldValidate: true });
            }}
          />
          <div className="flex-1 space-y-1">
            <Label htmlFor="name">{t("characters.name")}</Label>
            <FieldBadge error={errorText(errors.name, { min: 0, max: 0 })}>
              {(invalid) => <Input id="name" aria-invalid={invalid} {...register("name")} />}
            </FieldBadge>
          </div>
          <span className="pb-2 text-xs text-muted-foreground uppercase">
            {asTemplate ? t("templates.badge") : t(`characters.kind.${entry.kind}`)}
          </span>
          {entry.kind === "npc" && !asTemplate && (
            <Button type="button" variant="outline" onClick={() => void onSaveAsTemplate()}>
              {t("encounters.save_as_template")}
            </Button>
          )}
        </div>
          <div className="cut space-y-3 p-3 [--cut:6px]">
            <div className="flex items-end gap-3">
              <div className="w-28 space-y-1">
                <Label htmlFor="hp">{t("characters.hp")}</Label>
                <FieldBadge error={errorText(errors.hp, { min: 0, max: derived.max_hp })}>
                  {(invalid) => {
                    const hpField = num("hp");
                    return (
                      <NumberInput
                        id="hp"
                        min={0}
                        className="num"
                        aria-invalid={invalid}
                        {...hpField}
                        // Typing 999 is the quick way to "heal fully": the field is clamped to max HP on leaving.
                        onBlur={(e) => {
                          if (Number(getValues("hp")) > derived.max_hp) {
                            setValue("hp", derived.max_hp, { shouldDirty: true, shouldValidate: true });
                          }
                          return hpField.onBlur(e);
                        }}
                      />
                    );
                  }}
                </FieldBadge>
              </div>
              <HpBar
                current={Math.min(Number(watch("hp")) || 0, derived.max_hp)}
                max={derived.max_hp}
                className="flex-1 pb-3"
              />
              <WoundBadge state={derived.wound_state} className="w-32 text-nowrap shrink-0 pb-3 text-right" />
            </div>
            <div className="grid grid-cols-2 gap-2">
              {armorField("armor_head", t("armor.head"))}
              {armorField("armor_body", t("armor.body"))}
            </div>
          </div>
          <TabsList variant="line" className="w-full justify-start">
            {SHEET_TABS.map(({ value, label }) => (
              <TabsTrigger key={value} value={value} className="flex-none px-3">
                {t(label)}
                {tabHasError(value) && <span className="ml-1 text-destructive-text">!</span>}
              </TabsTrigger>
            ))}
          </TabsList>
        </div>

        <TabsContent value="general" forceMount className="space-y-6 data-[state=inactive]:hidden">
        <Section title={t("characters.section.stats")}>
          <div className="flex flex-wrap justify-between gap-2">
            {STAT_KEYS.map(({ key, label }) => (
              <Chip
                key={key}
                className="w-28"
                title={
                  <Hint
                    label={
                      <span className="block max-w-64 space-y-1">
                        <span className="block font-semibold">{t(`stat_info.${label}.name`)}</span>
                        <span className="block">{t(`stat_info.${label}.summary`)}</span>
                      </span>
                    }
                  >
                    <Label htmlFor={`stats.${key}`} className="inline-block cursor-help font-display text-base leading-none font-semibold tracking-wider text-destructive-text uppercase">
                      {t(`stats.${label}`)}
                    </Label>
                  </Hint>
                }
                caption={t(`stat_info.${label}.name`)}
              >
                <FieldBadge
                  error={errorText(errors.stats?.[key], {
                    min: key === "emp" || key === "luck" ? 0 : STAT_MIN,
                    max: STAT_MAX,
                  })}
                >
                  {(invalid) => (
                    <NumberInput
                      id={`stats.${key}`}
                      min={key === "emp" || key === "luck" ? 0 : STAT_MIN}
                      max={STAT_MAX}
                      className="num h-10 border-transparent bg-transparent px-0 text-center text-3xl font-bold shadow-none dark:bg-transparent"
                      aria-invalid={invalid}
                      {...num(`stats.${key}`)}
                    />
                  )}
                </FieldBadge>
              </Chip>
            ))}
          </div>
          <ReputationMeter
            value={Number(watch("reputation"))}
            onChange={(v) => setValue("reputation", v, { shouldDirty: true, shouldValidate: true })}
          />
        </Section>

        <Section title={t("characters.section.derived")}>
            <div className="grid grid-cols-4 gap-2">
              <StatTile label={t("characters.max_hp")} value={derived.max_hp} icon={<Icon name="favorite" />} />
              <StatTile label={t("characters.serious_threshold")} value={derived.serious_threshold} icon={<Icon name="warning" />} />
              <StatTile label={t("characters.death_save")} value={`≤ ${derived.death_save}`} icon={<Icon name="skull" />} />
              <StatTile label={t("characters.humanity")} value={derived.max_humanity} icon={<Icon name="person" />} />
            </div>
        </Section>
        </TabsContent>

        <TabsContent value="state" forceMount className="space-y-6 data-[state=inactive]:hidden">
        <Section title={t("glossary.humanity_full")}>
          <div className="flex flex-wrap items-end gap-2">
            <div className="grid flex-1 grid-cols-3 gap-2">
              <StatTile
                label={t("glossary.humanity")}
                value={`${derived.humanity} / ${derived.max_humanity}`}
                icon={<Icon name="person" />}
              />
              <StatTile label={t("humanity.emp_now")} value={derived.effective_emp} icon={<Icon name="bolt" />} />
              <StatTile label={t("humanity.emp_base")} value={entry.character.stats.emp} icon={<Icon name="memory" />} />
            </div>
            <Hint label={t("humanity.loss_hint")}>
              <Button type="button" size="sm" variant="outline" disabled={alive.dead} onClick={onLoseHumanity}>
                <BrainCircuit />
                {t("humanity.loss_button")}
              </Button>
            </Hint>
            <Hint label={t("humanity.therapy_hint")}>
              <Button
                type="button"
                size="sm"
                variant="outline"
                disabled={alive.dead || derived.humanity >= derived.max_humanity}
                onClick={onTherapy}
              >
                <Stethoscope />
                {t("humanity.therapy_button")}
              </Button>
            </Hint>
          </div>
          {derived.cyberpsychosis && (
            <Hint label={t("humanity.cyberpsychosis_hint")}>
              <p className="font-display text-sm font-semibold text-state-mortal-text uppercase">
                {t("humanity.cyberpsychosis")}
              </p>
            </Hint>
          )}
        </Section>

        <Section title={t("characters.section.injuries")}>
          {entry.character.injuries.length === 0 ? (
            <p className="text-sm text-muted-foreground">{t("characters.no_injuries")}</p>
          ) : (
            <ul className="space-y-1 text-sm">
              {entry.character.injuries.map((id, index) => {
                const injury = injuryTable.find((i) => i.id === id);
                // The same injury may repeat; the first-aid marks go to its earliest instances.
                const nth = entry.character.injuries.slice(0, index).filter((i) => i === id).length;
                const aided = nth < entry.character.first_aided.filter((i) => i === id).length;
                const quick = injury?.quick_fix_removes_effect === true;
                const canFirstAid = injury && !alive.dead && injury.first_aid.length > 0 && !aided;
                const canTreat = injury && !alive.dead && injury.treatment.length > 0;
                return (
                  <li key={`${id}:${index}`} className="cut flex items-center gap-2 px-3 py-1.5 [--cut:4px]">
                    <InjuryIcon id={id} className="size-5 text-emphasis" />
                    <span className="flex-1">
                      {t(`injury.${id}.name`, { defaultValue: id })}
                      {aided && (
                        <span className="ml-2 text-xs text-state-ok">{t("heal.lifted_badge")}</span>
                      )}
                    </span>
                    {canFirstAid && (
                      <Hint label={quick ? t("heal.quick_fix_hint") : t("heal.first_aid_hint")}>
                        <Button
                          type="button"
                          size="xs"
                          variant="outline"
                          onClick={() => onHeal(injury, "first_aid")}
                        >
                          {quick ? t("heal.quick_fix") : t("heal.first_aid")}
                        </Button>
                      </Hint>
                    )}
                    {canTreat && (
                      <Hint label={t("heal.treatment_hint")}>
                        <Button
                          type="button"
                          size="xs"
                          variant="outline"
                          onClick={() => onHeal(injury, "treatment")}
                        >
                          {t("heal.treatment")}
                        </Button>
                      </Hint>
                    )}
                    <Hint label={t("characters.injury_remove")}>
                      <Button
                        type="button"
                        size="icon-xs"
                        variant="ghost"
                        aria-label={t("characters.injury_remove")}
                        onClick={() => onRemoveInjury(id)}
                      >
                        <X />
                      </Button>
                    </Hint>
                  </li>
                );
              })}
            </ul>
          )}
          <Popover>
            <PopoverTrigger asChild>
              <Button type="button" size="xs" variant="outline" className="mt-2">
                <Plus />
                {t("characters.injury_add")}
              </Button>
            </PopoverTrigger>
            <PopoverContent
              align="start"
              className="w-80 space-y-2"
              // Focus would land on the first icon and open its tooltip at once; keep it on the panel.
              onOpenAutoFocus={(e) => {
                e.preventDefault();
                if (e.target instanceof HTMLElement) e.target.focus();
              }}
            >
              <p className="text-xs text-muted-foreground">{t("characters.injury_add_hint")}</p>
              {(["body", "head"] as const).map((area) => (
                <div key={area} className="space-y-1">
                  <div className="text-xs text-muted-foreground uppercase">{t(`glossary.area.${area}`)}</div>
                  <div className="flex flex-wrap gap-1">
                    {injuryTable
                      .filter((i) => i.area === area)
                      .sort((a, b) => a.roll - b.roll)
                      .map((injury) => (
                        <Hint
                          key={injury.id}
                          label={
                            <span className="block max-w-64">
                              <span className="block font-semibold">{t(injury.name_key)}</span>
                              <span className="block">{t(injury.effect_key)}</span>
                            </span>
                          }
                        >
                          <Button
                            type="button"
                            size="icon"
                            variant="outline"
                            aria-label={t(injury.name_key)}
                            onClick={() => onAddInjury(injury.id)}
                          >
                            <InjuryIcon id={injury.id} className="size-6" />
                          </Button>
                        </Hint>
                      ))}
                  </div>
                </div>
              ))}
            </PopoverContent>
          </Popover>
        </Section>
        </TabsContent>

        <TabsContent value="skills" forceMount className="space-y-6 data-[state=inactive]:hidden">
        <Section title={t("characters.section.skills")}>
          <SkillsSection characterId={entry.id} form={form} skills={skills} onChange={scheduleSave} />
        </Section>
        </TabsContent>

        <TabsContent value="roles" forceMount className="space-y-6 data-[state=inactive]:hidden">
        <Section title={t("characters.section.roles")}>
          <RolesSection form={form} roles={roles} derived={derived} onAdminister={onAdminister} />
        </Section>
        </TabsContent>

        <TabsContent value="gear" forceMount className="space-y-6 data-[state=inactive]:hidden">
        <Section title={t("characters.notes")}>
          <textarea
            rows={4}
            className="w-full border border-input bg-transparent px-3 py-2 text-sm outline-none focus-visible:border-ring"
            {...register("notes")}
          />
        </Section>
        </TabsContent>
      </Tabs>

    </form>
  );
}
