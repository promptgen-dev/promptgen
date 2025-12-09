import { create } from "zustand";
import type { Library, LibrarySummary } from "@promptgen/backend";

interface LibraryState {
  // State
  libraries: LibrarySummary[];
  activeLibrary: Library | null;
  isLoading: boolean;
  error: string | null;

  // Actions
  setLibraries: (libraries: LibrarySummary[]) => void;
  setActiveLibrary: (library: Library | null) => void;
  setLoading: (loading: boolean) => void;
  setError: (error: string | null) => void;
  reset: () => void;
}

const initialState = {
  libraries: [],
  activeLibrary: null,
  isLoading: false,
  error: null,
};

export const useLibraryStore = create<LibraryState>((set) => ({
  ...initialState,

  setLibraries: (libraries) => set({ libraries }),
  setActiveLibrary: (library) => set({ activeLibrary: library }),
  setLoading: (loading) => set({ isLoading: loading }),
  setError: (error) => set({ error }),
  reset: () => set(initialState),
}));
