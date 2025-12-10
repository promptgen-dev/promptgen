import { FolderOpen } from "lucide-react";
import {
  Select,
  SelectContent,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from "../ui/select";
import type { LibrarySummary } from "@promptgen/backend";

interface LibrarySelectorProps {
  libraries: LibrarySummary[];
  activeLibraryId: string | undefined;
  onLibraryChange: (id: string) => void;
}

export function LibrarySelector({
  libraries,
  activeLibraryId,
  onLibraryChange,
}: LibrarySelectorProps) {
  const sortedLibraries = [...libraries].sort((a, b) =>
    a.name.localeCompare(b.name)
  );

  return (
    <div className="px-3 pb-2">
      <Select value={activeLibraryId || ""} onValueChange={onLibraryChange}>
        <SelectTrigger className="h-8 text-xs">
          <FolderOpen className="h-3.5 w-3.5 mr-2 shrink-0" />
          <SelectValue placeholder="Select a library" />
        </SelectTrigger>
        <SelectContent>
          {sortedLibraries.map((lib) => (
            <SelectItem key={lib.id} value={lib.id}>
              {lib.name}
            </SelectItem>
          ))}
        </SelectContent>
      </Select>
    </div>
  );
}
