//! Application state management.
//!
//! This module contains all the state types for the Promptgen UI application,
//! organized into logical sub-modules.

mod autocomplete;
mod dialogs;
mod editor;
mod import_export;
mod preview;
mod sidebar;
mod tabs;

// Re-export all public types for convenient access
pub use autocomplete::{AutocompleteManager, AutocompleteMode, AutocompleteState};
pub use dialogs::{ConfirmDialog, DialogState, PendingLibraryAction};
pub use editor::{EditorFocus, EditorMode, EditorState, VariableEditorState};
pub use import_export::{
    ImportedPrompt, ImportedVariable, PromptExportState, PromptImportState, VariableExportState,
    VariableImportState,
};
pub use preview::{PreviewState, SlotManualEditState};
pub use sidebar::{SidebarMode, SidebarState, SidebarViewMode, VariableSortOrder};
pub use tabs::{PromptSource, PromptTab, TabState};

use std::path::PathBuf;

use promptgen_core::{
    Cardinality, EvalContext, Library, PickSource, RenderError, SlotDefKind, SlotDefinition, render,
};

/// A group of options to display (either a real variable or a pseudo-group like "Options").
#[derive(Clone)]
pub struct OptionGroup {
    /// Display name (variable name or "Options" for free-form literals)
    pub name: String,
    /// The options in this group
    pub options: Vec<String>,
    /// Whether this group can be edited (false for "Options" pseudo-group)
    pub is_editable: bool,
}

/// Main application state - composes all sub-states
pub struct AppState {
    // Core data
    pub library: Library,
    pub library_path: Option<PathBuf>,

    // Sub-states
    pub tabs: TabState,
    pub editor: EditorState,
    pub preview: PreviewState,
    pub sidebar: SidebarState,
    pub autocomplete: AutocompleteManager,
    pub dialogs: DialogState,
    pub variable_editor: VariableEditorState,
    pub slot_manual_edit: SlotManualEditState,

    // Import/Export states
    pub variable_export: VariableExportState,
    pub variable_import: VariableImportState,
    pub prompt_export: PromptExportState,
    pub prompt_import: PromptImportState,
}

impl Default for AppState {
    fn default() -> Self {
        let first_tab = PromptTab::default();
        let mut tabs = TabState::default();
        tabs.tabs.push(first_tab);
        tabs.active_index = Some(0);

        Self {
            library: Library::default(),
            library_path: None,
            tabs,
            editor: EditorState::default(),
            preview: PreviewState::default(),
            sidebar: SidebarState::default(),
            autocomplete: AutocompleteManager::default(),
            dialogs: DialogState::default(),
            variable_editor: VariableEditorState::default(),
            slot_manual_edit: SlotManualEditState::default(),
            variable_export: VariableExportState::default(),
            variable_import: VariableImportState::default(),
            prompt_export: PromptExportState::default(),
            prompt_import: PromptImportState::default(),
        }
    }
}

impl AppState {
    // ==========================================================================
    // Convenience accessors (delegate to sub-states)
    // ==========================================================================

    /// Get the active tab (immutable reference)
    pub fn get_active_tab(&self) -> Option<&PromptTab> {
        self.tabs.get_active()
    }

    /// Get the active tab (mutable reference)
    pub fn get_active_tab_mut(&mut self) -> Option<&mut PromptTab> {
        self.tabs.get_active_mut()
    }

    /// Check if we're in a "blank" state (no active tab)
    pub fn is_blank(&self) -> bool {
        self.tabs.is_blank()
    }

    /// Check if there are any unsaved tabs
    pub fn has_unsaved_tabs(&self) -> bool {
        self.tabs.has_unsaved()
    }

    /// Get the names of all unsaved tabs
    pub fn get_unsaved_tab_names(&self) -> Vec<String> {
        self.tabs.get_unsaved_names()
    }

    // ==========================================================================
    // Editor focus management (coordinates editor + sidebar)
    // ==========================================================================

