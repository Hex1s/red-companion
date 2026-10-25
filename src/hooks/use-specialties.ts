import { useEffect, useState } from "react";
import { useTranslation } from "react-i18next";
import type { Specialty } from "@/bindings/Specialty";
import { api, hasBackend } from "@/lib/api";

/** Suggested skill specialties (languages, instruments) in the interface language; empty until loaded. */
export function useSpecialties(): Specialty[] {
  const { i18n } = useTranslation();
  const [list, setList] = useState<Specialty[]>([]);

  useEffect(() => {
    if (!hasBackend) return;
    api.listSpecialties(i18n.language).then(setList, () => setList([]));
  }, [i18n.language]);

  return list;
}
