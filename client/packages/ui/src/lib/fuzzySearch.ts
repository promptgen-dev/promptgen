import Fuse, { type IFuseOptions } from "fuse.js";

// Fuse.js options for fuzzy matching
const defaultFuseOptions: IFuseOptions<string> = {
  threshold: 0.4, // 0 = exact match, 1 = match anything
  distance: 100,
  includeScore: true,
  includeMatches: true,
};

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
 * Fuzzy search an array of strings
 */
export function fuzzySearchStrings(
  items: string[],
  query: string,
  options?: IFuseOptions<string>
): Set<number> {
  if (!query) {
    return new Set();
  }

  const fuse = new Fuse(items, { ...defaultFuseOptions, ...options });
  const results = fuse.search(query);

  return new Set(results.map((r) => r.refIndex));
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
 * Filter variables based on parsed query
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
      matchingOptionIndices: new Set(),
      showAllOptions: true,
    }));
  }

  switch (query.type) {
    case "options": {
      // Search all options across all groups
      const results: FilteredVariable[] = [];

      for (const [name, options] of entries) {
        const matchingIndices = fuzzySearchStrings(options, query.optionQuery);
        if (matchingIndices.size > 0) {
          results.push({
            name,
            options,
            matchingOptionIndices: matchingIndices,
            showAllOptions: false,
          });
        }
      }

      return results;
    }

    case "groups": {
      // Search group names only, show all options
      const groupNames = entries.map(([name]) => name);
      const matchingGroupIndices = fuzzySearchStrings(
        groupNames,
        query.groupQuery
      );

      return entries
        .filter((_, idx) => matchingGroupIndices.has(idx))
        .map(([name, options]) => ({
          name,
          options,
          matchingOptionIndices: new Set(),
          showAllOptions: true,
        }));
    }

    case "groups-with-options": {
      // Search groups matching groupQuery that have options matching optionQuery
      const groupNames = entries.map(([name]) => name);
      const matchingGroupIndices = fuzzySearchStrings(
        groupNames,
        query.groupQuery
      );

      const results: FilteredVariable[] = [];

      for (let i = 0; i < entries.length; i++) {
        if (!matchingGroupIndices.has(i)) continue;

        const [name, options] = entries[i];

        if (!query.optionQuery) {
          // No option query - show all options for matching groups
          results.push({
            name,
            options,
            matchingOptionIndices: new Set(),
            showAllOptions: true,
          });
        } else {
          // Filter options within matching groups
          const matchingIndices = fuzzySearchStrings(options, query.optionQuery);
          if (matchingIndices.size > 0) {
            results.push({
              name,
              options,
              matchingOptionIndices: matchingIndices,
              showAllOptions: false,
            });
          }
        }
      }

      return results;
    }
  }
}

/**
 * Fuzzy search templates by name
 */
export function fuzzySearchTemplates<T extends { name: string }>(
  templates: T[],
  query: string
): T[] {
  if (!query.trim()) {
    return [...templates].sort((a, b) => a.name.localeCompare(b.name));
  }

  const fuseOptions: IFuseOptions<T> = {
    threshold: 0.4,
    distance: 100,
    includeScore: true,
    keys: ["name"],
  };

  const fuse = new Fuse(templates, fuseOptions);
  const results = fuse.search(query.trim());
  return results.map((r) => r.item);
}
