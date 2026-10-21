import { useEffect, useState } from "react";
import type { Skill } from "@/bindings/Skill";
import { api, hasBackend } from "@/lib/api";

/** Skill catalog from the Rust data layer (loaded once; it never changes at runtime). */
export function useSkills(): Skill[] {
  const [skills, setSkills] = useState<Skill[]>([]);

  useEffect(() => {
    if (!hasBackend) return;
    api.listSkills().then(setSkills, () => setSkills([]));
  }, []);

  return skills;
}
