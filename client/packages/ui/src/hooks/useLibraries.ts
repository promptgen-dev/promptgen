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
  };
}
