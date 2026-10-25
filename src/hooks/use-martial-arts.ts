import { useEffect, useState } from "react";
import { useTranslation } from "react-i18next";
import type { MartialArt } from "@/bindings/MartialArt";
import { api, hasBackend } from "@/lib/api";

/** Martial arts in the interface language; empty until loaded (or without the backend). */
export function useMartialArts(): MartialArt[] {
  const { i18n } = useTranslation();
  const [arts, setArts] = useState<MartialArt[]>([]);

  useEffect(() => {
    if (!hasBackend) return;
    api.listMartialArts(i18n.language).then(setArts, () => setArts([]));
  }, [i18n.language]);

  return arts;
}
