import { useCallback, useEffect, useState } from "react";
import { useTranslation } from "react-i18next";
import type { ExplorerNode } from "@/bindings/ExplorerNode";
import { api, hasBackend } from "@/lib/api";
import { status } from "@/store/status-store";

/**
 * The explorer tree: sessions and folders with the characters lying in them. The first session
 * is created here, so that its name follows the interface language. The active session is the
 * one the app works with (its scene, fight and members).
 */
export function useExplorer() {
  const { t } = useTranslation();
  const [nodes, setNodes] = useState<ExplorerNode[]>([]);
  const [ready, setReady] = useState(false);

  // No session at all (first run, or the last one was deleted) gets a fresh one at once.
  const withFallback = useCallback(
    async (list: ExplorerNode[]) => {
      if (list.some((n) => n.kind === "session")) return list;
      await api.createSession(t("sessions.default_name", { n: 1 }));
      return api.explorerTree();
    },
    [t],
  );

  const reload = useCallback(async () => {
    if (!hasBackend) return;
    try {
      setNodes(await withFallback(await api.explorerTree()));
    } catch (e) {
      status.fail(e, "explorer");
    }
  }, [withFallback]);

  useEffect(() => {
    if (!hasBackend) return;
    api
      .explorerTree()
      .then((list) => withFallback(list))
      .then(setNodes)
      .catch((e: unknown) => status.fail(e, "explorer"))
      .finally(() => setReady(true));
    // Only at startup: the language of the first session's name is the one at that moment.
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, []);

  /** Runs a tree command; the backend answers with the updated tree. */
  const run = useCallback(
    async (command: Promise<unknown>, reloadAfter = false) => {
      try {
        const answer = await command;
        const list = reloadAfter || !Array.isArray(answer) ? await api.explorerTree() : (answer as ExplorerNode[]);
        setNodes(await withFallback(list));
        return true;
      } catch (e) {
        status.fail(e, "explorer");
        return false;
      }
    },
    [withFallback],
  );

  const active = nodes.find((n) => n.kind === "session" && n.active) ?? null;

  return {
    nodes,
    active,
    /** False until the first load has finished: nothing session-related may be asked before. */
    ready,
    reload,
    /** Runs a tree command and shows the tree it answers with. */
    run,
    createSession: (name: string) => run(api.createSession(name), true),
    createFolder: (parent: number | null, name: string) => run(api.createFolder(parent, name)),
    rename: (id: number, name: string) => run(api.renameNode(id, name)),
    remove: (id: number) => run(api.deleteNode(id)),
    moveFolder: (id: number, parent: number | null) => run(api.moveFolder(id, parent)),
    switchTo: (id: number) => run(api.switchSession(id), true),
    addTo: (containerId: number, characterId: number) =>
      run(api.addToContainer(containerId, characterId)),
    removeFrom: (containerId: number, characterId: number) =>
      run(api.removeFromContainer(containerId, characterId)),
  };
}
