import { useEffect, useState } from "react";
import { useTranslation } from "react-i18next";
import type { Assessment } from "@/bindings/Assessment";
import type { NpcTier } from "@/bindings/NpcTier";
import type { Encounter } from "@/bindings/Encounter";
import type { EncounterRecord } from "@/bindings/EncounterRecord";
import type { NpcTemplate } from "@/bindings/NpcTemplate";
import type { UserTemplateRecord } from "@/bindings/UserTemplateRecord";
import { EncounterEditor } from "@/components/encounter/encounter-editor";
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
import { Select, SelectContent, SelectItem, SelectTrigger, SelectValue } from "@/components/ui/select";
import { errorMessage } from "@/hooks/use-characters";
import { api } from "@/lib/api";
import { useUiStore } from "@/store/ui-store";
import { templateKey, templateLabel } from "@/lib/encounter";
import { hasBackend } from "@/lib/api";
import { status } from "@/store/status-store";

type EncountersPageProps = {
  bookTemplates: NpcTemplate[];
  userTemplates: UserTemplateRecord[];
  encounters: EncounterRecord[];
  onSaveEncounter: (id: number | null, encounter: Encounter) => Promise<unknown>;
  onDeleteEncounter: (id: number) => Promise<unknown>;
  onDeleteTemplate: (id: number) => Promise<unknown>;
  onSetTemplateTier: (id: number, tier: NpcTier) => Promise<unknown>;
  /** Clones every NPC of the encounter into the session. */
  onLaunch: (encounter: EncounterRecord) => Promise<void>;
  /** Clones one NPC of the GM's own template into the session. */
  onCloneTemplate: (template: UserTemplateRecord) => Promise<void>;
};

const TIERS: NpcTier[] = ["mook", "lieutenant", "mini_boss", "boss"];
const PLAYERS_MAX = 12;
const VERDICT_STYLE = {
  weak: "text-muted-foreground",
  fair: "text-state-ok",
  strong: "text-state-serious",
  very_strong: "text-state-mortal",
} as const;

type PendingDelete = { kind: "encounter" | "template"; id: number; name: string };

/** "Strength 6 / 4 - strong": a reference estimate, the tooltip says where the numbers come from. */
function StrengthBadge({ assessment }: { assessment: Assessment }) {
  const { t } = useTranslation();
  const { weight, players, verdict, skipped } = assessment;
  return (
    <Hint
      side="left"
      label={
        <span className="block max-w-72 whitespace-pre-line">
          {t("encounters.strength_hint", { weight, players })}
          {skipped > 0 && `\n${t("encounters.strength_skipped", { count: skipped })}`}
        </span>
      }
    >
      <span className={`num cursor-help text-xs uppercase ${VERDICT_STYLE[verdict]}`}>
        {t("encounters.strength", {
          weight,
          players,
          verdict: t(`encounters.verdict.${verdict}`),
        })}
      </span>
    </Hint>
  );
}

