use std::path::PathBuf;

use crate::components::{
    EditorPanel, HelpWindow, PreviewPanel, SidebarPanel, SlotPanel, TabBarPanel,
    VariableEditorPanel, dialogs, help,
};
use egui_commonmark::CommonMarkCache;
use crate::state::{
    AppState, ConfirmDialog, EditorMode, PendingLibraryAction, PromptTab, SidebarViewMode,
    VariableSortOrder,
};
use crate::theme;

#[cfg(not(target_arch = "wasm32"))]
use crate::storage::{NativeStorage, StorageBackend};

/// Main application struct - implements eframe::App
#[derive(Default, serde::Deserialize, serde::Serialize)]
#[serde(default)]
pub struct PromptGenApp {
    /// Persisted library file path
    library_file_path: Option<PathBuf>,

    /// Persisted prompt tabs (global, not per-library)
    #[serde(default)]
    prompt_tabs: Vec<PromptTab>,

    /// Persisted active tab index
    #[serde(default)]
    active_tab_index: Option<usize>,

    /// Persisted variable sort order
    #[serde(default)]
    variable_sort_order: VariableSortOrder,

    /// Persisted auto-copy setting
    #[serde(default)]
    auto_copy: bool,

    /// Persisted sidebar view mode (Prompts vs Variables)
    #[serde(default)]
    sidebar_view_mode: SidebarViewMode,

    /// Persisted slot picker sort order (separate from sidebar variable sort)
    #[serde(default)]
    slot_picker_sort_order: VariableSortOrder,

    /// Persisted slot picker option sort order (separate from sidebar option sort)
    #[serde(default)]
    slot_picker_option_sort_order: VariableSortOrder,

    /// Persisted option sort order (for variable options in sidebar)
    #[serde(default)]
    option_sort_order: VariableSortOrder,

    /// Persisted prompt sort order
    #[serde(default)]
    prompt_sort_order: VariableSortOrder,

    #[serde(skip)]
    state: AppState,

    #[cfg(not(target_arch = "wasm32"))]
    #[serde(skip)]
    storage: NativeStorage,

    // Create Library dialog state (ephemeral)
    #[cfg(not(target_arch = "wasm32"))]
    #[serde(skip)]
    show_create_library_dialog: bool,

    #[cfg(not(target_arch = "wasm32"))]
    #[serde(skip)]
    create_library_name: String,

    #[cfg(not(target_arch = "wasm32"))]
    #[serde(skip)]
    create_library_path: Option<PathBuf>,

    // Edit Library dialog state (ephemeral)
    #[cfg(not(target_arch = "wasm32"))]
    #[serde(skip)]
    show_edit_library_dialog: bool,

    #[cfg(not(target_arch = "wasm32"))]
    #[serde(skip)]
    edit_library_name: String,

    // Overwrite confirmation dialog state
    #[cfg(not(target_arch = "wasm32"))]
    #[serde(skip)]
    show_overwrite_confirm: bool,

    #[cfg(not(target_arch = "wasm32"))]
    #[serde(skip)]
    pending_rename_path: Option<PathBuf>,

    // Help system state (ephemeral)
    #[serde(skip)]
    show_getting_started_help: bool,

    #[serde(skip)]
    help_cache: CommonMarkCache,
}

impl PromptGenApp {
    /// Called once before the first frame.
    pub fn new(cc: &eframe::CreationContext<'_>) -> Self {
        // Initialize material icons font
        egui_material_icons::initialize(&cc.egui_ctx);

        // Apply custom font sizes
        theme::apply_font_sizes(&cc.egui_ctx);

        // Load previous app state (if any).
        let mut app: Self = if let Some(storage) = cc.storage {
            eframe::get_value(storage, eframe::APP_KEY).unwrap_or_default()
        } else {
            Self::default()
        };

        // If we have a saved library path, try to load it
        #[cfg(not(target_arch = "wasm32"))]
        if let Some(path) = &app.library_file_path {
            app.storage.set_library_path(path.clone());
            app.load_library();
        }

        // Restore persisted tabs
        app.restore_tabs();

        app
    }

