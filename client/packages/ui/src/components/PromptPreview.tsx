import { RefreshCw, Copy, Check } from "lucide-react";
import { useState } from "react";
import { Button } from "./ui/button";
import { ScrollArea } from "./ui/scroll-area";
import { useTemplateEditor } from "../hooks/useTemplateEditor";
import { cn } from "../lib/utils";

export function PromptPreview() {
  const { renderedOutput, isRendering, render } = useTemplateEditor();
  const [copied, setCopied] = useState(false);

  const handleCopy = async () => {
    if (!renderedOutput) return;
    await navigator.clipboard.writeText(renderedOutput);
    setCopied(true);
    setTimeout(() => setCopied(false), 2000);
  };

  const handleReroll = () => {
    render(Math.floor(Math.random() * 1000000));
  };

  return (
    <div className="flex h-full flex-col border-l">
      <div className="flex items-center justify-between border-b px-4 py-2">
        <h3 className="text-sm font-medium">Preview</h3>
        <div className="flex gap-1">
          <Button
            variant="ghost"
            size="icon"
            className="h-7 w-7"
            onClick={handleReroll}
            disabled={isRendering}
          >
            <RefreshCw
              className={cn("h-4 w-4", isRendering && "animate-spin")}
            />
          </Button>
          <Button
            variant="ghost"
            size="icon"
            className="h-7 w-7"
            onClick={handleCopy}
            disabled={!renderedOutput}
          >
            {copied ? (
              <Check className="h-4 w-4 text-green-500" />
            ) : (
              <Copy className="h-4 w-4" />
            )}
          </Button>
        </div>
      </div>
      <ScrollArea className="flex-1">
        <div className="p-4">
          {renderedOutput ? (
            <p className="whitespace-pre-wrap text-sm">{renderedOutput}</p>
          ) : (
            <p className="text-sm text-muted-foreground">
              Select a template and click render to see output
            </p>
          )}
        </div>
      </ScrollArea>
    </div>
  );
}
