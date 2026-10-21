import { z } from "zod";
import type { Character } from "@/bindings/Character";

// Mirrors the limits in `rules-core::character`; Rust stays the final judge.
export const STAT_MIN = 1;
export const STAT_MAX = 10;
export const SP_MAX = 20;

const stat = z.number().int().min(STAT_MIN).max(STAT_MAX);
const armorSlot = z
  .object({ sp: z.number().int().min(0).max(SP_MAX), max_sp: z.number().int().min(0).max(SP_MAX) })
  .refine((a) => a.sp <= a.max_sp, { path: ["sp"], message: "sp_over_max" });

export const characterSchema = z.object({
  schema_version: z.number().int(),
  name: z.string().trim().min(1, "name"),
  stats: z.object({
    int: stat,
    reflex: stat,
    dex: stat,
    tech: stat,
    cool: stat,
    will: stat,
    luck: stat,
    movement: stat,
    body: stat,
    emp: z.number().int().min(0).max(STAT_MAX),
  }),
  hp: z.number().int().min(0),
  armor_head: armorSlot,
  armor_body: armorSlot,
  injuries: z.array(z.string()),
  death_save_penalty: z.number().int().min(0),
  dead: z.boolean(),
  notes: z.string(),
}) satisfies z.ZodType<Character>;
