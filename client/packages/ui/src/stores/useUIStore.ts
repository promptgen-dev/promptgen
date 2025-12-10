import { create } from "zustand";
import { persist } from "zustand/middleware";

export type SidebarViewMode = "templates" | "variables";

interface UIState {
  // Sidebar
  sidebarWidth: number;
  setSidebarWidth: (width: number) => void;
  sidebarViewMode: SidebarViewMode;
  setSidebarViewMode: (mode: SidebarViewMode) => void;
  // Selected library (persisted)
  selectedLibraryId: string | null;
  setSelectedLibraryId: (id: string | null) => void;
}

const MIN_SIDEBAR_WIDTH = 180;
const MAX_SIDEBAR_WIDTH = 1200;
const DEFAULT_SIDEBAR_WIDTH = 400;

export const useUIStore = create<UIState>()(
  persist(
    (set) => ({
      sidebarWidth: DEFAULT_SIDEBAR_WIDTH,
      setSidebarWidth: (width) =>
        set({
          sidebarWidth: Math.max(MIN_SIDEBAR_WIDTH, Math.min(MAX_SIDEBAR_WIDTH, width)),
        }),
      sidebarViewMode: "templates" as SidebarViewMode,
      setSidebarViewMode: (mode) => set({ sidebarViewMode: mode }),
      selectedLibraryId: null,
      setSelectedLibraryId: (id) => set({ selectedLibraryId: id }),
    }),
    {
      name: "promptgen-ui-settings",
      partialize: (state) => ({
        sidebarWidth: state.sidebarWidth,
        sidebarViewMode: state.sidebarViewMode,
        selectedLibraryId: state.selectedLibraryId,
      }),
    }
  )
);

export { MIN_SIDEBAR_WIDTH, MAX_SIDEBAR_WIDTH, DEFAULT_SIDEBAR_WIDTH };
