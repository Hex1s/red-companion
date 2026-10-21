import { create } from "zustand";

export type AppTab = "combat" | "characters" | "rules";

/** UI state only: game data lives in Rust and comes through Tauri commands. */
type UiState = {
  tab: AppTab;
  selectedCharacterId: number | null;
  setTab: (tab: AppTab) => void;
  selectCharacter: (id: number | null) => void;
};

export const useUiStore = create<UiState>((set) => ({
  tab: "characters",
  selectedCharacterId: null,
  setTab: (tab) => set({ tab }),
  selectCharacter: (id) => set({ selectedCharacterId: id }),
}));
