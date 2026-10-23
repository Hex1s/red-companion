import type { CheckRoll } from "@/bindings/CheckRoll";

/** Whole numbers typed as "10 4" or "10,4"; `null` for empty text, `NaN` entries for junk. */
export function parseFaces(text: string): number[] | null {
  const parts = text.split(/[\s,]+/).filter(Boolean);
  return parts.length === 0 ? null : parts.map(Number);
}

/** True when every face is a whole number on a d10. */
export function validD10Faces(faces: number[]): boolean {
  return faces.length <= 2 && faces.every((f) => Number.isInteger(f) && f >= 1 && f <= 10);
}

/** The extra die of a check that exploded: up after a natural 10, down after a natural 1. */
export type Explosion = { direction: "up" | "down"; first: number; extra: number };

/** `null` for an ordinary roll (or a total typed in from physical dice: there are no faces). */
export function explosionOf(roll: CheckRoll | null | undefined): Explosion | null {
  if (!roll || roll.kind === "normal" || roll.dice.length < 2) return null;
  return {
    direction: roll.kind === "critical_failure" ? "down" : "up",
    first: roll.dice[0],
    extra: roll.dice[1],
  };
}

/** "7", "10 + 4" (natural 10 adds a die) or "1 − 4" (natural 1 subtracts one). */
export function formatCheck(roll: CheckRoll): string {
  const [first, extra] = roll.dice;
  // A total typed in from physical dice has no faces: the table already did the sum.
  if (first === undefined) return String(roll.total);
  if (extra === undefined) return String(first);
  return `${first} ${roll.kind === "critical_failure" ? "−" : "+"} ${extra}`;
}
