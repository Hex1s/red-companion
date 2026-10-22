import { create } from "zustand";

export type AppTab = "characters" | "encounters" | "notes" | "rules";
export type ReferenceSection = "injuries" | "cover" | "armor" | "weapons" | "skills" | "npc";

/** UI state only: game data lives in Rust and comes through Tauri commands. */
type UiState = {
  tab: AppTab;
  selectedCharacterId: number | null;
  /** Character the "damage taken" window is open for. */
  damageTargetId: number | null;
  referenceSection: ReferenceSection;
  referenceQuery: string;
  setReferenceSection: (section: ReferenceSection) => void;
  setReferenceQuery: (query: string) => void;
  openDamage: (id: number | null) => void;
  setTab: (tab: AppTab) => void;
  selectCharacter: (id: number | null) => void;
};

export const useUiStore = create<UiState>((set) => ({
  tab: "characters",
  selectedCharacterId: null,
  damageTargetId: null,
  referenceSection: "injuries",
  referenceQuery: "",
  setReferenceSection: (referenceSection) => set({ referenceSection }),
  setReferenceQuery: (referenceQuery) => set({ referenceQuery }),
  openDamage: (id) => set({ damageTargetId: id }),
  setTab: (tab) => set({ tab }),
  selectCharacter: (id) => set({ selectedCharacterId: id }),
}));
