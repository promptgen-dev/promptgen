import type {
  LibrarySummary,
  Library,
  Template,
  PromptGroup,
  ParseResult,
  RenderInput,
  RenderResult,
  AuthState,
  LoginInput,
  FeatureFlags,
} from "./types";

/**
 * Backend interface that the UI talks to.
 * Implementations: DesktopBackend (Tauri), WebBackend (fetch)
 */
export interface PromptgenBackend {
  // Library home (workspace) operations
  setLibraryHome?(path: string): Promise<void>;
  getLibraryHome?(): Promise<string | null>;
  pickFolder?(): Promise<string | null>;

  // Library operations
  listLibraries(): Promise<LibrarySummary[]>;
  loadLibrary(id: string): Promise<Library>;
  saveLibrary(lib: Library): Promise<void>;
  createLibrary(name: string): Promise<Library>;
  deleteLibrary(id: string): Promise<void>;

  // Prompt group operations
  createPromptGroup?(libraryId: string, name: string): Promise<PromptGroup>;
  updatePromptGroup?(libraryId: string, name: string, options: string[]): Promise<PromptGroup>;
  renamePromptGroup?(libraryId: string, oldName: string, newName: string): Promise<PromptGroup>;
  deletePromptGroup?(libraryId: string, name: string): Promise<void>;

  // Template CRUD operations
  createTemplate?(libraryId: string, name: string, content: string): Promise<Template>;
  updateTemplate?(libraryId: string, templateId: string, name: string, content: string): Promise<Template>;
  deleteTemplate?(libraryId: string, templateId: string): Promise<void>;

  // Template parsing/rendering
  parseTemplate(text: string): Promise<ParseResult>;
  renderTemplate(input: RenderInput): Promise<RenderResult>;

  // File operations (desktop only)
  openFile?(path: string): Promise<Library>;
  watchFile?(path: string, callback: (lib: Library) => void): () => void;

  // Auth (cloud features - optional)
  login?(credentials: LoginInput): Promise<AuthState>;
  logout?(): Promise<void>;
  getAuthState?(): Promise<AuthState>;
  getFeatures?(): Promise<FeatureFlags>;
}
