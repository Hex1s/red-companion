import { ChevronRight, ChevronsDownUp, Plus } from "@/components/ui/icons";
import { useState, type DragEvent, type ReactNode } from "react";
import { useTranslation } from "react-i18next";
import type { CharacterKind } from "@/bindings/CharacterKind";
import type { ExplorerNode } from "@/bindings/ExplorerNode";
import type { NpcTemplate } from "@/bindings/NpcTemplate";
import type { NpcTier } from "@/bindings/NpcTier";
import type { UserTemplateRecord } from "@/bindings/UserTemplateRecord";
import {
  ContextEntries,
  MoreMenu,
  item,
  separator,
  tidy,
  type MenuEntry,
} from "@/components/explorer/node-menu";
import { ContextMenu, ContextMenuContent, ContextMenuTrigger } from "@/components/ui/context-menu";
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
import type { CharacterEntry } from "@/hooks/use-characters";
import {
  CHARACTER_DRAG_TYPE,
  FOLDER_DRAG_TYPE,
  TEMPLATE_DRAG_TYPE,
  draggedCharacter,
  draggedFolder,
  draggedTemplate,
} from "@/lib/drag";
import { templateKey } from "@/lib/encounter";
import { cn } from "@/lib/utils";
import { useUiStore, type AppTab } from "@/store/ui-store";

const NAME_MAX = 60;
const TIERS: NpcTier[] = ["mook", "lieutenant", "mini_boss", "boss"];

export type ExplorerActions = {
  openCharacter: (id: number) => void;
  openNotes: (sessionId: number) => void;
  openEncounters: () => void;
  openReference: () => void;
  activateSession: (id: number) => void;
  createSession: (name: string) => Promise<boolean>;
  createFolder: (parent: number | null, name: string) => Promise<boolean>;
  rename: (id: number, name: string) => Promise<boolean>;
  removeNode: (id: number) => Promise<boolean>;
  addToContainer: (containerId: number, characterId: number) => void;
  addToCombat: (characterId: number) => void;
  removeFromCombat: (characterId: number) => void;
  removeFromContainer: (containerId: number, characterId: number) => void;
  /** A new character, and a shortcut to it in `containerId` when given. */
  newCharacter: (kind: CharacterKind, containerId?: number) => void;
  moveFolder: (id: number, parent: number | null) => void;
  /** Opens a template in the main area: an own one for editing, a book one in the reference. */
  openTemplate: (key: string) => void;
  /** Deletes the GM's copy of a book template: the book one shows again. */
  resetTemplate: (id: number) => void;
  /** Keeps an instance: it becomes a library character and stays after its session is deleted. */
  saveInstanceToLibrary: (id: number) => void;
  /** Deletes every instance of a session (the app offers "Undo" afterwards). */
  clearInstances: (sessionId: number) => void;
  /**
   * A fresh NPC copy per `count` from a template (`book:<code>` / `custom:<id>`), numbered after the
   * existing ones; into `containerId` (default: the active session), optionally into the fight.
   */
  spawnTemplate: (key: string, count: number, toCombat: boolean, containerId?: number) => void;
  importCharacters: () => void;
  importSession: () => void;
  exportCharacter: (id: number) => void;
  exportSession: () => void;
  saveAsTemplate: (id: number) => void;
  /** Deletes the character itself (not a shortcut); the app offers "Undo" afterwards. */
  deleteCharacter: (id: number) => void;
};

type ExplorerProps = {
  nodes: ExplorerNode[];
  entries: CharacterEntry[];
  selectedId: number | null;
  tab: AppTab;
  /** Commands need the Rust backend; without it (plain browser) the menus offer nothing. */
  enabled: boolean;
  /** Ready-made NPCs from the book and the GM's own templates: the library keeps them as references. */
  bookTemplates: NpcTemplate[];
  userTemplates: UserTemplateRecord[];
  /** Ids in the initiative order: who fights is separate from who lies in the session. */
  fighters: number[];
  actions: ExplorerActions;
};

