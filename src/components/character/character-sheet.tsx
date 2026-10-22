import { zodResolver } from "@hookform/resolvers/zod";
import { useEffect, useMemo, useRef, useState, type ReactNode } from "react";
import { BedDouble } from "lucide-react";
import { useForm, type Path } from "react-hook-form";
import { useTranslation } from "react-i18next";
import type { Character } from "@/bindings/Character";
import type { CriticalInjury } from "@/bindings/CriticalInjury";
import type { HealKind } from "@/bindings/HealKind";
import type { Derived } from "@/bindings/Derived";
import type { Stats } from "@/bindings/Stats";
import { InjuryIcon } from "@/components/combat/injury-icons";
import { HpBar } from "@/components/character/hp-bar";
import { SkillsSection } from "@/components/character/skills-section";
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
import { FieldBadge } from "@/components/ui/field-badge";
import { Hint } from "@/components/ui/hint";
import { Input } from "@/components/ui/input";
import { Label } from "@/components/ui/label";
import { errorMessage, type CharacterEntry } from "@/hooks/use-characters";
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
  onDelete: () => Promise<unknown>;
  /** Saves a copy of this NPC as one of the GM's own templates; only NPCs offer it. */
  onSaveAsTemplate: () => Promise<unknown>;
  /** Critical injury table, to offer the ways each injury can be healed. */
  injuryTable: CriticalInjury[];
  /** Opens the healing window for one injury of this character. */
  onHeal: (injury: CriticalInjury, kind: HealKind) => void;
  /** Opens the rest window. */
  onRest: () => void;
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
export function CharacterSheet({
  entry,
  onSave,
  onDelete,
  onSaveAsTemplate,
  injuryTable,
  onHeal,
  onRest,
}: CharacterSheetProps) {
  const { t } = useTranslation();
  const [confirmDelete, setConfirmDelete] = useState(false);
  const [derived, setDerived] = useState<Derived>(entry.derived);
  const skills = useSkills();
  const alive = { dead: derived.wound_state === "dead" };
  // Rest needs a living character who is not mortally wounded (stabilise first) and not at full HP.
  const canRest =
    !alive.dead && derived.wound_state !== "mortal" && entry.character.hp < derived.max_hp;

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
    getValues,
    reset,
    formState: { errors },
  } = form;

  // New values from the backend (a save, a hit taken) must not wipe what is being typed right
  // now, and must not look like an edit: `reset` announces every field to `watch`, so the
  // autosave ignores those announcements while the flag is up.
  const resetting = useRef(false);
  useEffect(() => {
    resetting.current = true;
    try {
      reset(values, { keepDirtyValues: true });
    } finally {
      resetting.current = false;
    }
  }, [values, reset]);

  // The sheet saves itself shortly after the last edit; invalid values wait until fixed. The
  // progress shows in the status area under one id, so repeated saves update one message.
  const saveId = `save:${entry.id}`;
  const pendingSave = useRef(false);
  const submit = handleSubmit(async (values) => {
    pendingSave.current = false;
    status.show({ id: saveId, kind: "loading", text: t("status.saving") });
    const started = Date.now();
    try {
      // Level 0 is the default, so only trained skills are stored.
      const trained = Object.entries(values.skills).filter(([, level]) => level > 0);
      await onSave({ ...values, skills: Object.fromEntries(trained) });
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
                <Input
                  id={`${area}.sp`}
                  type="number"
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
                <Input
                  id={`${area}.max_sp`}
                  type="number"
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
    <form onSubmit={(e) => e.preventDefault()} className="max-w-3xl space-y-6" noValidate>
      <div className="flex items-end gap-3">
        <div className="flex-1 space-y-1">
          <Label htmlFor="name">{t("characters.name")}</Label>
          <FieldBadge error={errorText(errors.name, { min: 0, max: 0 })}>
            {(invalid) => <Input id="name" aria-invalid={invalid} {...register("name")} />}
          </FieldBadge>
        </div>
        <span className="pb-2 text-xs text-muted-foreground uppercase">
          {t(`characters.kind.${entry.kind}`)}
        </span>
        {entry.kind === "npc" && (
          <Button type="button" variant="outline" onClick={() => void onSaveAsTemplate()}>
            {t("encounters.save_as_template")}
          </Button>
        )}
        <Button type="button" variant="outline" onClick={() => setConfirmDelete(true)}>
          {t("characters.delete")}
        </Button>
      </div>
      <Section title={t("characters.section.stats")}>
        <div className="grid grid-cols-5 gap-2">
          {STAT_KEYS.map(({ key, label }) => (
            <div key={key} className="cut [--cut:6px] space-y-1 p-3">
              <Label htmlFor={`stats.${key}`} className="text-xs text-muted-foreground">
                {t(`stats.${label}`)}
              </Label>
              <FieldBadge
                error={errorText(errors.stats?.[key], {
                  min: key === "emp" || key === "luck" ? 0 : STAT_MIN,
                  max: STAT_MAX,
                })}
              >
                {(invalid) => (
                  <Input
                    id={`stats.${key}`}
                    type="number"
                    min={key === "emp" || key === "luck" ? 0 : STAT_MIN}
                    max={STAT_MAX}
                    className="num text-lg font-bold"
                    aria-invalid={invalid}
                    {...num(`stats.${key}`)}
                  />
                )}
              </FieldBadge>
            </div>
          ))}
        </div>
      </Section>

      <Section title={t("characters.section.derived")}>
        <div className="cut [--cut:6px] space-y-3 p-3">
          <div className="flex items-end gap-3">
            <div className="w-28 space-y-1">
              <Label htmlFor="hp">{t("characters.hp")}</Label>
              <FieldBadge error={errorText(errors.hp, { min: 0, max: derived.max_hp })}>
                {(invalid) => (
                  <Input
                    id="hp"
                    type="number"
                    min={0}
                    className="num"
                    aria-invalid={invalid}
                    {...num("hp")}
                  />
                )}
              </FieldBadge>
            </div>
            <HpBar current={Number(watch("hp")) || 0} max={derived.max_hp} className="flex-1 pb-3" />
            <WoundBadge state={derived.wound_state} className="pb-2" />
            {canRest && (
              <Hint label={t("rest.hint")}>
                <Button type="button" size="sm" variant="outline" className="mb-1.5" onClick={onRest}>
                  <BedDouble />
                  {t("rest.button")}
                </Button>
              </Hint>
            )}
          </div>
          <div className="grid grid-cols-4 gap-2">
            <StatTile label={t("characters.max_hp")} value={derived.max_hp} />
            <StatTile label={t("characters.serious_threshold")} value={derived.serious_threshold} />
            <StatTile label={t("characters.death_save")} value={`≤ ${derived.death_save}`} />
            <StatTile label={t("characters.humanity")} value={derived.max_humanity} />
          </div>
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
            {entry.character.injuries.map((id) => {
              const injury = injuryTable.find((i) => i.id === id);
              const aided = entry.character.first_aided.includes(id);
              const quick = injury?.quick_fix_removes_effect === true;
              const canFirstAid = injury && !alive.dead && injury.first_aid.length > 0 && !aided;
              const canTreat = injury && !alive.dead && injury.treatment.length > 0;
              return (
                <li key={id} className="cut flex items-center gap-2 px-3 py-1.5 [--cut:4px]">
                  <InjuryIcon id={id} className="size-5 text-primary" />
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
                </li>
              );
            })}
          </ul>
        )}
      </Section>

      <Section title={t("characters.section.skills")}>
        <SkillsSection characterId={entry.id} form={form} skills={skills} onChange={scheduleSave} />
      </Section>

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
