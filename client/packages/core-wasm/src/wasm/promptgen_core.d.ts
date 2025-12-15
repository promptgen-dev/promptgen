/* tslint:disable */
/* eslint-disable */

export class WasmWorkspace {
  free(): void;
  [Symbol.dispose](): void;
  /**
   * Add or update a library in the workspace.
   * Returns a new workspace (immutable update).
   */
  withLibrary(library_js: any): WasmWorkspace;
  /**
   * Search for groups matching the query across all libraries.
   *
   * Returns all groups if query is empty. Results are sorted by score (highest first).
   * Search is case-insensitive.
   */
  searchGroups(query: string): any;
  /**
   * Extract library references from a template source.
   */
  getReferences(source: string): any;
  /**
   * Parse and validate a template against the workspace.
   * Returns ParseResult with AST (if valid) and any errors/warnings.
   */
  parseTemplate(source: string): any;
  /**
   * Search for options matching the query, optionally filtered to a specific group.
   *
   * Returns all options if query is empty. Results are sorted by best match score.
   * Search is case-insensitive.
   */
  searchOptions(query: string, group_filter?: string | null): any;
  /**
   * Get autocomplete suggestions at a cursor position.
   */
  getCompletions(source: string, cursor_pos: number): any;
  /**
   * Get all group names across all libraries.
   */
  getGroupNames(library_id?: string | null): any;
  /**
   * Get all library IDs in the workspace.
   */
  getLibraryIds(): string[];
  /**
   * Remove a library from the workspace.
   * Returns a new workspace (immutable update).
   */
  withoutLibrary(library_id: string): WasmWorkspace;
  /**
   * Create an empty workspace.
   */
  constructor();
  /**
   * Render a template with the given slot values and optional seed.
   * Returns RenderResult with the output text and chosen options.
   */
  render(source: string, slot_values_js: any, seed?: bigint | null): any;
  /**
   * Unified search with syntax parsing.
   *
   * Supports the following query syntax:
   * - `@group` or `@group_query` - Search for groups
   * - `@group/option` - Search for options within a specific group
   * - `@/option` - Search for options across all groups
   * - Plain text without `@` prefix - Search for groups (default)
   */
  search(query: string): any;
  /**
   * Extract slot names from a template source.
   */
  getSlots(source: string): string[];
}

export class WasmWorkspaceBuilder {
  free(): void;
  [Symbol.dispose](): void;
  /**
   * Add a library to the workspace builder.
   */
  addLibrary(library_js: any): WasmWorkspaceBuilder;
  /**
   * Create a new workspace builder.
   */
  constructor();
  /**
   * Build the workspace.
   */
  build(): WasmWorkspace;
}

/**
 * Initialize panic hook for better error messages in browser console.
 */
export function init(): void;

/**
 * Parse a template source without a workspace (for syntax checking only).
 * Returns parse errors but cannot validate library references.
 */
export function parseTemplateSource(source: string): any;

export type InitInput = RequestInfo | URL | Response | BufferSource | WebAssembly.Module;

export interface InitOutput {
  readonly memory: WebAssembly.Memory;
  readonly __wbg_wasmworkspace_free: (a: number, b: number) => void;
  readonly __wbg_wasmworkspacebuilder_free: (a: number, b: number) => void;
  readonly parseTemplateSource: (a: number, b: number) => [number, number, number];
  readonly wasmworkspace_getCompletions: (a: number, b: number, c: number, d: number) => [number, number, number];
  readonly wasmworkspace_getGroupNames: (a: number, b: number, c: number) => [number, number, number];
  readonly wasmworkspace_getLibraryIds: (a: number) => [number, number];
  readonly wasmworkspace_getReferences: (a: number, b: number, c: number) => [number, number, number];
  readonly wasmworkspace_getSlots: (a: number, b: number, c: number) => [number, number, number, number];
  readonly wasmworkspace_new: () => number;
  readonly wasmworkspace_parseTemplate: (a: number, b: number, c: number) => [number, number, number];
  readonly wasmworkspace_render: (a: number, b: number, c: number, d: any, e: number, f: bigint) => [number, number, number];
  readonly wasmworkspace_search: (a: number, b: number, c: number) => [number, number, number];
  readonly wasmworkspace_searchGroups: (a: number, b: number, c: number) => [number, number, number];
  readonly wasmworkspace_searchOptions: (a: number, b: number, c: number, d: number, e: number) => [number, number, number];
  readonly wasmworkspace_withLibrary: (a: number, b: any) => [number, number, number];
  readonly wasmworkspace_withoutLibrary: (a: number, b: number, c: number) => number;
  readonly wasmworkspacebuilder_addLibrary: (a: number, b: any) => [number, number, number];
  readonly wasmworkspacebuilder_build: (a: number) => number;
  readonly init: () => void;
  readonly wasmworkspacebuilder_new: () => number;
  readonly __wbindgen_malloc: (a: number, b: number) => number;
  readonly __wbindgen_realloc: (a: number, b: number, c: number, d: number) => number;
  readonly __wbindgen_exn_store: (a: number) => void;
  readonly __externref_table_alloc: () => number;
  readonly __wbindgen_externrefs: WebAssembly.Table;
  readonly __wbindgen_free: (a: number, b: number, c: number) => void;
  readonly __externref_table_dealloc: (a: number) => void;
  readonly __externref_drop_slice: (a: number, b: number) => void;
  readonly __wbindgen_start: () => void;
}

export type SyncInitInput = BufferSource | WebAssembly.Module;

/**
* Instantiates the given `module`, which can either be bytes or
* a precompiled `WebAssembly.Module`.
*
* @param {{ module: SyncInitInput }} module - Passing `SyncInitInput` directly is deprecated.
*
* @returns {InitOutput}
*/
export function initSync(module: { module: SyncInitInput } | SyncInitInput): InitOutput;

/**
* If `module_or_path` is {RequestInfo} or {URL}, makes a request and
* for everything else, calls `WebAssembly.instantiate` directly.
*
* @param {{ module_or_path: InitInput | Promise<InitInput> }} module_or_path - Passing `InitInput` directly is deprecated.
*
* @returns {Promise<InitOutput>}
*/
export default function __wbg_init (module_or_path?: { module_or_path: InitInput | Promise<InitInput> } | InitInput | Promise<InitInput>): Promise<InitOutput>;
