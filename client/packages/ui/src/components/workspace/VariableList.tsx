import { useState } from "react";
import {
  AtSign,
  ChevronDown,
  ChevronRight,
  Pencil,
  Plus,
} from "lucide-react";
import { Button } from "../ui/button";

interface VariableListProps {
  variables: Record<string, string[]>;
  onEditVariable: (name: string, options: string[], e: React.MouseEvent) => void;
  onCreateVariable: () => void;
}

export function VariableList({
  variables,
  onEditVariable,
  onCreateVariable,
}: VariableListProps) {
  const [expandedVariables, setExpandedVariables] = useState<Set<string>>(
    new Set()
  );

  const toggleVariableExpanded = (name: string, e: React.MouseEvent) => {
    e.stopPropagation();
    setExpandedVariables((prev) => {
      const next = new Set(prev);
      if (next.has(name)) {
        next.delete(name);
      } else {
        next.add(name);
      }
      return next;
    });
  };

  const sortedVariables = Object.entries(variables).sort(([a], [b]) =>
    a.localeCompare(b)
  );

  return (
    <>
      {sortedVariables.map(([name, options]) => {
        const isExpanded = expandedVariables.has(name);
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
              <span className="text-xs text-muted-foreground">
                {options.length}
              </span>
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
                ) : (
                  options.slice(0, 10).map((option, idx) => (
                    <div
                      key={idx}
                      className="px-2 py-0.5 text-xs text-muted-foreground truncate"
                      title={option}
                    >
                      {option}
                    </div>
                  ))
                )}
                {options.length > 10 && (
                  <p className="px-2 py-0.5 text-xs text-muted-foreground italic">
                    +{options.length - 10} more...
                  </p>
                )}
              </div>
            )}
          </div>
        );
      })}
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
