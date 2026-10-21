import { useEffect, useState } from "react";
import type { CriticalInjury } from "@/bindings/CriticalInjury";
import { api, hasBackend } from "@/lib/api";

/** Critical injury tables (body and head) from the reference data. */
export function useInjuries() {
  const [injuries, setInjuries] = useState<CriticalInjury[]>([]);

  useEffect(() => {
    if (!hasBackend) return;
    api.listInjuries().then(setInjuries).catch(() => setInjuries([]));
  }, []);

  return injuries;
}