    /// Focus the main editor (and unfocus any slots, returning sidebar to normal)
    pub fn focus_main_editor(&mut self) {
        self.editor.focus_main_editor();
        self.sidebar.exit_slot_picker();
    }

    /// Focus a textarea slot (and unfocus any pick slots, returning sidebar to normal)
    pub fn focus_textarea_slot(&mut self, slot_label: &str) {
        self.editor.focus_textarea_slot(slot_label);
        self.sidebar.exit_slot_picker();
    }

    /// Focus a pick slot and switch sidebar to picker mode
    pub fn focus_slot(&mut self, slot_label: &str) {
        self.editor.focus_pick_slot(slot_label);
        self.sidebar.enter_slot_picker(slot_label);
    }

    /// Unfocus the current editor/slot and return sidebar to normal mode
    pub fn unfocus_slot(&mut self) {
        self.editor.unfocus();
        self.sidebar.exit_slot_picker();
    }

    /// Check if a specific slot is focused (pick or textarea)
    pub fn is_slot_focused(&self, slot_label: &str) -> bool {
        self.editor.is_slot_focused(slot_label)
    }

    /// Check if the main editor is focused
    pub fn is_main_editor_focused(&self) -> bool {
        self.editor.is_main_editor_focused()
    }

    // ==========================================================================
    // Parse and render
    // ==========================================================================

    /// Update parse result when editor content changes
    pub fn update_parse_result(&mut self) {
        self.editor.parse_result = Some(self.library.parse_prompt(&self.editor.content));

        // Update slot values map - add new slots, keep existing values
        if let Some(result) = &self.editor.parse_result
            && let Some(ast) = &result.ast
        {
            // Use get_slot_definitions to get all slots including expanded reference slots
            let defaults = self
                .tabs
                .get_active()
                .map(|t| t.slot_defaults.clone())
                .unwrap_or_default();
            let definitions = self.library.get_slot_definitions(ast, &defaults);
            let current_slots: Vec<String> = definitions.iter().map(|d| d.label.clone()).collect();

            // Remove slots that no longer exist
            self.preview
                .slot_values
                .retain(|name, _| current_slots.contains(name));
            // Add new slots with empty values
            for slot in current_slots {
                self.preview.slot_values.entry(slot).or_default();
            }
            // Clear focused slot if it no longer exists
            if let EditorFocus::PickSlot { ref label } | EditorFocus::TextareaSlot { ref label } =
                self.editor.focus
                && !self.preview.slot_values.contains_key(label)
            {
                self.editor.unfocus();
                self.sidebar.exit_slot_picker();
            }
        }
    }

    /// Render the current prompt with the given seed
    pub fn render_prompt(&mut self) -> Result<(), RenderError> {
        if let Some(result) = &self.editor.parse_result
            && let Some(ast) = &result.ast
        {
            let seed = self.preview.seed.unwrap_or_else(|| {
                use std::time::{SystemTime, UNIX_EPOCH};
                SystemTime::now()
                    .duration_since(UNIX_EPOCH)
                    .map(|d| d.as_nanos() as u64)
                    .unwrap_or(42)
            });

            let mut ctx = EvalContext::with_seed(&self.library, seed);

            if let Some(tab) = self.tabs.get_active() {
                ctx.set_slot_defaults(tab.slot_defaults.clone());
            }

            for (name, values) in &self.preview.slot_values {
                if !values.is_empty() {
                    ctx.set_slot_values(name.clone(), values.clone());
                }
            }

            let render_result = render(ast, &mut ctx)?;
            self.preview.output = render_result.text.trim().to_string();
            self.preview.seed = Some(seed);

            return Ok(());
        }
        self.preview.output.clear();
        Ok(())
    }

    /// Request a preview render (if auto_render is enabled).
    pub fn request_render(&mut self) {
        self.preview.request_render();
    }

    /// Process any pending render request.
    pub fn process_pending_render(&mut self) {
        if self.preview.dirty {
            if self.preview.auto_randomize_seed {
                self.preview.randomize_seed();
            }
            let _ = self.render_prompt();
            self.preview.dirty = false;
        }
    }

