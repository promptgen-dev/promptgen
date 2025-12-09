import type {
  LibrarySummary,
  Library,
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
  // Library operations
  listLibraries(): Promise<LibrarySummary[]>;
  loadLibrary(id: string): Promise<Library>;
  saveLibrary(lib: Library): Promise<void>;
  createLibrary(name: string, path: string): Promise<Library>;
  deleteLibrary(id: string): Promise<void>;

  // Template operations
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
