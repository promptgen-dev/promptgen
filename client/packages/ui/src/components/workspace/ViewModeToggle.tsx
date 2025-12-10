import { AtSign, FileText } from "lucide-react";
import { Button } from "../ui/button";
import type { SidebarViewMode } from "../../stores/useUIStore";

interface ViewModeToggleProps {
  viewMode: SidebarViewMode;
  onViewModeChange: (mode: SidebarViewMode) => void;
  variablesCount?: number;
  templatesCount?: number;
}

export function ViewModeToggle({
  viewMode,
  onViewModeChange,
  variablesCount,
  templatesCount,
}: ViewModeToggleProps) {
  return (
    <div className="flex px-3 pb-2 gap-1">
      <Button
        variant={viewMode === "variables" ? "secondary" : "ghost"}
        size="sm"
        className="flex-1 h-7 text-xs"
        onClick={() => onViewModeChange("variables")}
      >
        <AtSign className="h-3 w-3 mr-1" />
        Variables
        {variablesCount !== undefined && (
          <span className="ml-1 text-muted-foreground">[{variablesCount}]</span>
        )}
      </Button>
      <Button
        variant={viewMode === "templates" ? "secondary" : "ghost"}
        size="sm"
        className="flex-1 h-7 text-xs"
        onClick={() => onViewModeChange("templates")}
      >
        <FileText className="h-3 w-3 mr-1" />
        Templates
        {templatesCount !== undefined && (
          <span className="ml-1 text-muted-foreground">[{templatesCount}]</span>
        )}
      </Button>
    </div>
  );
}