    /// Get slot definitions from the current prompt
    pub fn get_slot_definitions(&self) -> Vec<SlotDefinition> {
        if let Some(result) = &self.editor.parse_result
            && let Some(ast) = &result.ast
        {
            let defaults = self
                .tabs
                .get_active()
                .map(|t| t.slot_defaults.clone())
                .unwrap_or_default();
            return self.library.get_slot_definitions(ast, &defaults);
        }
        Vec::new()
    }

    // ==========================================================================
    // Slot value management
    // ==========================================================================

    /// Get the cardinality for a pick slot
    pub fn get_slot_cardinality(&self, slot_label: &str) -> Option<Cardinality> {
        let definitions = self.get_slot_definitions();
        definitions
            .iter()
            .find(|d| d.label == slot_label)
            .and_then(|def| {
                if let SlotDefKind::Pick { cardinality, .. } = &def.kind {
                    Some(cardinality.clone())
                } else {
                    None
                }
            })
    }

    /// Get option groups for a pick slot
    pub fn get_pick_option_groups(&self, slot_label: &str) -> Vec<OptionGroup> {
        let definitions = self.get_slot_definitions();
        let Some(def) = definitions.iter().find(|d| d.label == slot_label) else {
            return Vec::new();
        };
        let SlotDefKind::Pick { sources, .. } = &def.kind else {
            return Vec::new();
        };

        let mut groups = Vec::new();
        let mut literals = Vec::new();

        for source in sources {
            match source {
                PickSource::VariableRef(lib_ref) => {
                    if let Some(variable) = self.library.find_variable(&lib_ref.variable) {
                        groups.push(OptionGroup {
                            name: variable.name.clone(),
                            options: variable.options.clone(),
                            is_editable: true,
                        });
                    }
                }
                PickSource::Literal { value, .. } => {
                    literals.push(value.clone());
                }
            }
        }

        if !literals.is_empty() {
            groups.insert(
                0,
                OptionGroup {
                    name: "Options".to_string(),
                    options: literals,
                    is_editable: false,
                },
            );
        }

        groups
    }

    /// Add a value to a slot (for pick slots)
    pub fn add_slot_value(&mut self, slot_label: &str, value: String) {
        let cardinality = self.get_slot_cardinality(slot_label);

        if let Some(values) = self.preview.slot_values.get_mut(slot_label) {
            if let Some(Cardinality::One) = cardinality {
                values.clear();
            } else if let Some(Cardinality::Many { max: Some(max) }) = cardinality
                && values.len() >= max as usize
            {
                return;
            }
            if !values.contains(&value) {
                values.push(value);
                self.tabs.mark_active_dirty();
            }
        }
    }

    /// Remove a value from a slot
    pub fn remove_slot_value(&mut self, slot_label: &str, value: &str) {
        if let Some(values) = self.preview.slot_values.get_mut(slot_label) {
            let len_before = values.len();
            values.retain(|v| v != value);
            if values.len() != len_before {
                self.tabs.mark_active_dirty();
            }
        }
    }

    /// Set all values for a slot (used for reordering)
    pub fn set_slot_values(&mut self, slot_label: &str, new_values: Vec<String>) {
        if let Some(values) = self.preview.slot_values.get_mut(slot_label)
            && *values != new_values
        {
            *values = new_values;
            self.tabs.mark_active_dirty();
        }
    }

    /// Set the single value for a textarea slot
    pub fn set_textarea_value(&mut self, slot_label: &str, value: String) {
        if self.preview.set_textarea_value(slot_label, value) {
            self.tabs.mark_active_dirty();
        }
    }

    /// Get the textarea value for a slot
    pub fn get_textarea_value(&self, slot_label: &str) -> String {
        self.preview.get_textarea_value(slot_label)
    }