    /// Restore tabs from persisted data into AppState
    fn restore_tabs(&mut self) {
        // Restore persisted settings
        self.state.variable_sort_order = self.variable_sort_order;
        self.state.auto_copy = self.auto_copy;
        self.state.sidebar_view_mode = self.sidebar_view_mode;
        self.state.slot_picker_sort_order = self.slot_picker_sort_order;
        self.state.slot_picker_option_sort_order = self.slot_picker_option_sort_order;
        self.state.option_sort_order = self.option_sort_order;
        self.state.prompt_sort_order = self.prompt_sort_order;

        // If we have persisted tabs, restore them
        if !self.prompt_tabs.is_empty() {
            self.state.prompt_tabs = self.prompt_tabs.clone();

            // Validate and restore active tab index
            if let Some(idx) = self.active_tab_index {
                if idx < self.state.prompt_tabs.len() {
                    self.state.active_tab_index = Some(idx);
                } else {
                    // Index out of bounds, use last tab
                    self.state.active_tab_index = Some(self.state.prompt_tabs.len() - 1);
                }
            } else if !self.state.prompt_tabs.is_empty() {
                // No saved index, default to first tab
                self.state.active_tab_index = Some(0);
            }

            // Sync editor content with active tab
            if let Some(idx) = self.state.active_tab_index
                && let Some(tab) = self.state.prompt_tabs.get(idx)
            {
                self.state.editor_content = tab.content.clone();
                self.state.slot_values = crate::state::AppState::slot_values_to_vec_map(&tab.slots);
                self.state.update_parse_result();
                // Render immediately so preview is populated on app launch
                self.state.request_render();
            }
        }
        // If no persisted tabs, the AppState default already creates "Prompt 1"
    }

    /// Save tabs from AppState to persisted fields
    fn save_tabs(&mut self) {
        // Sync current slot values to active tab before saving
        self.state.save_slot_values_to_active_tab();

        self.prompt_tabs = self.state.prompt_tabs.clone();
        self.active_tab_index = self.state.active_tab_index;
        self.variable_sort_order = self.state.variable_sort_order;
        self.auto_copy = self.state.auto_copy;
        self.sidebar_view_mode = self.state.sidebar_view_mode;
        self.slot_picker_sort_order = self.state.slot_picker_sort_order;
        self.slot_picker_option_sort_order = self.state.slot_picker_option_sort_order;
        self.option_sort_order = self.state.option_sort_order;
        self.prompt_sort_order = self.state.prompt_sort_order;
    }

    /// Request to open a library - checks for unsaved changes first, then shows file picker
    #[cfg(not(target_arch = "wasm32"))]
    fn open_library_dialog(&mut self) {
        if self.state.has_unsaved_tabs() {
            // Show confirmation dialog first, file picker will open after
            self.state.confirm_dialog = Some(ConfirmDialog::OpenNewLibrary {
                pending_action: PendingLibraryAction::OpenFilePicker,
            });
        } else {
            // No unsaved changes, show file picker directly
            self.show_open_file_picker();
        }
    }

    /// Actually show the file picker and load the selected library
    #[cfg(not(target_arch = "wasm32"))]
    fn show_open_file_picker(&mut self) {
        if let Some(path) = rfd::FileDialog::new()
            .set_title("Open Library File")
            .add_filter("YAML files", &["yaml", "yml"])
            .pick_file()
        {
            self.set_library_path(path);
        }
    }

    /// Set the library path and load it (resets state first)
    #[cfg(not(target_arch = "wasm32"))]
    fn set_library_path(&mut self, path: PathBuf) {
        // Reset to blank state before loading new library
        self.state.reset_to_blank();

        self.library_file_path = Some(path.clone());
        self.storage.set_library_path(path);
        self.load_library();
    }

    /// Load the library from the current file path
    #[cfg(not(target_arch = "wasm32"))]
    fn load_library(&mut self) {
        match self.storage.load_library() {
            Ok((library, path)) => {
                self.state.library = library;
                self.state.library_path = Some(path);
            }
            Err(e) => {
                log::error!("Failed to load library: {}", e);
                self.state.library = promptgen_core::Library::default();
                self.state.library_path = None;
            }
        }
    }

    /// Save all dirty tabs to the library
    #[cfg(not(target_arch = "wasm32"))]
    fn save_all_dirty_tabs(&mut self) {
        // Save each dirty tab
        for idx in 0..self.state.prompt_tabs.len() {
            if self.state.prompt_tabs[idx].dirty {
                self.state.switch_to_tab(idx);
                self.state.save_active_tab_to_library();
            }
        }

        // Persist library to disk
        if let Some(path) = &self.state.library_path
            && let Err(e) = promptgen_core::save_library(&self.state.library, path)
        {
            log::error!("Failed to save library: {}", e);
        }
    }