/** What the name dialog is doing right now. */
type NameDialog =
  | { kind: "create-session"; initial: string }
  | { kind: "create-folder"; parent: number | null; initial: string }
  | { kind: "rename"; node: ExplorerNode };

/** Left panel: sessions, folders and characters as a tree, the way a file manager shows them. */
export function Explorer({
  nodes,
  entries,
  selectedId,
  tab,
  enabled,
  bookTemplates,
  userTemplates,
  fighters,
  actions,
}: ExplorerProps) {
  const { t } = useTranslation();
  const expanded = useUiStore((s) => s.expanded);
  const setExpanded = useUiStore((s) => s.setExpanded);
  const selectedTemplateKey = useUiStore((s) => s.selectedTemplateKey);
  const [nameDialog, setNameDialog] = useState<NameDialog | null>(null);
  const [deleting, setDeleting] = useState<ExplorerNode | null>(null);
  const [dropTarget, setDropTarget] = useState<number | null>(null);
  const [resetting, setResetting] = useState<UserTemplateRecord | null>(null);

  const sessions = nodes.filter((n) => n.kind === "session");
  const rootFolders = nodes.filter((n) => n.kind === "folder" && n.parent_id === null);
  const folderCount = nodes.filter((n) => n.kind === "folder").length;
  const active = sessions.find((s) => s.active) ?? null;
  const byId = new Map(entries.map((e) => [e.id, e]));
  // Temporary copies from templates: they live in their session only, not in the library lists.
  const instanceIds = new Set(nodes.flatMap((n) => n.instances));

  const isOpen = (key: string, fallback: boolean) => expanded[key] ?? fallback;
  const toggle = (key: string, fallback: boolean) => setExpanded(key, !isOpen(key, fallback));

  const characterEntries = (ids: number[]) =>
    ids.flatMap((id) => {
      const entry = byId.get(id);
      return entry ? [entry] : [];
    });

  const askCreateSession = () =>
    setNameDialog({
      kind: "create-session",
      initial: t("sessions.default_name", { n: sessions.length + 1 }),
    });
  const askCreateFolder = (parent: number | null) =>
    setNameDialog({
      kind: "create-folder",
      parent,
      initial: t("explorer.folder_default_name", { n: folderCount + 1 }),
    });

  const sessionsAdd: MenuEntry[] = [item("new", t("sessions.create_title"), askCreateSession)];
  const libraryAdd: MenuEntry[] = tidy([
    item("new-pc", t("characters.new_pc"), () => actions.newCharacter("pc")),
    item("new-npc", t("characters.new_npc"), () => actions.newCharacter("npc")),
    separator("s1"),
    item("folder", t("explorer.new_folder"), () => askCreateFolder(null)),
    separator("s2"),
    item("import", t("characters.import"), actions.importCharacters),
    item("import-session", t("characters.import_session"), actions.importSession),
  ]);

  const sessionMenu = (node: ExplorerNode): MenuEntry[] =>
    tidy([
      item("activate", t("explorer.activate"), () => actions.activateSession(node.id), {
        disabled: node.active,
      }),
      item("notes", t("nav.notes"), () => actions.openNotes(node.id)),
      item("clear-npcs", t("explorer.clear_npcs"), () => actions.clearInstances(node.id), {
        disabled: node.instances.length === 0,
      }),
      separator("s1"),
      item("rename", t("sessions.rename"), () => setNameDialog({ kind: "rename", node })),
      item("export", t("explorer.export_session"), actions.exportSession, { disabled: !node.active }),
      separator("s2"),
      item("delete", t("sessions.delete"), () => setDeleting(node), { destructive: true }),
    ]);

  const folderMenu = (node: ExplorerNode): MenuEntry[] =>
    tidy([
      item("subfolder", t("explorer.new_subfolder"), () => askCreateFolder(node.id), {
        disabled: node.parent_id !== null,
      }),
      item("new-pc", t("characters.new_pc"), () => actions.newCharacter("pc", node.id)),
      item("new-npc", t("characters.new_npc"), () => actions.newCharacter("npc", node.id)),
      separator("s1"),
      item("rename", t("sessions.rename"), () => setNameDialog({ kind: "rename", node })),
      ...(node.parent_id !== null
        ? [item("to-root", t("explorer.to_top_level"), () => actions.moveFolder(node.id, null))]
        : []),
      separator("s2"),
      item("delete", t("sessions.delete"), () => setDeleting(node), { destructive: true }),
    ]);

  const characterMenu = (entry: CharacterEntry, container: ExplorerNode | null): MenuEntry[] => {
    if (instanceIds.has(entry.id)) {
      return tidy([
        item("open", t("explorer.open"), () => actions.openCharacter(entry.id)),
        fighters.includes(entry.id)
          ? item("combat", t("initiative.remove_from_fight"), () => actions.removeFromCombat(entry.id))
          : item("combat", t("initiative.add"), () => actions.addToCombat(entry.id)),
        item("keep", t("explorer.keep_in_library"), () => actions.saveInstanceToLibrary(entry.id)),
        separator("s1"),
        item("template", t("encounters.save_as_template"), () => actions.saveAsTemplate(entry.id)),
        item("export", t("characters.export_selected"), () => actions.exportCharacter(entry.id)),
        separator("s2"),
        item("delete", t("characters.delete"), () => actions.deleteCharacter(entry.id), { destructive: true }),
      ]);
    }
    return tidy([
      item("open", t("explorer.open"), () => actions.openCharacter(entry.id)),
      item(
        "to-active",
        t("explorer.to_active_session", { name: active?.name ?? "" }),
        () => active && actions.addToContainer(active.id, entry.id),
        { disabled: !active || active.members.includes(entry.id) },
      ),
      fighters.includes(entry.id)
        ? item("combat", t("initiative.remove_from_fight"), () => actions.removeFromCombat(entry.id))
        : item("combat", t("initiative.add"), () => actions.addToCombat(entry.id)),
      ...(container
        ? [
            item(
              "remove-here",
              t(container.kind === "session" ? "sessions.leave" : "explorer.remove_from_folder"),
              () => actions.removeFromContainer(container.id, entry.id),
            ),
          ]
        : []),
      separator("s1"),
      item("export", t("characters.export_selected"), () => actions.exportCharacter(entry.id)),
      ...(entry.kind === "npc"
        ? [item("template", t("encounters.save_as_template"), () => actions.saveAsTemplate(entry.id))]
        : []),
      separator("s2"),
      item("delete", t("characters.delete"), () => actions.deleteCharacter(entry.id), { destructive: true }),
    ]);
  };

  const templateMenu = (key: string, edited?: UserTemplateRecord): MenuEntry[] => tidy([
    item("to-combat", t("templates.to_combat"), () => actions.spawnTemplate(key, 1, true), { disabled: !active }),
    item("to-session", t("templates.to_session"), () => actions.spawnTemplate(key, 1, false), { disabled: !active }),
    separator("s1"),
    item("open", t("explorer.open"), () => actions.openTemplate(key)),
    ...(edited ? [item("reset", t("templates.reset"), () => setResetting(edited))] : []),
  ]);

  const templateRow = (key: string, name: string, edited?: UserTemplateRecord) => (
    <TreeRow
      key={key}
      depth={2}
      label={name}
      tag={edited ? t("templates.edited") : undefined}
      menu={enabled ? templateMenu(key, edited) : []}
      selected={tab === "characters" && selectedTemplateKey === key}
      onClick={() => actions.openTemplate(key)}
      dragTemplate={key}
    />
  );

  // A book template the GM edited is shown as their copy; the original is hidden until it is reset.
  const bookCodes = new Set(bookTemplates.map((x) => x.code));
  const editedOf = (code: string) => userTemplates.find((x) => x.based_on === code);
  const ownTemplates = userTemplates.filter((x) => !x.based_on || !bookCodes.has(x.based_on));

  const tierGroup = (tier: NpcTier) => {
    const list = bookTemplates.filter((x) => x.tier === tier);
    if (list.length === 0) return null;
    const groupKey = `templates:${tier}`;
    const open = isOpen(groupKey, false);
    return (
      <div key={groupKey}>
        <TreeRow
          depth={1}
          label={t(`npc.tier.${tier}`)}
          count={list.length}
          open={open}
          onToggle={() => toggle(groupKey, false)}
          onClick={() => toggle(groupKey, false)}
        />
        {open && (
          <div className="animate-in fade-in-0 slide-in-from-top-1 duration-150">
            {list.map((x) => {
              const copy = editedOf(x.code);
              return copy
                ? templateRow(templateKey("custom", String(copy.id)), copy.character.name, copy)
                : templateRow(templateKey("book", x.code), x.name);
            })}
          </div>
        )}
      </div>
    );
  };

  const ownGroup = () => {
    const groupKey = "templates:own";
    const open = isOpen(groupKey, true);
    return (
      <div key={groupKey}>
        <TreeRow
          depth={1}
          label={t("templates.own")}
          count={ownTemplates.length}
          open={open}
          onToggle={() => toggle(groupKey, true)}
          onClick={() => toggle(groupKey, true)}
        />
        {open &&
          (ownTemplates.length === 0 ? (
            <p className="py-1 text-xs text-muted-foreground" style={{ paddingLeft: indent(2) }}>
              {t("templates.own_empty")}
            </p>
          ) : (
            <div className="animate-in fade-in-0 slide-in-from-top-1 duration-150">
              {ownTemplates.map((x) => templateRow(templateKey("custom", String(x.id)), x.character.name))}
            </div>
          ))}
      </div>
    );
  };

  // Dropping a character on a session or a folder puts a shortcut there.
  const dropProps = (containerId: number, accepts: "any" | "characters" = "any") => ({
    onDragOver: (e: DragEvent) => {
      const types = e.dataTransfer.types;
      const template = accepts === "characters" && types.includes(TEMPLATE_DRAG_TYPE);
      if (!template && !types.includes(CHARACTER_DRAG_TYPE) && !(accepts === "any" && types.includes(FOLDER_DRAG_TYPE))) return;
      e.preventDefault();
      e.dataTransfer.dropEffect = "copy";
      setDropTarget(containerId);
    },
    onDragLeave: () => setDropTarget((current) => (current === containerId ? null : current)),
    onDrop: (e: DragEvent) => {
      const id = draggedCharacter(e);
      const folder = accepts === "any" ? draggedFolder(e) : null;
      const template = accepts === "characters" ? draggedTemplate(e) : null;
      setDropTarget(null);
      if (template !== null) {
        e.preventDefault();
        actions.spawnTemplate(template, 1, false, containerId);
      } else if (id !== null) {
        e.preventDefault();
        actions.addToContainer(containerId, id);
      } else if (folder !== null) {
        e.preventDefault();
        actions.moveFolder(folder, containerId);
      }
    },
  });

  const characterRow = (entry: CharacterEntry, depth: number, container: ExplorerNode | null) => (
    <TreeRow
      key={`${container?.id ?? "all"}-${entry.id}`}
      depth={depth}
      label={entry.character.name}
      color={entry.character.color}
      tag={t(instanceIds.has(entry.id) ? "explorer.kind.instance" : `explorer.kind.${entry.kind}`)}
      selected={tab === "characters" && selectedId === entry.id}
      menu={characterMenu(entry, container)}
      onClick={() => actions.openCharacter(entry.id)}
      dragCharacter={entry.id}
    />
  );

  const containerChildren = (node: ExplorerNode, depth: number) => {
    const members = characterEntries(node.members);
    return (
      <>
        {nodes
          .filter((child) => child.parent_id === node.id)
          .map((child) => folderRow(child, depth))}
        {members.map((entry) => characterRow(entry, depth, node))}
        {node.kind === "session" && (
          <TreeRow
            depth={depth}
            label={t("nav.notes")}
            muted
            selected={tab === "notes" && node.active}
            onClick={() => actions.openNotes(node.id)}
          />
        )}
        {members.length === 0 && node.kind === "folder" && (
          <p className="py-1 text-xs text-muted-foreground" style={{ paddingLeft: indent(depth) }}>
            {t("explorer.empty_container")}
          </p>
        )}
      </>
    );
  };

  const folderRow = (node: ExplorerNode, depth: number) => {
    const key = `folder:${node.id}`;
    const open = isOpen(key, false);
    return (
      <div key={key}>
        <TreeRow
          depth={depth}
          label={node.name}
          count={node.members.length}
          open={open}
          onToggle={() => toggle(key, false)}
          menu={folderMenu(node)}
          highlighted={dropTarget === node.id}
          onClick={() => toggle(key, false)}
          dragFolder={node.id}
          drop={dropProps(node.id)}
        />
        {open && <div className="animate-in fade-in-0 slide-in-from-top-1 duration-150">{containerChildren(node, depth + 1)}</div>}
      </div>
    );
  };

  const sessionRow = (node: ExplorerNode) => {
    const key = `session:${node.id}`;
    const open = isOpen(key, node.active);
    return (
      <div key={key}>
        <TreeRow
          depth={1}
          label={node.name}
          count={node.members.length}
          open={open}
          marked={node.active}
          selected={node.active && tab === "characters" && selectedId === null}
          onToggle={() => toggle(key, node.active)}
          menu={sessionMenu(node)}
          highlighted={dropTarget === node.id}
          onClick={() => {
            actions.activateSession(node.id);
            if (!open) setExpanded(key, true);
          }}
          drop={dropProps(node.id, "characters")}
        />
        {open && <div className="animate-in fade-in-0 slide-in-from-top-1 duration-150">{containerChildren(node, 2)}</div>}
      </div>
    );
  };

  const allOpen = isOpen("all", false);
  const libraryEntries = entries.filter((e) => !instanceIds.has(e.id));

  // Open nodes that are on screen right now, with their nesting level (sections are level 0 and stay).
  const visibleOpen = (): { key: string; depth: number }[] => {
    const found: { key: string; depth: number }[] = [];
    const walkFolder = (node: ExplorerNode, depth: number) => {
      const key = `folder:${node.id}`;
      if (!isOpen(key, false)) return;
      found.push({ key, depth });
      nodes.filter((child) => child.parent_id === node.id).forEach((child) => walkFolder(child, depth + 1));
    };
    if (isOpen("sessions", true)) {
      for (const session of sessions) {
        const key = `session:${session.id}`;
        if (!isOpen(key, session.active)) continue;
        found.push({ key, depth: 1 });
        nodes.filter((child) => child.parent_id === session.id).forEach((child) => walkFolder(child, 2));
      }
    }
    if (isOpen("library", true)) {
      rootFolders.forEach((folder) => walkFolder(folder, 1));
      if (allOpen) found.push({ key: "all", depth: 1 });
    }
    if (isOpen("templates", true)) {
      if (isOpen("templates:own", true)) found.push({ key: "templates:own", depth: 1 });
      for (const tier of TIERS) {
        const key = `templates:${tier}`;
        if (bookTemplates.some((x) => x.tier === tier) && isOpen(key, false)) found.push({ key, depth: 1 });
      }
    }
    return found;
  };
  const openNodes = visibleOpen();
  const collapseDeepest = () => {
    const deepest = Math.max(...openNodes.map((n) => n.depth));
    openNodes.filter((n) => n.depth === deepest).forEach((n) => setExpanded(n.key, false));
  };

  return (
    <nav aria-label={t("explorer.title")} className="space-y-3">
      <Section
        label={t("explorer.tools")}
        open
        onToggle={() => {}}
        add={[]}
        plain
        action={
          <Button
            size="icon-xs"
            variant="ghost"
            aria-label={t("explorer.collapse_level")}
            title={t("explorer.collapse_level")}
            disabled={openNodes.length === 0}
            onClick={collapseDeepest}
          >
            <ChevronsDownUp className="size-4" />
          </Button>
        }
      >
        <TreeRow
          depth={1}
          label={t("nav.encounters")}
          selected={tab === "encounters"}
          onClick={actions.openEncounters}
        />
        <TreeRow
          depth={1}
          label={t("nav.rules")}
          selected={tab === "rules"}
          onClick={actions.openReference}
        />
      </Section>

      <Section
        label={t("explorer.sessions")}
        open={isOpen("sessions", true)}
        onToggle={() => toggle("sessions", true)}
        add={enabled ? sessionsAdd : []}
      >
        {sessions.map(sessionRow)}
      </Section>

      <Section
        label={t("explorer.library")}
        open={isOpen("library", true)}
        onToggle={() => toggle("library", true)}
        add={enabled ? libraryAdd : []}
        onDropFolder={(id) => actions.moveFolder(id, null)}
      >
        {rootFolders.map((folder) => folderRow(folder, 1))}
        <TreeRow
          depth={1}
          label={t("sessions.all_characters")}
          count={libraryEntries.length}
          open={allOpen}
          onToggle={() => toggle("all", false)}
          onClick={() => toggle("all", false)}
        />
        {allOpen && libraryEntries.map((entry) => characterRow(entry, 2, null))}
      </Section>

      <Section
        label={t("templates.title")}
        open={isOpen("templates", true)}
        onToggle={() => toggle("templates", true)}
        add={[]}
      >
        {ownGroup()}
        {TIERS.map(tierGroup)}
      </Section>

      <NameDialogView
        dialog={nameDialog}
        onClose={() => setNameDialog(null)}
        onSubmit={async (name) => {
          if (!nameDialog) return false;
          if (nameDialog.kind === "create-session") return actions.createSession(name);
          if (nameDialog.kind === "create-folder") {
            const ok = await actions.createFolder(nameDialog.parent, name);
            if (ok && nameDialog.parent !== null) setExpanded(`folder:${nameDialog.parent}`, true);
            return ok;
          }
          return actions.rename(nameDialog.node.id, name);
        }}
      />

      <Dialog open={resetting !== null} onOpenChange={(o) => !o && setResetting(null)}>
        <DialogContent>
          <DialogHeader>
            <DialogTitle>{t("templates.reset_title")}</DialogTitle>
            <DialogDescription>
              {t("templates.reset_body", { name: resetting?.character.name ?? "" })}
            </DialogDescription>
          </DialogHeader>
          <DialogFooter>
            <Button type="button" variant="outline" onClick={() => setResetting(null)}>
              {t("characters.cancel")}
            </Button>
            <Button
              type="button"
              onClick={() => {
                if (resetting) actions.resetTemplate(resetting.id);
                setResetting(null);
              }}
            >
              {t("templates.reset")}
            </Button>
          </DialogFooter>
        </DialogContent>
      </Dialog>

      <Dialog open={deleting !== null} onOpenChange={(o) => !o && setDeleting(null)}>
        <DialogContent>
          <DialogHeader>
            <DialogTitle>
              {t(deleting?.kind === "session" ? "sessions.delete_title" : "explorer.delete_folder_title")}
            </DialogTitle>
            <DialogDescription>
              {t(deleting?.kind === "session" ? "sessions.delete_body" : "explorer.delete_folder_body", {
                name: deleting?.name ?? "",
              })}
            </DialogDescription>
          </DialogHeader>
          <DialogFooter>
            <Button type="button" variant="outline" onClick={() => setDeleting(null)}>
              {t("characters.cancel")}
            </Button>
            <Button
              type="button"
              onClick={async () => {
                if (deleting && (await actions.removeNode(deleting.id))) setDeleting(null);
              }}
            >
              {t("sessions.delete")}
            </Button>
          </DialogFooter>
        </DialogContent>
      </Dialog>
    </nav>
  );
}

