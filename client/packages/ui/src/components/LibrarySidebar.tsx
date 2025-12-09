import { useEffect } from "react";
import { FileText, FolderOpen, Plus } from "lucide-react";
import { Button } from "./ui/button";
import { ScrollArea } from "./ui/scroll-area";
import { Separator } from "./ui/separator";
import { useLibraries } from "../hooks/useLibraries";
import { cn } from "../lib/utils";

export function LibrarySidebar() {
  const {
    libraries,
    activeLibrary,
    isLoading,
    loadLibraries,
    loadLibrary,
  } = useLibraries();

  useEffect(() => {
    loadLibraries();
  }, [loadLibraries]);

  return (
    <div className="flex h-full w-64 flex-col border-r bg-muted/30">
      <div className="flex items-center justify-between p-4">
        <h2 className="text-sm font-semibold">Libraries</h2>
        <Button variant="ghost" size="icon" className="h-7 w-7">
          <Plus className="h-4 w-4" />
        </Button>
      </div>
      <Separator />
      <ScrollArea className="flex-1">
        <div className="p-2">
          {isLoading && libraries.length === 0 ? (
            <div className="p-4 text-center text-sm text-muted-foreground">
              Loading...
            </div>
          ) : libraries.length === 0 ? (
            <div className="p-4 text-center text-sm text-muted-foreground">
              No libraries yet
            </div>
          ) : (
            <div className="space-y-1">
              {libraries.map((lib) => (
                <button
                  key={lib.id}
                  onClick={() => loadLibrary(lib.id)}
                  className={cn(
                    "flex w-full items-center gap-2 rounded-md px-3 py-2 text-sm transition-colors",
                    "hover:bg-accent hover:text-accent-foreground",
                    activeLibrary?.id === lib.id &&
                      "bg-accent text-accent-foreground"
                  )}
                >
                  <FolderOpen className="h-4 w-4 shrink-0" />
                  <span className="truncate">{lib.name}</span>
                  <span className="ml-auto text-xs text-muted-foreground">
                    {lib.templateCount}
                  </span>
                </button>
              ))}
            </div>
          )}
        </div>
      </ScrollArea>
      {activeLibrary && (
        <>
          <Separator />
          <div className="p-2">
            <div className="mb-2 px-3 text-xs font-medium text-muted-foreground">
              Templates
            </div>
            <div className="space-y-1">
              {activeLibrary.templates.map((template) => (
                <button
                  key={template.id}
                  className={cn(
                    "flex w-full items-center gap-2 rounded-md px-3 py-1.5 text-sm transition-colors",
                    "hover:bg-accent hover:text-accent-foreground"
                  )}
                >
                  <FileText className="h-3.5 w-3.5 shrink-0" />
                  <span className="truncate">{template.name}</span>
                </button>
              ))}
            </div>
          </div>
        </>
      )}
    </div>
  );
}
