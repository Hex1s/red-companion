import { useEffect, useState } from "react";
import { useTranslation } from "react-i18next";
import type { CharacterKind } from "@/bindings/CharacterKind";
import type { DeathSave } from "@/bindings/DeathSave";
import type { ExportFile } from "@/bindings/ExportFile";
import { CharacterCard } from "@/components/character/character-card";
import { CharacterList } from "@/components/character/character-list";
import { CharacterSheet } from "@/components/character/character-sheet";
import { ImportDialog } from "@/components/character/import-dialog";
import { NpcTemplateDialog, type CloneRequest } from "@/components/character/npc-template-dialog";
import { EncountersPage } from "@/components/encounter/encounters-page";
import { CombatBar } from "@/components/combat/combat-bar";
import { DamageDialog } from "@/components/combat/damage-dialog";
import { HealDialog, type HealTarget } from "@/components/combat/heal-dialog";
import { RestDialog } from "@/components/combat/rest-dialog";
import type { HealKind } from "@/bindings/HealKind";
import { StabilizeDialog } from "@/components/combat/stabilize-dialog";
import { ReferencePage } from "@/components/reference/reference-page";
import { AppShell } from "@/components/layout/app-shell";
import { SessionSwitcher } from "@/components/layout/session-switcher";
import { CoverPicker } from "@/components/scene/cover-picker";
import { Button } from "@/components/ui/button";
import { Tabs, TabsList, TabsTrigger } from "@/components/ui/tabs";
import { TooltipProvider } from "@/components/ui/tooltip";
import { errorMessage, useCharacters } from "@/hooks/use-characters";
import { useEncounters } from "@/hooks/use-encounters";
import { useInjuries } from "@/hooks/use-injuries";
import { useNpcTemplates } from "@/hooks/use-npc-templates";
import { useScene } from "@/hooks/use-scene";
import { useSessions } from "@/hooks/use-sessions";
import { api, hasBackend } from "@/lib/api";
import { coverLabel } from "@/lib/cover-label";
import { cloneNames, npcNotes, templateKey } from "@/lib/encounter";
import type { EncounterRecord } from "@/bindings/EncounterRecord";
import type { UserTemplateRecord } from "@/bindings/UserTemplateRecord";
import { SUPPORTED_LANGUAGES } from "@/i18n";
import { status } from "@/store/status-store";
import { useUiStore, type AppTab } from "@/store/ui-store";