const indent = (depth: number) => `${depth * 14 + 6}px`;

type SectionProps = {
  label: string;
  open: boolean;
  onToggle: () => void;
  add: MenuEntry[];
  /** A heading without a collapse arrow. */
  plain?: boolean;
  /** Extra control at the right end of the heading. */
  action?: ReactNode;
  /** A folder dropped on the heading goes back to the root of the library. */
  onDropFolder?: (id: number) => void;
  children: ReactNode;
};

function Section({ label, open, onToggle, add, plain, action, onDropFolder, children }: SectionProps) {
  return (
    <section
      onDragOver={(e) => {
        if (onDropFolder && e.dataTransfer.types.includes(FOLDER_DRAG_TYPE)) e.preventDefault();
      }}
      onDrop={(e) => {
        const id = onDropFolder ? draggedFolder(e) : null;
        if (id !== null) {
          e.preventDefault();
          onDropFolder?.(id);
        }
      }}
    >
      <div className="group flex items-center pr-1">
        <button
          type="button"
          onClick={onToggle}
          disabled={plain}
          className="flex min-w-0 flex-1 items-center gap-1 px-1.5 py-1 touch:py-3 text-left font-display text-xs font-semibold uppercase tracking-wider text-muted-foreground outline-none hover:text-foreground disabled:pointer-events-none"
        >
          {!plain && <ChevronRight className={cn("size-3.5 transition-transform duration-150", open && "rotate-90")} />}
          <span className="truncate">{label}</span>
        </button>
        {action}
        <MoreMenu entries={add} alwaysVisible icon={<Plus className="size-4" />} />
      </div>
      {open && <div className="animate-in fade-in-0 slide-in-from-top-1 duration-150">{children}</div>}
    </section>
  );
}

