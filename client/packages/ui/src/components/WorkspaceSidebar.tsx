import { useEffect, useState, useRef, useCallback } from "react";
import { ScrollArea } from "./ui/scroll-area";
import { Separator } from "./ui/separator";
import { useLibraries } from "../hooks/useLibraries";
import {
  useUIStore,
  MIN_SIDEBAR_WIDTH,
  MAX_SIDEBAR_WIDTH,
} from "../stores/useUIStore";
import { cn } from "../lib/utils";

// Workspace components
import {
  SidebarHeader,
  ViewModeToggle,
  LibrarySelector,
  SearchInput,
  TemplateList,
  VariableList,
  NoFolderSelected,
  LoadingState,
  NoLibraries,
  SelectLibrary,
} from "./workspace";

// Dialog components
import {
  CreateLibraryDialog,
  DeleteLibraryDialog,
  CreateTemplateDialog,
  EditTemplateDialog,
  DeleteTemplateDialog,
  CreateVariableDialog,
  EditVariableDialog,
  DeleteVariableDialog,
} from "./workspace/dialogs";

export function WorkspaceSidebar() {
  const {
    libraryHome,
    libraries,
    activeLibrary,
    selectedLibraryId,
    selectedTemplateId,
    isLoading,
    loadLibraryHome,
    pickFolder,
    setLibraryHome,
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
    updateTemplate,
    deleteTemplate,
  } = useLibraries();

  const { sidebarWidth, setSidebarWidth, sidebarViewMode, setSidebarViewMode } =
    useUIStore();

  // Resize state
  const [isResizing, setIsResizing] = useState(false);
  const sidebarRef = useRef<HTMLDivElement>(null);

  // Search state
  const [searchQuery, setSearchQuery] = useState("");

  // Dialog state
  const [createLibraryDialogOpen, setCreateLibraryDialogOpen] = useState(false);
  const [deleteLibraryDialogOpen, setDeleteLibraryDialogOpen] = useState(false);
  const [libraryToDelete, setLibraryToDelete] = useState<string | null>(null);

  const [createTemplateDialogOpen, setCreateTemplateDialogOpen] =
    useState(false);
  const [editTemplateDialogOpen, setEditTemplateDialogOpen] = useState(false);
  const [editingTemplate, setEditingTemplate] = useState<{
    id: string;
    name: string;
  } | null>(null);
  const [deleteTemplateDialogOpen, setDeleteTemplateDialogOpen] =
    useState(false);
  const [templateToDelete, setTemplateToDelete] = useState<string | null>(null);

  const [createVariableDialogOpen, setCreateVariableDialogOpen] =
    useState(false);
  const [editVariableDialogOpen, setEditVariableDialogOpen] = useState(false);
  const [editingVariable, setEditingVariable] = useState<{
    name: string;
    options: string[];
  } | null>(null);
  const [deleteVariableDialogOpen, setDeleteVariableDialogOpen] =
    useState(false);
  const [variableToDelete, setVariableToDelete] = useState<string | null>(null);

  // Load library home on mount
  useEffect(() => {
    loadLibraryHome();
  }, [loadLibraryHome]);

  // Load libraries when home is set
  useEffect(() => {
    if (libraryHome) {
      loadLibraries();
    }
  }, [libraryHome, loadLibraries]);

  // Auto-load persisted library selection
  useEffect(() => {
    if (libraries.length > 0 && selectedLibraryId && !activeLibrary) {
      const libraryExists = libraries.some(
        (lib) => lib.id === selectedLibraryId
      );
      if (libraryExists) {
        loadLibrary(selectedLibraryId);
      }
    }
  }, [libraries, selectedLibraryId, activeLibrary, loadLibrary]);

  // Resize handling
  const handleMouseDown = useCallback((e: React.MouseEvent) => {
    e.preventDefault();
    setIsResizing(true);
  }, []);

  useEffect(() => {
    if (!isResizing) return;

    const handleMouseMove = (e: MouseEvent) => {
      if (sidebarRef.current) {
        setSidebarWidth(e.clientX);
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

  // Handlers
  const handleFolderSelect = async () => {
    const path = await pickFolder();
    if (path) {
      await setLibraryHome(path);
    }
  };

  const handleLibraryChange = (id: string) => {
    if (id && activeLibrary?.id !== id) {
      loadLibrary(id);
    }
  };

  const handleCreateLibrary = async (name: string) => {
    await createLibrary(name);
  };

  const handleDeleteLibrary = async () => {
    if (!libraryToDelete) return;
    await deleteLibrary(libraryToDelete);
    setLibraryToDelete(null);
  };

  const handleEditTemplate = (
    id: string,
    name: string,
    e: React.MouseEvent
  ) => {
    e.stopPropagation();
    setEditingTemplate({ id, name });
    setEditTemplateDialogOpen(true);
  };

  const handleSaveTemplate = async (id: string, name: string) => {
    const template = activeLibrary?.templates.find((t) => t.id === id);
    if (template) {
      await updateTemplate(id, name, template.content);
    }
  };

  const handleDeleteTemplateFromEdit = (id: string) => {
    setTemplateToDelete(id);
    // Keep edit dialog open, layer delete dialog on top
    setDeleteTemplateDialogOpen(true);
  };

  const handleDeleteTemplate = async () => {
    if (!templateToDelete) return;
    await deleteTemplate(templateToDelete);
    setTemplateToDelete(null);
    // Close both dialogs after successful delete
    setDeleteTemplateDialogOpen(false);
    setEditTemplateDialogOpen(false);
    setEditingTemplate(null);
  };

  const handleCancelDeleteTemplate = () => {
    setDeleteTemplateDialogOpen(false);
    setTemplateToDelete(null);
    // Edit dialog stays open
  };

  const handleEditVariable = (
    name: string,
    options: string[],
    e: React.MouseEvent
  ) => {
    e.stopPropagation();
    setEditingVariable({ name, options });
    setEditVariableDialogOpen(true);
  };

  const handleSaveVariable = async (
    originalName: string,
    newName: string,
    options: string[]
  ) => {
    if (newName !== originalName) {
      await renamePromptGroup(originalName, newName);
      await updatePromptGroup(newName, options);
    } else {
      await updatePromptGroup(originalName, options);
    }
  };

  const handleDeleteVariableFromEdit = (name: string) => {
    setVariableToDelete(name);
    // Keep edit dialog open, layer delete dialog on top
    setDeleteVariableDialogOpen(true);
  };

  const handleDeleteVariable = async () => {
    if (!variableToDelete) return;
    await deletePromptGroup(variableToDelete);
    setVariableToDelete(null);
    // Close both dialogs after successful delete
    setDeleteVariableDialogOpen(false);
    setEditVariableDialogOpen(false);
    setEditingVariable(null);
  };

  const handleCancelDeleteVariable = () => {
    setDeleteVariableDialogOpen(false);
    setVariableToDelete(null);
    // Edit dialog stays open
  };

  const handleCreateTemplate = async (name: string) => {
    await createTemplate(name);
  };

  const handleCreateVariable = async (name: string) => {
    await createPromptGroup(name);
  };

  // Render content based on state
  const renderContent = () => {
    if (!libraryHome) {
      return <NoFolderSelected />;
    }

    if (isLoading && libraries.length === 0) {
      return <LoadingState />;
    }

    if (libraries.length === 0) {
      return <NoLibraries />;
    }

    if (!activeLibrary) {
      return <SelectLibrary />;
    }

    return (
      <div className="space-y-0.5">
        {sidebarViewMode === "templates" && (
          <TemplateList
            templates={activeLibrary.templates}
            selectedTemplateId={selectedTemplateId}
            onSelectTemplate={selectTemplate}
            onEditTemplate={handleEditTemplate}
            onCreateTemplate={() => setCreateTemplateDialogOpen(true)}
            searchQuery={searchQuery}
          />
        )}
        {sidebarViewMode === "variables" && (
          <VariableList
            variables={activeLibrary.wildcards}
            onEditVariable={handleEditVariable}
            onCreateVariable={() => setCreateVariableDialogOpen(true)}
            searchQuery={searchQuery}
          />
        )}
      </div>
    );
  };

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
      <SidebarHeader
        libraryHome={libraryHome}
        onFolderSelect={handleFolderSelect}
        onCreateLibrary={() => setCreateLibraryDialogOpen(true)}
      />

      {libraryHome && libraries.length > 0 && (
        <LibrarySelector
          libraries={libraries}
          activeLibraryId={activeLibrary?.id}
          onLibraryChange={handleLibraryChange}
        />
      )}

      {libraryHome && (
        <ViewModeToggle
          viewMode={sidebarViewMode}
          onViewModeChange={setSidebarViewMode}
          variablesCount={activeLibrary ? Object.keys(activeLibrary.wildcards).length : undefined}
          templatesCount={activeLibrary?.templates.length}
        />
      )}

      {activeLibrary && (
        <SearchInput
          value={searchQuery}
          onChange={setSearchQuery}
          placeholder={sidebarViewMode === "variables" ? "Search (@group/option)" : "Search..."}
        />
      )}

      <Separator />

      <ScrollArea className="flex-1">
        <div className="p-2">{renderContent()}</div>
      </ScrollArea>

      {/* Resize Handle */}
      <div
        className={cn(
          "absolute right-0 top-0 bottom-0 w-1 cursor-col-resize hover:bg-primary/20 transition-colors",
          isResizing && "bg-primary/30"
        )}
        onMouseDown={handleMouseDown}
      />

      {/* Library Dialogs */}
      <CreateLibraryDialog
        open={createLibraryDialogOpen}
        onOpenChange={setCreateLibraryDialogOpen}
        onCreateLibrary={handleCreateLibrary}
      />
      <DeleteLibraryDialog
        open={deleteLibraryDialogOpen}
        onOpenChange={setDeleteLibraryDialogOpen}
        onDeleteLibrary={handleDeleteLibrary}
      />

      {/* Template Dialogs */}
      <CreateTemplateDialog
        open={createTemplateDialogOpen}
        onOpenChange={setCreateTemplateDialogOpen}
        onCreateTemplate={handleCreateTemplate}
      />
      <EditTemplateDialog
        open={editTemplateDialogOpen}
        onOpenChange={setEditTemplateDialogOpen}
        template={editingTemplate}
        onSaveTemplate={handleSaveTemplate}
        onDeleteTemplate={handleDeleteTemplateFromEdit}
      />
      <DeleteTemplateDialog
        open={deleteTemplateDialogOpen}
        onOpenChange={setDeleteTemplateDialogOpen}
        templateName={editingTemplate?.name ?? null}
        onDeleteTemplate={handleDeleteTemplate}
        onCancel={handleCancelDeleteTemplate}
      />

      {/* Variable Dialogs */}
      <CreateVariableDialog
        open={createVariableDialogOpen}
        onOpenChange={setCreateVariableDialogOpen}
        onCreateVariable={handleCreateVariable}
      />
      <EditVariableDialog
        open={editVariableDialogOpen}
        onOpenChange={setEditVariableDialogOpen}
        variable={editingVariable}
        onSaveVariable={handleSaveVariable}
        onDeleteVariable={handleDeleteVariableFromEdit}
      />
      <DeleteVariableDialog
        open={deleteVariableDialogOpen}
        onOpenChange={setDeleteVariableDialogOpen}
        variableName={variableToDelete}
        onDeleteVariable={handleDeleteVariable}
        onCancel={handleCancelDeleteVariable}
      />
    </div>
  );
}
