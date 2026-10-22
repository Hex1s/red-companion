import type { StatKey } from "@/bindings/StatKey";

/** Locale key suffix (`stats.<ABBR>`) of every stat. */
export const STAT_ABBR: Record<StatKey, string> = {
  int: "INT",
  reflex: "REF",
  dex: "DEX",
  tech: "TECH",
  cool: "COOL",
  will: "WILL",
  luck: "LUCK",
  movement: "MOVE",
  body: "BODY",
  emp: "EMP",
};
