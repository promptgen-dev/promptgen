import {
  WasmWorkspaceBuilder,
  type GroupSearchResult,
  type OptionSearchResult,
} from "@promptgen/core-wasm";

/**
 * Parse a variable search query.
 * Formats:
 * - "blue" -> search all options
 * - "@Ey" -> search group names only, show all options
 * - "@Ey/bl" -> search groups matching "Ey" that have options matching "bl"
 * - "@/bl" -> search all options (same as "blue")
 */
export interface ParsedVariableQuery {
  type: "options" | "groups" | "groups-with-options";
  groupQuery: string;
  optionQuery: string;
}

export function parseVariableQuery(searchQuery: string): ParsedVariableQuery {
  const trimmed = searchQuery.trim();

  if (!trimmed.startsWith("@")) {
    // Plain search - search options only
    return { type: "options", groupQuery: "", optionQuery: trimmed };
  }

  // Remove @ prefix
  const afterAt = trimmed.slice(1);

  if (!afterAt.includes("/")) {
    // @GroupName - search groups only
    return { type: "groups", groupQuery: afterAt, optionQuery: "" };
  }

  // @GroupName/optionQuery
  const [groupQuery, optionQuery] = afterAt.split("/", 2);

  if (!groupQuery) {
    // @/optionQuery - search all options
    return { type: "options", groupQuery: "", optionQuery: optionQuery || "" };
  }

  return {
    type: "groups-with-options",
    groupQuery,
    optionQuery: optionQuery || "",
  };
}

/**
 * Result of filtering variables
 */
export interface FilteredVariable {
  name: string;
  options: string[];
  matchingOptionIndices: Set<number>;
  showAllOptions: boolean; // true when group matched, false when filtering by options
}

/**
 * Create a temporary WASM workspace from variables for searching.
 */
function createSearchWorkspace(variables: Record<string, string[]>) {
  const groups = Object.entries(variables).map(([name, options]) => ({
    name,
    options,
  }));

  return new WasmWorkspaceBuilder()
    .addLibrary({
      id: "search",
      name: "Search",
      groups,
    })
    .build();
}

/**
 * Filter variables based on parsed query using WASM fuzzy search.
 */
export function filterVariables(
  variables: Record<string, string[]>,
  query: ParsedVariableQuery
): FilteredVariable[] {
  const entries = Object.entries(variables).sort(([a], [b]) =>
    a.localeCompare(b)
  );

  // No query - return all
  if (!query.groupQuery && !query.optionQuery) {
    return entries.map(([name, options]) => ({
      name,
      options,
      matchingOptionIndices: new Set<number>(),
      showAllOptions: true,
    }));
  }

  const workspace = createSearchWorkspace(variables);

  try {
    switch (query.type) {
      case "options": {
        // Search all options across all groups
        const searchResults = workspace.searchOptions(
          query.optionQuery,
          null
        ) as OptionSearchResult[];

        return searchResults.map((result) => {
          const options = variables[result.group_name] || [];
          // Build set of matching indices from the option texts
          const matchingIndices = new Set<number>();
          for (const match of result.matches) {
            const idx = options.indexOf(match.text);
            if (idx !== -1) {
              matchingIndices.add(idx);
            }
          }

          return {
            name: result.group_name,
            options,
            matchingOptionIndices: matchingIndices,
            showAllOptions: false,
          };
        });
      }

      case "groups": {
        // Search group names only, show all options
        const searchResults = workspace.searchGroups(
          query.groupQuery
        ) as GroupSearchResult[];

        return searchResults.map((result) => ({
          name: result.group_name,
          options: result.options,
          matchingOptionIndices: new Set<number>(),
          showAllOptions: true,
        }));
      }

      case "groups-with-options": {
        // First filter groups, then search options within those groups
        const groupResults = workspace.searchGroups(
          query.groupQuery
        ) as GroupSearchResult[];

        if (!query.optionQuery) {
          // No option query - show all options for matching groups
          return groupResults.map((result) => ({
            name: result.group_name,
            options: result.options,
            matchingOptionIndices: new Set<number>(),
            showAllOptions: true,
          }));
        }

        // Filter options within matching groups
        const results: FilteredVariable[] = [];

        for (const groupResult of groupResults) {
          const optionResults = workspace.searchOptions(
            query.optionQuery,
            groupResult.group_name
          ) as OptionSearchResult[];

          if (optionResults.length > 0 && optionResults[0].matches.length > 0) {
            const options = variables[groupResult.group_name] || [];
            const matchingIndices = new Set<number>();

            for (const match of optionResults[0].matches) {
              const idx = options.indexOf(match.text);
              if (idx !== -1) {
                matchingIndices.add(idx);
              }
            }

            results.push({
              name: groupResult.group_name,
              options,
              matchingOptionIndices: matchingIndices,
              showAllOptions: false,
            });
          }
        }

        return results;
      }
    }
  } finally {
    // Clean up the WASM workspace
    workspace.free();
  }
}

/**
 * Fuzzy search templates by name using WASM search.
 */
export function fuzzySearchTemplates<T extends { name: string }>(
  templates: T[],
  query: string
): T[] {
  if (!query.trim()) {
    return [...templates].sort((a, b) => a.name.localeCompare(b.name));
  }

  // Create a workspace with template names as groups
  // This is a workaround since we don't have a dedicated template search in WASM
  const groups = templates.map((t, idx) => ({
    name: t.name,
    options: [String(idx)], // Store index as option
  }));

  const workspace = new WasmWorkspaceBuilder()
    .addLibrary({
      id: "templates",
      name: "Templates",
      groups,
    })
    .build();

  try {
    const results = workspace.searchGroups(query.trim()) as GroupSearchResult[];

    // Map results back to templates
    const matchedTemplates: T[] = [];
    for (const result of results) {
      const template = templates.find((t) => t.name === result.group_name);
      if (template) {
        matchedTemplates.push(template);
      }
    }

    return matchedTemplates;
  } finally {
    workspace.free();
  }
}
