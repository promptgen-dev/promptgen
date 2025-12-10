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

interface DeleteLibraryDialogProps {
  open: boolean;
  onOpenChange: (open: boolean) => void;
  onDeleteLibrary: () => Promise<void>;
}

export function DeleteLibraryDialog({
  open,
  onOpenChange,
  onDeleteLibrary,
}: DeleteLibraryDialogProps) {
  const handleDelete = async () => {
    await onDeleteLibrary();
    onOpenChange(false);
  };

  return (
    <AlertDialog open={open} onOpenChange={onOpenChange}>
      <AlertDialogContent>
        <AlertDialogHeader>
          <AlertDialogTitle>Delete Library</AlertDialogTitle>
          <AlertDialogDescription>
            Are you sure you want to delete this library? This action cannot be
            undone and will permanently delete the library file.
          </AlertDialogDescription>
        </AlertDialogHeader>
        <AlertDialogFooter>
          <AlertDialogCancel>Cancel</AlertDialogCancel>
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
