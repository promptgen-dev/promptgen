//! Dialog state for confirmation and input dialogs.

/// Pending library action (used with ConfirmDialog::OpenNewLibrary)
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PendingLibraryAction {
    /// Show the file picker dialog (user hasn't selected a file yet)
    OpenFilePicker,
    /// Show the create library dialog
    ShowCreateDialog,
}

/// Active confirmation dialog
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ConfirmDialog {
    /// Confirm discarding unsaved variable editor changes
    DiscardVariableChanges,
    /// Confirm deleting a variable
    DeleteVariable { variable_name: String },
    /// Confirm closing a tab with unsaved changes
    CloseUnsavedTab { tab_index: usize },
    /// Confirm deleting a prompt from the library
    DeletePrompt { prompt_name: String },
    /// Confirm discarding unsaved tabs when opening/creating a new library
    OpenNewLibrary {
        /// The pending action after confirmation
        pending_action: PendingLibraryAction,
    },
}

/// State for all dialogs in the application
#[derive(Debug, Clone, Default)]
pub struct DialogState {
    /// Currently active confirmation dialog (if any)
    pub confirm: Option<ConfirmDialog>,
}

impl DialogState {
    /// Close the current confirmation dialog
    pub fn close_confirm(&mut self) {
        self.confirm = None;
    }

    /// Request to delete a variable (shows confirmation dialog)
    pub fn request_delete_variable(&mut self, variable_name: &str) {
        self.confirm = Some(ConfirmDialog::DeleteVariable {
            variable_name: variable_name.to_string(),
        });
    }

    /// Request to delete a prompt (shows confirmation dialog)
    pub fn request_delete_prompt(&mut self, prompt_name: &str) {
        self.confirm = Some(ConfirmDialog::DeletePrompt {
            prompt_name: prompt_name.to_string(),
        });
    }

    /// Request to close an unsaved tab (shows confirmation dialog)
    pub fn request_close_unsaved_tab(&mut self, tab_index: usize) {
        self.confirm = Some(ConfirmDialog::CloseUnsavedTab { tab_index });
    }

    /// Request to discard variable changes (shows confirmation dialog)
    pub fn request_discard_variable_changes(&mut self) {
        self.confirm = Some(ConfirmDialog::DiscardVariableChanges);
    }

    /// Clear dialog state
    pub fn clear(&mut self) {
        self.confirm = None;
    }
}
