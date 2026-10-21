import type { ReactNode } from "react";
import { cn } from "@/lib/utils";

/**
 * One glyph per critical injury (book p. 187-188), keyed by injury id. Line art on a 24x24
 * grid in `currentColor`: colour and size come from the surrounding element.
 */
const GLYPHS: Record<string, ReactNode> = {
  // Body table.
  dismembered_arm: (
    <>
      <path d="M2 9h9v6H2z" />
      <path d="M11 9l2.5 2.5L11 14l2.5 1" />
      <path d="M16 10h6v4h-6z" strokeDasharray="2 2" />
    </>
  ),
  dismembered_hand: (
    <>
      <path d="M5 13h8v7H5z" />
      <path d="M6.5 13V8M9 13V6M11.5 13V8" />
      <path d="M3 21L17 12" />
    </>
  ),
  collapsed_lung: (
    <>
      <path d="M12 4v8" />
      <path d="M12 8c-3 0-6 3-6 8 0 2 1 3 3 3 2 0 3-1 3-3" />
      <path d="M12 8c3 0 6 3 6 8 0 2-1 3-3 3-2 0-3-1-3-3" strokeDasharray="2 2" />
    </>
  ),
  broken_ribs: (
    <>
      <path d="M12 4v16" />
      <path d="M5 6c4 0 7 2 7 6M5 11c4 0 7 2 7 6" />
      <path d="M19 6c-4 0-7 2-7 6M19 11c-4 0-7 2-7 6" />
      <path d="M9 14l2-1.5" />
    </>
  ),
  broken_arm: <path d="M3 21l6-6 3-.5-1-3 3.5-.5.5-2 6-6" />,
  foreign_object_body: (
    <>
      <path d="M4 20l9-9" />
      <path d="M12 6l6 6-2 2-6-6z" />
      <path d="M16 3l5 5" />
    </>
  ),
  broken_leg: (
    <>
      <path d="M8 3v7l-2 2 4 1-2 3v5h6" />
      <path d="M13 11l3 1" />
    </>
  ),
  torn_muscle: (
    <>
      <path d="M3 12c3-5 15-5 18 0-3 5-15 5-18 0z" />
      <path d="M12 7.5v3L10.5 12l2 1-1 3.5" />
    </>
  ),
  spinal_injury: (
    <>
      <path d="M9 4h6M9 8h6M9 12h6M9 16h6M9 20h6" />
      <path d="M12 3v18" strokeDasharray="1 3" />
      <path d="M17 6l-3 5 3 1-3 6" />
    </>
  ),
  crushed_fingers: (
    <>
      <path d="M5 15h10v5H5z" />
      <path d="M6.5 15v-5M9 15V8M11.5 15v-5M14 15v-4" />
      <path d="M3 5h14M10 5v2" />
    </>
  ),
  dismembered_leg: (
    <>
      <path d="M5 3h5v9H5z" />
      <path d="M5 12l1.5 2L8 12l1.5 2 .5-2" />
      <path d="M13 16h8v4h-8z" strokeDasharray="2 2" />
    </>
  ),
  // Head table.
  lost_eye: (
    <>
      <path d="M2 12c3-5 7-7 10-7s7 2 10 7c-3 5-7 7-10 7s-7-2-10-7z" />
      <path d="M4 4l16 16" />
    </>
  ),
  brain_injury: (
    <>
      <path d="M12 6c-2-2-6-1-6 3 0 1 .5 1.5 1 2-1.5 1-1.5 4 .5 5 1.5 1 3 .5 4.5-1z" />
      <path d="M12 6c2-2 6-1 6 3 0 1-.5 1.5-1 2 1.5 1 1.5 4-.5 5-1.5 1-3 .5-4.5-1" strokeDasharray="2 2" />
      <path d="M12 6v12" />
    </>
  ),
  damaged_eye: (
    <>
      <path d="M2 12c3-5 7-7 10-7s7 2 10 7c-3 5-7 7-10 7s-7-2-10-7z" />
      <circle cx="12" cy="12" r="3" />
      <path d="M12 7.5l-1.5 3 2 1-1.5 3" />
    </>
  ),
  concussion: (
    <>
      <circle cx="12" cy="12" r="2.5" />
      <path d="M12 3v3M12 18v3M3 12h3M18 12h3M5.6 5.6l2 2M16.4 16.4l2 2M18.4 5.6l-2 2M7.6 16.4l-2 2" />
    </>
  ),
  broken_jaw: (
    <>
      <path d="M5 7c0 7 2 11 7 11s7-4 7-11" />
      <path d="M8 13h8" />
      <path d="M12 18l-1.5-2.5 3-1.5-1.5-2.5" />
    </>
  ),
  foreign_object_head: (
    <>
      <circle cx="13" cy="14" r="6" />
      <path d="M3 4l9 9" />
      <path d="M12 13l-3.5-.5M12 13l.5-3.5" />
    </>
  ),
  broken_neck: (
    <>
      <path d="M10 3c0 4 4 4 4 8s-4 4-4 8" />
      <path d="M8 17l2 2 2-2" />
      <path d="M14 3l2 1-2 1" />
    </>
  ),
  cracked_skull: (
    <>
      <path d="M12 3c-5 0-8 3-8 8 0 3 1.5 4.5 3 5.5V20h10v-3.5c1.5-1 3-2.5 3-5.5 0-5-3-8-8-8z" />
      <path d="M12 3l-1.5 4 2.5 2-1.5 3" />
      <path d="M8.5 13h.01M15.5 13h.01" strokeWidth="2.5" />
    </>
  ),
  damaged_ear: (
    <>
      <path d="M7 9a5 5 0 0110 0c0 3-3 4-3 7s-2 5-4 5" />
      <path d="M10 9a2 2 0 014 0" />
      <path d="M19 3l-2 3 2 1-2 3" />
    </>
  ),
  crushed_windpipe: (
    <>
      <path d="M8 3v6l3 3-3 3v6" />
      <path d="M16 3v6l-3 3 3 3v6" />
    </>
  ),
  lost_ear: (
    <>
      <path d="M7 9a5 5 0 0110 0c0 3-3 4-3 7s-2 5-4 5" />
      <path d="M10 9a2 2 0 014 0" />
      <path d="M4 4l16 16" />
    </>
  ),
};

type InjuryIconProps = { id: string; className?: string };

/** Line icon of a critical injury; unknown ids render a plain cross. */
export function InjuryIcon({ id, className }: InjuryIconProps) {
  return (
    <svg
      viewBox="0 0 24 24"
      fill="none"
      stroke="currentColor"
      strokeWidth="1.5"
      strokeLinecap="round"
      strokeLinejoin="round"
      aria-hidden="true"
      className={cn("size-6", className)}
    >
      {GLYPHS[id] ?? <path d="M6 6l12 12M18 6L6 18" />}
    </svg>
  );
}

/** Ids that have a glyph (a test keeps this in step with the injury table). */
export const INJURY_ICON_IDS = Object.keys(GLYPHS);
