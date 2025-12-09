import { useCallback, useEffect, useRef } from "react";
import { useBackend } from "@promptgen/backend";
import { useTemplateStore } from "../stores/useTemplateStore";
import { useLibraryStore } from "../stores/useLibraryStore";

export function useTemplateEditor() {
  const backend = useBackend();
  const {
    activeTemplate,
    editorContent,
    parseResult,
    renderedOutput,
    isRendering,
    error,
    setActiveTemplate,
    setEditorContent,
    setParseResult,
    setRenderedOutput,
    setRendering,
    setError,
  } = useTemplateStore();

  const { activeLibrary } = useLibraryStore();
  const parseTimeoutRef = useRef<ReturnType<typeof setTimeout> | null>(null);

  // Debounced parse on content change
  useEffect(() => {
    if (parseTimeoutRef.current) {
      clearTimeout(parseTimeoutRef.current);
    }

    parseTimeoutRef.current = setTimeout(async () => {
      if (!editorContent) {
        setParseResult(null);
        return;
      }

      try {
        const result = await backend.parseTemplate(editorContent);
        setParseResult(result);
      } catch (e) {
        setError(e instanceof Error ? e.message : "Parse failed");
      }
    }, 150);

    return () => {
      if (parseTimeoutRef.current) {
        clearTimeout(parseTimeoutRef.current);
      }
    };
  }, [editorContent, backend, setParseResult, setError]);

  const render = useCallback(
    async (seed?: number) => {
      if (!activeTemplate || !activeLibrary) return;

      setRendering(true);
      setError(null);
      try {
        const result = await backend.renderTemplate({
          templateId: activeTemplate.id,
          libraryId: activeLibrary.id,
          seed,
        });

        if (result.success && result.output) {
          setRenderedOutput(result.output);
        } else {
          setError(result.error ?? "Render failed");
        }
      } catch (e) {
        setError(e instanceof Error ? e.message : "Render failed");
      } finally {
        setRendering(false);
      }
    },
    [
      backend,
      activeTemplate,
      activeLibrary,
      setRenderedOutput,
      setRendering,
      setError,
    ]
  );

  const updateContent = useCallback(
    (content: string) => {
      setEditorContent(content);
    },
    [setEditorContent]
  );

  return {
    activeTemplate,
    editorContent,
    parseResult,
    renderedOutput,
    isRendering,
    error,
    setActiveTemplate,
    updateContent,
    render,
  };
}
