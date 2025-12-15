import { useEffect, useRef } from "react";
import { EditorState } from "@codemirror/state";
import { EditorView, keymap, lineNumbers, highlightActiveLine, highlightActiveLineGutter, placeholder } from "@codemirror/view";
import { defaultKeymap, history, historyKeymap } from "@codemirror/commands";
import { searchKeymap, highlightSelectionMatches } from "@codemirror/search";
import { bracketMatching } from "@codemirror/language";
import { useTemplateEditor } from "../../hooks/useTemplateEditor";
import { cn } from "../../lib/utils";
import { editorTheme } from "./theme";
import { promptgenLanguage } from "./language";
import { promptgenHighlighting } from "./highlighting";

export function TemplateEditor() {
  const containerRef = useRef<HTMLDivElement>(null);
  const viewRef = useRef<EditorView | null>(null);
  const { editorContent, parseResult, updateContent } = useTemplateEditor();

  const hasErrors = parseResult && !parseResult.success;

  // Create editor on mount
  useEffect(() => {
    if (!containerRef.current) return;

    const updateListener = EditorView.updateListener.of((update) => {
      if (update.docChanged) {
        updateContent(update.state.doc.toString());
      }
    });

    const state = EditorState.create({
      doc: editorContent,
      extensions: [
        lineNumbers(),
        highlightActiveLine(),
        highlightActiveLineGutter(),
        history(),
        bracketMatching(),
        highlightSelectionMatches(),
        promptgenLanguage,
        promptgenHighlighting,
        keymap.of([
          ...defaultKeymap,
          ...historyKeymap,
          ...searchKeymap,
        ]),
        editorTheme,
        updateListener,
        EditorView.lineWrapping,
        placeholder("Enter your prompt template here...\n\nUse @GroupName to reference variables.\nUse {option1|option2|option3} for inline choices.\nUse {{ slot_name }} for user-filled slots."),
      ],
    });

    const view = new EditorView({
      state,
      parent: containerRef.current,
    });

    viewRef.current = view;

    return () => {
      view.destroy();
      viewRef.current = null;
    };
  }, []); // Only run on mount

  // Sync external content changes to editor
  useEffect(() => {
    const view = viewRef.current;
    if (!view) return;

    const currentContent = view.state.doc.toString();
    if (currentContent !== editorContent) {
      view.dispatch({
        changes: {
          from: 0,
          to: currentContent.length,
          insert: editorContent,
        },
      });
    }
  }, [editorContent]);

  return (
    <div className="flex h-full flex-col">
      <div className="flex items-center justify-between border-b px-4 py-2">
        <h3 className="text-sm font-medium">Editor</h3>
        {hasErrors && (
          <span className="text-xs text-destructive">
            {parseResult.errors?.length ?? 0} error(s)
          </span>
        )}
      </div>
      <div className="flex-1 overflow-hidden">
        <div
          ref={containerRef}
          className={cn(
            "h-full w-full",
            hasErrors && "[&_.cm-editor]:border-destructive"
          )}
        />
      </div>
      {hasErrors && parseResult.errors && (
        <div className="border-t bg-destructive/10 p-3">
          <div className="space-y-1">
            {parseResult.errors.map((err, i) => (
              <p key={i} className="text-xs text-destructive">
                Line {err.span.start}: {err.message}
              </p>
            ))}
          </div>
        </div>
      )}
    </div>
  );
}
