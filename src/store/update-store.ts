import { create } from "zustand";
import type { UpdateInfo } from "@/lib/api";

type UpdateState = {
  /** The update the backend found (at start or from the About dialog); installing it is the dialog's job. */
  available: UpdateInfo | null;
  setAvailable: (update: UpdateInfo | null) => void;
};

/** UI state: is there a newer app version. Both the start-up check and the About dialog write it. */
export const useUpdateStore = create<UpdateState>((set) => ({
  available: null,
  setAvailable: (available) => set({ available }),
}));
