import { create } from "zustand";
import type { Library, LibrarySummary } from "@promptgen/backend";

interface LibraryState {
  // State
  libraryHome: string | null;
  libraries: LibrarySummary[];
  activeLibrary: Library | null;
  selectedTemplateId: string | null;
  isLoading: boolean;
  error: string | null;

  // Actions
  setLibraryHome: (path: string | null) => void;
  setLibraries: (libraries: LibrarySummary[]) => void;
  setActiveLibrary: (library: Library | null) => void;
  setSelectedTemplateId: (id: string | null) => void;
  setLoading: (loading: boolean) => void;
  setError: (error: string | null) => void;
  reset: () => void;
}

const initialState = {
  libraryHome: null,
  libraries: [],
  activeLibrary: null,
  selectedTemplateId: null,
  isLoading: false,
  error: null,
};

export const useLibraryStore = create<LibraryState>((set) => ({
  ...initialState,

  setLibraryHome: (path) => set({ libraryHome: path }),
  setLibraries: (libraries) => set({ libraries }),
  setActiveLibrary: (library) => set({ activeLibrary: library }),
  setSelectedTemplateId: (id) => set({ selectedTemplateId: id }),
  setLoading: (loading) => set({ isLoading: loading }),
  setError: (error) => set({ error }),
  reset: () => set(initialState),
}));
