import { useEffect, useState, useRef, useCallback } from "react";
import {
  FileText,
  FolderOpen,
  Plus,
  Trash2,
  FolderCog,
  ChevronRight,
  ChevronDown,
  Pencil,
  AtSign,
} from "lucide-react";
import * as YAML from "yaml";
import { Button } from "./ui/button";
import { ScrollArea } from "./ui/scroll-area";
import { Separator } from "./ui/separator";
import { Input } from "./ui/input";
import { Textarea } from "./ui/textarea";
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
import { useUIStore, MIN_SIDEBAR_WIDTH, MAX_SIDEBAR_WIDTH } from "../stores/useUIStore";
import { cn } from "../lib/utils";

function getLastPathSegment(path: string): string {
  // Handle both Unix and Windows paths
  const segments = path.split(/[/\\]/).filter(Boolean);
  return segments[segments.length - 1] || path;
}

export function WorkspaceSidebar() {
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
    createPromptGroup,
    updatePromptGroup,
    renamePromptGroup,
    deletePromptGroup,
    createTemplate,
    deleteTemplate,
  } = useLibraries();

  const { sidebarWidth, setSidebarWidth, sidebarViewMode, setSidebarViewMode } = useUIStore();

  const [expandedLibraries, setExpandedLibraries] = useState<Set<string>>(
    new Set()
  );
  const [newLibraryDialogOpen, setNewLibraryDialogOpen] = useState(false);
  const [newLibraryName, setNewLibraryName] = useState("");
  const [deleteDialogOpen, setDeleteDialogOpen] = useState(false);
  const [libraryToDelete, setLibraryToDelete] = useState<string | null>(null);
  const [isResizing, setIsResizing] = useState(false);

  // Template state
  const [newTemplateDialogOpen, setNewTemplateDialogOpen] = useState(false);
  const [newTemplateName, setNewTemplateName] = useState("");
  const [deleteTemplateDialogOpen, setDeleteTemplateDialogOpen] = useState(false);
  const [templateToDelete, setTemplateToDelete] = useState<string | null>(null);

  // Prompt Group state
  const [newGroupDialogOpen, setNewGroupDialogOpen] = useState(false);
  const [newGroupName, setNewGroupName] = useState("");
  const [deleteGroupDialogOpen, setDeleteGroupDialogOpen] = useState(false);
  const [groupToDelete, setGroupToDelete] = useState<string | null>(null);
  const [expandedVariables, setExpandedVariables] = useState<Set<string>>(new Set());
  const [editGroupDialogOpen, setEditGroupDialogOpen] = useState(false);
  const [editingGroup, setEditingGroup] = useState<{ name: string; options: string[] } | null>(null);
  const [editGroupName, setEditGroupName] = useState("");
  const [editGroupYaml, setEditGroupYaml] = useState("");
  const [editGroupError, setEditGroupError] = useState<string | null>(null);

  const sidebarRef = useRef<HTMLDivElement>(null);

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

  // Resize handling
  const handleMouseDown = useCallback((e: React.MouseEvent) => {
    e.preventDefault();
    setIsResizing(true);
  }, []);

  useEffect(() => {
    if (!isResizing) return;

    const handleMouseMove = (e: MouseEvent) => {
      if (sidebarRef.current) {
        const newWidth = e.clientX;
        setSidebarWidth(newWidth);
      }
    };

    const handleMouseUp = () => {
      setIsResizing(false);
    };

    document.addEventListener("mousemove", handleMouseMove);
    document.addEventListener("mouseup", handleMouseUp);

    return () => {
      document.removeEventListener("mousemove", handleMouseMove);
      document.removeEventListener("mouseup", handleMouseUp);
    };
  }, [isResizing, setSidebarWidth]);

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

  // Template handlers
  const handleCreateTemplate = async () => {
    if (!newTemplateName.trim()) return;
    await createTemplate(newTemplateName.trim());
    setNewTemplateName("");
    setNewTemplateDialogOpen(false);
  };

  const handleDeleteTemplate = async () => {
    if (!templateToDelete) return;
    await deleteTemplate(templateToDelete);
    setTemplateToDelete(null);
    setDeleteTemplateDialogOpen(false);
  };

  const confirmDeleteTemplate = (id: string, e: React.MouseEvent) => {
    e.stopPropagation();
    setTemplateToDelete(id);
    setDeleteTemplateDialogOpen(true);
  };

  // Prompt Group handlers
  const handleCreateGroup = async () => {
    if (!newGroupName.trim()) return;
    await createPromptGroup(newGroupName.trim());
    setNewGroupName("");
    setNewGroupDialogOpen(false);
  };

  const handleDeleteGroup = async () => {
    if (!groupToDelete) return;
    await deletePromptGroup(groupToDelete);
    setGroupToDelete(null);
    setDeleteGroupDialogOpen(false);
  };

  const confirmDeleteGroup = (name: string, e: React.MouseEvent) => {
    e.stopPropagation();
    setGroupToDelete(name);
    setDeleteGroupDialogOpen(true);
  };

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

  const openEditGroupDialog = (name: string, options: string[], e: React.MouseEvent) => {
    e.stopPropagation();
    setEditingGroup({ name, options });
    setEditGroupName(name);
    // Convert options array to YAML list format, or use default if empty
    const yamlContent = options.length > 0
      ? YAML.stringify(options)
      : "- option one\n- option two";
    setEditGroupYaml(yamlContent);
    setEditGroupError(null);
    setEditGroupDialogOpen(true);
  };

  const handleSaveGroup = async () => {
    if (!editingGroup) return;
    setEditGroupError(null);

    const trimmedName = editGroupName.trim();
    if (!trimmedName) {
      setEditGroupError("Variable name cannot be empty");
      return;
    }

    try {
      const parsed = YAML.parse(editGroupYaml);
      if (!Array.isArray(parsed)) {
        setEditGroupError("Content must be a YAML list (array)");
        return;
      }
      // Ensure all items are strings
      const options = parsed.map((item) => String(item));

      // If name changed, rename first then update options
      if (trimmedName !== editingGroup.name) {
        await renamePromptGroup(editingGroup.name, trimmedName);
        await updatePromptGroup(trimmedName, options);
      } else {
        await updatePromptGroup(editingGroup.name, options);
      }

      setEditGroupDialogOpen(false);
      setEditingGroup(null);
      setEditGroupName("");
      setEditGroupYaml("");
    } catch (e) {
      setEditGroupError(e instanceof Error ? e.message : "Invalid YAML");
    }
  };

  const folderName = libraryHome ? getLastPathSegment(libraryHome) : null;

  return (
    <div
      ref={sidebarRef}
      className="relative flex h-full flex-col border-r bg-muted/30"
      style={{
        width: sidebarWidth,
        minWidth: MIN_SIDEBAR_WIDTH,
        maxWidth: MAX_SIDEBAR_WIDTH,
      }}
    >
      {/* Header */}
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
            <p className="text-xs text-muted-foreground italic">No folder selected</p>
          )}
        </div>
        <div className="flex shrink-0 gap-1">
          <Button
            variant="ghost"
            size="icon"
            className="h-7 w-7"
            onClick={handleFolderSelect}
            title="Set workspace folder"
          >
            <FolderCog className="h-4 w-4" />
          </Button>
          <Button
            variant="ghost"
            size="icon"
            className="h-7 w-7"
            onClick={() => setNewLibraryDialogOpen(true)}
            disabled={!libraryHome}
            title={libraryHome ? "Create new library" : "Set workspace folder first"}
          >
            <Plus className="h-4 w-4" />
          </Button>
        </div>
      </div>

      {/* View Mode Toggle */}
      {libraryHome && (
        <div className="flex px-3 pb-2 gap-1">
          <Button
            variant={sidebarViewMode === "templates" ? "secondary" : "ghost"}
            size="sm"
            className="flex-1 h-7 text-xs"
            onClick={() => setSidebarViewMode("templates")}
          >
            <FileText className="h-3 w-3 mr-1" />
            Templates
          </Button>
          <Button
            variant={sidebarViewMode === "variables" ? "secondary" : "ghost"}
            size="sm"
            className="flex-1 h-7 text-xs"
            onClick={() => setSidebarViewMode("variables")}
          >
            <AtSign className="h-3 w-3 mr-1" />
            Variables
          </Button>
        </div>
      )}

      <Separator />

      {/* Libraries List */}
      <ScrollArea className="flex-1">
        <div className="p-2">
          {!libraryHome ? (
            <div className="p-4 text-center text-sm text-muted-foreground">
              <FolderCog className="mx-auto mb-2 h-8 w-8 opacity-50" />
              <p>Select a folder to use as your workspace</p>
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

                    {/* Expanded library content */}
                    {isExpanded && isActive && (
                      <div className="ml-6 mt-1 space-y-0.5">
                        {/* Templates view */}
                        {sidebarViewMode === "templates" && (
                          <>
                            {activeLibrary.templates.map((template) => (
                              <div
                                key={template.id}
                                onClick={() => selectTemplate(template.id)}
                                className={cn(
                                  "group/template flex w-full items-center gap-2 rounded-md px-2 py-1 text-sm transition-colors cursor-pointer",
                                  "hover:bg-accent hover:text-accent-foreground",
                                  selectedTemplateId === template.id &&
                                  "bg-primary/10 text-primary"
                                )}
                              >
                                <FileText className="h-3.5 w-3.5 shrink-0" />
                                <span className="flex-1 truncate">{template.name}</span>
                                <Button
                                  variant="ghost"
                                  size="icon"
                                  className="h-5 w-5 opacity-0 group-hover/template:opacity-100"
                                  onClick={(e) => confirmDeleteTemplate(template.id, e)}
                                >
                                  <Trash2 className="h-3 w-3" />
                                </Button>
                              </div>
                            ))}
                            {/* Add template button */}
                            <button
                              onClick={() => setNewTemplateDialogOpen(true)}
                              className="flex w-full items-center gap-2 rounded-md px-2 py-1 text-sm text-muted-foreground hover:bg-accent hover:text-accent-foreground transition-colors"
                            >
                              <Plus className="h-3.5 w-3.5 shrink-0" />
                              <span>New template</span>
                            </button>
                          </>
                        )}

                        {/* Variables view */}
                        {sidebarViewMode === "variables" && (
                          <>
                            {Object.entries(activeLibrary.wildcards).map(([name, options]) => {
                              const isVariableExpanded = expandedVariables.has(name);
                              return (
                                <div key={name}>
                                  <div
                                    className="group/group flex w-full items-center gap-1 rounded-md px-2 py-1 text-sm transition-colors hover:bg-accent hover:text-accent-foreground cursor-pointer"
                                    onClick={(e) => toggleVariableExpanded(name, e)}
                                  >
                                    {isVariableExpanded ? (
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
                                      onClick={(e) => openEditGroupDialog(name, options, e)}
                                      title="Edit variable"
                                    >
                                      <Pencil className="h-3 w-3" />
                                    </Button>
                                    <Button
                                      variant="ghost"
                                      size="icon"
                                      className="h-5 w-5 opacity-0 group-hover/group:opacity-100"
                                      onClick={(e) => confirmDeleteGroup(name, e)}
                                      title="Delete variable"
                                    >
                                      <Trash2 className="h-3 w-3" />
                                    </Button>
                                  </div>
                                  {/* Expanded options */}
                                  {isVariableExpanded && (
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
                            {/* Add variable button */}
                            <button
                              onClick={() => setNewGroupDialogOpen(true)}
                              className="flex w-full items-center gap-2 rounded-md px-2 py-1 text-sm text-muted-foreground hover:bg-accent hover:text-accent-foreground transition-colors"
                            >
                              <Plus className="h-3.5 w-3.5 shrink-0" />
                              <span>New variable</span>
                            </button>
                          </>
                        )}
                      </div>
                    )}
                  </div>
                );
              })}
            </div>
          )}
        </div>
      </ScrollArea>

      {/* Resize Handle */}
      <div
        className={cn(
          "absolute right-0 top-0 bottom-0 w-1 cursor-col-resize hover:bg-primary/20 transition-colors",
          isResizing && "bg-primary/30"
        )}
        onMouseDown={handleMouseDown}
      />

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

      {/* Delete Library Confirmation Dialog */}
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

      {/* Create Template Dialog */}
      <Dialog open={newTemplateDialogOpen} onOpenChange={setNewTemplateDialogOpen}>
        <DialogContent>
          <DialogHeader>
            <DialogTitle>Create New Template</DialogTitle>
            <DialogDescription>
              Enter a name for your new prompt template.
            </DialogDescription>
          </DialogHeader>
          <div className="py-4">
            <Input
              placeholder="Template name"
              value={newTemplateName}
              onChange={(e) => setNewTemplateName(e.target.value)}
              onKeyDown={(e) => {
                if (e.key === "Enter") {
                  handleCreateTemplate();
                }
              }}
              autoFocus
            />
          </div>
          <DialogFooter>
            <Button
              variant="outline"
              onClick={() => setNewTemplateDialogOpen(false)}
            >
              Cancel
            </Button>
            <Button
              onClick={handleCreateTemplate}
              disabled={!newTemplateName.trim()}
            >
              Create
            </Button>
          </DialogFooter>
        </DialogContent>
      </Dialog>

      {/* Delete Template Confirmation Dialog */}
      <AlertDialog open={deleteTemplateDialogOpen} onOpenChange={setDeleteTemplateDialogOpen}>
        <AlertDialogContent>
          <AlertDialogHeader>
            <AlertDialogTitle>Delete Template</AlertDialogTitle>
            <AlertDialogDescription>
              Are you sure you want to delete this template? This action cannot be undone.
            </AlertDialogDescription>
          </AlertDialogHeader>
          <AlertDialogFooter>
            <AlertDialogCancel>Cancel</AlertDialogCancel>
            <AlertDialogAction
              onClick={handleDeleteTemplate}
              className="bg-destructive text-destructive-foreground hover:bg-destructive/90"
            >
              Delete
            </AlertDialogAction>
          </AlertDialogFooter>
        </AlertDialogContent>
      </AlertDialog>

      {/* Create Variable Dialog */}
      <Dialog open={newGroupDialogOpen} onOpenChange={setNewGroupDialogOpen}>
        <DialogContent>
          <DialogHeader>
            <DialogTitle>Create New Variable</DialogTitle>
            <DialogDescription>
              Enter a name for your new variable. You can add options after creating it.
            </DialogDescription>
          </DialogHeader>
          <div className="py-4">
            <Input
              placeholder="Variable name (e.g., colors, animals)"
              value={newGroupName}
              onChange={(e) => setNewGroupName(e.target.value)}
              onKeyDown={(e) => {
                if (e.key === "Enter") {
                  handleCreateGroup();
                }
              }}
              autoFocus
            />
          </div>
          <DialogFooter>
            <Button
              variant="outline"
              onClick={() => setNewGroupDialogOpen(false)}
            >
              Cancel
            </Button>
            <Button
              onClick={handleCreateGroup}
              disabled={!newGroupName.trim()}
            >
              Create
            </Button>
          </DialogFooter>
        </DialogContent>
      </Dialog>

      {/* Delete Variable Confirmation Dialog */}
      <AlertDialog open={deleteGroupDialogOpen} onOpenChange={setDeleteGroupDialogOpen}>
        <AlertDialogContent>
          <AlertDialogHeader>
            <AlertDialogTitle>Delete Variable</AlertDialogTitle>
            <AlertDialogDescription>
              Are you sure you want to delete the variable "{groupToDelete}"? This action cannot be undone.
            </AlertDialogDescription>
          </AlertDialogHeader>
          <AlertDialogFooter>
            <AlertDialogCancel>Cancel</AlertDialogCancel>
            <AlertDialogAction
              onClick={handleDeleteGroup}
              className="bg-destructive text-destructive-foreground hover:bg-destructive/90"
            >
              Delete
            </AlertDialogAction>
          </AlertDialogFooter>
        </AlertDialogContent>
      </AlertDialog>

      {/* Edit Variable Dialog */}
      <Dialog open={editGroupDialogOpen} onOpenChange={setEditGroupDialogOpen}>
        <DialogContent className="sm:max-w-lg">
          <DialogHeader>
            <DialogTitle>Edit Variable</DialogTitle>
            <DialogDescription>
              Edit the name and options for this variable.
            </DialogDescription>
          </DialogHeader>
          <div className="py-4 space-y-4">
            <div className="space-y-2">
              <label className="text-sm font-medium">Name</label>
              <Input
                placeholder="Variable name"
                value={editGroupName}
                onChange={(e) => setEditGroupName(e.target.value)}
              />
            </div>
            <div className="space-y-2">
              <label className="text-sm font-medium">Options (YAML list)</label>
              <Textarea
                placeholder="- option1&#10;- option2&#10;- option3"
                value={editGroupYaml}
                onChange={(e) => setEditGroupYaml(e.target.value)}
                className="min-h-[200px] font-mono text-sm"
              />
              <p className="text-xs text-muted-foreground">
                Use YAML list format: each option starts with "- " on a new line.
                Multi-line options can use "|" block scalar syntax.
              </p>
            </div>
            {editGroupError && (
              <p className="text-sm text-destructive">{editGroupError}</p>
            )}
          </div>
          <DialogFooter>
            <Button
              variant="outline"
              onClick={() => {
                setEditGroupDialogOpen(false);
                setEditingGroup(null);
                setEditGroupName("");
                setEditGroupYaml("");
                setEditGroupError(null);
              }}
            >
              Cancel
            </Button>
            <Button onClick={handleSaveGroup}>
              Save
            </Button>
          </DialogFooter>
        </DialogContent>
      </Dialog>
    </div>
  );
}
