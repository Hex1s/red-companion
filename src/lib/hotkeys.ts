/**
 * The one registry of keyboard shortcuts. Two rules keep them from firing by accident:
 * - no bare letters: a shortcut always needs a modifier;
 * - `Ctrl`/`⌘` is for the interface (panels, search, help), `Alt`/`⌥` for game actions (turns,
 *   damage, tabs), because `Ctrl+Z`, `Ctrl+A`, `Ctrl+D`… belong to text fields and the browser.
 * Keys are physical (`KeyboardEvent.code`), so they work in any keyboard layout.
 */

export type HotkeyId =
  | "explorer"
  | "search"
  | "help"
  | "attackRef"
  | "prevTurn"
  | "nextTurn"
  | "damage"
  | "undoDamage"
  | "prevCharacter"
  | "nextCharacter"
  | "tab1"
  | "tab2"
  | "tab3"
  | "tab4";

export type HotkeyGroup = "interface" | "combat" | "navigation";

type Hotkey = {
  /** `ctrl` also accepts `⌘`; `alt` is `⌥` on a Mac. */
  mod: "ctrl" | "alt";
  /** `KeyboardEvent.code`. */
  code: string;
  group: HotkeyGroup;
  /** Needs a free hand: ignored while a dialog or popover is open. */
  game: boolean;
};

export const HOTKEYS: Record<HotkeyId, Hotkey> = {
  explorer: { mod: "ctrl", code: "KeyB", group: "interface", game: false },
  search: { mod: "ctrl", code: "KeyK", group: "interface", game: false },
  help: { mod: "ctrl", code: "Slash", group: "interface", game: false },
  attackRef: { mod: "alt", code: "KeyA", group: "combat", game: false },
  prevTurn: { mod: "alt", code: "KeyP", group: "combat", game: true },
  nextTurn: { mod: "alt", code: "KeyN", group: "combat", game: true },
  damage: { mod: "alt", code: "KeyD", group: "combat", game: true },
  undoDamage: { mod: "alt", code: "KeyZ", group: "combat", game: true },
  prevCharacter: { mod: "alt", code: "ArrowUp", group: "navigation", game: true },
  nextCharacter: { mod: "alt", code: "ArrowDown", group: "navigation", game: true },
  tab1: { mod: "alt", code: "Digit1", group: "navigation", game: true },
  tab2: { mod: "alt", code: "Digit2", group: "navigation", game: true },
  tab3: { mod: "alt", code: "Digit3", group: "navigation", game: true },
  tab4: { mod: "alt", code: "Digit4", group: "navigation", game: true },
};

export const HOTKEY_IDS = Object.keys(HOTKEYS) as HotkeyId[];

/** `AltGr` reports Ctrl+Alt on Windows, so a plain Alt shortcut must exclude Ctrl to leave it alone. */
export function matches(e: KeyboardEvent, hotkey: Hotkey): boolean {
  if (e.shiftKey || e.code !== hotkey.code) return false;
  return hotkey.mod === "ctrl" ? (e.ctrlKey || e.metaKey) && !e.altKey : e.altKey && !e.ctrlKey && !e.metaKey;
}

/** A dialog or popover is open: game shortcuts wait until it is closed. */
export function modalOpen(): boolean {
  return document.querySelector('[role="dialog"][data-state="open"], [role="alertdialog"][data-state="open"]') !== null;
}

const isMac = typeof navigator !== "undefined" && /Mac|iPhone|iPad/.test(navigator.platform);

const KEY_LABELS: Record<string, string> = { Slash: "/", ArrowUp: "↑", ArrowDown: "↓" };

/** The combination as shown to the user: `Ctrl+K`, `⌘K`, `Alt+↑`, `⌥↑`. */
export function formatHotkey(id: HotkeyId): string {
  const { mod, code } = HOTKEYS[id];
  const key = KEY_LABELS[code] ?? code.replace(/^(Key|Digit)/, "");
  if (isMac) return `${mod === "ctrl" ? "⌘" : "⌥"}${key}`;
  return `${mod === "ctrl" ? "Ctrl" : "Alt"}+${key}`;
}