    /// Clear all slot values
    pub fn clear_all_slot_values(&mut self) {
        if self.preview.clear_all_slot_values() {
            self.tabs.mark_active_dirty();
            self.request_render();
        }
    }

    /// Clear a single slot's values
    pub fn clear_slot(&mut self, slot_label: &str) {
        if self.preview.clear_slot(slot_label) {
            self.tabs.mark_active_dirty();
            self.request_render();
        }
    }

    /// Clear all slots with a given prefix (e.g., "Head - " clears "Head - Hair", "Head - Eyes", etc.)
    pub fn clear_slots_with_prefix(&mut self, prefix: &str) {
        let full_prefix = format!("{} - ", prefix);
        let slots_to_clear: Vec<String> = self
            .preview
            .slot_values
            .keys()
            .filter(|k| k.starts_with(&full_prefix))
            .cloned()
            .collect();

        if !slots_to_clear.is_empty() {
            for slot in slots_to_clear {
                self.preview.slot_values.get_mut(&slot).map(|v| v.clear());
            }
            self.tabs.mark_active_dirty();
            self.request_render();
        }
    }

    /// Check if any slots with a given prefix have values
    pub fn has_slots_with_prefix_values(&self, prefix: &str) -> bool {
        let full_prefix = format!("{} - ", prefix);
        self.preview
            .slot_values
            .iter()
            .any(|(k, v)| k.starts_with(&full_prefix) && !v.is_empty())
    }

    // ==========================================================================
    // Slot manual edit mode
    // ==========================================================================

    /// Check if a slot is in manual edit mode
    pub fn is_slot_manual_edit(&self, slot_label: &str) -> bool {
        self.slot_manual_edit.is_active(slot_label)
    }

    /// Get the manual edit text for a slot
    pub fn get_slot_manual_edit_text(&self, slot_label: &str) -> String {
        self.slot_manual_edit.get_text(slot_label)
    }

    /// Set the manual edit text for a slot
    pub fn set_slot_manual_edit_text(&mut self, slot_label: &str, text: String) {
        self.slot_manual_edit.set_text(slot_label, text);
    }

    /// Enter manual edit mode for a slot
    pub fn enter_slot_manual_edit(&mut self, slot_label: &str, separator: &str) {
        let values = self
            .preview
            .slot_values
            .get(slot_label)
            .cloned()
            .unwrap_or_default();
        self.slot_manual_edit.enter(slot_label, &values, separator);
    }

    /// Exit manual edit mode for a slot
    pub fn exit_slot_manual_edit(
        &mut self,
        slot_label: &str,
        separator: &str,
        is_single_select: bool,
    ) {
        if let Some(new_values) = self.slot_manual_edit.exit(slot_label, separator, is_single_select) {
            if let Some(values) = self.preview.slot_values.get_mut(slot_label) {
                *values = new_values;
            } else {
                self.preview.slot_values.insert(slot_label.to_string(), new_values);
            }
            self.tabs.mark_active_dirty();
            self.request_render();
        }
    }

    // ==========================================================================
    // Tab management
    // ==========================================================================

    /// Switch to a specific tab by index
    pub fn switch_to_tab(&mut self, index: usize) {
        if index < self.tabs.tabs.len() {
            // Save current slot values to the old active tab before switching
            self.save_slot_values_to_active_tab();

            self.tabs.active_index = Some(index);
            if let Some(tab) = self.tabs.tabs.get(index) {
                self.editor.content = tab.content.clone();
                self.preview.slot_values = PreviewState::slot_values_from_tab(&tab.slots);
                self.update_parse_result();
                self.request_render();
            }
        }
    }

    /// Save current slot_values to the active tab
    pub fn save_slot_values_to_active_tab(&mut self) {
        let definitions = self.get_slot_definitions();
        let slot_values = PreviewState::slot_values_to_tab(&self.preview.slot_values, &definitions);

        if let Some(tab) = self.tabs.get_active_mut()
            && tab.slots != slot_values
        {
            tab.slots = slot_values;
            tab.dirty = true;
        }
    }