    /// Render the create library dialog and handle actions
    #[cfg(not(target_arch = "wasm32"))]
    fn render_create_library_dialog(&mut self, ctx: &egui::Context) {
        use dialogs::CreateLibraryAction;

        let action = dialogs::render_create_library_dialog(
            ctx,
            &mut self.show_create_library_dialog,
            &mut self.create_library_name,
            &mut self.create_library_path,
        );

        match action {
            CreateLibraryAction::Create { name, path } => {
                // Close the dialog first
                self.show_create_library_dialog = false;
                self.create_library_name.clear();
                self.create_library_path = None;
                // Create the library (unsaved check already happened before showing this dialog)
                self.create_library(name, path);
            }
            CreateLibraryAction::Cancel => {
                self.show_create_library_dialog = false;
                self.create_library_name.clear();
                self.create_library_path = None;
            }
            CreateLibraryAction::None => {}
        }
    }

    /// Request to show the create library dialog - checks for unsaved changes first
    #[cfg(not(target_arch = "wasm32"))]
    fn request_show_create_dialog(&mut self) {
        if self.state.has_unsaved_tabs() {
            // Show confirmation dialog first, create dialog will open after
            self.state.confirm_dialog = Some(ConfirmDialog::OpenNewLibrary {
                pending_action: PendingLibraryAction::ShowCreateDialog,
            });
        } else {
            // No unsaved changes, show create dialog directly
            self.show_create_library_dialog = true;
        }
    }

    /// Create a new library file (resets state first)
    #[cfg(not(target_arch = "wasm32"))]
    fn create_library(&mut self, name: String, path: PathBuf) {
        // Reset to blank state before creating new library
        self.state.reset_to_blank();

        // Create empty library
        let library = promptgen_core::Library {
            name: name.clone(),
            description: String::new(),
            variables: Vec::new(),
            prompts: Vec::new(),
        };

        // Save to disk
        match promptgen_core::save_library(&library, &path) {
            Ok(()) => {
                // Load the new library
                self.state.library = library;
                self.state.library_path = Some(path.clone());
                self.library_file_path = Some(path);
            }
            Err(e) => {
                log::error!("Failed to create library: {}", e);
            }
        }
    }

    /// Render the edit library dialog and handle actions
    #[cfg(not(target_arch = "wasm32"))]
    fn render_edit_library_dialog(&mut self, ctx: &egui::Context) {
        use dialogs::EditLibraryAction;

        let action = dialogs::render_edit_library_dialog(
            ctx,
            &mut self.show_edit_library_dialog,
            &mut self.edit_library_name,
            &self.state.library.name,
        );

        match action {
            EditLibraryAction::Save { new_name } => {
                self.rename_library(new_name);
            }
            EditLibraryAction::Cancel => {
                self.show_edit_library_dialog = false;
                self.edit_library_name.clear();
            }
            EditLibraryAction::None => {}
        }
    }

    /// Rename the library and its file
    #[cfg(not(target_arch = "wasm32"))]
    fn rename_library(&mut self, new_name: String) {
        let Some(current_path) = &self.state.library_path else {
            log::error!("Cannot rename library: no library path");
            return;
        };

        // Generate new filename based on new library name
        let parent = current_path
            .parent()
            .unwrap_or_else(|| std::path::Path::new("."));
        let extension = current_path
            .extension()
            .and_then(|e| e.to_str())
            .unwrap_or("yaml");
        let new_filename = format!("{}.{}", new_name, extension);
        let new_path = parent.join(&new_filename);

        // Check if the new path already exists and is different from current
        if new_path != *current_path && new_path.exists() {
            // Show confirmation dialog
            self.pending_rename_path = Some(new_path);
            self.show_overwrite_confirm = true;
            return;
        }

        // Proceed with rename
        self.do_rename(new_name, new_path);
    }

