import { create } from "zustand";
import type { Template, ParseResult } from "@promptgen/backend";

interface TemplateState {
  // State
  activeTemplate: Template | null;
  editorContent: string;
  parseResult: ParseResult | null;
  renderedOutput: string | null;
  isRendering: boolean;
  error: string | null;

  // Actions
  setActiveTemplate: (template: Template | null) => void;
  setEditorContent: (content: string) => void;
  setParseResult: (result: ParseResult | null) => void;
  setRenderedOutput: (output: string | null) => void;
  setRendering: (rendering: boolean) => void;
  setError: (error: string | null) => void;
  reset: () => void;
}

const initialState = {
  activeTemplate: null,
  editorContent: "",
  parseResult: null,
  renderedOutput: null,
  isRendering: false,
  error: null,
};

export const useTemplateStore = create<TemplateState>((set) => ({
  ...initialState,

  setActiveTemplate: (template) =>
    set({
      activeTemplate: template,
      editorContent: template?.content ?? "",
    }),
  setEditorContent: (content) => set({ editorContent: content }),
  setParseResult: (result) => set({ parseResult: result }),
  setRenderedOutput: (output) => set({ renderedOutput: output }),
  setRendering: (rendering) => set({ isRendering: rendering }),
  setError: (error) => set({ error }),
  reset: () => set(initialState),
}));
