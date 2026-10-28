import { useEffect, useState } from "react";
import { useTranslation } from "react-i18next";
import type { MartialArt } from "@/bindings/MartialArt";
import { api, hasBackend } from "@/lib/api";
import { useContentStore } from "@/store/content-store";

/** Martial arts in the interface language; empty until loaded (or without the backend). */
export function useMartialArts(): MartialArt[] {
  const { i18n } = useTranslation();
  const [arts, setArts] = useState<MartialArt[]>([]);
  const contentVersion = useContentStore((s) => s.version);

  useEffect(() => {
    if (!hasBackend) return;
    api.listMartialArts(i18n.language).then(setArts, () => setArts([]));
  }, [i18n.language, contentVersion]);

  return arts;
}
