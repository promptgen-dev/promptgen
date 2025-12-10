import { FolderCog } from "lucide-react";

export function NoFolderSelected() {
  return (
    <div className="p-4 text-center text-sm text-muted-foreground">
      <FolderCog className="mx-auto mb-2 h-8 w-8 opacity-50" />
      <p>Select a folder to use as your workspace</p>
    </div>
  );
}

export function LoadingState() {
  return (
    <div className="p-4 text-center text-sm text-muted-foreground">
      Loading...
    </div>
  );
}

export function NoLibraries() {
  return (
    <div className="p-4 text-center text-sm text-muted-foreground">
      <p>No libraries yet</p>
      <p className="mt-1 text-xs">Click + to create one</p>
    </div>
  );
}

export function SelectLibrary() {
  return (
    <div className="p-4 text-center text-sm text-muted-foreground">
      <p>Select a library above</p>
    </div>
  );
}
