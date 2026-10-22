import { useEffect, useState } from "react";
import { useTranslation } from "react-i18next";
import type { Assessment } from "@/bindings/Assessment";
import type { TierHint } from "@/bindings/TierHint";
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
import { Hint } from "@/components/ui/hint";
import { Select, SelectContent, SelectItem, SelectTrigger, SelectValue } from "@/components/ui/select";
import { errorMessage } from "@/hooks/use-characters";
import { api } from "@/lib/api";
import { templateKey, templateLabel } from "@/lib/encounter";
import { hasBackend } from "@/lib/api";
import { status } from "@/store/status-store";

type EncountersPageProps = {
  bookTemplates: NpcTemplate[];
  userTemplates: UserTemplateRecord[];
  encounters: EncounterRecord[];
  /** Player characters in the active session: the group encounters are compared with. */
  players: number;
  onSaveEncounter: (id: number | null, encounter: Encounter) => Promise<unknown>;
  onDeleteEncounter: (id: number) => Promise<unknown>;
  onDeleteTemplate: (id: number) => Promise<unknown>;
  onSetTemplateTier: (id: number, tier: NpcTier) => Promise<unknown>;
  /** Writes the encounters and own templates into a file; an encounter takes its templates along. */
  onExport: (encounterIds: number[], templateIds: number[]) => Promise<void>;
  /** Starts the import: the app lists the encounter files of its export folder. */
  onImport: () => void;
  /** Clones every NPC of the encounter into the session. */
  onLaunch: (encounter: EncounterRecord) => Promise<void>;
  /** Clones one NPC of the GM's own template into the session. */
  onCloneTemplate: (template: UserTemplateRecord) => Promise<void>;
};

const TIERS: NpcTier[] = ["mook", "lieutenant", "mini_boss", "boss"];
const VERDICT_STYLE = {
  weak: "text-muted-foreground",
  fair: "text-state-ok",
  strong: "text-state-serious",
  very_strong: "text-state-mortal",
} as const;

type PendingDelete = { kind: "encounter" | "template"; id: number; name: string };

/** "By stats closer to a lieutenant": a hint under the template name; the tooltip shows the numbers. */
function TierHintLine({ hint }: { hint: TierHint }) {
  const { t } = useTranslation();
  return (
    <Hint
      side="bottom"
      label={
        <span className="block max-w-72 whitespace-pre-line">
          {t("encounters.tier_hint_hint", { hp: hint.hp, sp: hint.sp, skill: hint.skill })}
        </span>
      }
    >
      <span className="block cursor-help text-xs font-normal normal-case text-muted-foreground">
        {t("encounters.tier_hint", { tier: t(`npc.tier.${hint.tier}`).toLowerCase() })}
        {" · "}
        {t(`encounters.tier_reason.${hint.reason}`)}
      </span>
    </Hint>
  );
}

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
  players,
  onSaveEncounter,
  onDeleteEncounter,
  onDeleteTemplate,
  onSetTemplateTier,
  onExport,
  onImport,
  onLaunch,
  onCloneTemplate,
}: EncountersPageProps) {
  const { t } = useTranslation();
  const [editing, setEditing] = useState<{ record: EncounterRecord | null } | null>(null);
  const [pendingDelete, setPendingDelete] = useState<PendingDelete | null>(null);
  const [assessments, setAssessments] = useState<Record<number, Assessment>>({});
  const [hints, setHints] = useState<Record<number, TierHint>>({});

  // The assessment is computed in Rust; it is redone when the group, the lines or the tiers change.
  useEffect(() => {
    let cancelled = false;
    if (players < 1) {
      setAssessments({});
      return;
    }
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

  // The tier hint is computed in Rust from the sheet; it is redone when the templates change.
  useEffect(() => {
    let cancelled = false;
    Promise.all(
      userTemplates.map((x) => api.tierHint(x.character).then((h) => [x.id, h] as const)),
    ).then(
      (pairs) => !cancelled && setHints(Object.fromEntries(pairs)),
      () => !cancelled && setHints({}),
    );
    return () => {
      cancelled = true;
    };
  }, [userTemplates]);

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
        <Hint label={t("encounters.players_hint")}>
          <span className="num cursor-help text-xs text-muted-foreground">
            {t("encounters.players", { count: players })}
          </span>
        </Hint>
        <Button disabled={!hasBackend} onClick={() => setEditing({ record: null })}>
          {t("encounters.new")}
        </Button>
      </div>
      {players < 1 && <p className="text-xs text-muted-foreground">{t("encounters.no_players")}</p>}
      <div className="flex flex-wrap items-center gap-2">
        <Button variant="outline" size="xs" disabled={!hasBackend} onClick={onImport}>
          {t("characters.import")}
        </Button>
        <Button
          variant="outline"
          size="xs"
          disabled={!hasBackend || (encounters.length === 0 && userTemplates.length === 0)}
          onClick={() =>
            void run(
              () => onExport(encounters.map((e) => e.id), userTemplates.map((x) => x.id)),
              "transfer:encounters",
            )
          }
        >
          {t("encounters.export_all")}
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
                    onClick={() => void run(() => onExport([record.id], []), "transfer:encounters")}
                  >
                    {t("encounters.export")}
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
                    {hints[template.id] && <TierHintLine hint={hints[template.id]} />}
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
                    onClick={() => void run(() => onExport([], [template.id]), "transfer:encounters")}
                  >
                    {t("encounters.export")}
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
