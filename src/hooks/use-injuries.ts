import { useCallback, useEffect, useState } from "react";
import type { CriticalInjury } from "@/bindings/CriticalInjury";
import { api, hasBackend } from "@/lib/api";
import { status } from "@/store/status-store";
import i18n from "@/i18n";
import { errorMessage } from "@/lib/errors";

/** Critical injury tables (body and head) from the reference data. */
export function useInjuries() {
  const [injuries, setInjuries] = useState<CriticalInjury[]>([]);

  const load = useCallback(() => {
    api.listInjuries().then(
      (list) => {
        setInjuries(list);
        status.dismiss("injuries:load");
      },
      (e: unknown) =>
        status.fail(e, "injuries:load", {
          text: i18n.t("injury.load_error", { error: errorMessage(e) }),
          retry: { label: i18n.t("status.retry"), run: load },
        }),
    );
  }, []);

  useEffect(() => {
    if (hasBackend) load();
  }, [load]);

  return injuries;
}