    /// Actually perform the rename operation
    #[cfg(not(target_arch = "wasm32"))]
    fn do_rename(&mut self, new_name: String, new_path: PathBuf) {
        let Some(current_path) = &self.state.library_path.clone() else {
            log::error!("Cannot rename library: no library path");
            return;
        };

        // Update library name
        self.state.library.name = new_name.clone();

        // Save library with new name to current file
        if let Err(e) = promptgen_core::save_library(&self.state.library, current_path) {
            log::error!("Failed to save library with new name: {}", e);
            // TODO: Show error in dialog instead of just logging
            return;
        }

        // Rename the file if the path changed
        if new_path != *current_path {
            // If target exists, delete it first (we already confirmed with user)
            if new_path.exists()
                && let Err(e) = std::fs::remove_file(&new_path)
            {
                log::error!("Failed to delete existing file: {}", e);
                // TODO: Show error in dialog instead of just logging
                return;
            }

            match std::fs::rename(current_path, &new_path) {
                Ok(()) => {
                    // Update paths
                    self.state.library_path = Some(new_path.clone());
                    self.library_file_path = Some(new_path.clone());
                    self.storage.set_library_path(new_path);

                    log::info!("Library renamed to: {}", new_name);
                }
                Err(e) => {
                    log::error!("Failed to rename library file: {}", e);
                    // TODO: Show error in dialog instead of just logging
                    // Note: Library name in YAML was already updated, but file wasn't renamed
                }
            }
        }

        // Close dialogs
        self.show_edit_library_dialog = false;
        self.edit_library_name.clear();
        self.show_overwrite_confirm = false;
        self.pending_rename_path = None;
    }

    /// Render the overwrite confirmation dialog and handle actions
    #[cfg(not(target_arch = "wasm32"))]
    fn render_overwrite_confirm_dialog(&mut self, ctx: &egui::Context) {
        use dialogs::OverwriteConfirmAction;

        let Some(new_path) = &self.pending_rename_path else {
            return;
        };

        let filename = new_path
            .file_name()
            .and_then(|f| f.to_str())
            .unwrap_or("unknown")
            .to_string();

        let action = dialogs::render_overwrite_confirm_dialog(
            ctx,
            &mut self.show_overwrite_confirm,
            &filename,
        );

        match action {
            OverwriteConfirmAction::Confirm => {
                let new_name = self.edit_library_name.trim().to_string();
                let new_path = self.pending_rename_path.clone().unwrap();
                self.do_rename(new_name, new_path);
            }
            OverwriteConfirmAction::Cancel => {
                self.show_overwrite_confirm = false;
                self.pending_rename_path = None;
            }
            OverwriteConfirmAction::None => {}
        }
    }

    /// Render the close unsaved tab confirmation dialog and handle actions
    fn render_close_unsaved_tab_dialog(&mut self, ctx: &egui::Context) {
        use crate::state::ConfirmDialog;
        use dialogs::CloseUnsavedTabAction;

        // Check if we have a CloseUnsavedTab dialog active
        let tab_index = match &self.state.confirm_dialog {
            Some(ConfirmDialog::CloseUnsavedTab { tab_index }) => *tab_index,
            _ => return,
        };

        // Get the tab name for display
        let tab_name = self
            .state
            .prompt_tabs
            .get(tab_index)
            .map(|t| t.name.clone())
            .unwrap_or_else(|| "Prompt".to_string());

        let action = dialogs::render_close_unsaved_tab_dialog(ctx, &tab_name);

        match action {
            CloseUnsavedTabAction::Save => {
                // Switch to the tab, save it, then close it
                self.state.switch_to_tab(tab_index);
                self.state.save_active_tab_to_library();

                // Persist library to disk
                #[cfg(not(target_arch = "wasm32"))]
                if let Some(path) = &self.state.library_path
                    && let Err(e) = promptgen_core::save_library(&self.state.library, path)
                {
                    log::error!("Failed to save library: {}", e);
                }

                // Now close the tab (it's clean now)
                self.state.close_tab_force(tab_index);
                self.state.confirm_dialog = None;
            }
            CloseUnsavedTabAction::DontSave => {
                // Close without saving
                self.state.close_tab_force(tab_index);
                self.state.confirm_dialog = None;
            }
            CloseUnsavedTabAction::Cancel => {
                // Just close the dialog
                self.state.confirm_dialog = None;
            }
            CloseUnsavedTabAction::None => {}
        }
    }