    /// Create a new tab with an auto-generated unique name
    pub fn create_new_tab(&mut self) -> usize {
        let library_prompts: Vec<String> = self.library.prompts.iter().map(|p| p.name.clone()).collect();
        let name = self.tabs.find_next_prompt_name(&library_prompts);
        let tab = PromptTab::new(name);
        self.tabs.tabs.push(tab);
        let new_index = self.tabs.tabs.len() - 1;
        self.switch_to_tab(new_index);
        new_index
    }

    /// Try to close a tab by index
    pub fn try_close_tab(&mut self, index: usize) -> bool {
        if index >= self.tabs.tabs.len() {
            return false;
        }

        if let Some(tab) = self.tabs.tabs.get(index)
            && tab.dirty
        {
            self.dialogs.request_close_unsaved_tab(index);
            return false;
        }

        self.close_tab_force(index)
    }

    /// Close a tab by index without checking for unsaved changes
    pub fn close_tab_force(&mut self, index: usize) -> bool {
        if !self.tabs.close_tab(index) {
            return false;
        }

        if self.tabs.tabs.is_empty() {
            self.editor.clear();
            self.preview.clear();
        } else if let Some(idx) = self.tabs.active_index {
            if let Some(tab) = self.tabs.tabs.get(idx) {
                self.editor.content = tab.content.clone();
                self.preview.slot_values = PreviewState::slot_values_from_tab(&tab.slots);
            }
            self.update_parse_result();
            self.request_render();
        }

        true
    }

    /// Sync active tab content from editor.content
    pub fn sync_active_tab_content(&mut self) {
        let new_content = self.editor.content.clone();
        if let Some(tab) = self.tabs.get_active_mut()
            && tab.content != new_content
        {
            tab.content = new_content;
            tab.dirty = true;
        }
    }

    /// Open a saved prompt from the library in a tab
    pub fn open_library_prompt(&mut self, prompt_name: &str) -> Option<usize> {
        if let Some(index) = self.tabs.find_by_library_prompt(prompt_name) {
            self.switch_to_tab(index);
            return Some(index);
        }

        let prompt = self.library.prompts.iter().find(|p| p.name == prompt_name)?;

        let tab = PromptTab::from_library_prompt(
            prompt.name.clone(),
            prompt.content.clone(),
            prompt.slots.clone(),
            prompt.slot_defaults.clone(),
        );

        self.tabs.tabs.push(tab);
        let new_index = self.tabs.tabs.len() - 1;
        self.switch_to_tab(new_index);
        Some(new_index)
    }

    /// Save the active tab to the library
    pub fn save_active_tab_to_library(&mut self) -> bool {
        self.save_slot_values_to_active_tab();

        let Some(idx) = self.tabs.active_index else {
            return false;
        };
        let Some(tab) = self.tabs.tabs.get(idx) else {
            return false;
        };

        let tab_name = tab.name.clone();
        let tab_content = tab.content.clone();
        let tab_slots = tab.slots.clone();
        let tab_slot_defaults = tab.slot_defaults.clone();
        let tab_source = tab.source.clone();

        match &tab_source {
            PromptSource::New => {
                let saved_prompt = promptgen_core::SavedPrompt {
                    name: tab_name.clone(),
                    content: tab_content,
                    slots: tab_slots,
                    slot_defaults: tab_slot_defaults,
                };
                self.library.prompts.push(saved_prompt);

                if let Some(tab) = self.tabs.tabs.get_mut(idx) {
                    tab.source = PromptSource::FromLibrary {
                        original_name: tab_name,
                    };
                    tab.dirty = false;
                }
            }
            PromptSource::FromLibrary { original_name } => {
                if let Some(prompt) = self
                    .library
                    .prompts
                    .iter_mut()
                    .find(|p| p.name == *original_name)
                {
                    if prompt.name != tab_name {
                        prompt.name = tab_name.clone();
                    }
                    prompt.content = tab_content;
                    prompt.slots = tab_slots;
                    prompt.slot_defaults = tab_slot_defaults;

                    if let Some(tab) = self.tabs.tabs.get_mut(idx) {
                        tab.source = PromptSource::FromLibrary {
                            original_name: tab_name,
                        };
                        tab.dirty = false;
                    }
                } else {
                    let saved_prompt = promptgen_core::SavedPrompt {
                        name: tab_name.clone(),
                        content: tab_content,
                        slots: tab_slots,
                        slot_defaults: tab_slot_defaults,
                    };
                    self.library.prompts.push(saved_prompt);

                    if let Some(tab) = self.tabs.tabs.get_mut(idx) {
                        tab.source = PromptSource::FromLibrary {
                            original_name: tab_name,
                        };
                        tab.dirty = false;
                    }
                }
            }
        }

        true
    }

