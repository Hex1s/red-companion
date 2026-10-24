import { useDeferredValue, useEffect, useState } from "react";
import { useTranslation } from "react-i18next";
import type { CharacterKind } from "@/bindings/CharacterKind";
import type { DeathSave } from "@/bindings/DeathSave";
import type { ExportFile } from "@/bindings/ExportFile";
import type { ExportKind } from "@/bindings/ExportKind";
import { CharacterCard } from "@/components/character/character-card";
import { CharacterList } from "@/components/character/character-list";
import { CharacterSheet } from "@/components/character/character-sheet";
import { ImportDialog } from "@/components/character/import-dialog";
import { NpcTemplateDialog, type CloneRequest } from "@/components/character/npc-template-dialog";
import { EncountersPage } from "@/components/encounter/encounters-page";
import { CombatBar } from "@/components/combat/combat-bar";
import { DamageDialog } from "@/components/combat/damage-dialog";
import { DrugDialog, type DrugRequest } from "@/components/combat/drug-dialog";
import { HealDialog, type HealTarget } from "@/components/combat/heal-dialog";
import { LoseHumanityDialog } from "@/components/combat/humanity-dialog";
import { RestDialog } from "@/components/combat/rest-dialog";
import { TherapyDialog } from "@/components/combat/therapy-dialog";
import type { HealKind } from "@/bindings/HealKind";
import { StabilizeDialog } from "@/components/combat/stabilize-dialog";
import { ReferencePage } from "@/components/reference/reference-page";
import { AppShell } from "@/components/layout/app-shell";
import { SessionSwitcher } from "@/components/layout/session-switcher";
import { CoverPicker } from "@/components/scene/cover-picker";
import { SessionNotes } from "@/components/session/session-notes";
import { Button } from "@/components/ui/button";
import { Tabs, TabsList, TabsTrigger } from "@/components/ui/tabs";
import { TooltipProvider } from "@/components/ui/tooltip";
import { errorMessage } from "@/lib/errors";
import { useCharacters } from "@/hooks/use-characters";
import { useEncounters } from "@/hooks/use-encounters";
import { useInjuries } from "@/hooks/use-injuries";
import { useNpcTemplates } from "@/hooks/use-npc-templates";
import { useScene } from "@/hooks/use-scene";
import { useSessions } from "@/hooks/use-sessions";
import { api, hasBackend } from "@/lib/api";
import { coverLabel } from "@/lib/cover-label";
import { formatCheck } from "@/lib/dice";
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
  const [humanityLossTargetId, setHumanityLossTargetId] = useState<number | null>(null);
  const [therapyTargetId, setTherapyTargetId] = useState<number | null>(null);
  const [healTarget, setHealTarget] = useState<{ id: number; injuryId: string; kind: HealKind } | null>(null);
  const [drugRequest, setDrugRequest] = useState<DrugRequest | null>(null);
  const [importFiles, setImportFiles] = useState<ExportFile[] | null>(null);
  // Which type of file the open import window is for (characters or a session).
  const [importKind, setImportKind] = useState<"character" | "session">("character");
  const [encounterImportFiles, setEncounterImportFiles] = useState<ExportFile[] | null>(null);
  const [lastSave, setLastSave] = useState<{ id: number; save: DeathSave } | null>(null);
  const damageTarget = entries.find((e) => e.id === damageTargetId) ?? null;
  const selected = entries.find((e) => e.id === selectedCharacterId) ?? null;
  // The sheet is heavy to build; while the next one renders, the previous stays on screen
  // (dimmed) instead of the page going blank and flickering.
  const deferredCharacterId = useDeferredValue(selectedCharacterId);
  const shown = entries.find((e) => e.id === deferredCharacterId) ?? selected;

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
    if (!error) return;
    status.fail(error, "characters:load", {
      text: t("characters.load_error", { error: errorMessage(error) }),
      retry: { label: t("status.retry"), run: () => void reload() },
    });
  }, [error, t, reload]);
  useEffect(() => {
    if (encounterState.error) {
      status.fail(encounterState.error, "encounters:load", {
        text: t("encounters.load_error", { error: errorMessage(encounterState.error) }),
        retry: { label: t("status.retry"), run: () => void encounterState.reload() },
      });
    }
  }, [encounterState.error, encounterState.reload, t]);

  // A death save result is shown until the turn moves on.
  useEffect(() => setLastSave(null), [combat?.active, combat?.round]);

  // The rail shows the session's characters only: fighters first, in acting order, then the
  // others in the order they joined.
  const sessionEntries = members.flatMap((id) => entries.filter((e) => e.id === id));
  // A dose can go to anyone in the session, or to the Medtech themselves.
  const drugHelper = drugRequest ? (entries.find((e) => e.id === drugRequest.helperId) ?? null) : null;
  const drugTargets =
    drugHelper && !sessionEntries.some((e) => e.id === drugHelper.id)
      ? [drugHelper, ...sessionEntries]
      : sessionEntries;
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
    setDrugRequest(null);
    setLastSave(null);
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [sessionId]);

  const leaveSession = async (id: number) => {
    try {
      sceneState.replaceScene(await api.removeFromSession(id));
      await reload();
    } catch (e) {
      status.fail(e, "session:leave");
    }
  };

  const rollInitiative = async (id: number, name: string) => {
    const record = await sceneState.combatRoll(id);
    const rolled = record?.scene.combat.combatants.find((c) => c.character_id === id);
    if (!rolled?.roll) return;
    status.show({
      kind: "info",
      id: `initiative:${id}`,
      text: t(rolled.bonus !== 0 ? "initiative.rolled_bonus" : "initiative.rolled", {
        name,
        reflex: rolled.reflex,
        bonus: rolled.bonus,
        die: formatCheck(rolled.roll),
        total: rolled.initiative,
      }),
    });
  };

  // The GM adds or removes an injury by hand; the sheet reads the injuries from the saved record.
  const changeInjury = async (id: number, change: "add" | "remove", injuryId: string) => {
    try {
      await (change === "add" ? api.addInjury(id, injuryId) : api.removeInjury(id, injuryId));
      await reload();
    } catch (e) {
      status.fail(e, "injury");
    }
  };

  const joinSession = async (id: number) => {
    try {
      await addToSession(id);
    } catch (e) {
      status.fail(e, "session:join");
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
      status.fail(e, "encounter:template", { text: t("encounters.save_error", { error: errorMessage(e) }) });
    }
  };

  const openExportsFolder = (kind?: ExportKind) =>
    api.openExportsFolder(kind).catch((e: unknown) => status.fail(e, "transfer:open"));

  /** Writes one character, or the active session, into the export folder and says where it went. */
  const exportCharacters = async (what: { ids: number[] } | "session") => {
    const kind: ExportKind = what === "session" ? "session" : "character";
    try {
      const saved = await (what === "session"
        ? api.exportSession()
        : api.exportCharacters(what.ids));
      status.show({
        id: "transfer",
        kind: "success",
        text: t(saved.count === 1 ? "transfer.exported_one" : "transfer.exported_many", {
          count: saved.count,
          path: saved.path,
        }),
        action: { label: t("transfer.open_folder"), run: () => void openExportsFolder(kind) },
        autoHideMs: 9000,
      });
    } catch (e) {
      status.fail(e, "transfer", { text: t("transfer.error", { error: errorMessage(e) }) });
    }
  };

  /**
   * Import reads only from the app's export folder and shows only the files of its own type; an
   * empty folder is reported, not opened.
   */
  const startImport = async (kind: "character" | "session") => {
    try {
      const files = await api.listExports(kind);
      if (files.length === 0) {
        status.show({
          id: "transfer",
          kind: "info",
          text: t(kind === "session" ? "transfer.empty_session" : "transfer.empty"),
          action: { label: t("transfer.open_folder"), run: () => void openExportsFolder(kind) },
          autoHideMs: 9000,
        });
      } else {
        setImportKind(kind);
        setImportFiles(files);
      }
    } catch (e) {
      status.fail(e, "transfer", { text: t("transfer.error", { error: errorMessage(e) }) });
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
        action: { label: t("transfer.open_folder"), run: () => void openExportsFolder("encounters") },
        autoHideMs: 9000,
      });
    } catch (e) {
      status.fail(e, "transfer:encounters", { text: t("transfer.error", { error: errorMessage(e) }) });
    }
  };

  const startEncounterImport = async () => {
    try {
      const files = await api.listExports("encounters");
      if (files.length === 0) {
        status.show({
          id: "transfer:encounters",
          kind: "info",
          text: t("encounters.import_empty"),
          action: { label: t("transfer.open_folder"), run: () => void openExportsFolder("encounters") },
          autoHideMs: 9000,
        });
      } else {
        setEncounterImportFiles(files);
      }
    } catch (e) {
      status.fail(e, "transfer:encounters", { text: t("transfer.error", { error: errorMessage(e) }) });
    }
  };

  const undoDamage = async () => {
    try {
      const result = await api.undoDamage();
      sceneState.replaceScene(result.scene);
      await reload();
    } catch (e) {
      status.fail(e, "damage:undo");
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
      status.fail(e, "death-save");
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
                <TabsTrigger value="notes">{t("nav.notes")}</TabsTrigger>
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
                <Button variant="outline" size="xs" disabled={!hasBackend} onClick={() => void startImport("character")}>
                  {t("characters.import")}
                </Button>
                <Button variant="outline" size="xs" disabled={!hasBackend} onClick={() => void startImport("session")}>
                  {t("characters.import_session")}
                </Button>
                <Button
                  variant="outline"
                  size="xs"
                  disabled={!hasBackend || !selected}
                  onClick={() => selected && exportCharacters({ ids: [selected.id] })}
                >
                  {t("characters.export_selected")}
                </Button>
                <Button
                  variant="outline"
                  size="xs"
                  disabled={!hasBackend || members.length === 0}
                  onClick={() => exportCharacters("session")}
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
              {shown ? (
                <div className={shown.id === selectedCharacterId ? undefined : "opacity-60 transition-opacity"}>
                <CharacterSheet
                  key={shown.id}
                  entry={shown}
                  onSave={(character) => save(shown.id, character)}
                  onSaveAsTemplate={() => saveAsTemplate(shown.id)}
                  injuryTable={injuries}
                  onHeal={(injury, kind) =>
                    setHealTarget({ id: shown.id, injuryId: injury.id, kind })
                  }
                  onAddInjury={(injuryId) => void changeInjury(shown.id, "add", injuryId)}
                  onRemoveInjury={(injuryId) => void changeInjury(shown.id, "remove", injuryId)}
                  onRest={() => setRestTargetId(shown.id)}
                  onAdminister={(drug) => setDrugRequest({ helperId: shown.id, drug })}
                  onLoseHumanity={() => setHumanityLossTargetId(shown.id)}
                  onTherapy={() => setTherapyTargetId(shown.id)}
                  onDelete={async () => {
                    try {
                      await remove(shown.id);
                      // The backend passed the turn on if the deleted one was active.
                      sceneState.replaceScene(await api.getScene());
                      selectCharacter(null);
                    } catch (e) {
                      status.fail(e, "character:delete");
                    }
                  }}
                />
                </div>
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
          ) : tab === "notes" ? (
            sessionState.active ? (
              <SessionNotes
                key={sessionState.active.id}
                sessionId={sessionState.active.id}
                sessionName={sessionState.active.name}
              />
            ) : (
              <p className="text-muted-foreground">{t("notes.no_session")}</p>
            )
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
                  initiativeRoll={fighter?.roll ?? null}
                  initiativeBonus={fighter?.bonus ?? 0}
                  onRollInitiative={() => void rollInitiative(e.id, e.character.name)}
                  onShift={(delta) => void sceneState.combatShift(e.id, delta)}
                  onStabilize={() => setStabilizeTargetId(e.id)}
                  onRest={() => setRestTargetId(e.id)}
                  onRevive={() =>
                    api.reviveCharacter(e.id).then(
                      () => reload(),
                      (err: unknown) => status.fail(err, "revive"),
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
                const records = await api.importExportFile(importKind, name);
                void reload();
                if (records.length === 1) selectCharacter(records[0].id);
                status.show({
                  id: "transfer",
                  kind: "success",
                  text: t("transfer.imported", { count: records.length }),
                });
              }}
              onOpenFolder={() => void openExportsFolder(importKind)}
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
              onOpenFolder={() => void openExportsFolder("encounters")}
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
            <LoseHumanityDialog
              target={entries.find((e) => e.id === humanityLossTargetId) ?? null}
              onDone={(applied) => {
                void reload();
                const { loss, character } = applied;
                status.show({
                  id: "humanity",
                  kind: loss.became_cyberpsycho ? "error" : "success",
                  text: t(loss.became_cyberpsycho ? "humanity.lost_psycho" : "humanity.lost", {
                    name: character.character.name,
                    from: loss.from,
                    to: loss.to,
                    emp: loss.emp_to,
                  }),
                });
              }}
              onClose={() => setHumanityLossTargetId(null)}
            />
            <TherapyDialog
              target={entries.find((e) => e.id === therapyTargetId) ?? null}
              onDone={(applied) => {
                void reload();
                if (!applied.result.success) return;
                status.show({
                  id: "humanity",
                  kind: "success",
                  text: t("humanity.done", {
                    name: applied.character.character.name,
                    restored: applied.result.restored,
                    from: applied.result.from,
                    to: applied.result.to,
                  }),
                });
              }}
              onClose={() => setTherapyTargetId(null)}
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
            <DrugDialog
              request={drugRequest}
              targets={drugTargets}
              onDone={(applied, drugName) => {
                void reload();
                status.show({
                  id: "drug",
                  kind: "success",
                  text: t(applied.result.restored_hp > 0 ? "drug.done_hp" : "drug.done", {
                    name: applied.character.character.name,
                    drug: drugName,
                    hp: applied.result.restored_hp,
                  }),
                });
              }}
              onClose={() => setDrugRequest(null)}
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
