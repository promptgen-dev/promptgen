import { FileText, Pencil, Plus } from "lucide-react";
import { Button } from "../ui/button";
import { cn } from "../../lib/utils";
import type { Template } from "@promptgen/backend";

interface TemplateListProps {
  templates: Template[];
  selectedTemplateId: string | null;
  onSelectTemplate: (id: string) => void;
  onEditTemplate: (id: string, name: string, e: React.MouseEvent) => void;
  onCreateTemplate: () => void;
}

export function TemplateList({
  templates,
  selectedTemplateId,
  onSelectTemplate,
  onEditTemplate,
  onCreateTemplate,
}: TemplateListProps) {
  const sortedTemplates = [...templates].sort((a, b) =>
    a.name.localeCompare(b.name)
  );

  return (
    <>
      {sortedTemplates.map((template) => (
        <div
          key={template.id}
          onClick={() => onSelectTemplate(template.id)}
          className={cn(
            "group/template flex w-full items-center gap-2 rounded-md px-2 py-1 text-sm transition-colors cursor-pointer",
            "hover:bg-accent hover:text-accent-foreground",
            selectedTemplateId === template.id && "bg-primary/10 text-primary"
          )}
        >
          <FileText className="h-3.5 w-3.5 shrink-0" />
          <span className="flex-1 truncate">{template.name}</span>
          <Button
            variant="ghost"
            size="icon"
            className="h-5 w-5 opacity-0 group-hover/template:opacity-100"
            onClick={(e) => onEditTemplate(template.id, template.name, e)}
            title="Edit template"
          >
            <Pencil className="h-3 w-3" />
          </Button>
        </div>
      ))}
      <button
        onClick={onCreateTemplate}
        className="flex w-full items-center gap-2 rounded-md px-2 py-1 text-sm text-muted-foreground hover:bg-accent hover:text-accent-foreground transition-colors"
      >
        <Plus className="h-3.5 w-3.5 shrink-0" />
        <span>New template</span>
      </button>
    </>
  );
}
