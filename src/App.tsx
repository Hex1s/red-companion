import { Info, Keyboard, PanelLeft, Search } from "@/components/ui/icons";
import { useDeferredValue, useEffect, useState } from "react";
import { useTranslation } from "react-i18next";
import { useHotkeys } from "@/hooks/use-hotkeys";
import { useLayoutMode } from "@/hooks/use-layout-mode";
import { CombatDock } from "@/components/combat/combat-dock";
import { AboutDialog } from "@/components/layout/about-dialog";
import { ShortcutsDialog } from "@/components/layout/shortcuts-dialog";
import { formatHotkey } from "@/lib/hotkeys";
import type { CharacterKind } from "@/bindings/CharacterKind";
import type { DeathSave } from "@/bindings/DeathSave";
import type { ExportFile } from "@/bindings/ExportFile";
import type { ExportKind } from "@/bindings/ExportKind";
import { TemplateEditor } from "@/components/character/template-editor";
import { arrayMove } from "@dnd-kit/sortable";
import { SortableCard, SortableRail } from "@/components/character/sortable-rail";
import { CharacterCard } from "@/components/character/character-card";
import { CharacterSheet } from "@/components/character/character-sheet";
import { Explorer } from "@/components/explorer/explorer";
import { ImportDialog } from "@/components/character/import-dialog";
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
import { ReferenceSearch } from "@/components/reference/reference-search";
import { AppShell } from "@/components/layout/app-shell";
import { CoverPicker } from "@/components/scene/cover-picker";
import { SessionNotes } from "@/components/session/session-notes";
import { Button } from "@/components/ui/button";
import { TooltipProvider } from "@/components/ui/tooltip";
import { errorMessage } from "@/lib/errors";
import { useCharacters } from "@/hooks/use-characters";
import { useEncounters } from "@/hooks/use-encounters";
import { useInjuries } from "@/hooks/use-injuries";
import { useNpcTemplates } from "@/hooks/use-npc-templates";
import { useScene } from "@/hooks/use-scene";
import { useExplorer } from "@/hooks/use-explorer";
import { api, hasBackend } from "@/lib/api";
import { coverLabel } from "@/lib/cover-label";
import { formatCheck } from "@/lib/dice";
import { cloneNames, npcNotes, templateKey } from "@/lib/encounter";
import type { DeletedCharacter } from "@/bindings/DeletedCharacter";
import type { EncounterRecord } from "@/bindings/EncounterRecord";
import type { UserTemplateRecord } from "@/bindings/UserTemplateRecord";
import { SUPPORTED_LANGUAGES } from "@/i18n";
import { status } from "@/store/status-store";
import { useUiStore } from "@/store/ui-store";
import { AppPath } from "@/components/layout/app-path";

/** How long the "deleted — undo" message stays. */
const UNDO_DELETE_MS = 8000;
/** Copies of one template that the "add more" button of the toast builds up to. */
const SPAWN_MAX = 3;