    /// Render the delete prompt confirmation dialog and handle actions
    fn render_delete_prompt_dialog(&mut self, ctx: &egui::Context) {
        use crate::state::ConfirmDialog;
        use dialogs::DeletePromptAction;

        // Check if we have a DeletePrompt dialog active
        let prompt_name = match &self.state.confirm_dialog {
            Some(ConfirmDialog::DeletePrompt { prompt_name }) => prompt_name.clone(),
            _ => return,
        };

        let action = dialogs::render_delete_prompt_dialog(ctx, &prompt_name);

        match action {
            DeletePromptAction::Delete => {
                // Delete the prompt
                self.state.delete_prompt(&prompt_name);

                // Persist library to disk
                #[cfg(not(target_arch = "wasm32"))]
                if let Some(path) = &self.state.library_path
                    && let Err(e) = promptgen_core::save_library(&self.state.library, path)
                {
                    log::error!("Failed to save library after delete: {}", e);
                }
            }
            DeletePromptAction::Cancel => {
                self.state.confirm_dialog = None;
            }
            DeletePromptAction::None => {}
        }
    }

    /// Render the rename prompt dialog and handle actions
    fn render_rename_prompt_dialog(&mut self, ctx: &egui::Context) {
        use dialogs::RenamePromptAction;

        // Check if we have an active rename
        let Some(index) = self.state.tab_rename_index else {
            return;
        };

        let Some(tab) = self.state.prompt_tabs.get(index) else {
            self.state.cancel_tab_rename();
            return;
        };

        let current_name = tab.name.clone();
        let is_valid = self.state.is_tab_rename_valid();

        let action = dialogs::render_rename_prompt_dialog(
            ctx,
            &current_name,
            &mut self.state.tab_rename_text,
            is_valid,
        );

        match action {
            RenamePromptAction::Rename { new_name } => {
                // Apply the new name
                self.state.tab_rename_text = new_name;
                if self.state.commit_tab_rename() {
                    // Persist library to disk if this was a library prompt
                    #[cfg(not(target_arch = "wasm32"))]
                    if let Some(path) = &self.state.library_path
                        && let Err(e) = promptgen_core::save_library(&self.state.library, path)
                    {
                        log::error!("Failed to save library after rename: {}", e);
                    }
                }
            }
            RenamePromptAction::Cancel => {
                self.state.cancel_tab_rename();
            }
            RenamePromptAction::None => {}
        }
    }

    /// Render the import variables dialog and handle actions
    fn render_import_variables_dialog(&mut self, ctx: &egui::Context) {
        use dialogs::ImportVariablesAction;

        if !self.state.import_dialog_open {
            return;
        }

        let can_import = self.state.can_import();

        // Pre-compute flags for each imported variable
        let originally_conflicting: Vec<bool> = (0..self.state.import_parsed_variables.len())
            .map(|idx| self.state.is_import_originally_conflicting(idx))
            .collect();

        let currently_conflicting: Vec<bool> = (0..self.state.import_parsed_variables.len())
            .map(|idx| self.state.is_import_name_conflicting(idx))
            .collect();

        let action = dialogs::render_import_variables_dialog(
            ctx,
            &mut self.state.import_yaml_text,
            &self.state.import_parsed_variables,
            self.state.import_parse_error.as_deref(),
            can_import,
            &originally_conflicting,
            &currently_conflicting,
        );

        match action {
            ImportVariablesAction::Import => {
                // Perform the import
                self.state.perform_import();

                // Persist library to disk
                #[cfg(not(target_arch = "wasm32"))]
                if let Some(path) = &self.state.library_path
                    && let Err(e) = promptgen_core::save_library(&self.state.library, path)
                {
                    log::error!("Failed to save library after import: {}", e);
                }

                self.state.close_import_dialog();
            }
            ImportVariablesAction::Cancel => {
                self.state.close_import_dialog();
            }
            ImportVariablesAction::YamlChanged => {
                self.state.parse_import_yaml();
            }
            ImportVariablesAction::RenamedChanged { index, new_name } => {
                self.state.update_import_renamed_name(index, new_name);
            }
            ImportVariablesAction::None => {}
        }
    }