export default function App() {
  const { t, i18n } = useTranslation();
  const { tab, setTab, selectedCharacterId, selectCharacter, damageTargetId, openDamage } =
    useUiStore();
  const sessionState = useSessions();
  const sessionId = sessionState.active?.id ?? null;
  const { entries, members, error, create, save, remove, reload, addToSession } =
    useCharacters(sessionId);
  const sceneState = useScene(sessionId);
  const injuries = useInjuries();
  const npcTemplates = useNpcTemplates();
  const encounterState = useEncounters();
  const [templatesOpen, setTemplatesOpen] = useState(false);
  const [stabilizeTargetId, setStabilizeTargetId] = useState<number | null>(null);
  const [restTargetId, setRestTargetId] = useState<number | null>(null);
  const [healTarget, setHealTarget] = useState<{ id: number; injuryId: string; kind: HealKind } | null>(null);
  const [importFiles, setImportFiles] = useState<ExportFile[] | null>(null);
  const [encounterImportFiles, setEncounterImportFiles] = useState<ExportFile[] | null>(null);
  const [lastSave, setLastSave] = useState<{ id: number; save: DeathSave } | null>(null);
  const damageTarget = entries.find((e) => e.id === damageTargetId) ?? null;
  const selected = entries.find((e) => e.id === selectedCharacterId) ?? null;

  // The healer may be anyone in the session, or the patient themselves (self-help is allowed).
  const healPatient = healTarget ? (entries.find((e) => e.id === healTarget.id) ?? null) : null;
  const healInjury = healTarget ? injuries.find((i) => i.id === healTarget.injuryId) : undefined;
  const healDialogTarget: HealTarget | null =
    healTarget && healPatient && healInjury
      ? { character: healPatient, injury: healInjury, kind: healTarget.kind }
      : null;

  const scene = sceneState.scene;
  const combat = scene?.combat ?? null;
  const coverOf = (id: number) => scene?.covers.find((c) => c.characters.includes(id)) ?? null;
  const damageCover = damageTarget ? coverOf(damageTarget.id) : null;

  // Failed loads are reported in the status area, not in the page.
  useEffect(() => {
    if (error) status.error(t("characters.load_error", { error }), "characters:load");
  }, [error, t]);
  useEffect(() => {
    if (encounterState.error) {
      status.error(t("encounters.load_error", { error: encounterState.error }), "encounters:load");
    }
  }, [encounterState.error, t]);

  // A death save result is shown until the turn moves on.
  useEffect(() => setLastSave(null), [combat?.active, combat?.round]);

  // The rail shows the session's characters only: fighters first, in acting order, then the
  // others in the order they joined.
  const sessionEntries = members.flatMap((id) => entries.filter((e) => e.id === id));
  const healHelpers =
    healPatient && !sessionEntries.some((e) => e.id === healPatient.id)
      ? [healPatient, ...sessionEntries]
      : sessionEntries;
  const railEntries = combat
    ? [
        ...combat.combatants.flatMap((c) => sessionEntries.filter((e) => e.id === c.character_id)),
        ...sessionEntries.filter((e) => !combat.combatants.some((c) => c.character_id === e.id)),
      ]
    : sessionEntries;
  // The group an encounter is compared with: the session's player characters.
  const players = sessionEntries.filter((e) => e.kind === "pc").length;

  // Dialogs and one-off results belong to the session they were opened in.
  useEffect(() => {
    openDamage(null);
    setStabilizeTargetId(null);
    setRestTargetId(null);
    setHealTarget(null);
    setLastSave(null);
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [sessionId]);

  const leaveSession = async (id: number) => {
    try {
      sceneState.replaceScene(await api.removeFromSession(id));
      await reload();
    } catch (e) {
      status.error(errorMessage(e), "session:leave");
    }
  };

  const joinSession = async (id: number) => {
    try {
      await addToSession(id);
    } catch (e) {
      status.error(errorMessage(e), "session:join");
    }
  };

  const createCharacter = async (kind: CharacterKind) => {
    const record = await create(kind, t(`characters.default_name_${kind}`));
    selectCharacter(record.id);
  };

  /** Clones a ready-made NPC once per name. */
  const cloneTemplate = async ({ templateId, names, notes }: CloneRequest) => {
    let last: number | null = null;
    try {
      for (const name of names) {
        last = (await api.createNpcFromTemplate(templateId, name, notes)).id;
      }
    } finally {
      // Even after a partial failure the clones that exist must show up.
      await reload();
    }
    if (last !== null) selectCharacter(last);
  };

  /** Clones one own template into the session; returns the new NPC's id. */
  const cloneUserTemplate = async (template: UserTemplateRecord, taken: string[]) => {
    const [name] = cloneNames(template.character.name, 1, taken);
    const record = await api.createNpcFromUserTemplate(template.id, name);
    taken.push(name);
    return record.id;
  };

  /** Clones every NPC of an encounter into the session (all NPCs get their own HP and SP). */
  const launchEncounter = async ({ encounter }: EncounterRecord) => {
    const taken = entries.map((e) => e.character.name);
    let last: number | null = null;
    let created = 0;
    let missing = 0;
    try {
      for (const line of encounter.entries) {
        const key = templateKey(line.source, line.template);
        const book = npcTemplates.find((x) => templateKey("book", x.code) === key);
        const own = encounterState.templates.find((x) => templateKey("custom", String(x.id)) === key);
        if (!book && !own) {
          missing += line.count;
          continue;
        }
        const base = book?.name ?? own?.character.name ?? "";
        const names = cloneNames(base, line.count, taken);
        for (const name of names) {
          const record = book
            ? await api.createNpcFromTemplate(book.code, name, npcNotes(t, book))
            : await api.createNpcFromUserTemplate(own!.id, name);
          taken.push(name);
          last = record.id;
          created += 1;
        }
      }
    } finally {
      // Even after a partial failure the clones that exist must show up.
      await reload();
    }
    if (last !== null) selectCharacter(last);
    status.show({
      id: "encounter:launch",
      kind: missing > 0 ? "info" : "success",
      text: t(missing > 0 ? "encounters.launched_missing" : "encounters.launched", {
        count: created,
        missing,
      }),
      autoHideMs: 6000,
    });
  };

  const saveAsTemplate = async (id: number) => {
    try {
      const saved = await api.saveCharacterAsTemplate(id, null);
      await encounterState.reload();
      status.show({
        id: "encounter:template",
        kind: "success",
        text: t("encounters.template_saved", { name: saved.character.name }),
      });
    } catch (e) {
      status.error(t("encounters.save_error", { error: errorMessage(e) }), "encounter:template");
    }
  };

  const openExportsFolder = () =>
    api.openExportsFolder().catch((e: unknown) => status.error(errorMessage(e), "transfer:open"));

  /** Writes `ids` (or everyone) into the app's export folder and says where the file went. */
  const exportCharacters = async (ids: number[]) => {
    try {
      const saved = await api.exportCharacters(ids);
      status.show({
        id: "transfer",
        kind: "success",
        text: t(saved.count === 1 ? "transfer.exported_one" : "transfer.exported_many", {
          count: saved.count,
          path: saved.path,
        }),
        action: { label: t("transfer.open_folder"), run: () => void openExportsFolder() },
        autoHideMs: 9000,
      });
    } catch (e) {
      status.error(t("transfer.error", { error: errorMessage(e) }), "transfer");
    }
  };

  /** Import reads only from the app's export folder; an empty folder is reported, not opened. */
  const startImport = async () => {
    try {
      const files = await api.listExports();
      if (files.length === 0) {
        status.show({
          id: "transfer",
          kind: "info",
          text: t("transfer.empty"),
          action: { label: t("transfer.open_folder"), run: () => void openExportsFolder() },
          autoHideMs: 9000,
        });
      } else {
        setImportFiles(files);
      }
    } catch (e) {
      status.error(t("transfer.error", { error: errorMessage(e) }), "transfer");
    }
  };

  /** Writes encounters and own templates into one file; the file takes the templates its encounters use. */
  const exportEncounters = async (encounterIds: number[], templateIds: number[]) => {
    try {
      const saved = await api.exportEncounters(encounterIds, templateIds);
      status.show({
        id: "transfer:encounters",
        kind: "success",
        text: t("encounters.exported", {
          encounters: saved.encounters,
          templates: saved.templates,
          path: saved.path,
        }),
        action: { label: t("transfer.open_folder"), run: () => void openExportsFolder() },
        autoHideMs: 9000,
      });
    } catch (e) {
      status.error(t("transfer.error", { error: errorMessage(e) }), "transfer:encounters");
    }
  };

  const startEncounterImport = async () => {
    try {
      const files = await api.listEncounterExports();
      if (files.length === 0) {
        status.show({
          id: "transfer:encounters",
          kind: "info",
          text: t("encounters.import_empty"),
          action: { label: t("transfer.open_folder"), run: () => void openExportsFolder() },
          autoHideMs: 9000,
        });
      } else {
        setEncounterImportFiles(files);
      }
    } catch (e) {
      status.error(t("transfer.error", { error: errorMessage(e) }), "transfer:encounters");
    }
  };

  const undoDamage = async () => {
    try {
      const result = await api.undoDamage();
      sceneState.replaceScene(result.scene);
      await reload();
    } catch (e) {
      status.error(errorMessage(e), "damage:undo");
    }
  };

  const deathSave = async (id: number, manual: number | null) => {
    try {
      const applied = await api.deathSave(id, manual);
      sceneState.replaceScene(applied.scene);
      setLastSave({ id, save: applied.save });
      await reload();
      return true;
    } catch (e) {
      status.error(errorMessage(e), "death-save");
      return false;
    }
  };

  return (
    <TooltipProvider>
      <AppShell
        header={
          <>
            <h1 className="text-lg font-bold text-primary">{t("app.title")}</h1>
            <Tabs value={tab} onValueChange={(v) => setTab(v as AppTab)} className="flex-1">
              <TabsList>
                <TabsTrigger value="characters">{t("nav.characters")}</TabsTrigger>
                <TabsTrigger value="encounters">{t("nav.encounters")}</TabsTrigger>
                <TabsTrigger value="rules">{t("nav.rules")}</TabsTrigger>
              </TabsList>
            </Tabs>
            <SessionSwitcher
              sessions={sessionState.sessions}
              disabled={!hasBackend}
              onSwitch={(id) => void sessionState.switchTo(id)}
              onCreate={(name) => sessionState.create(name)}
              onRename={sessionState.rename}
              onDelete={sessionState.remove}
            />
            <div className="flex gap-1">
              {SUPPORTED_LANGUAGES.map((lng) => (
                <Button
                  key={lng}
                  size="xs"
                  variant={i18n.resolvedLanguage === lng ? "default" : "outline"}
                  onClick={() => i18n.changeLanguage(lng)}
                >
                  {lng}
                </Button>
              ))}
            </div>
          </>
        }
        main={
          tab === "characters" ? (
            <div className="space-y-6">
              <div className="flex items-center gap-2">
                <h2 className="mr-auto text-2xl font-bold">{t("characters.title")}</h2>
                <Button disabled={!hasBackend} onClick={() => createCharacter("pc")}>
                  {t("characters.new_pc")}
                </Button>
                <Button variant="outline" disabled={!hasBackend} onClick={() => createCharacter("npc")}>
                  {t("characters.new_npc")}
                </Button>
                <Button
                  variant="outline"
                  disabled={!hasBackend || npcTemplates.length === 0}
                  onClick={() => setTemplatesOpen(true)}
                >
                  {t("npc.from_template")}
                </Button>
              </div>
              <div className="flex flex-wrap items-center gap-2">
                <Button variant="outline" size="xs" disabled={!hasBackend} onClick={() => void startImport()}>
                  {t("characters.import")}
                </Button>
                <Button
                  variant="outline"
                  size="xs"
                  disabled={!hasBackend || !selected}
                  onClick={() => selected && exportCharacters([selected.id])}
                >
                  {t("characters.export_selected")}
                </Button>
                <Button
                  variant="outline"
                  size="xs"
                  disabled={!hasBackend || members.length === 0}
                  onClick={() => exportCharacters(members)}
                >
                  {t("characters.export_all")}
                </Button>
              </div>
              {!hasBackend && <p className="text-muted-foreground">{t("characters.no_backend")}</p>}
              <CharacterList
                entries={entries}
                members={members}
                selectedId={selectedCharacterId}
                onSelect={selectCharacter}
                onAdd={(id) => void joinSession(id)}
                onLeave={(id) => void leaveSession(id)}
              />
              {selected ? (
                <CharacterSheet
                  key={selected.id}
                  entry={selected}
                  onSave={(character) => save(selected.id, character)}
                  onSaveAsTemplate={() => saveAsTemplate(selected.id)}
                  injuryTable={injuries}
                  onHeal={(injury, kind) =>
                    setHealTarget({ id: selected.id, injuryId: injury.id, kind })
                  }
                  onRest={() => setRestTargetId(selected.id)}
                  onDelete={async () => {
                    try {
                      await remove(selected.id);
                      selectCharacter(null);
                    } catch (e) {
                      status.error(errorMessage(e), "character:delete");
                    }
                  }}
                />
              ) : (
                hasBackend && (
                  <p className="text-muted-foreground">
                    {entries.length === 0 ? t("characters.empty") : t("characters.select_hint")}
                  </p>
                )
              )}
            </div>
          ) : tab === "encounters" ? (
            <EncountersPage
              bookTemplates={npcTemplates}
              userTemplates={encounterState.templates}
              encounters={encounterState.encounters}
              players={players}
              onSaveEncounter={encounterState.saveEncounter}
              onDeleteEncounter={encounterState.deleteEncounter}
              onDeleteTemplate={encounterState.deleteTemplate}
              onSetTemplateTier={encounterState.setTemplateTier}
              onExport={exportEncounters}
              onImport={() => void startEncounterImport()}
              onLaunch={launchEncounter}
              onCloneTemplate={async (template) => {
                const id = await cloneUserTemplate(template, entries.map((e) => e.character.name));
                await reload();
                selectCharacter(id);
              }}
            />
          ) : (
            <ReferencePage />
          )
        }
        rail={
          <>
            {combat && (
              <CombatBar
                combat={combat}
                hasCharacters={sessionEntries.length > 0}
                lastDamage={scene?.last_damage ?? null}
                onStart={() => void sceneState.combatStart()}
                onNextTurn={() => void sceneState.combatNextTurn()}
                onEnd={() => void sceneState.combatEnd()}
                onUndo={() => void undoDamage()}
              />
            )}
            {railEntries.map((e) => {
              const fighter = combat?.combatants.find((c) => c.character_id === e.id);
              const index = fighter && combat ? combat.combatants.indexOf(fighter) : -1;
              const cover = coverOf(e.id);
              return (
                <CharacterCard
                  key={e.id}
                  entry={e}
                  initiative={fighter?.initiative ?? null}
                  place={
                    fighter && combat
                      ? { first: index === 0, last: index === combat.combatants.length - 1 }
                      : null
                  }
                  active={combat?.active === e.id}
                  selected={e.id === selectedCharacterId}
                  inCover={cover !== null && cover.hp > 0}
                  coverControl={
                    scene && (
                      <CoverPicker
                        scene={scene}
                        presets={sceneState.presets}
                        current={cover}
                        onAssign={(coverId) => void sceneState.setCharacterCover(e.id, coverId)}
                        onCreate={(preset, name, hp) =>
                          void sceneState.addCoverFor(e.id, preset, name, hp)
                        }
                        onSetHp={(c, hp) => void sceneState.editCover(c.id, c.name, hp, c.max_hp)}
                      />
                    )
                  }
                  deathSaveDue={combat?.death_save_due === true && combat.active === e.id}
                  lastSave={lastSave?.id === e.id ? lastSave.save : null}
                  onSelect={() => {
                    selectCharacter(e.id);
                    setTab("characters");
                  }}
                  onDamage={() => openDamage(e.id)}
                  onSetInitiative={(value) => void sceneState.combatSetInitiative(e.id, value)}
                  onRollInitiative={() => void sceneState.combatRoll(e.id)}
                  onShift={(delta) => void sceneState.combatShift(e.id, delta)}
                  onStabilize={() => setStabilizeTargetId(e.id)}
                  onRest={() => setRestTargetId(e.id)}
                  onRevive={() =>
                    api.reviveCharacter(e.id).then(
                      () => reload(),
                      (err: unknown) => status.error(errorMessage(err), "revive"),
                    )
                  }
                  onDeathSave={(manual) => deathSave(e.id, manual)}
                  onLeave={() => void leaveSession(e.id)}
                />
              );
            })}
            {sessionEntries.length === 0 && sessionState.active && (
              <p className="p-2 text-sm text-muted-foreground">{t("sessions.empty_rail")}</p>
            )}
            <NpcTemplateDialog
              open={templatesOpen}
              templates={npcTemplates}
              existingNames={entries.map((e) => e.character.name)}
              onClone={cloneTemplate}
              onClose={() => setTemplatesOpen(false)}
            />
            <ImportDialog
              files={importFiles}
              onImport={async (name) => {
                const records = await api.importExportFile(name);
                void reload();
                if (records.length === 1) selectCharacter(records[0].id);
                status.show({
                  id: "transfer",
                  kind: "success",
                  text: t("transfer.imported", { count: records.length }),
                });
              }}
              onOpenFolder={() => void openExportsFolder()}
              onClose={() => setImportFiles(null)}
            />
            <ImportDialog
              files={encounterImportFiles}
              onImport={async (name) => {
                const added = await api.importEncounterFile(name);
                await encounterState.reload();
                status.show({
                  id: "transfer:encounters",
                  kind: "success",
                  text: t("encounters.imported", {
                    encounters: added.encounters.length,
                    templates: added.templates.length,
                  }),
                });
              }}
              onOpenFolder={() => void openExportsFolder()}
              onClose={() => setEncounterImportFiles(null)}
            />
            <StabilizeDialog
              target={entries.find((e) => e.id === stabilizeTargetId) ?? null}
              helpers={sessionEntries}
              onDone={() => void reload()}
              onClose={() => setStabilizeTargetId(null)}
            />
            <RestDialog
              target={entries.find((e) => e.id === restTargetId) ?? null}
              onDone={(applied) => {
                void reload();
                status.show({
                  id: "rest",
                  kind: "success",
                  text: t("rest.done", {
                    name: applied.character.character.name,
                    days: applied.plan.days,
                    hp: applied.plan.final_hp,
                  }),
                });
              }}
              onClose={() => setRestTargetId(null)}
            />
            <HealDialog
              target={healDialogTarget}
              helpers={healHelpers}
              onDone={(applied) => {
                void reload();
                if (!applied.result.success) return;
                status.show({
                  id: "heal",
                  kind: "success",
                  text: t(applied.result.removed ? "heal.success_removed" : "heal.success_lifted", {
                    name: applied.character.character.name,
                    injury: t(`injury.${applied.result.injury_id}.name`, {
                      defaultValue: applied.result.injury_id,
                    }),
                  }),
                });
              }}
              onClose={() => setHealTarget(null)}
            />
            <DamageDialog
              target={damageTarget}
              cover={
                damageCover && damageCover.hp > 0
                  ? { id: damageCover.id, label: coverLabel(damageCover, sceneState.presets, t) }
                  : null
              }
              injuries={injuries}
              onApplied={(result) => {
                sceneState.replaceScene(result.scene);
                void reload();
              }}
              onClose={() => openDamage(null)}
            />
          </>
        }
      />
    </TooltipProvider>
  );
}