    // ==========================================================================
    // Tab rename
    // ==========================================================================

    /// Start renaming a tab
    pub fn start_tab_rename(&mut self, index: usize) {
        self.tabs.start_rename(index);
    }

    /// Cancel tab rename
    pub fn cancel_tab_rename(&mut self) {
        self.tabs.cancel_rename();
    }

    /// Commit tab rename if valid
    pub fn commit_tab_rename(&mut self) -> bool {
        let library_prompts: Vec<String> = self.library.prompts.iter().map(|p| p.name.clone()).collect();
        self.tabs.commit_rename(&library_prompts)
    }

    /// Validate the current tab rename text
    pub fn is_tab_rename_valid(&self) -> bool {
        let library_prompts: Vec<String> = self.library.prompts.iter().map(|p| p.name.clone()).collect();
        self.tabs.is_rename_valid(&library_prompts)
    }

    // ==========================================================================
    // Variable editor
    // ==========================================================================

    /// Enter variable editor mode for an existing variable
    pub fn enter_variable_editor(&mut self, variable_name: &str) {
        let variable_data = self
            .library
            .variables
            .iter()
            .find(|g| g.name == variable_name)
            .map(|variable| (variable.name.clone(), variable.options.clone()));

        if let Some((name, options)) = variable_data {
            self.variable_editor.name = name.clone();
            self.variable_editor.content = VariableEditorState::options_to_text(&options);
            self.variable_editor.original_name = Some(name);
            self.variable_editor.dirty = false;
            self.editor.mode = EditorMode::VariableEditor {
                variable_name: variable_name.to_string(),
            };
            if !matches!(self.sidebar.mode, SidebarMode::SlotPicker { .. }) {
                self.sidebar.view_mode = SidebarViewMode::Variables;
                self.sidebar.mode = SidebarMode::Normal;
            }
        }
    }

    /// Enter variable editor mode for creating a new variable
    pub fn enter_new_variable_editor(&mut self) {
        self.variable_editor.clear();
        self.editor.mode = EditorMode::NewVariable;
        if !matches!(self.sidebar.mode, SidebarMode::SlotPicker { .. }) {
            self.sidebar.view_mode = SidebarViewMode::Variables;
            self.sidebar.mode = SidebarMode::Normal;
        }
    }

    /// Try to exit variable editor mode
    pub fn try_exit_variable_editor(&mut self) -> bool {
        if self.variable_editor.dirty {
            self.dialogs.request_discard_variable_changes();
            return false;
        }
        self.exit_variable_editor_force();
        true
    }

    /// Force exit variable editor mode
    pub fn exit_variable_editor_force(&mut self) {
        self.editor.mode = EditorMode::Prompt;
        self.variable_editor.clear();
        self.dialogs.close_confirm();
        self.update_parse_result();
    }

    /// Validate variable name
    pub fn validate_variable_name(&self) -> Option<String> {
        let name = self.variable_editor.name.trim();

        if name.is_empty() {
            return Some("Variable name cannot be empty".to_string());
        }

        let is_duplicate = self.library.variables.iter().any(|g| {
            g.name == name && Some(&g.name) != self.variable_editor.original_name.as_ref()
        });
        if is_duplicate {
            return Some(format!("A variable named \"{}\" already exists", name));
        }

        None
    }

