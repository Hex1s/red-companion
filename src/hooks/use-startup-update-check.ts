import { useEffect } from "react";
import { useTranslation } from "react-i18next";
import { api, hasBackend } from "@/lib/api";
import { status } from "@/store/status-store";
import { useUpdateStore } from "@/store/update-store";

/** Wait a little after the start so the check never competes with the first screen. */
const START_DELAY_MS = 8000;
/** Restarting the app often does not ask the server every time. */
const MIN_INTERVAL_MS = 6 * 60 * 60 * 1000;
const LAST_CHECK_KEY = "rc.update.checked_at";
const NOTICE_MS = 12000;

function recentlyChecked(): boolean {
  try {
    const last = Number(window.localStorage.getItem(LAST_CHECK_KEY));
    return Number.isFinite(last) && Date.now() - last < MIN_INTERVAL_MS;
  } catch {
    return false;
  }
}

function rememberCheck() {
  try {
    window.localStorage.setItem(LAST_CHECK_KEY, String(Date.now()));
  } catch {
    // The check just runs again on the next start.
  }
}

/**
 * Looks for a newer app version a few seconds after the start, quietly: no network or any error is
 * ignored (the About dialog reports errors of a manual check). Never installs anything, it only
 * tells: a notice in the status area and the mark on the About button (see `useUpdateStore`).
 */
export function useStartupUpdateCheck(openAbout: () => void) {
  const { t } = useTranslation();
  const setAvailable = useUpdateStore((s) => s.setAvailable);

  useEffect(() => {
    if (!hasBackend || recentlyChecked()) return;
    let cancelled = false;
    const timer = window.setTimeout(() => {
      api.checkUpdate().then(
        (found) => {
          if (cancelled) return;
          rememberCheck();
          setAvailable(found);
          if (!found) return;
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
