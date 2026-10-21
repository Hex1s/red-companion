import { create } from "zustand";

export type AppTab = "combat" | "characters" | "rules";

/** UI state only: game data lives in Rust and comes through Tauri commands. */
type UiState = {
  tab: AppTab;
  selectedCharacterId: number | null;
  /** Character the "damage taken" window is open for. */
  damageTargetId: number | null;
  openDamage: (id: number | null) => void;
  setTab: (tab: AppTab) => void;
  selectCharacter: (id: number | null) => void;
};

export const useUiStore = create<UiState>((set) => ({
  tab: "characters",
  selectedCharacterId: null,
  damageTargetId: null,
  openDamage: (id) => set({ damageTargetId: id }),
  setTab: (tab) => set({ tab }),
  selectCharacter: (id) => set({ selectedCharacterId: id }),
}));