    // ==========================================================================
    // Prompt management
    // ==========================================================================

    /// Delete a prompt from the library
    pub fn delete_prompt(&mut self, prompt_name: &str) {
        let tabs_to_close: Vec<usize> = self
            .tabs
            .tabs
            .iter()
            .enumerate()
            .filter_map(|(i, tab)| {
                if let PromptSource::FromLibrary { original_name } = &tab.source
                    && original_name == prompt_name
                {
                    return Some(i);
                }
                None
            })
            .collect();

        for idx in tabs_to_close.into_iter().rev() {
            self.close_tab_force(idx);
        }

        self.library.prompts.retain(|p| p.name != prompt_name);
        self.dialogs.close_confirm();
    }

    // ==========================================================================
    // Autocomplete delegation
    // ==========================================================================

    pub fn get_autocomplete(&self, editor_id: &str) -> Option<&AutocompleteState> {
        self.autocomplete.get(editor_id)
    }

    pub fn take_autocomplete_scroll_flag(&mut self, editor_id: &str) -> bool {
        self.autocomplete.take_scroll_flag(editor_id)
    }

    pub fn is_autocomplete_active(&self, editor_id: &str) -> bool {
        self.autocomplete.is_active(editor_id)
    }

    pub fn try_activate_autocomplete(&mut self, editor_id: &str, trigger_position: usize) -> bool {
        self.autocomplete.try_activate(editor_id, trigger_position)
    }

    pub fn force_activate_autocomplete(&mut self, editor_id: &str, trigger_position: usize) {
        self.autocomplete.force_activate(editor_id, trigger_position)
    }

    pub fn deactivate_autocomplete(&mut self, editor_id: &str) {
        self.autocomplete.deactivate(editor_id)
    }

    pub fn deactivate_autocomplete_escaped(&mut self, editor_id: &str) {
        self.autocomplete.deactivate_escaped(editor_id)
    }

    pub fn take_autocomplete_needs_refocus(&mut self, editor_id: &str) -> bool {
        self.autocomplete.take_needs_refocus(editor_id)
    }

    pub fn clear_dismissed_autocomplete(&mut self, editor_id: &str) {
        self.autocomplete.clear_dismissed(editor_id)
    }

    pub fn deactivate_autocomplete_except(&mut self, editor_id: &str) {
        self.autocomplete.deactivate_except(editor_id)
    }

    pub fn update_autocomplete_query(&mut self, editor_id: &str, content: &str, cursor_pos: usize) {
        self.autocomplete.update_query(editor_id, content, cursor_pos)
    }

    pub fn autocomplete_move_up(&mut self, editor_id: &str, total_items: usize) {
        self.autocomplete.move_up(editor_id, total_items)
    }

    pub fn autocomplete_move_down(&mut self, editor_id: &str, total_items: usize) {
        self.autocomplete.move_down(editor_id, total_items)
    }

    pub fn set_pending_cursor_position(&mut self, editor_id: &str, position: usize) {
        self.autocomplete.set_pending_cursor_position(editor_id, position)
    }

    pub fn take_pending_cursor_position(&mut self, editor_id: &str) -> Option<usize> {
        self.autocomplete.take_pending_cursor_position(editor_id)
    }

    // ==========================================================================
    // State reset
    // ==========================================================================

    /// Reset prompt state to blank (used when opening/creating a new library)
    pub fn reset_to_blank(&mut self) {
        self.tabs.clear();
        self.editor.clear();
        self.preview.clear();
        self.sidebar.clear();
        self.autocomplete.clear();
        self.dialogs.clear();
        self.variable_editor.clear();
        self.slot_manual_edit.clear();
        self.variable_export = VariableExportState::default();
        self.variable_import = VariableImportState::default();
        self.prompt_export = PromptExportState::default();
        self.prompt_import = PromptImportState::default();
    }
}
