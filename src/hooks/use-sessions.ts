import { useCallback, useEffect, useState } from "react";
import { useTranslation } from "react-i18next";
import type { SessionInfo } from "@/bindings/SessionInfo";
import { api, hasBackend } from "@/lib/api";
import { status } from "@/store/status-store";

/**
 * Sessions (named folders with their own scene, fight and members) and the active one. The
 * first session is created here, so that its name follows the interface language.
 */
export function useSessions() {
  const { t } = useTranslation();
  const [sessions, setSessions] = useState<SessionInfo[]>([]);
  const [ready, setReady] = useState(false);

  const defaultName = useCallback(
    (list: SessionInfo[]) => t("sessions.default_name", { n: list.length + 1 }),
    [t],
  );

  // An empty list (first run, or the last session was deleted) gets a fresh session at once.
  const withFallback = useCallback(
    async (list: SessionInfo[]) =>
      list.length > 0 ? list : api.createSession(t("sessions.default_name", { n: 1 })),
    [t],
  );

  useEffect(() => {
    if (!hasBackend) return;
    api
      .listSessions()
      .then((list) => withFallback(list))
      .then(setSessions)
      .catch((e: unknown) => status.fail(e, "sessions"))
      .finally(() => setReady(true));
    // Only at startup: the language of the first session's name is the one at that moment.
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, []);

  /** Runs a session command; the backend answers with the updated list. */
  const run = useCallback(
    async (command: Promise<SessionInfo[]>) => {
      try {
        setSessions(await withFallback(await command));
        return true;
      } catch (e) {
        status.fail(e, "sessions");
        return false;
      }
    },
    [withFallback],
  );

  const active = sessions.find((s) => s.active) ?? null;

  return {
    sessions,
    active,
    /** False until the first load has finished: nothing session-related may be asked before. */
    ready,
    create: (name: string | null) => run(api.createSession(name ?? defaultName(sessions))),
    rename: (id: number, name: string) => run(api.renameSession(id, name)),
    switchTo: (id: number) => run(api.switchSession(id)),
    remove: (id: number) => run(api.deleteSession(id)),
  };
}
