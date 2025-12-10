import {
  AlertDialog,
  AlertDialogAction,
  AlertDialogCancel,
  AlertDialogContent,
  AlertDialogDescription,
  AlertDialogFooter,
  AlertDialogHeader,
  AlertDialogTitle,
} from "../../ui/alert-dialog";

interface DeleteVariableDialogProps {
  open: boolean;
  onOpenChange: (open: boolean) => void;
  variableName: string | null;
  onDeleteVariable: () => Promise<void>;
  onCancel?: () => void;
}

export function DeleteVariableDialog({
  open,
  onOpenChange,
  variableName,
  onDeleteVariable,
  onCancel,
}: DeleteVariableDialogProps) {
  const handleDelete = async () => {
    await onDeleteVariable();
    // Parent handles closing both dialogs
  };

  const handleCancel = () => {
    if (onCancel) {
      onCancel();
    } else {
      onOpenChange(false);
    }
  };

  // Prevent onOpenChange from closing when clicking outside if we have a custom cancel handler
  const handleOpenChange = (newOpen: boolean) => {
    if (!newOpen && onCancel) {
      onCancel();
    } else {
      onOpenChange(newOpen);
    }
  };

  return (
    <AlertDialog open={open} onOpenChange={handleOpenChange}>
      <AlertDialogContent>
        <AlertDialogHeader>
          <AlertDialogTitle>Delete Variable</AlertDialogTitle>
          <AlertDialogDescription>
            Are you sure you want to delete the variable "{variableName}"? This
            action cannot be undone.
          </AlertDialogDescription>
        </AlertDialogHeader>
        <AlertDialogFooter>
          <AlertDialogCancel onClick={handleCancel}>Cancel</AlertDialogCancel>
          <AlertDialogAction
            onClick={handleDelete}
            className="bg-destructive text-destructive-foreground hover:bg-destructive/90"
          >
            Delete
          </AlertDialogAction>
        </AlertDialogFooter>
      </AlertDialogContent>
    </AlertDialog>
  );
}
