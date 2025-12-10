import { useCallback } from "react";
import { useBackend } from "@promptgen/backend";
import { useLibraryStore } from "../stores/useLibraryStore";

export function useLibraries() {
  const backend = useBackend();
  const {
    libraryHome,
    libraries,
    activeLibrary,
    selectedTemplateId,
    isLoading,
    error,
    setLibraryHome: setLibraryHomeState,
    setLibraries,
    setActiveLibrary,
    setSelectedTemplateId,
    setLoading,
    setError,
  } = useLibraryStore();

  const loadLibraryHome = useCallback(async () => {
    if (!backend.getLibraryHome) return;
    try {
      const home = await backend.getLibraryHome();
      setLibraryHomeState(home);
    } catch (e) {
      console.error("Failed to get library home:", e);
    }
  }, [backend, setLibraryHomeState]);

  const setLibraryHome = useCallback(
    async (path: string) => {
      if (!backend.setLibraryHome) return;
      setLoading(true);
      setError(null);
      try {
        await backend.setLibraryHome(path);
        setLibraryHomeState(path);
        // Clear current state and reload libraries from new home
        setActiveLibrary(null);
        setSelectedTemplateId(null);
        const libs = await backend.listLibraries();
        setLibraries(libs);
      } catch (e) {
        setError(e instanceof Error ? e.message : "Failed to set library home");
      } finally {
        setLoading(false);
      }
    },
    [backend, setLibraryHomeState, setLibraries, setActiveLibrary, setSelectedTemplateId, setLoading, setError]
  );

  const pickFolder = useCallback(async () => {
    if (!backend.pickFolder) return null;
    try {
      return await backend.pickFolder();
    } catch (e) {
      console.error("Failed to pick folder:", e);
      return null;
    }
  }, [backend]);

  const loadLibraries = useCallback(async () => {
    setLoading(true);
    setError(null);
    try {
      const libs = await backend.listLibraries();
      setLibraries(libs);
    } catch (e) {
      setError(e instanceof Error ? e.message : "Failed to load libraries");
    } finally {
      setLoading(false);
    }
  }, [backend, setLibraries, setLoading, setError]);

  const loadLibrary = useCallback(
    async (id: string) => {
      setLoading(true);
      setError(null);
      try {
        const lib = await backend.loadLibrary(id);
        setActiveLibrary(lib);
        // Select first template if available
        if (lib.templates.length > 0) {
          setSelectedTemplateId(lib.templates[0].id);
        } else {
          setSelectedTemplateId(null);
        }
      } catch (e) {
        setError(e instanceof Error ? e.message : "Failed to load library");
      } finally {
        setLoading(false);
      }
    },
    [backend, setActiveLibrary, setSelectedTemplateId, setLoading, setError]
  );

  const createLibrary = useCallback(
    async (name: string) => {
      setLoading(true);
      setError(null);
      try {
        const lib = await backend.createLibrary(name);
        setActiveLibrary(lib);
        setSelectedTemplateId(null);
        await loadLibraries();
        return lib;
      } catch (e) {
        setError(e instanceof Error ? e.message : "Failed to create library");
        return null;
      } finally {
        setLoading(false);
      }
    },
    [backend, setActiveLibrary, setSelectedTemplateId, setLoading, setError, loadLibraries]
  );

  const saveLibrary = useCallback(async () => {
    if (!activeLibrary) return;
    setLoading(true);
    setError(null);
    try {
      await backend.saveLibrary(activeLibrary);
      await loadLibraries();
    } catch (e) {
      setError(e instanceof Error ? e.message : "Failed to save library");
    } finally {
      setLoading(false);
    }
  }, [backend, activeLibrary, setLoading, setError, loadLibraries]);

  const deleteLibrary = useCallback(
    async (id: string) => {
      setLoading(true);
      setError(null);
      try {
        await backend.deleteLibrary(id);
        if (activeLibrary?.id === id) {
          setActiveLibrary(null);
          setSelectedTemplateId(null);
        }
        await loadLibraries();
      } catch (e) {
        setError(e instanceof Error ? e.message : "Failed to delete library");
      } finally {
        setLoading(false);
      }
    },
    [backend, activeLibrary, setActiveLibrary, setSelectedTemplateId, setLoading, setError, loadLibraries]
  );

  const selectTemplate = useCallback(
    (templateId: string | null) => {
      setSelectedTemplateId(templateId);
    },
    [setSelectedTemplateId]
  );

  // Prompt Group CRUD operations
  const createPromptGroup = useCallback(
    async (name: string) => {
      if (!activeLibrary || !backend.createPromptGroup) return null;
      setLoading(true);
      setError(null);
      try {
        const group = await backend.createPromptGroup(activeLibrary.id, name);
        // Reload the library to get updated groups
        const lib = await backend.loadLibrary(activeLibrary.id);
        setActiveLibrary(lib);
        return group;
      } catch (e) {
        setError(e instanceof Error ? e.message : "Failed to create prompt group");
        return null;
      } finally {
        setLoading(false);
      }
    },
    [backend, activeLibrary, setActiveLibrary, setLoading, setError]
  );

  const updatePromptGroup = useCallback(
    async (name: string, options: string[]) => {
      if (!activeLibrary || !backend.updatePromptGroup) return null;
      setLoading(true);
      setError(null);
      try {
        const group = await backend.updatePromptGroup(activeLibrary.id, name, options);
        // Reload the library to get updated groups
        const lib = await backend.loadLibrary(activeLibrary.id);
        setActiveLibrary(lib);
        return group;
      } catch (e) {
        setError(e instanceof Error ? e.message : "Failed to update prompt group");
        return null;
      } finally {
        setLoading(false);
      }
    },
    [backend, activeLibrary, setActiveLibrary, setLoading, setError]
  );

  const renamePromptGroup = useCallback(
    async (oldName: string, newName: string) => {
      if (!activeLibrary || !backend.renamePromptGroup) return null;
      setLoading(true);
      setError(null);
      try {
        const group = await backend.renamePromptGroup(activeLibrary.id, oldName, newName);
        // Reload the library to get updated groups
        const lib = await backend.loadLibrary(activeLibrary.id);
        setActiveLibrary(lib);
        return group;
      } catch (e) {
        setError(e instanceof Error ? e.message : "Failed to rename prompt group");
        return null;
      } finally {
        setLoading(false);
      }
    },
    [backend, activeLibrary, setActiveLibrary, setLoading, setError]
  );

  const deletePromptGroup = useCallback(
    async (name: string) => {
      if (!activeLibrary || !backend.deletePromptGroup) return;
      setLoading(true);
      setError(null);
      try {
        await backend.deletePromptGroup(activeLibrary.id, name);
        // Reload the library to get updated groups
        const lib = await backend.loadLibrary(activeLibrary.id);
        setActiveLibrary(lib);
      } catch (e) {
        setError(e instanceof Error ? e.message : "Failed to delete prompt group");
      } finally {
        setLoading(false);
      }
    },
    [backend, activeLibrary, setActiveLibrary, setLoading, setError]
  );

  // Template CRUD operations
  const createTemplate = useCallback(
    async (name: string, content: string = "") => {
      if (!activeLibrary || !backend.createTemplate) return null;
      setLoading(true);
      setError(null);
      try {
        const template = await backend.createTemplate(activeLibrary.id, name, content);
        // Reload the library to get updated templates
        const lib = await backend.loadLibrary(activeLibrary.id);
        setActiveLibrary(lib);
        setSelectedTemplateId(template.id);
        await loadLibraries();
        return template;
      } catch (e) {
        setError(e instanceof Error ? e.message : "Failed to create template");
        return null;
      } finally {
        setLoading(false);
      }
    },
    [backend, activeLibrary, setActiveLibrary, setSelectedTemplateId, setLoading, setError, loadLibraries]
  );

  const updateTemplate = useCallback(
    async (templateId: string, name: string, content: string) => {
      if (!activeLibrary || !backend.updateTemplate) return null;
      setLoading(true);
      setError(null);
      try {
        const template = await backend.updateTemplate(activeLibrary.id, templateId, name, content);
        // Reload the library to get updated templates
        const lib = await backend.loadLibrary(activeLibrary.id);
        setActiveLibrary(lib);
        return template;
      } catch (e) {
        setError(e instanceof Error ? e.message : "Failed to update template");
        return null;
      } finally {
        setLoading(false);
      }
    },
    [backend, activeLibrary, setActiveLibrary, setLoading, setError]
  );

  const deleteTemplate = useCallback(
    async (templateId: string) => {
      if (!activeLibrary || !backend.deleteTemplate) return;
      setLoading(true);
      setError(null);
      try {
        await backend.deleteTemplate(activeLibrary.id, templateId);
        // Reload the library to get updated templates
        const lib = await backend.loadLibrary(activeLibrary.id);
        setActiveLibrary(lib);
        // Select another template if the deleted one was selected
        if (selectedTemplateId === templateId) {
          if (lib.templates.length > 0) {
            setSelectedTemplateId(lib.templates[0].id);
          } else {
            setSelectedTemplateId(null);
          }
        }
        await loadLibraries();
      } catch (e) {
        setError(e instanceof Error ? e.message : "Failed to delete template");
      } finally {
        setLoading(false);
      }
    },
    [backend, activeLibrary, selectedTemplateId, setActiveLibrary, setSelectedTemplateId, setLoading, setError, loadLibraries]
  );

  return {
    libraryHome,
    libraries,
    activeLibrary,
    selectedTemplateId,
    isLoading,
    error,
    loadLibraryHome,
    setLibraryHome,
    pickFolder,
    loadLibraries,
    loadLibrary,
    createLibrary,
    saveLibrary,
    deleteLibrary,
    selectTemplate,
    // Prompt Group CRUD
    createPromptGroup,
    updatePromptGroup,
    renamePromptGroup,
    deletePromptGroup,
    // Template CRUD
    createTemplate,
    updateTemplate,
    deleteTemplate,
  };
}
