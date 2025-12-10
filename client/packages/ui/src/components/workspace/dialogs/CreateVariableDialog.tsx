import { useState } from "react";
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

interface CreateVariableDialogProps {
  open: boolean;
  onOpenChange: (open: boolean) => void;
  onCreateVariable: (name: string) => Promise<void>;
}

export function CreateVariableDialog({
  open,
  onOpenChange,
  onCreateVariable,
}: CreateVariableDialogProps) {
  const [name, setName] = useState("");

  const handleCreate = async () => {
    if (!name.trim()) return;
    await onCreateVariable(name.trim());
    setName("");
    onOpenChange(false);
  };

  return (
    <Dialog open={open} onOpenChange={onOpenChange}>
      <DialogContent>
        <DialogHeader>
          <DialogTitle>Create New Variable</DialogTitle>
          <DialogDescription>
            Enter a name for your new variable. You can add options after
            creating it.
          </DialogDescription>
        </DialogHeader>
        <div className="py-4">
          <Input
            placeholder="Variable name (e.g., colors, animals)"
            value={name}
            onChange={(e) => setName(e.target.value)}
            onKeyDown={(e) => {
              if (e.key === "Enter") {
                handleCreate();
              }
            }}
            autoFocus
          />
        </div>
        <DialogFooter>
          <Button variant="outline" onClick={() => onOpenChange(false)}>
            Cancel
          </Button>
          <Button onClick={handleCreate} disabled={!name.trim()}>
            Create
          </Button>
        </DialogFooter>
      </DialogContent>
    </Dialog>
  );
}
