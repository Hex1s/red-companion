import { z } from "zod";
import type { Character } from "@/bindings/Character";

// Mirrors the limits in `rules-core::character`; Rust stays the final judge.
// EMP and LUCK may be 0 (cyberware burns EMP, the book's filler NPCs have no luck).
export const STAT_MIN = 1;
// Stats may legitimately exceed the usual 1..10 (gear, cyberware); 99 only catches typos.
export const STAT_MAX = 99;
export const SP_MAX = 20;
export const REPUTATION_MAX = 10;
export const SKILL_MAX = 10;
export const RANK_MAX = 10;
/** A new role may be taken once every role held is at this rank (p. 143). */
export const MULTICLASS_MIN_RANK = 4;
/** Rank of a starting character's first role (p. 142); later roles start at 1 (p. 143). */
export const STARTING_RANK = 4;

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
    luck: z.number().int().min(0).max(STAT_MAX),
    movement: stat,
    body: stat,
    emp: z.number().int().min(0).max(STAT_MAX),
  }),
  hp: z.number().int().min(0),
  armor_head: armorSlot,
  armor_body: armorSlot,
  injuries: z.array(z.string()),
  first_aided: z.array(z.string()),
  death_save_penalty: z.number().int().min(0),
  dead: z.boolean(),
  humanity: z.number().int().nullable(),
  reputation: z.number().int().min(-REPUTATION_MAX).max(REPUTATION_MAX),
  notes: z.string(),
  doses: z.array(z.string()),
  roles: z.array(
    z.object({
      role: z.string().min(1),
      rank: z.number().int().min(0).max(RANK_MAX),
      points: z.record(z.string(), z.number().int().min(0).max(RANK_MAX)),
      picks: z.array(z.string()),
    }),
  ),
  skills: z.record(z.string(), z.number().int().min(0).max(SKILL_MAX)),
}) satisfies z.ZodType<Character>;
