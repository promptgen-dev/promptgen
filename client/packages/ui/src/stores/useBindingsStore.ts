import { create } from "zustand";
import type { BindingValue } from "@promptgen/backend";

interface BindingsState {
  // State
  bindings: Record<string, BindingValue>;
  resolvedValues: Record<string, string>;

  // Actions
  setBinding: (name: string, value: BindingValue) => void;
  setBindings: (bindings: Record<string, BindingValue>) => void;
  removeBinding: (name: string) => void;
  setResolvedValue: (name: string, value: string) => void;
  setResolvedValues: (values: Record<string, string>) => void;
  reset: () => void;
}

const initialState = {
  bindings: {},
  resolvedValues: {},
};

export const useBindingsStore = create<BindingsState>((set) => ({
  ...initialState,

  setBinding: (name, value) =>
    set((state) => ({
      bindings: { ...state.bindings, [name]: value },
    })),
  setBindings: (bindings) => set({ bindings }),
  removeBinding: (name) =>
    set((state) => {
      const { [name]: _, ...rest } = state.bindings;
      return { bindings: rest };
    }),
  setResolvedValue: (name, value) =>
    set((state) => ({
      resolvedValues: { ...state.resolvedValues, [name]: value },
    })),
  setResolvedValues: (values) => set({ resolvedValues: values }),
  reset: () => set(initialState),
}));
