import { useEffect, useState } from "react";
import { useTranslation } from "react-i18next";
import { api, hasBackend, onContentFetchFailed } from "@/lib/api";
import { errorMessage } from "@/lib/errors";
import { useContentStore } from "@/store/content-store";
import { status } from "@/store/status-store";

const WIZARD_KEY = "rc.content.wizard";
/** A failed download on request is reported at most this often: offline at the table must not nag. */
const FAILURE_NOTICE_MS = 10 * 60 * 1000;

function wizardDone(): boolean {
  try {
    return window.localStorage.getItem(WIZARD_KEY) !== null;
  } catch {
    return false;
  }
}

function markWizardDone() {
  try {
    window.localStorage.setItem(WIZARD_KEY, "1");
  } catch {
    // The wizard just shows again on the next start.
  }
}

/**
 * Start-up of the content download: opens the references dialog as a first-start wizard when
 * nothing was ever registered or downloaded, and tells the user (rarely) when a reference could not
 * be fetched on request. Returns whether the open dialog is the wizard and how to leave it.
 */
export function useContentBootstrap() {
  const { t } = useTranslation();
  const setManagerOpen = useContentStore((s) => s.setManagerOpen);
  const [firstRun, setFirstRun] = useState(false);

  useEffect(() => {
    if (!hasBackend || wizardDone()) return;
    let cancelled = false;
    api.contentStatus().then(
      (state) => {
        if (cancelled || state.registered || state.cached.length > 0) return;
        setFirstRun(true);
        setManagerOpen(true);
      },
      () => {},
    );
    return () => {
      cancelled = true;
    };
  }, [setManagerOpen]);

  useEffect(() => {
    let last = 0;
    onContentFetchFailed((error) => {
      if (Date.now() - last < FAILURE_NOTICE_MS) return;
      last = Date.now();
      status.show({
        id: "content:fetch",
        kind: "error",
        text: t("content.fetch_failed", { error: errorMessage(error) }),
        action: { label: t("content.open"), run: () => setManagerOpen(true) },
      });
    });
  }, [t, setManagerOpen]);

  return {
    firstRun,
    /** The wizard was left (skipped or finished): it does not come back. */
    finishFirstRun: () => {
      markWizardDone();
      setFirstRun(false);
    },
  };
}