    /// Render the import prompts dialog and handle actions
    fn render_import_prompts_dialog(&mut self, ctx: &egui::Context) {
        use dialogs::ImportPromptsAction;

        if !self.state.prompt_import_dialog_open {
            return;
        }

        let can_import = self.state.can_import_prompts();

        // Pre-compute flags for each imported prompt
        let originally_conflicting: Vec<bool> = (0..self.state.prompt_import_parsed_prompts.len())
            .map(|idx| self.state.is_prompt_import_originally_conflicting(idx))
            .collect();

        let currently_conflicting: Vec<bool> = (0..self.state.prompt_import_parsed_prompts.len())
            .map(|idx| self.state.is_prompt_import_name_conflicting(idx))
            .collect();

        let action = dialogs::render_import_prompts_dialog(
            ctx,
            &mut self.state.prompt_import_yaml_text,
            &self.state.prompt_import_parsed_prompts,
            self.state.prompt_import_parse_error.as_deref(),
            can_import,
            &originally_conflicting,
            &currently_conflicting,
        );

        match action {
            ImportPromptsAction::Import => {
                // Perform the import
                self.state.perform_prompt_import();

                // Persist library to disk
                #[cfg(not(target_arch = "wasm32"))]
                if let Some(path) = &self.state.library_path
                    && let Err(e) = promptgen_core::save_library(&self.state.library, path)
                {
                    log::error!("Failed to save library after import: {}", e);
                }

                self.state.close_prompt_import_dialog();
            }
            ImportPromptsAction::Cancel => {
                self.state.close_prompt_import_dialog();
            }
            ImportPromptsAction::YamlChanged => {
                self.state.parse_prompt_import_yaml();
            }
            ImportPromptsAction::RenamedChanged { index, new_name } => {
                self.state
                    .update_prompt_import_renamed_name(index, new_name);
            }
            ImportPromptsAction::None => {}
        }
    }

    /// Render the unsaved prompts confirmation dialog (when opening/creating a new library)
    #[cfg(not(target_arch = "wasm32"))]
    fn render_unsaved_prompts_dialog(&mut self, ctx: &egui::Context) {
        use dialogs::UnsavedPromptsAction;

        // Check if we have an OpenNewLibrary dialog active
        let pending_action = match &self.state.confirm_dialog {
            Some(ConfirmDialog::OpenNewLibrary { pending_action }) => pending_action.clone(),
            _ => return,
        };

        let unsaved_names = self.state.get_unsaved_tab_names();

        let action = dialogs::render_unsaved_prompts_dialog(ctx, &unsaved_names);

        match action {
            UnsavedPromptsAction::SaveAll => {
                // Save all dirty tabs to current library
                self.save_all_dirty_tabs();

                // Clear the dialog
                self.state.confirm_dialog = None;

                // Proceed with the pending action
                match pending_action {
                    PendingLibraryAction::OpenFilePicker => {
                        self.show_open_file_picker();
                    }
                    PendingLibraryAction::ShowCreateDialog => {
                        self.show_create_library_dialog = true;
                    }
                }
            }
            UnsavedPromptsAction::DiscardAll => {
                // Clear the dialog
                self.state.confirm_dialog = None;

                // Proceed with the pending action (discarding unsaved changes)
                match pending_action {
                    PendingLibraryAction::OpenFilePicker => {
                        self.show_open_file_picker();
                    }
                    PendingLibraryAction::ShowCreateDialog => {
                        self.show_create_library_dialog = true;
                    }
                }
            }
            UnsavedPromptsAction::Cancel => {
                // Just cancel - don't open/create the new library
                self.state.confirm_dialog = None;
            }
            UnsavedPromptsAction::None => {}
        }
    }
}

impl eframe::App for PromptGenApp {
    /// Called by the framework to save state before shutdown.
    fn save(&mut self, storage: &mut dyn eframe::Storage) {
        // Sync tabs from AppState to persisted fields before saving
        self.save_tabs();
        eframe::set_value(storage, eframe::APP_KEY, self);
    }

    /// Called each time the UI needs repainting.
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        // Ensure catppuccin theme is applied (handles dark/light mode switches)
        theme::ensure_theme_applied(ctx);

        // Ensure custom font sizes are applied (theme switches may reset them)
        theme::apply_font_sizes(ctx);

        // Ensure text cursor is visible (theme switches may change this)
        theme::ensure_cursor_visible(ctx);

