import { useEffect, useState } from "react";
import {
  FileText,
  FolderOpen,
  Plus,
  Trash2,
  FolderCog,
  ChevronRight,
  ChevronDown,
} from "lucide-react";
import { Button } from "./ui/button";
import { ScrollArea } from "./ui/scroll-area";
import { Separator } from "./ui/separator";
import { Input } from "./ui/input";
import {
  Dialog,
  DialogContent,
  DialogDescription,
  DialogFooter,
  DialogHeader,
  DialogTitle,
} from "./ui/dialog";
import {
  AlertDialog,
  AlertDialogAction,
  AlertDialogCancel,
  AlertDialogContent,
  AlertDialogDescription,
  AlertDialogFooter,
  AlertDialogHeader,
  AlertDialogTitle,
} from "./ui/alert-dialog";
import { useLibraries } from "../hooks/useLibraries";
import { cn } from "../lib/utils";

export function LibrarySidebar() {
  const {
    libraryHome,
    libraries,
    activeLibrary,
    selectedTemplateId,
    isLoading,
    loadLibraryHome,
    setLibraryHome,
    pickFolder,
    loadLibraries,
    loadLibrary,
    createLibrary,
    deleteLibrary,
    selectTemplate,
  } = useLibraries();

  const [expandedLibraries, setExpandedLibraries] = useState<Set<string>>(
    new Set()
  );
  const [newLibraryDialogOpen, setNewLibraryDialogOpen] = useState(false);
  const [newLibraryName, setNewLibraryName] = useState("");
  const [deleteDialogOpen, setDeleteDialogOpen] = useState(false);
  const [libraryToDelete, setLibraryToDelete] = useState<string | null>(null);

  useEffect(() => {
    loadLibraryHome();
  }, [loadLibraryHome]);

  useEffect(() => {
    if (libraryHome) {
      loadLibraries();
    }
  }, [libraryHome, loadLibraries]);

  // Auto-expand active library
  useEffect(() => {
    if (activeLibrary) {
      setExpandedLibraries((prev) => new Set(prev).add(activeLibrary.id));
    }
  }, [activeLibrary]);

  const handleFolderSelect = async () => {
    const path = await pickFolder();
    if (path) {
      await setLibraryHome(path);
    }
  };

  const handleCreateLibrary = async () => {
    if (!newLibraryName.trim()) return;
    await createLibrary(newLibraryName.trim());
    setNewLibraryName("");
    setNewLibraryDialogOpen(false);
  };

  const handleDeleteLibrary = async () => {
    if (!libraryToDelete) return;
    await deleteLibrary(libraryToDelete);
    setLibraryToDelete(null);
    setDeleteDialogOpen(false);
  };

  const toggleLibraryExpanded = (id: string) => {
    setExpandedLibraries((prev) => {
      const next = new Set(prev);
      if (next.has(id)) {
        next.delete(id);
      } else {
        next.add(id);
      }
      return next;
    });
  };

  const handleLibraryClick = (id: string) => {
    if (activeLibrary?.id !== id) {
      loadLibrary(id);
    }
    toggleLibraryExpanded(id);
  };

  const confirmDeleteLibrary = (id: string, e: React.MouseEvent) => {
    e.stopPropagation();
    setLibraryToDelete(id);
    setDeleteDialogOpen(true);
  };

  return (
    <div className="flex h-full w-64 flex-col border-r bg-muted/30">
      {/* Header */}
      <div className="flex items-center justify-between p-4">
        <h2 className="text-sm font-semibold">Libraries</h2>
        <div className="flex gap-1">
          <Button
            variant="ghost"
            size="icon"
            className="h-7 w-7"
            onClick={handleFolderSelect}
            title="Set library home folder"
          >
            <FolderCog className="h-4 w-4" />
          </Button>
          <Button
            variant="ghost"
            size="icon"
            className="h-7 w-7"
            onClick={() => setNewLibraryDialogOpen(true)}
            disabled={!libraryHome}
            title={libraryHome ? "Create new library" : "Set library home first"}
          >
            <Plus className="h-4 w-4" />
          </Button>
        </div>
      </div>

      {/* Library Home Path */}
      {libraryHome && (
        <div className="px-4 pb-2">
          <p className="truncate text-xs text-muted-foreground" title={libraryHome}>
            {libraryHome}
          </p>
        </div>
      )}

      <Separator />

      {/* Libraries List */}
      <ScrollArea className="flex-1">
        <div className="p-2">
          {!libraryHome ? (
            <div className="p-4 text-center text-sm text-muted-foreground">
              <FolderCog className="mx-auto mb-2 h-8 w-8 opacity-50" />
              <p>Select a folder to use as your library home</p>
            </div>
          ) : isLoading && libraries.length === 0 ? (
            <div className="p-4 text-center text-sm text-muted-foreground">
              Loading...
            </div>
          ) : libraries.length === 0 ? (
            <div className="p-4 text-center text-sm text-muted-foreground">
              <p>No libraries yet</p>
              <p className="mt-1 text-xs">Click + to create one</p>
            </div>
          ) : (
            <div className="space-y-1">
              {libraries.map((lib) => {
                const isExpanded = expandedLibraries.has(lib.id);
                const isActive = activeLibrary?.id === lib.id;

                return (
                  <div key={lib.id}>
                    {/* Library Row */}
                    <div
                      className={cn(
                        "group flex w-full items-center gap-1 rounded-md px-2 py-1.5 text-sm transition-colors cursor-pointer",
                        "hover:bg-accent hover:text-accent-foreground",
                        isActive && "bg-accent text-accent-foreground"
                      )}
                      onClick={() => handleLibraryClick(lib.id)}
                    >
                      {isExpanded ? (
                        <ChevronDown className="h-4 w-4 shrink-0" />
                      ) : (
                        <ChevronRight className="h-4 w-4 shrink-0" />
                      )}
                      <FolderOpen className="h-4 w-4 shrink-0" />
                      <span className="flex-1 truncate">{lib.name}</span>
                      <span className="text-xs text-muted-foreground">
                        {lib.templateCount}
                      </span>
                      <Button
                        variant="ghost"
                        size="icon"
                        className="h-6 w-6 opacity-0 group-hover:opacity-100"
                        onClick={(e) => confirmDeleteLibrary(lib.id, e)}
                      >
                        <Trash2 className="h-3 w-3" />
                      </Button>
                    </div>

                    {/* Templates (when expanded and loaded) */}
                    {isExpanded && isActive && activeLibrary.templates.length > 0 && (
                      <div className="ml-6 mt-1 space-y-0.5">
                        {activeLibrary.templates.map((template) => (
                          <button
                            key={template.id}
                            onClick={() => selectTemplate(template.id)}
                            className={cn(
                              "flex w-full items-center gap-2 rounded-md px-2 py-1 text-sm transition-colors",
                              "hover:bg-accent hover:text-accent-foreground",
                              selectedTemplateId === template.id &&
                                "bg-primary/10 text-primary"
                            )}
                          >
                            <FileText className="h-3.5 w-3.5 shrink-0" />
                            <span className="truncate">{template.name}</span>
                          </button>
                        ))}
                      </div>
                    )}
                  </div>
                );
              })}
            </div>
          )}
        </div>
      </ScrollArea>

      {/* Create Library Dialog */}
      <Dialog open={newLibraryDialogOpen} onOpenChange={setNewLibraryDialogOpen}>
        <DialogContent>
          <DialogHeader>
            <DialogTitle>Create New Library</DialogTitle>
            <DialogDescription>
              Enter a name for your new prompt library.
            </DialogDescription>
          </DialogHeader>
          <div className="py-4">
            <Input
              placeholder="Library name"
              value={newLibraryName}
              onChange={(e) => setNewLibraryName(e.target.value)}
              onKeyDown={(e) => {
                if (e.key === "Enter") {
                  handleCreateLibrary();
                }
              }}
              autoFocus
            />
          </div>
          <DialogFooter>
            <Button
              variant="outline"
              onClick={() => setNewLibraryDialogOpen(false)}
            >
              Cancel
            </Button>
            <Button
              onClick={handleCreateLibrary}
              disabled={!newLibraryName.trim()}
            >
              Create
            </Button>
          </DialogFooter>
        </DialogContent>
      </Dialog>

      {/* Delete Confirmation Dialog */}
      <AlertDialog open={deleteDialogOpen} onOpenChange={setDeleteDialogOpen}>
        <AlertDialogContent>
          <AlertDialogHeader>
            <AlertDialogTitle>Delete Library</AlertDialogTitle>
            <AlertDialogDescription>
              Are you sure you want to delete this library? This action cannot be
              undone and will permanently delete the library file.
            </AlertDialogDescription>
          </AlertDialogHeader>
          <AlertDialogFooter>
            <AlertDialogCancel>Cancel</AlertDialogCancel>
            <AlertDialogAction
              onClick={handleDeleteLibrary}
              className="bg-destructive text-destructive-foreground hover:bg-destructive/90"
            >
              Delete
            </AlertDialogAction>
          </AlertDialogFooter>
        </AlertDialogContent>
      </AlertDialog>
    </div>
  );
}