export default function App() {
  const { t, i18n } = useTranslation();
  const {
    tab,
    setTab,
    selectedCharacterId,
    selectedTemplateKey,
    selectTemplate,
    selectCharacter,
    damageTargetId,
    openDamage,
    explorerOpen,
    toggleExplorer,
    explorerDrawerOpen,
    setExplorerDrawerOpen,
    railDrawerOpen,
    setRailDrawerOpen,
  } = useUiStore();
  const layoutMode = useLayoutMode();
  // Wide: the explorer is a pinned column; narrower: a drawer that opens on demand.
  const explorerShown = layoutMode === "wide" ? explorerOpen : explorerDrawerOpen;
  const toggleExplorerPanel = () =>
    layoutMode === "wide" ? toggleExplorer() : setExplorerDrawerOpen(!explorerDrawerOpen);
  // Drawers belong to the narrower layouts: leaving them must not leave a stale open flag behind.
  useEffect(() => {
    if (layoutMode === "wide") setExplorerDrawerOpen(false);
    if (layoutMode !== "narrow") setRailDrawerOpen(false);
  }, [layoutMode, setExplorerDrawerOpen, setRailDrawerOpen]);
  // Picking something in the explorer drawer reveals it in the main area.
  useEffect(() => {
    setExplorerDrawerOpen(false);
  }, [tab, selectedCharacterId, selectedTemplateKey, setExplorerDrawerOpen]);
  const explorer = useExplorer();
  const sessionId = explorer.active?.id ?? null;
  const { entries, error, create, save, remove, restore, reload } = useCharacters(sessionId, explorer.reload);
  // Who lies in the active session; the tree is the single source for it.
  const members = explorer.active?.members ?? [];
  const sceneState = useScene(sessionId);
  const injuries = useInjuries();
  const npcTemplates = useNpcTemplates();
  const encounterState = useEncounters();
  const [searchOpen, setSearchOpen] = useState(false);
  const [helpOpen, setHelpOpen] = useState(false);
  const [aboutOpen, setAboutOpen] = useState(false);
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
  const openedTemplate =
    encounterState.templates.find((x) => templateKey("custom", String(x.id)) === selectedTemplateKey) ?? null;
  // A book template opened as printed: nothing is stored until it is edited.
  const openedBook =
    openedTemplate?.based_on !== undefined && openedTemplate.based_on !== null
      ? (npcTemplates.find((x) => x.code === openedTemplate.based_on) ?? null)
      : (npcTemplates.find((x) => templateKey("book", x.code) === selectedTemplateKey) ?? null);
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

  const sessionEntries = members.flatMap((id) => entries.filter((e) => e.id === id));
  // The rail shows the fight, not the session: its order is the initiative order. The session's
  // members who are not fighting (an absent player) wait in the explorer.
  const fighters = combat
    ? combat.combatants.flatMap((c) => entries.filter((e) => e.id === c.character_id))
    : [];
  const railEntries = fighters;
  // A dragged card keeps its new place at once; the order the backend answers with replaces it.
  const [pendingOrder, setPendingOrder] = useState<number[] | null>(null);
  const railShown =
    pendingOrder && pendingOrder.length === railEntries.length
      ? pendingOrder.flatMap((id) => railEntries.filter((e) => e.id === id))
      : railEntries;
  const moveFighter = (id: number, to: number) => {
    const ids = railEntries.map((e) => e.id);
    setPendingOrder(arrayMove(ids, ids.indexOf(id), to));
    void sceneState.combatMove(id, to).finally(() => setPendingOrder(null));
  };
  // Helpers (healers, who gives a dose) may be anyone at the table: fighters and the session.
  const tableEntries = [...fighters, ...sessionEntries.filter((e) => !fighters.some((f) => f.id === e.id))];
  // A dose can go to anyone in the session, or to the Medtech themselves.
  const drugHelper = drugRequest ? (entries.find((e) => e.id === drugRequest.helperId) ?? null) : null;
  const drugTargets =
    drugHelper && !tableEntries.some((e) => e.id === drugHelper.id)
      ? [drugHelper, ...tableEntries]
      : tableEntries;
  const healHelpers =
    healPatient && !tableEntries.some((e) => e.id === healPatient.id)
      ? [healPatient, ...tableEntries]
      : tableEntries;
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

  /** Puts a character into the fight (initiative is rolled) and says what was rolled. */
  const addToCombat = async (id: number, name: string) => {
    if (combat?.combatants.some((c) => c.character_id === id)) return;
    await rollInitiative(id, name);
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

  /** Creates a character; with `containerId` it also gets a shortcut in that folder or session. */
  const createCharacter = async (kind: CharacterKind, containerId?: number) => {
    const record = await create(kind, t(`characters.default_name_${kind}`));
    if (containerId !== undefined) await explorer.addTo(containerId, record.id);
    selectCharacter(record.id);
    setTab("characters");
  };

  /**
   * The template a key stands for. A book template the GM has edited resolves to their copy: the
   * copy takes the original's place in new fights (and in encounters that name the original).
   */
  const resolveTemplate = (key: string) => {
    const book = npcTemplates.find((x) => templateKey("book", x.code) === key);
    const edited = book ? encounterState.templates.find((x) => x.based_on === book.code) : undefined;
    const own =
      edited ?? encounterState.templates.find((x) => templateKey("custom", String(x.id)) === key);
    return { book: edited ? undefined : book, own };
  };

  /**
   * Makes `count` fresh NPCs from a template (`book:<code>` / `custom:<id>`): instances of the
   * target session (`containerId`, default the active one), numbered after the existing names, each
   * with its own HP and SP. With `toCombat` they also join the fight (active session only). The
   * toast offers more copies up to `SPAWN_MAX` in total; `already` is how many this burst made.
   */
  const spawnTemplate = async (
    key: string,
    count: number,
    toCombat: boolean,
    containerId?: number,
    already = 0,
  ) => {
    const { book, own } = resolveTemplate(key);
    const target = containerId ?? sessionId;
    if ((!book && !own) || target === null) return;
    const names = cloneNames(book?.name ?? own!.character.name, count, entries.map((e) => e.character.name));
    const created: { id: number; name: string }[] = [];
    try {
      for (const name of names) {
        const record = book
          ? await api.createNpcFromTemplate(book.code, name, npcNotes(t, book), target)
          : await api.createNpcFromUserTemplate(own!.id, name, target);
        created.push({ id: record.id, name });
      }
    } catch (e) {
      status.fail(e, "npc:spawn");
    } finally {
      // Even after a partial failure the copies that exist must show up.
      await reload();
    }
    if (toCombat && target === sessionId) {
      for (const { id, name } of created) await addToCombat(id, name);
    }
    if (created.length === 0) return;
    const total = already + created.length;
    const more = SPAWN_MAX - total;
    status.show({
      id: `npc:spawn:${key}`,
      kind: "success",
      text: t(toCombat ? "templates.added_to_combat" : "templates.added", { names: created.map((c) => c.name).join(", ") }),
      action:
        more > 0
          ? {
              label: t("templates.add_more", { count: more }),
              run: () => void spawnTemplate(key, more, toCombat, containerId, total),
            }
          : undefined,
      autoHideMs: 6000,
    });
  };

  /** Opens a template in the main area; a book one the GM has edited opens as their copy. */
  const openTemplate = (key: string) => {
    const edited = key.startsWith("book:")
      ? encounterState.templates.find((x) => x.based_on === key.slice("book:".length))
      : undefined;
    selectTemplate(edited ? templateKey("custom", String(edited.id)) : key);
  };

  /** Throws the GM's copy away: the book template shows again, as printed. */
  const resetTemplate = async (id: number) => {
    const record = encounterState.templates.find((x) => x.id === id);
    try {
      await encounterState.deleteTemplate(id);
      if (record?.based_on && selectedTemplateKey === templateKey("custom", String(id))) {
        selectTemplate(templateKey("book", record.based_on));
      }
    } catch (e) {
      status.fail(e, "template:reset");
    }
  };

  /** Deletes all instances of a session; "Undo" in the status area brings them all back. */
  const clearInstances = async (session: number) => {
    const toastId = `session:clear:${session}`;
    try {
      const deleted = await api.clearSessionInstances(session);
      if (deleted.length === 0) return;
      if (selectedCharacterId !== null && deleted.some((d) => d.id === selectedCharacterId)) selectCharacter(null);
      await reload();
      sceneState.replaceScene(await api.getScene());
      status.show({
        id: toastId,
        kind: "info",
        text: t("explorer.cleared_npcs", { count: deleted.length }),
        action: {
          label: t("damage.undo"),
          run: () => {
            status.dismiss(toastId);
            void (async () => {
              try {
                for (const d of deleted) await api.restoreCharacter(d);
                await reload();
                sceneState.replaceScene(await api.getScene());
              } catch (e) {
                status.fail(e, "session:clear");
              }
            })();
          },
        },
        autoHideMs: UNDO_DELETE_MS,
      });
    } catch (e) {
      status.fail(e, "session:clear");
    }
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
    const launched: number[] = [];
    try {
      for (const line of encounter.entries) {
        const key = templateKey(line.source, line.template);
        const { book, own } = resolveTemplate(key);
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
          launched.push(record.id);
          created += 1;
        }
      }
    } finally {
      // Even after a partial failure the clones that exist must show up.
      await reload();
    }
    // A launched encounter is the enemy side of a fight: its NPCs go into the initiative order.
    for (const id of launched) await sceneState.combatAdd(id);
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

  /**
   * Deletes a character without asking; the status area offers "Undo" for a few seconds. The
   * backend deleted it already, the snapshot it returned is what brings everything back.
   */
  const deleteCharacter = async (id: number) => {
    const name = entries.find((e) => e.id === id)?.character.name ?? "";
    const toastId = `character:delete:${id}`;
    try {
      const deleted = await remove(id);
      if (selectedCharacterId === id) selectCharacter(null);
      // The backend passed the turn on if the deleted one was active.
      sceneState.replaceScene(await api.getScene());
      status.show({
        id: toastId,
        kind: "info",
        text: t("characters.deleted", { name }),
        action: {
          label: t("damage.undo"),
          run: () => {
            status.dismiss(toastId);
            void undoDelete(deleted);
          },
        },
        autoHideMs: UNDO_DELETE_MS,
      });
    } catch (e) {
      status.fail(e, "character:delete");
    }
  };

  const undoDelete = async (deleted: DeletedCharacter) => {
    try {
      await restore(deleted);
      sceneState.replaceScene(await api.getScene());
      selectCharacter(deleted.id);
      status.show({
        id: `character:delete:${deleted.id}`,
        kind: "success",
        text: t("characters.restored", { name: deleted.name }),
      });
    } catch (e) {
      status.fail(e, "character:restore");
    }
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

  // Every shortcut lives in `lib/hotkeys`; this is what each one does here.
  const stepCharacter = (delta: number) => {
    const list = railEntries.length > 0 ? railEntries : sessionEntries;
    if (list.length === 0) return;
    const at = list.findIndex((e) => e.id === selectedCharacterId);
    const next = list[(at + delta + list.length) % list.length];
    selectCharacter(next.id);
    setTab("characters");
  };
  useHotkeys({
    explorer: toggleExplorerPanel,
    search: () => setSearchOpen(true),
    help: () => setHelpOpen((open) => !open),
    attackRef: () => {
      const { attackRefOpen, setAttackRefOpen } = useUiStore.getState();
      setAttackRefOpen(!attackRefOpen);
    },
    prevTurn: () => combat && void sceneState.combatPrevTurn(),
    nextTurn: () => combat && void sceneState.combatNextTurn(),
    damage: () => {
      // The one whose turn it is; outside a fight, the one open on the sheet.
      const target = combat?.active ?? selectedCharacterId;
      if (target !== null && entries.some((e) => e.id === target)) openDamage(target);
    },
    undoDamage: () => scene?.last_damage && void undoDamage(),
    prevCharacter: () => stepCharacter(-1),
    nextCharacter: () => stepCharacter(1),
    tab1: () => setTab("characters"),
    tab2: () => setTab("encounters"),
    tab3: () => setTab("notes"),
    tab4: () => setTab("rules"),
  });

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
        onRailDrop={(id) => {
          const entry = entries.find((e) => e.id === id);
          if (entry) void addToCombat(id, entry.character.name);
        }}
        onRailDropTemplate={(key) => void spawnTemplate(key, 1, true)}
        explorerOpen={explorerOpen}
        explorerDrawerOpen={explorerDrawerOpen}
        onExplorerDrawerChange={setExplorerDrawerOpen}
        railDrawerOpen={railDrawerOpen}
        onRailDrawerChange={setRailDrawerOpen}
        explorer={
          <Explorer
            nodes={explorer.nodes}
            entries={entries}
            selectedId={selectedCharacterId}
            tab={tab}
            enabled={hasBackend}
            bookTemplates={npcTemplates}
            userTemplates={encounterState.templates}
            fighters={combat?.combatants.map((c) => c.character_id) ?? []}
            actions={{
              openCharacter: (id) => {
                selectCharacter(id);
                setTab("characters");
              },
              openNotes: (id) => {
                if (id !== sessionId) void explorer.switchTo(id);
                setTab("notes");
              },
              openEncounters: () => setTab("encounters"),
              openReference: () => setTab("rules"),
              activateSession: (id) => {
                if (id !== sessionId) void explorer.switchTo(id);
                setTab("characters");
              },
              createSession: explorer.createSession,
              createFolder: explorer.createFolder,
              rename: explorer.rename,
              removeNode: explorer.remove,
              addToContainer: (containerId, characterId) =>
                void explorer.addTo(containerId, characterId),
              removeFromContainer: (containerId, characterId) =>
                void explorer.removeFrom(containerId, characterId),
              addToCombat: (id) => {
                const entry = entries.find((e) => e.id === id);
                if (entry) void addToCombat(id, entry.character.name);
              },
              removeFromCombat: (id) => void sceneState.combatRemove(id),
              newCharacter: (kind, containerId) => void createCharacter(kind, containerId),
              moveFolder: (id, parent) => void explorer.moveFolder(id, parent),
              spawnTemplate: (key, count, toCombat, containerId) =>
                void spawnTemplate(key, count, toCombat, containerId),
              openTemplate,
              resetTemplate: (id) => void resetTemplate(id),
              saveInstanceToLibrary: (id) => void explorer.run(api.saveInstanceToLibrary(id)),
              clearInstances: (id) => void clearInstances(id),
              importCharacters: () => void startImport("character"),
              importSession: () => void startImport("session"),
              exportCharacter: (id) => void exportCharacters({ ids: [id] }),
              exportSession: () => void exportCharacters("session"),
              saveAsTemplate: (id) => void saveAsTemplate(id),
              deleteCharacter: (id) => void deleteCharacter(id),
            }}
          />
        }
        header={
          <>
            <Button
              size="xs"
              variant="outline"
              aria-label={t(explorerShown ? "explorer.hide" : "explorer.show")}
              title={`${t(explorerShown ? "explorer.hide" : "explorer.show")} ${formatHotkey("explorer")}`}
              onClick={toggleExplorerPanel}
            >
              <PanelLeft className="size-4" />
            </Button>
            <AppPath
              segments={[
                tab,
                tab === "characters"
                  ? openedTemplate || openedBook
                    ? (openedTemplate?.character.name ?? openedBook?.name ?? "")
                    : (selected?.character.name ?? "")
                  : "",
              ]}
            />
            <Button size="xs" variant="outline" onClick={() => setSearchOpen(true)}>
              <Search className="size-4" />
              <span className="max-mid:sr-only">
                {t("reference.global_button")} {formatHotkey("search")}
              </span>
            </Button>
            <Button
              size="xs"
              variant="outline"
              className="max-mid:hidden"
              aria-label={t("hotkeys.button")}
              title={`${t("hotkeys.button")} ${formatHotkey("help")}`}
              onClick={() => setHelpOpen(true)}
            >
              <Keyboard className="size-4" />
            </Button>
            <Button
              size="xs"
              variant="outline"
              aria-label={t("about.button")}
              title={t("about.button")}
              onClick={() => setAboutOpen(true)}
            >
              <Info className="size-4" />
            </Button>
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
          <div key={tab} className="h-full animate-fade">
          {tab === "characters" ? (
            <div className="space-y-6">
              {!hasBackend && <p className="text-muted-foreground">{t("characters.no_backend")}</p>}
              {openedTemplate || openedBook ? (
                <TemplateEditor
                  key={openedTemplate?.based_on ? templateKey("book", openedTemplate.based_on) : selectedTemplateKey}
                  record={openedTemplate}
                  book={openedBook}
                  notes={openedBook ? npcNotes(t, openedBook) : ""}
                  injuries={injuries}
                  onUpdate={encounterState.updateTemplate}
                  onFork={async (code, character) => {
                    const copy = await encounterState.saveBookCopy(code, character);
                    selectTemplate(templateKey("custom", String(copy.id)));
                  }}
                />
              ) : shown ? (
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
                  onAdminister={(drug) => setDrugRequest({ helperId: shown.id, drug })}
                  onLoseHumanity={() => setHumanityLossTargetId(shown.id)}
                  onTherapy={() => setTherapyTargetId(shown.id)}
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
            explorer.active ? (
              <SessionNotes
                key={explorer.active.id}
                sessionId={explorer.active.id}
                sessionName={explorer.active.name}
              />
            ) : (
              <p className="text-muted-foreground">{t("notes.no_session")}</p>
            )
          ) : (
            <ReferencePage />
          )}
          </div>
        }
        dock={
          <CombatDock
            combat={combat}
            active={railShown.find((e) => e.id === combat?.active) ?? null}
            hasCharacters={sessionEntries.length > 0 || fighters.length > 0}
            onExpand={() => setRailDrawerOpen(true)}
            onStart={() => void sceneState.combatStart()}
            onNextTurn={() => void sceneState.combatNextTurn()}
            onDamage={openDamage}
          />
        }
        rail={
          <>
            {combat && (
              <CombatBar
                combat={combat}
                hasCharacters={sessionEntries.length > 0 || fighters.length > 0}
                lastDamage={scene?.last_damage ?? null}
                onStart={() => void sceneState.combatStart()}
                onPrevTurn={() => void sceneState.combatPrevTurn()}
                onNextTurn={() => void sceneState.combatNextTurn()}
                onEnd={() => void sceneState.combatEnd()}
                onUndo={() => void undoDamage()}
              />
            )}
            <SortableRail ids={railShown.map((e) => e.id)} onMove={moveFighter}>
            {railShown.map((e) => {
              const fighter = combat?.combatants.find((c) => c.character_id === e.id);
              const index = fighter && combat ? combat.combatants.indexOf(fighter) : -1;
              const cover = coverOf(e.id);
              return (
                <SortableCard key={e.id} id={e.id}>
                <CharacterCard
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
                  onSetMark={(change) =>
                    void save(e.id, { ...e.character, ...change }).catch((err) =>
                      status.fail(err, "character:mark", { text: errorMessage(err) }),
                    )
                  }
                  onDamage={() => openDamage(e.id)}
                  onSetInitiative={(value) => void sceneState.combatSetInitiative(e.id, value)}
                  initiativeRoll={fighter?.roll ?? null}
                  initiativeBonus={fighter?.bonus ?? 0}
                  onRollInitiative={() => void rollInitiative(e.id, e.character.name)}
                  onShift={(delta) => void sceneState.combatShift(e.id, delta)}
                  onStabilize={() => setStabilizeTargetId(e.id)}
                  onRest={() => setRestTargetId(e.id)}
                  onLoseHumanity={() => setHumanityLossTargetId(e.id)}
                  onTherapy={() => setTherapyTargetId(e.id)}
                  inSession={members.includes(e.id)}
                  onLeaveSession={() => sessionId !== null && void explorer.removeFrom(sessionId, e.id)}
                  onKeepInLibrary={
                    explorer.active?.instances.includes(e.id)
                      ? () => void explorer.run(api.saveInstanceToLibrary(e.id))
                      : undefined
                  }
                  onExport={() => void exportCharacters({ ids: [e.id] })}
                  onSaveAsTemplate={() => void saveAsTemplate(e.id)}
                  onDelete={() => void deleteCharacter(e.id)}
                  onRevive={() =>
                    api.reviveCharacter(e.id).then(
                      () => reload(),
                      (err: unknown) => status.fail(err, "revive"),
                    )
                  }
                  onDeathSave={(manual) => deathSave(e.id, manual)}
                  onLeave={() => void sceneState.combatRemove(e.id)}
                />
                </SortableCard>
              );
            })}
            </SortableRail>
            {railEntries.length === 0 && explorer.active && (
              <p className="p-2 text-sm text-muted-foreground">{t("rail.empty")}</p>
            )}
            <ReferenceSearch open={searchOpen} onOpenChange={setSearchOpen} />
            <ShortcutsDialog open={helpOpen} onOpenChange={setHelpOpen} />
            <AboutDialog open={aboutOpen} onOpenChange={setAboutOpen} />
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
              helpers={tableEntries}
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