        // Top menu bar
        egui::TopBottomPanel::top("top_panel").show(ctx, |ui| {
            egui::MenuBar::new().ui(ui, |ui| {
                #[cfg(not(target_arch = "wasm32"))]
                {
                    ui.menu_button("File", |ui| {
                        if ui.button("Create Library...").clicked() {
                            ui.close();
                            self.request_show_create_dialog();
                        }
                        if ui.button("Open Library...").clicked() {
                            ui.close();
                            self.open_library_dialog();
                        }

                        // Edit Library - only enabled when a library is loaded
                        let has_library = self.state.library_path.is_some();
                        if ui
                            .add_enabled(has_library, egui::Button::new("Edit Library..."))
                            .clicked()
                        {
                            ui.close();
                            // Initialize dialog with current library name
                            self.edit_library_name = self.state.library.name.clone();
                            self.show_edit_library_dialog = true;
                        }

                        ui.separator();
                        if ui.button("Quit").clicked() {
                            ctx.send_viewport_cmd(egui::ViewportCommand::Close);
                        }
                    });
                    ui.add_space(16.0);
                }

                egui::widgets::global_theme_preference_buttons(ui);

                // Help menu
                ui.menu_button("Help", |ui| {
                    if ui.button("Getting Started").clicked() {
                        ui.close();
                        self.show_getting_started_help = true;
                    }
                });
            });
        });

        // Left sidebar
        egui::SidePanel::left("sidebar")
            .resizable(true)
            .default_width(250.0)
            .width_range(180.0..=400.0)
            .show(ctx, |ui| {
                let open_dialog = SidebarPanel::show(ui, &mut self.state, &self.library_file_path);

                #[cfg(not(target_arch = "wasm32"))]
                if open_dialog {
                    self.open_library_dialog();
                }

                #[cfg(target_arch = "wasm32")]
                let _ = open_dialog;
            });

        // Right preview panel
        egui::SidePanel::right("preview")
            .resizable(true)
            .default_width(400.0)
            .width_range(400.0..=500.0)
            .show(ctx, |ui| {
                PreviewPanel::show(ui, &mut self.state);
            });

        // Handle Escape key to close slot picker
        if ctx.input(|i| i.key_pressed(egui::Key::Escape)) {
            self.state.unfocus_slot();
        }

