import { useState, useEffect } from "react";
import { Trash2 } from "lucide-react";
import { Button } from "../../ui/button";
import { Input } from "../../ui/input";
import {
  Dialog,
  DialogContent,
  DialogDescription,
  DialogFooter,
  DialogHeader,
  DialogTitle,
} from "../../ui/dialog";

interface EditTemplateDialogProps {
  open: boolean;
  onOpenChange: (open: boolean) => void;
  template: { id: string; name: string } | null;
  onSaveTemplate: (id: string, name: string) => Promise<void>;
  onDeleteTemplate: (id: string) => void;
}

export function EditTemplateDialog({
  open,
  onOpenChange,
  template,
  onSaveTemplate,
  onDeleteTemplate,
}: EditTemplateDialogProps) {
  const [name, setName] = useState("");

  useEffect(() => {
    if (template) {
      setName(template.name);
    }
  }, [template]);

  const handleSave = async () => {
    if (!template || !name.trim()) return;
    await onSaveTemplate(template.id, name.trim());
    onOpenChange(false);
  };

  const handleDelete = () => {
    if (!template) return;
    onDeleteTemplate(template.id);
  };

  return (
    <Dialog open={open} onOpenChange={onOpenChange}>
      <DialogContent>
        <DialogHeader>
          <DialogTitle>Edit Template</DialogTitle>
          <DialogDescription>Rename this template.</DialogDescription>
        </DialogHeader>
        <div className="py-4">
          <Input
            placeholder="Template name"
            value={name}
            onChange={(e) => setName(e.target.value)}
            onKeyDown={(e) => {
              if (e.key === "Enter") {
                handleSave();
              }
            }}
            autoFocus
          />
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
          <Button onClick={handleSave} disabled={!name.trim()}>
            Save
          </Button>
        </DialogFooter>
      </DialogContent>
    </Dialog>
  );
}