type TreeRowProps = {
  depth: number;
  label: string;
  /** Small kind tag after the name (PC / NPC). */
  tag?: string;
  /** The GM's colour label of a character (`#rrggbb`); a dot before the name. */
  color?: string | null;
  /** How many shortcuts a container holds. */
  count?: number;
  /** Set for a node that has children: the arrow shows its state. */
  open?: boolean;
  onToggle?: () => void;
  selected?: boolean;
  /** The working session. */
  marked?: boolean;
  muted?: boolean;
  /** A character is being dragged over this container. */
  highlighted?: boolean;
  menu?: MenuEntry[];
  onClick?: () => void;
  dragCharacter?: number;
  dragFolder?: number;
  /** Key of the NPC template this row stands for; dragging copies it into a session or the fight. */
  dragTemplate?: string;
  drop?: {
    onDragOver: (e: DragEvent) => void;
    onDragLeave: () => void;
    onDrop: (e: DragEvent) => void;
  };
};

function TreeRow({
  depth,
  label,
  tag,
  color,
  count,
  open,
  onToggle,
  selected,
  marked,
  muted,
  highlighted,
  menu = [],
  onClick,
  dragCharacter,
  dragFolder,
  dragTemplate,
  drop,
}: TreeRowProps) {
  const expandable = open !== undefined;
  const row = (
    <div
      className={cn(
        "group flex h-7 touch:h-11 items-center border-l-2 border-transparent pr-1 text-sm",
        selected ? "border-primary bg-accent text-foreground" : "hover:bg-accent/50",
        highlighted && "border-primary bg-primary/20",
      )}
      draggable={dragCharacter !== undefined || dragFolder !== undefined || dragTemplate !== undefined}
      onDragStart={(e) => {
        if (dragTemplate !== undefined) {
          e.dataTransfer.setData(TEMPLATE_DRAG_TYPE, dragTemplate);
          e.dataTransfer.effectAllowed = "copy";
        } else if (dragCharacter !== undefined) {
          e.dataTransfer.setData(CHARACTER_DRAG_TYPE, String(dragCharacter));
          e.dataTransfer.effectAllowed = "copy";
        } else if (dragFolder !== undefined) {
          e.dataTransfer.setData(FOLDER_DRAG_TYPE, String(dragFolder));
          e.dataTransfer.effectAllowed = "move";
        }
      }}
      {...drop}
    >
      <span style={{ width: indent(depth) }} className="shrink-0" />
      {expandable ? (
        <button
          type="button"
          tabIndex={-1}
          aria-hidden
          onClick={onToggle}
          className="flex size-4 touch:size-8 shrink-0 items-center justify-center text-muted-foreground"
        >
          <ChevronRight className={cn("size-3.5 transition-transform duration-150", open && "rotate-90")} />
        </button>
      ) : (
        <span className="size-4 shrink-0" />
      )}
      {marked && <span aria-hidden className="mr-1.5 size-2 shrink-0 bg-primary" />}
      {color && (
        <span
          aria-hidden
          style={{ "--mark": color } as React.CSSProperties}
          className="mr-1.5 size-2 shrink-0 bg-(--mark)"
        />
      )}
      <button
        type="button"
        onClick={onClick}
        className={cn(
          "min-w-0 flex-1 truncate text-left outline-none focus-visible:underline",
          muted && "text-muted-foreground",
        )}
      >
        {label}
      </button>
      {tag && <span className="ml-1 shrink-0 text-[10px] uppercase text-muted-foreground">{tag}</span>}
      {count !== undefined && (
        <span className="num ml-1 shrink-0 text-xs text-muted-foreground">{count}</span>
      )}
      <MoreMenu entries={menu} />
    </div>
  );
  if (menu.length === 0) return row;
  return (
    <ContextMenu>
      <ContextMenuTrigger asChild>{row}</ContextMenuTrigger>
      <ContextMenuContent>
        <ContextEntries entries={menu} />
      </ContextMenuContent>
    </ContextMenu>
  );
}