        // Central panel with unified scroll area for editor + slots
        egui::CentralPanel::default().show(ctx, |ui| {
            // Show blank state if no active tab
            if self.state.is_blank() {
                ui.vertical_centered(|ui| {
                    ui.add_space(100.0);

                    ui.heading("Welcome to PromptGen");
                    ui.add_space(16.0);

                    if self.state.library_path.is_some() {
                        ui.label("Create a new prompt to get started.");
                        ui.add_space(16.0);

                        if ui.button("+ New Prompt").clicked() {
                            self.state.create_new_tab();
                        }

                        ui.add_space(8.0);
                        ui.label(egui::RichText::new("Or select a prompt from the sidebar").weak());
                    } else {
                        ui.label("Open or create a library to get started.");
                        ui.add_space(16.0);

                        ui.horizontal(|ui| {
                            ui.add_space(ui.available_width() / 2.0 - 100.0);

                            #[cfg(not(target_arch = "wasm32"))]
                            {
                                if ui.button("Open Library...").clicked() {
                                    self.open_library_dialog();
                                }

                                if ui.button("Create Library...").clicked() {
                                    self.request_show_create_dialog();
                                }
                            }
                        });
                    }
                });
                return;
            }

            // Choose which editor to show based on editor mode
            match &self.state.editor_mode {
                EditorMode::Prompt => {
                    // Tab bar (fixed at top, outside scroll area)
                    let tab_result = TabBarPanel::show(ui, &mut self.state);

                    // Persist library to disk if it was modified
                    #[cfg(not(target_arch = "wasm32"))]
                    if tab_result.library_modified
                        && let Some(path) = &self.state.library_path
                        && let Err(e) = promptgen_core::save_library(&self.state.library, path)
                    {
                        log::error!("Failed to save library: {}", e);
                    }

                    #[cfg(target_arch = "wasm32")]
                    let _ = tab_result;

                    ui.separator();

                    // Scrollable content area
                    egui::ScrollArea::vertical()
                        .id_salt("main_scroll")
                        .auto_shrink([false, false])
                        .show(ui, |ui| {
                            // Prompt editor section (collapsible, per-tab state)
                            let is_expanded = self
                                .state
                                .get_active_tab()
                                .map(|t| t.prompt_editor_expanded)
                                .unwrap_or(true);

                            // Custom collapsible header with right-aligned arrow
                            use egui_material_icons::icons::{
                                ICON_CHEVRON_RIGHT, ICON_EXPAND_MORE,
                            };

                            let header_response = ui
                                .horizontal(|ui| {
                                    ui.heading("Prompt");
                                    ui.with_layout(
                                        egui::Layout::right_to_left(egui::Align::Center),
                                        |ui| {
                                            let icon = if is_expanded {
                                                ICON_EXPAND_MORE
                                            } else {
                                                ICON_CHEVRON_RIGHT
                                            };
                                            ui.heading(icon);
                                        },
                                    );
                                })
                                .response
                                .interact(egui::Sense::click());

                            // Toggle on click
                            if header_response.clicked()
                                && let Some(tab) = self.state.get_active_tab_mut()
                            {
                                tab.prompt_editor_expanded = !tab.prompt_editor_expanded;
                            }

                            // Show cursor hint on hover
                            if header_response.hovered() {
                                ui.ctx().set_cursor_icon(egui::CursorIcon::PointingHand);
                            }

                            // Show content when expanded
                            if is_expanded {
                                EditorPanel::show(ui, &mut self.state);
                            }

                            // Slots section (only show if there are slots)
                            let has_slots = !self.state.get_slot_definitions().is_empty();
                            if has_slots {
                                ui.separator();

                                // Slots header with config
                                ui.horizontal(|ui| {
                                    ui.heading("Slots");
                                    ui.add_space(16.0);

                                    // Get current defaults from active tab
                                    let (mut sep, mut suffix) = if let Some(tab) =
                                        self.state.get_active_tab()
                                    {
                                        (
                                            tab.slot_defaults.sep.clone().unwrap_or_default(),
                                            tab.slot_defaults.suffix.clone().unwrap_or_default(),
                                        )
                                    } else {
                                        (String::new(), String::new())
                                    };

                                    ui.label("Default Separator:");
                                    let sep_response = ui.add(
                                        egui::TextEdit::singleline(&mut sep)
                                            .desired_width(60.0)
                                            .hint_text(", "),
                                    );

                                    ui.add_space(8.0);
                                    ui.label("Default Suffix:");
                                    let suffix_response = ui.add(
                                        egui::TextEdit::singleline(&mut suffix)
                                            .desired_width(60.0)
                                            .hint_text("none"),
                                    );

                                    // Update state if changed
                                    if sep_response.changed() || suffix_response.changed() {
                                        if let Some(tab) = self.state.get_active_tab_mut() {
                                            tab.slot_defaults.sep =
                                                if sep.is_empty() { None } else { Some(sep) };
                                            tab.slot_defaults.suffix = if suffix.is_empty() {
                                                None
                                            } else {
                                                Some(suffix)
                                            };
                                            tab.dirty = true;
                                        }
                                        self.state.request_render();
                                    }

                                    ui.add_space(16.0);
                                    if ui.button("Clear All").clicked() {
                                        self.state.clear_all_slot_values();
                                    }
                                });

                                SlotPanel::show(ui, &mut self.state);
                            } else {
                                ui.add_space(300.0);
                            }
                        });
                }
                EditorMode::VariableEditor { .. } | EditorMode::NewVariable => {
                    // Variable editor (no tabs)
                    egui::ScrollArea::vertical()
                        .id_salt("main_scroll")
                        .auto_shrink([false, false])
                        .show(ui, |ui| {
                            VariableEditorPanel::show(ui, &mut self.state);
                        });
                }
            }
        });

        // Render create library dialog (if active)
        #[cfg(not(target_arch = "wasm32"))]
        self.render_create_library_dialog(ctx);

        // Render edit library dialog (if active)
        #[cfg(not(target_arch = "wasm32"))]
        self.render_edit_library_dialog(ctx);

        // Render overwrite confirmation dialog (if active)
        #[cfg(not(target_arch = "wasm32"))]
        self.render_overwrite_confirm_dialog(ctx);

        // Render close unsaved tab dialog (if active)
        self.render_close_unsaved_tab_dialog(ctx);

        // Render delete prompt dialog (if active)
        self.render_delete_prompt_dialog(ctx);

        // Render rename prompt dialog (if active)
        self.render_rename_prompt_dialog(ctx);

        // Render import variables dialog (if active)
        self.render_import_variables_dialog(ctx);

        // Render import prompts dialog (if active)
        self.render_import_prompts_dialog(ctx);

        // Render unsaved prompts dialog (when opening/creating new library)
        #[cfg(not(target_arch = "wasm32"))]
        self.render_unsaved_prompts_dialog(ctx);

        // Render help windows
        HelpWindow::new("getting_started_help", "Getting Started").show(
            ctx,
            &mut self.show_getting_started_help,
            help::content::GETTING_STARTED,
            &mut self.help_cache,
        );
    }
}
