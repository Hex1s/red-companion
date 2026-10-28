import { useEffect } from "react";
import { useTranslation } from "react-i18next";
import { api, hasBackend } from "@/lib/api";
import { status } from "@/store/status-store";
import { useUpdateStore } from "@/store/update-store";

/** Wait a little after the start so the check never competes with the first screen. */
const START_DELAY_MS = 8000;
/** The notice for one version is not repeated on every restart; the mark on the About button stays. */
const NOTICE_INTERVAL_MS = 6 * 60 * 60 * 1000;
const NOTICED_KEY = "rc.update.noticed";
const NOTICE_MS = 12000;

function alreadyNoticed(version: string): boolean {
  try {
    const noticed = JSON.parse(window.localStorage.getItem(NOTICED_KEY) ?? "null") as { version?: string; at?: number } | null;
    return noticed?.version === version && Date.now() - (noticed.at ?? 0) < NOTICE_INTERVAL_MS;
  } catch {
    return false;
  }
}

function rememberNotice(version: string) {
  try {
    window.localStorage.setItem(NOTICED_KEY, JSON.stringify({ version, at: Date.now() }));
  } catch {
    // The notice just shows again on the next start.
  }
}

/**
 * Looks for a newer app version a few seconds after every start, quietly: no network or any error is
 * ignored (the About dialog reports errors of a manual check). Never installs anything, it only
 * tells: a notice in the status area and the mark on the About button (see `useUpdateStore`).
 */
export function useStartupUpdateCheck(openAbout: () => void) {
  const { t } = useTranslation();
  const setAvailable = useUpdateStore((s) => s.setAvailable);

  useEffect(() => {
    if (!hasBackend) return;
    let cancelled = false;
    const timer = window.setTimeout(() => {
      api.checkUpdate().then(
        (found) => {
          if (cancelled) return;
          setAvailable(found);
          if (!found || alreadyNoticed(found.version)) return;
          rememberNotice(found.version);
          status.show({
            id: "update:available",
            kind: "info",
            text: t("about.startup_available", { version: found.version }),
            action: { label: t("about.startup_open"), run: openAbout },
            autoHideMs: NOTICE_MS,
          });
        },
        () => {},
      );
    }, START_DELAY_MS);
    return () => {
      cancelled = true;
      window.clearTimeout(timer);
    };
    // Once per start: the language or the callback changing must not run a second check.
  }, []);
}
