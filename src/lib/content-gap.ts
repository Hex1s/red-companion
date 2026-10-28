import type { ContentStatus } from "@/lib/api";

/**
 * Why a stored code is not in the references:
 * - `not_downloaded`: no copy of the collection is on this device;
 * - `pack_off`: the copies are there, but their pack is switched off;
 * - `absent`: the collection is there and has no such entry (another pack, removed on the server).
 */
export type MissingReason = "not_downloaded" | "pack_off" | "absent";

/** The two-letter content language of an interface language (`ru-RU` → `ru`). */
export const contentLang = (language: string) => language.slice(0, 2).toLowerCase();

/** Packs that hold a downloaded copy of the collection in this language, with their switch. */
function holders(collection: string, lang: string, status: ContentStatus) {
  const switches = new Map(status.packs.map((p) => [p.code, p.enabled]));
  const codes = new Set(status.cached.filter((c) => c.collection === collection && c.lang === lang).map((c) => c.pack));
  return [...codes].map((pack) => ({ pack, enabled: switches.get(pack) ?? true }));
}

/**
 * The reason a code of `collection` is missing, from what is downloaded. `listLoaded` says the
 * collection's list is in memory; while it is not, "absent" cannot be told from "still loading", so
 * the answer is `null` (show nothing yet).
 */
export function explainMissing(collection: string, lang: string, status: ContentStatus | null, listLoaded: boolean): MissingReason | null {
  if (!status) return null;
  const copies = holders(collection, lang, status);
  if (copies.length === 0) return "not_downloaded";
  if (copies.every((c) => !c.enabled)) return "pack_off";
  return listLoaded ? "absent" : null;
}

/** The packs to switch on to bring the collection back (those that hold a copy and are off). */
export function packsToEnable(collection: string, lang: string, status: ContentStatus): string[] {
  return holders(collection, lang, status).filter((c) => !c.enabled).map((c) => c.pack);
}
