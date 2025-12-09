import { invoke } from "@tauri-apps/api/core";
import {
  BackendProvider,
  type PromptgenBackend,
  type Library,
  type LibrarySummary,
  type ParseResult,
  type RenderInput,
  type RenderResult,
} from "@promptgen/backend";
import type { ReactNode } from "react";

const desktopBackend: PromptgenBackend = {
  listLibraries: () => invoke<LibrarySummary[]>("list_libraries"),

  loadLibrary: (id) => invoke<Library>("load_library", { id }),

  saveLibrary: (lib) => invoke<void>("save_library", { lib }),

  createLibrary: (name, path) =>
    invoke<Library>("create_library", { name, path }),

  deleteLibrary: (id) => invoke<void>("delete_library", { id }),

  parseTemplate: (text) => invoke<ParseResult>("parse_template", { text }),

  renderTemplate: (input: RenderInput) =>
    invoke<RenderResult>("render_template", { input }),

  // Desktop-specific file operations
  openFile: (path) => invoke<Library>("open_file", { path }),
};

export function DesktopBackendProvider({ children }: { children: ReactNode }) {
  return <BackendProvider backend={desktopBackend}>{children}</BackendProvider>;
}