type NameDialogViewProps = {
  dialog: NameDialog | null;
  onClose: () => void;
  /** True when the command worked; on a failure the dialog stays so the name can be fixed. */
  onSubmit: (name: string) => Promise<boolean>;
};

function NameDialogView({ dialog, onClose, onSubmit }: NameDialogViewProps) {
  return (
    <Dialog open={dialog !== null} onOpenChange={(o) => !o && onClose()}>
      {dialog && <NameDialogBody key={dialogKey(dialog)} dialog={dialog} onClose={onClose} onSubmit={onSubmit} />}
    </Dialog>
  );
}

const dialogKey = (dialog: NameDialog) =>
  dialog.kind === "rename" ? `rename:${dialog.node.id}` : dialog.kind;

function NameDialogBody({
  dialog,
  onClose,
  onSubmit,
}: {
  dialog: NameDialog;
  onClose: () => void;
  onSubmit: (name: string) => Promise<boolean>;
}) {
  const { t } = useTranslation();
  const [name, setName] = useState(dialog.kind === "rename" ? dialog.node.name : dialog.initial);
  const trimmed = name.trim();
  const valid = trimmed.length >= 1 && trimmed.length <= NAME_MAX;
  const isSession =
    dialog.kind === "create-session" || (dialog.kind === "rename" && dialog.node.kind === "session");
  const title = t(
    dialog.kind === "create-session"
      ? "sessions.create_title"
      : dialog.kind === "create-folder"
        ? "explorer.create_folder_title"
        : isSession
          ? "sessions.rename_title"
          : "explorer.rename_folder_title",
  );

  const submit = async () => {
    if (await onSubmit(trimmed)) onClose();
  };

  return (
    <DialogContent>
      <DialogHeader>
        <DialogTitle>{title}</DialogTitle>
      </DialogHeader>
      <Input
        autoFocus
        value={name}
        maxLength={NAME_MAX}
        aria-label={t(isSession ? "sessions.name" : "explorer.folder_name")}
        onChange={(e) => setName(e.target.value)}
        onKeyDown={(e) => e.key === "Enter" && valid && void submit()}
      />
      <DialogFooter>
        <Button type="button" variant="outline" onClick={onClose}>
          {t("characters.cancel")}
        </Button>
        <Button type="button" disabled={!valid} onClick={() => void submit()}>
          {t(dialog.kind === "rename" ? "sessions.save" : "sessions.create")}
        </Button>
      </DialogFooter>
    </DialogContent>
  );
}
