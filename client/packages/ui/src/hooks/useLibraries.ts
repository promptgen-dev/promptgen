import { useCallback } from "react";
import { useBackend } from "@promptgen/backend";
import { useLibraryStore } from "../stores/useLibraryStore";

export function useLibraries() {
  const backend = useBackend();
  const {
    libraries,
    activeLibrary,
    isLoading,
    error,
    setLibraries,
    setActiveLibrary,
    setLoading,
    setError,
  } = useLibraryStore();

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
      } catch (e) {
        setError(e instanceof Error ? e.message : "Failed to load library");
      } finally {
        setLoading(false);
      }
    },
    [backend, setActiveLibrary, setLoading, setError]
  );

  const createLibrary = useCallback(
    async (name: string, path: string) => {
      setLoading(true);
      setError(null);
      try {
        const lib = await backend.createLibrary(name, path);
        setActiveLibrary(lib);
        await loadLibraries();
        return lib;
      } catch (e) {
        setError(e instanceof Error ? e.message : "Failed to create library");
        return null;
      } finally {
        setLoading(false);
      }
    },
    [backend, setActiveLibrary, setLoading, setError, loadLibraries]
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
        }
        await loadLibraries();
      } catch (e) {
        setError(e instanceof Error ? e.message : "Failed to delete library");
      } finally {
        setLoading(false);
      }
    },
    [backend, activeLibrary, setActiveLibrary, setLoading, setError, loadLibraries]
  );

  return {
    libraries,
    activeLibrary,
    isLoading,
    error,
    loadLibraries,
    loadLibrary,
    createLibrary,
    saveLibrary,
    deleteLibrary,
  };
}
