// TypeScript types for WASM search results
// These mirror the Rust types in promptgen-core/src/search.rs

/**
 * Result of a fuzzy search for a group.
 */
export interface GroupSearchResult {
  /** ID of the library containing this group */
  library_id: string;
  /** Display name of the library */
  library_name: string;
  /** Name of the matched group */
  group_name: string;
  /** All options in this group */
  options: string[];
  /** Raw fuzzy match score (higher is better) */
  score: number;
  /** Indices of matched characters in the group name */
  match_indices: number[];
}

/**
 * A single option match within a group.
 */
export interface OptionMatch {
  /** The option text */
  text: string;
  /** Raw fuzzy match score (higher is better) */
  score: number;
  /** Indices of matched characters in the option text */
  match_indices: number[];
}

/**
 * Result of a fuzzy search for options.
 */
export interface OptionSearchResult {
  /** ID of the library containing this group */
  library_id: string;
  /** Display name of the library */
  library_name: string;
  /** Name of the group containing matched options */
  group_name: string;
  /** Matched options with their scores */
  matches: OptionMatch[];
}

/**
 * Unified search result that can contain either groups or options.
 */
export type SearchResult =
  | { Groups: GroupSearchResult[] }
  | { Options: OptionSearchResult[] };

/**
 * Library input format for creating a WASM workspace.
 */
export interface LibraryInput {
  id: string;
  name: string;
  groups: GroupInput[];
}

/**
 * Group input format for library creation.
 */
export interface GroupInput {
  name: string;
  options: string[];
}