/** Third tab: the GM's own NPC templates and encounters (named lists "template x count"). */
export function EncountersPage({
  bookTemplates,
  userTemplates,
  encounters,
  onSaveEncounter,
  onDeleteEncounter,
  onDeleteTemplate,
  onSetTemplateTier,
  onLaunch,
  onCloneTemplate,
}: EncountersPageProps) {
  const { t } = useTranslation();
  const [editing, setEditing] = useState<{ record: EncounterRecord | null } | null>(null);
  const [pendingDelete, setPendingDelete] = useState<PendingDelete | null>(null);
  const { encounterPlayers: players, setEncounterPlayers } = useUiStore();
  const [assessments, setAssessments] = useState<Record<number, Assessment>>({});
  const [playersInput, setPlayersInput] = useState(String(players));
  const playersNumber = Number(playersInput);
  const playersValid =
    Number.isInteger(playersNumber) && playersNumber >= 1 && playersNumber <= PLAYERS_MAX;

  // The assessment is computed in Rust; it is redone when the group, the lines or the tiers change.
  useEffect(() => {
    let cancelled = false;
    Promise.all(
      encounters.map((e) =>
        api.assessEncounter(e.encounter, players).then((a) => [e.id, a] as const),
      ),
    ).then(
      (pairs) => !cancelled && setAssessments(Object.fromEntries(pairs)),
      () => !cancelled && setAssessments({}),
    );
    return () => {
      cancelled = true;
    };
  }, [encounters, userTemplates, players]);

  const run = async (task: () => Promise<unknown>, id: string) => {
    try {
      await task();
    } catch (e) {
      status.error(errorMessage(e), id);
    }
  };

  const summary = (encounter: Encounter) =>
    encounter.entries
      .map((e) => {
        const name =
          templateLabel(templateKey(e.source, e.template), bookTemplates, userTemplates) ??
          t("encounters.missing_template");
        return `${name} ×${e.count}`;
      })
      .join(", ");

  const total = (encounter: Encounter) => encounter.entries.reduce((sum, e) => sum + e.count, 0);

  return (
    <div className="space-y-6">
      <div className="flex items-center gap-2">
        <h2 className="mr-auto text-2xl font-bold">{t("encounters.title")}</h2>
        <Label htmlFor="encounter-players" className="font-normal">
          <Hint label={t("encounters.players_hint")}>
            <span>{t("encounters.players")}</span>
          </Hint>
          <FieldBadge
            className="ml-2 inline-block"
            error={playersValid ? null : t("validation.count", { max: PLAYERS_MAX })}
          >
            {(invalid) => (
              <Input
                id="encounter-players"
                type="number"
                min={1}
                max={PLAYERS_MAX}
                value={playersInput}
                onChange={(e) => {
                  setPlayersInput(e.target.value);
                  const n = Number(e.target.value);
                  if (Number.isInteger(n) && n >= 1 && n <= PLAYERS_MAX) setEncounterPlayers(n);
                }}
                aria-invalid={invalid}
                className="num inline-block h-8 w-16"
              />
            )}
          </FieldBadge>
        </Label>
        <Button disabled={!hasBackend} onClick={() => setEditing({ record: null })}>
          {t("encounters.new")}
        </Button>
      </div>

      <section className="space-y-2">
        <h3 className="text-sm text-muted-foreground">{t("encounters.section_encounters")}</h3>
        {encounters.length === 0 ? (
          <p className="text-muted-foreground">{t("encounters.empty")}</p>
        ) : (
          <ul className="space-y-2">
            {encounters.map((record) => (
              <li key={record.id} className="cut space-y-2 p-3 [--cut:6px]">
                <div className="flex flex-wrap items-baseline gap-3">
                  <span className="mr-auto font-display text-base font-semibold uppercase">
                    {record.encounter.name}
                  </span>
                  <span className="num text-xs text-muted-foreground">
                    {t("encounters.total", { count: total(record.encounter) })}
                  </span>
                  {assessments[record.id] && (
                    <StrengthBadge assessment={assessments[record.id]} />
                  )}
                </div>
                <p className="text-sm">{summary(record.encounter)}</p>
                {record.encounter.notes && (
                  <p className="whitespace-pre-line text-xs text-muted-foreground">
                    {record.encounter.notes}
                  </p>
                )}
                <div className="flex gap-2">
                  <Button
                    size="xs"
                    onClick={() => void run(() => onLaunch(record), "encounter:launch")}
                  >
                    {t("encounters.launch")}
                  </Button>
                  <Button size="xs" variant="outline" onClick={() => setEditing({ record })}>
                    {t("encounters.edit")}
                  </Button>
                  <Button
                    size="xs"
                    variant="outline"
                    onClick={() =>
                      setPendingDelete({
                        kind: "encounter",
                        id: record.id,
                        name: record.encounter.name,
                      })
                    }
                  >
                    {t("characters.delete")}
                  </Button>
                </div>
              </li>
            ))}
          </ul>
        )}
      </section>

      <section className="space-y-2">
        <h3 className="text-sm text-muted-foreground">{t("encounters.section_templates")}</h3>
        <p className="text-xs text-muted-foreground">{t("encounters.templates_hint")}</p>
        {userTemplates.length > 0 && (
          <ul className="space-y-1">
            {userTemplates.map((template) => {
              const { character } = template;
              return (
                <li key={template.id} className="cut flex items-baseline gap-3 px-3 py-1.5 [--cut:4px]">
                  <span className="flex-1 font-display text-sm font-semibold uppercase">
                    {character.name}
                  </span>
                  <span className="num text-xs text-muted-foreground">
                    {t("armor.head")} {character.armor_head.max_sp} / {t("armor.body")}{" "}
                    {character.armor_body.max_sp}
                  </span>
                  <Select
                    value={template.tier}
                    onValueChange={(tier) =>
                      void run(() => onSetTemplateTier(template.id, tier as NpcTier), "encounter:tier")
                    }
                  >
                    <SelectTrigger className="h-8 w-36" aria-label={t("encounters.tier")}>
                      <SelectValue />
                    </SelectTrigger>
                    <SelectContent>
                      {TIERS.map((tier) => (
                        <SelectItem key={tier} value={tier}>
                          {t(`npc.tier.${tier}`)}
                        </SelectItem>
                      ))}
                    </SelectContent>
                  </Select>
                  <Button
                    size="xs"
                    variant="outline"
                    onClick={() =>
                      void run(() => onCloneTemplate(template), "encounter:clone-template")
                    }
                  >
                    {t("npc.clone")}
                  </Button>
                  <Button
                    size="xs"
                    variant="outline"
                    onClick={() =>
                      setPendingDelete({ kind: "template", id: template.id, name: character.name })
                    }
                  >
                    {t("characters.delete")}
                  </Button>
                </li>
              );
            })}
          </ul>
        )}
      </section>

      <EncounterEditor
        open={editing !== null}
        initial={editing?.record ?? null}
        bookTemplates={bookTemplates}
        userTemplates={userTemplates}
        onSave={onSaveEncounter}
        onClose={() => setEditing(null)}
      />

      <Dialog open={pendingDelete !== null} onOpenChange={(o) => !o && setPendingDelete(null)}>
        <DialogContent>
          <DialogHeader>
            <DialogTitle>{t("encounters.delete_title")}</DialogTitle>
            <DialogDescription>
              {t(
                pendingDelete?.kind === "template"
                  ? "encounters.delete_template_body"
                  : "encounters.delete_encounter_body",
                { name: pendingDelete?.name ?? "" },
              )}
            </DialogDescription>
          </DialogHeader>
          <DialogFooter>
            <Button type="button" variant="outline" onClick={() => setPendingDelete(null)}>
              {t("characters.cancel")}
            </Button>
            <Button
              type="button"
              onClick={() => {
                const target = pendingDelete;
                setPendingDelete(null);
                if (!target) return;
                void run(
                  () =>
                    target.kind === "template"
                      ? onDeleteTemplate(target.id)
                      : onDeleteEncounter(target.id),
                  "encounter:delete",
                );
              }}
            >
              {t("characters.delete")}
            </Button>
          </DialogFooter>
        </DialogContent>
      </Dialog>
    </div>
  );
}
