import { LibrarySidebar } from "./components/LibrarySidebar";
import { TemplateEditor } from "./components/TemplateEditor";
import { PromptPreview } from "./components/PromptPreview";

export function App() {
  return (
    <div className="flex h-screen bg-background text-foreground">
      {/* Sidebar */}
      <LibrarySidebar />

      {/* Main content area */}
      <div className="flex flex-1 flex-col">
        {/* Header */}
        <header className="flex h-12 items-center border-b px-4">
          <h1 className="text-lg font-semibold">PromptGen</h1>
        </header>

        {/* Editor and Preview */}
        <div className="flex flex-1 overflow-hidden">
          <div className="flex-1">
            <TemplateEditor />
          </div>
          <div className="w-80">
            <PromptPreview />
          </div>
        </div>
      </div>
    </div>
  );
}
