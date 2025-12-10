import { FolderCog, Plus } from "lucide-react";
import { Button } from "../ui/button";

function getLastPathSegment(path: string): string {
  const segments = path.split(/[/\\]/).filter(Boolean);
  return segments[segments.length - 1] || path;
}

interface SidebarHeaderProps {
  libraryHome: string | null;
  onFolderSelect: () => void;
  onCreateLibrary: () => void;
}

export function SidebarHeader({
  libraryHome,
  onFolderSelect,
  onCreateLibrary,
}: SidebarHeaderProps) {
  const folderName = libraryHome ? getLastPathSegment(libraryHome) : null;

  return (
    <div className="flex items-center justify-between gap-2 p-3">
      <div className="flex-1 min-w-0">
        <h2 className="text-sm font-semibold">Workspace</h2>
        {folderName ? (
          <p
            className="truncate text-xs text-muted-foreground"
            title={libraryHome || undefined}
          >
            {folderName}
          </p>
        ) : (
          <p className="text-xs text-muted-foreground italic">
            No folder selected
          </p>
        )}
      </div>
      <div className="flex shrink-0 gap-1">
        <Button
          variant="ghost"
          size="icon"
          className="h-7 w-7"
          onClick={onFolderSelect}
          title="Set workspace folder"
        >
          <FolderCog className="h-4 w-4" />
        </Button>
        <Button
          variant="ghost"
          size="icon"
          className="h-7 w-7"
          onClick={onCreateLibrary}
          disabled={!libraryHome}
          title={libraryHome ? "Create new library" : "Set workspace folder first"}
        >
          <Plus className="h-4 w-4" />
        </Button>
      </div>
    </div>
  );
}
