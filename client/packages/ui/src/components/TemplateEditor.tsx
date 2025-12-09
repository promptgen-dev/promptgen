import { useTemplateEditor } from "../hooks/useTemplateEditor";
import { cn } from "../lib/utils";

export function TemplateEditor() {
  const { editorContent, parseResult, updateContent } = useTemplateEditor();

  const hasErrors = parseResult && !parseResult.success;

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
      <div className="flex-1 p-4">
        <textarea
          value={editorContent}
          onChange={(e) => updateContent(e.target.value)}
          placeholder="Enter your prompt template here..."
          className={cn(
            "h-full w-full resize-none rounded-md border bg-transparent p-3 font-mono text-sm",
            "focus:outline-none focus:ring-2 focus:ring-ring",
            hasErrors && "border-destructive focus:ring-destructive"
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
