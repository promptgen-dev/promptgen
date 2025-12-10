import { invoke } from "@tauri-apps/api/core";
import { open } from "@tauri-apps/plugin-dialog";
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
  // Library home (workspace) operations
  setLibraryHome: (path) => invoke<void>("set_library_home", { path }),

  getLibraryHome: () => invoke<string | null>("get_library_home_cmd"),

  pickFolder: async () => {
    const selected = await open({
      directory: true,
      multiple: false,
      title: "Select Library Home Folder",
    });
    if (selected && typeof selected === "string") {
      return selected;
    }
    return null;
  },

  // Library operations
  listLibraries: () => invoke<LibrarySummary[]>("list_libraries"),

  loadLibrary: (id) => invoke<Library>("load_library", { id }),

  saveLibrary: (lib) => invoke<void>("save_library", { lib }),

  createLibrary: (name) => invoke<Library>("create_library", { name }),

  deleteLibrary: (id) => invoke<void>("delete_library", { id }),

  // Template operations
  parseTemplate: (text) => invoke<ParseResult>("parse_template_cmd", { text }),

  renderTemplate: (input: RenderInput) =>
    invoke<RenderResult>("render_template", { input }),

  // Desktop-specific file operations
  openFile: (path) => invoke<Library>("open_file", { path }),
};

export function DesktopBackendProvider({ children }: { children: ReactNode }) {
  return <BackendProvider backend={desktopBackend}>{children}</BackendProvider>;
}
