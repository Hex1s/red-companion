import { create } from "zustand";

export type AppTab = "characters" | "encounters" | "notes" | "rules";
/** Tabs of a character sheet; the choice is kept while switching between characters. */
export type SheetTab = "general" | "state" | "skills" | "roles" | "gear";
export type ReferenceSection = "articles" | "injuries" | "cover" | "armor" | "weapons" | "gear" | "ammo" | "weapon_mods" | "clothing" | "street_drugs" | "skills" | "martial_arts" | "roles" | "reputation";

const EXPLORER_KEY = "rc.explorer";

type StoredExplorer = { open: boolean; expanded: Record<string, boolean> };

/** The explorer's look is a per-viewer convenience: kept in the browser, never required. */
function loadExplorer(): StoredExplorer {
  try {
    const raw = localStorage.getItem(EXPLORER_KEY);
    if (raw) {
      const parsed = JSON.parse(raw) as Partial<StoredExplorer>;
      return { open: parsed.open !== false, expanded: parsed.expanded ?? {} };
    }
  } catch {
    // Storage may be blocked or hold junk; the defaults below are fine.
  }
  return { open: true, expanded: {} };
}

function saveExplorer(state: StoredExplorer) {
  try {
    localStorage.setItem(EXPLORER_KEY, JSON.stringify(state));
  } catch {
    // Not being able to remember the layout is harmless.
  }
}

/** UI state only: game data lives in Rust and comes through Tauri commands. */
type UiState = {
  tab: AppTab;
  selectedCharacterId: number | null;
  /** Own template open in the main area (`custom:<id>`); `null` when a character is shown. */
  selectedTemplateKey: string | null;
  selectTemplate: (key: string | null) => void;
  /** Character the "damage taken" window is open for. */
  damageTargetId: number | null;
  /** The quick attack reference popover (hotkey A). */
  attackRefOpen: boolean;
  setAttackRefOpen: (open: boolean) => void;
  sheetTab: SheetTab;
  setSheetTab: (tab: SheetTab) => void;
  referenceSection: ReferenceSection;
  referenceQuery: string;
  setReferenceSection: (section: ReferenceSection) => void;
  setReferenceQuery: (query: string) => void;
  /** Entry shown in the reference card; `null` = the first one of the list. */
  referenceId: string | null;
  setReferenceId: (id: string | null) => void;
  /** Jumps to one reference entry on the reference page. */
  openReference: (section: ReferenceSection, id: string) => void;
  openDamage: (id: number | null) => void;
  setTab: (tab: AppTab) => void;
  selectCharacter: (id: number | null) => void;
  /** Whether the explorer panel is shown. */
  explorerOpen: boolean;
  toggleExplorer: () => void;
  /** Slide-in panels of the narrower layouts (ROADMAP 8.1); transient, never saved. */
  explorerDrawerOpen: boolean;
  setExplorerDrawerOpen: (open: boolean) => void;
  railDrawerOpen: boolean;
  setRailDrawerOpen: (open: boolean) => void;
  /** Open/closed state of tree nodes by key ("session:3", "library", ...). */
  expanded: Record<string, boolean>;
  setExpanded: (key: string, open: boolean) => void;
};

const initialExplorer = loadExplorer();

export const useUiStore = create<UiState>((set, get) => ({
  tab: "characters",
  selectedCharacterId: null,
  selectedTemplateKey: null,
  selectTemplate: (selectedTemplateKey) => set({ selectedTemplateKey, tab: "characters" }),
  damageTargetId: null,
  attackRefOpen: false,
  setAttackRefOpen: (attackRefOpen) => set({ attackRefOpen }),
  sheetTab: "general",
  setSheetTab: (sheetTab) => set({ sheetTab }),
  referenceSection: "injuries",
  referenceQuery: "",
  setReferenceSection: (referenceSection) => set({ referenceSection, referenceId: null }),
  setReferenceQuery: (referenceQuery) => set({ referenceQuery }),
  referenceId: null,
  setReferenceId: (referenceId) => set({ referenceId }),
  openReference: (referenceSection, referenceId) =>
    set({ tab: "rules", referenceSection, referenceId, referenceQuery: "" }),
  openDamage: (id) => set({ damageTargetId: id }),
  setTab: (tab) => set({ tab }),
  selectCharacter: (id) => set({ selectedCharacterId: id, selectedTemplateKey: null }),
  explorerOpen: initialExplorer.open,
  toggleExplorer: () => {
    set({ explorerOpen: !get().explorerOpen });
    saveExplorer({ open: get().explorerOpen, expanded: get().expanded });
  },
  explorerDrawerOpen: false,
  setExplorerDrawerOpen: (explorerDrawerOpen) => set({ explorerDrawerOpen }),
  railDrawerOpen: false,
  setRailDrawerOpen: (railDrawerOpen) => set({ railDrawerOpen }),
  expanded: initialExplorer.expanded,
  setExpanded: (key, open) => {
    set({ expanded: { ...get().expanded, [key]: open } });
    saveExplorer({ open: get().explorerOpen, expanded: get().expanded });
  },
}));
