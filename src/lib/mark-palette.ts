/**
 * The colour labels a GM can give a character: 16 colours tuned for the dark HUD (mid lightness, no neon),
 * in two rows. The value is stored in the character as `#rrggbb`, so old labels and exports keep working;
 * `id` keys the name in the locales (`color.names.<id>`). User data, not interface colours: see CLAUDE.md.
 */
export const MARK_PALETTE = [
  { id: "red", color: "#e0525a" },
  { id: "orange", color: "#ee8a3c" },
  { id: "yellow", color: "#e3c34a" },
  { id: "lime", color: "#9bd24a" },
  { id: "green", color: "#3fcf8e" },
  { id: "teal", color: "#2fc4b8" },
  { id: "cyan", color: "#3ec1e8" },
  { id: "blue", color: "#4a8ff0" },
  { id: "indigo", color: "#7a72f0" },
  { id: "violet", color: "#a86cf0" },
  { id: "magenta", color: "#e05ccf" },
  { id: "pink", color: "#f07aa6" },
  { id: "white", color: "#e6e9ee" },
  { id: "steel", color: "#8b95a1" },
  { id: "rust", color: "#b5714b" },
  { id: "khaki", color: "#8c9a5b" },
] as const;

export type MarkColorId = (typeof MARK_PALETTE)[number]["id"];
