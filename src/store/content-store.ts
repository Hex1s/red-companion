import { create } from "zustand";

type ContentState = {
  /** Bumped whenever downloaded content changed: lists that show it load again. */
  version: number;
  bump: () => void;
  /** The references manager (and the first-start wizard) is open. */
  managerOpen: boolean;
  setManagerOpen: (open: boolean) => void;
};

/** UI state of the content download: the dialog and a counter the lists watch. */
export const useContentStore = create<ContentState>((set) => ({
  version: 0,
  bump: () => set((s) => ({ version: s.version + 1 })),
  managerOpen: false,
  setManagerOpen: (managerOpen) => set({ managerOpen }),
}));
