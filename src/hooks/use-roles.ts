import { useEffect, useState } from "react";
import { useTranslation } from "react-i18next";
import type { Role } from "@/bindings/Role";
import { api, hasBackend } from "@/lib/api";

/** Role catalog (server content): reloaded when the language changes. */
export function useRoles(): Role[] {
  const { i18n } = useTranslation();
  const [roles, setRoles] = useState<Role[]>([]);

  useEffect(() => {
    if (!hasBackend) return;
    api.listRoles(i18n.language).then(setRoles, () => setRoles([]));
  }, [i18n.language]);

  return roles;
}
