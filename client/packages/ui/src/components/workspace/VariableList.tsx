import { useState, useMemo, useEffect } from "react";
import {
  AtSign,
  ChevronDown,
  ChevronRight,
  Pencil,
  Plus,
} from "lucide-react";
import { Button } from "../ui/button";
import {
  parseVariableQuery,
  filterVariables,
  type FilteredVariable,
} from "../../lib/fuzzySearch";

interface VariableListProps {
  variables: Record<string, string[]>;
  onEditVariable: (name: string, options: string[], e: React.MouseEvent) => void;
  onCreateVariable: () => void;
  searchQuery?: string;
}

export function VariableList({
  variables,
  onEditVariable,
  onCreateVariable,
  searchQuery = "",
}: VariableListProps) {
  // Track collapsed variables (all expanded by default)
  const [collapsedVariables, setCollapsedVariables] = useState<Set<string>>(
    new Set()
  );

  const toggleVariableExpanded = (name: string, e: React.MouseEvent) => {
    e.stopPropagation();
    setCollapsedVariables((prev) => {
      const next = new Set(prev);
      if (next.has(name)) {
        next.delete(name);
      } else {
        next.add(name);
      }
      return next;
    });
  };

  // Parse and filter variables using WASM fuzzy search
  const filteredVariables = useMemo((): FilteredVariable[] => {
    const query = parseVariableQuery(searchQuery);
    return filterVariables(variables, query);
  }, [variables, searchQuery]);

  // Auto-expand variables with matching options (remove from collapsed set)
  useEffect(() => {
    const variablesWithMatches = filteredVariables
      .filter((v) => v.matchingOptionIndices.size > 0)
      .map((v) => v.name);

    if (variablesWithMatches.length > 0) {
      setCollapsedVariables((prev) => {
        const next = new Set(prev);
        variablesWithMatches.forEach((name) => next.delete(name));
        return next;
      });
    }
  }, [filteredVariables]);

  return (
    <>
      {filteredVariables.map((variable) => {
        const { name, options, matchingOptionIndices, showAllOptions } = variable;
        const isExpanded = !collapsedVariables.has(name);
        const hasMatchingOptions = matchingOptionIndices.size > 0;

        return (
          <div key={name}>
            <div
              className="group/group flex w-full items-center gap-1 rounded-md px-2 py-1 text-sm transition-colors hover:bg-accent hover:text-accent-foreground cursor-pointer"
              onClick={(e) => toggleVariableExpanded(name, e)}
            >
              {isExpanded ? (
                <ChevronDown className="h-3 w-3 shrink-0" />
              ) : (
                <ChevronRight className="h-3 w-3 shrink-0" />
              )}
              <AtSign className="h-3.5 w-3.5 shrink-0 text-muted-foreground" />
              <span className="flex-1 truncate">{name}</span>
              {hasMatchingOptions ? (
                <span className="text-xs text-primary">
                  {matchingOptionIndices.size} match
                  {matchingOptionIndices.size !== 1 ? "es" : ""}
                </span>
              ) : (
                <span className="text-xs text-muted-foreground">
                  {options.length}
                </span>
              )}
              <Button
                variant="ghost"
                size="icon"
                className="h-5 w-5 opacity-0 group-hover/group:opacity-100"
                onClick={(e) => onEditVariable(name, options, e)}
                title="Edit variable"
              >
                <Pencil className="h-3 w-3" />
              </Button>
            </div>
            {isExpanded && (
              <div className="ml-6 mt-0.5 space-y-0.5">
                {options.length === 0 ? (
                  <p className="px-2 py-1 text-xs text-muted-foreground italic">
                    No options yet
                  </p>
                ) : showAllOptions ? (
                  // Show all options (first 10 + "more")
                  <>
                    {options.slice(0, 10).map((option, idx) => (
                      <div
                        key={idx}
                        className="px-2 py-0.5 text-xs text-muted-foreground truncate"
                        title={option}
                      >
                        {option}
                      </div>
                    ))}
                    {options.length > 10 && (
                      <p className="px-2 py-0.5 text-xs text-muted-foreground italic">
                        +{options.length - 10} more...
                      </p>
                    )}
                  </>
                ) : (
                  // Show only matching options, highlighted
                  <>
                    {options.map((option, idx) => {
                      if (!matchingOptionIndices.has(idx)) return null;
                      return (
                        <div
                          key={idx}
                          className="px-2 py-0.5 text-xs text-primary font-medium truncate"
                          title={option}
                        >
                          {option}
                        </div>
                      );
                    })}
                  </>
                )}
              </div>
            )}
          </div>
        );
      })}
      {filteredVariables.length === 0 && searchQuery && (
        <p className="px-2 py-4 text-xs text-muted-foreground text-center">
          No variables match "{searchQuery}"
        </p>
      )}
      <button
        onClick={onCreateVariable}
        className="flex w-full items-center gap-2 rounded-md px-2 py-1 text-sm text-muted-foreground hover:bg-accent hover:text-accent-foreground transition-colors"
      >
        <Plus className="h-3.5 w-3.5 shrink-0" />
        <span>New variable</span>
      </button>
    </>
  );
}
