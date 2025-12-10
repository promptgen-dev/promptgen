import { useState, useEffect } from "react";
import * as YAML from "yaml";
import { Trash2 } from "lucide-react";
import { Button } from "../../ui/button";
import { Input } from "../../ui/input";
import { Textarea } from "../../ui/textarea";
import {
  Dialog,
  DialogContent,
  DialogDescription,
  DialogFooter,
  DialogHeader,
  DialogTitle,
} from "../../ui/dialog";

interface EditVariableDialogProps {
  open: boolean;
  onOpenChange: (open: boolean) => void;
  variable: { name: string; options: string[] } | null;
  onSaveVariable: (
    originalName: string,
    newName: string,
    options: string[]
  ) => Promise<void>;
  onDeleteVariable: (name: string) => void;
}

export function EditVariableDialog({
  open,
  onOpenChange,
  variable,
  onSaveVariable,
  onDeleteVariable,
}: EditVariableDialogProps) {
  const [name, setName] = useState("");
  const [yamlContent, setYamlContent] = useState("");
  const [error, setError] = useState<string | null>(null);

  useEffect(() => {
    if (variable) {
      setName(variable.name);
      const yaml =
        variable.options.length > 0
          ? YAML.stringify(variable.options)
          : "- option one\n- option two";
      setYamlContent(yaml);
      setError(null);
    }
  }, [variable]);

  const handleSave = async () => {
    if (!variable) return;
    setError(null);

    const trimmedName = name.trim();
    if (!trimmedName) {
      setError("Variable name cannot be empty");
      return;
    }

    try {
      const parsed = YAML.parse(yamlContent);
      if (!Array.isArray(parsed)) {
        setError("Content must be a YAML list (array)");
        return;
      }
      const options = parsed.map((item) => String(item));

      await onSaveVariable(variable.name, trimmedName, options);
      onOpenChange(false);
    } catch (e) {
      setError(e instanceof Error ? e.message : "Invalid YAML");
    }
  };

  const handleDelete = () => {
    if (!variable) return;
    onDeleteVariable(variable.name);
  };

  return (
    <Dialog open={open} onOpenChange={onOpenChange}>
      <DialogContent className="sm:max-w-lg">
        <DialogHeader>
          <DialogTitle>Edit Variable</DialogTitle>
          <DialogDescription>
            Edit the name and options for this variable.
          </DialogDescription>
        </DialogHeader>
        <div className="py-4 space-y-4">
          <div className="space-y-2">
            <label className="text-sm font-medium">Name</label>
            <Input
              placeholder="Variable name"
              value={name}
              onChange={(e) => setName(e.target.value)}
            />
          </div>
          <div className="space-y-2">
            <label className="text-sm font-medium">Options (YAML list)</label>
            <Textarea
              placeholder="- option1&#10;- option2&#10;- option3"
              value={yamlContent}
              onChange={(e) => setYamlContent(e.target.value)}
              className="min-h-[200px] font-mono text-sm"
            />
            <p className="text-xs text-muted-foreground">
              Use YAML list format: each option starts with "- " on a new line.
              Multi-line options can use "|" block scalar syntax.
            </p>
          </div>
          {error && <p className="text-sm text-destructive">{error}</p>}
        </div>
        <DialogFooter className="flex-col sm:flex-row gap-2">
          <Button
            variant="destructive"
            onClick={handleDelete}
            className="sm:mr-auto"
          >
            <Trash2 className="h-4 w-4 mr-2" />
            Delete
          </Button>
          <Button variant="outline" onClick={() => onOpenChange(false)}>
            Cancel
          </Button>
          <Button onClick={handleSave}>Save</Button>
        </DialogFooter>
      </DialogContent>
    </Dialog>
  );
}
