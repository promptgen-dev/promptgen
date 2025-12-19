use std::collections::HashMap;

use promptgen_core::{
    Cardinality, EvalContext, Library, ParseResult, PickSource, RenderError, SlotDefKind,
    SlotDefinition, Workspace, render,
};
use serde::{Deserialize, Serialize};

/// Sidebar view mode - what to show in the sidebar list
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
pub enum SidebarViewMode {
    #[default]
    Templates,
    Variables,
}

/// Sidebar mode - normal navigation vs slot picker overlay
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub enum SidebarMode {
    /// Normal mode showing templates/variables
    #[default]
    Normal,
    /// Slot picker overlay showing options for a pick slot
    SlotPicker {
        /// The slot label being edited
        slot_label: String,
    },
}

/// What editor element currently has focus
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub enum EditorFocus {
    /// No editor focused
    #[default]
    None,
    /// Main template editor is focused
    MainEditor,
    /// A textarea slot is focused
    TextareaSlot { label: String },
    /// A pick slot is focused (opens sidebar picker)
    PickSlot { label: String },
}

/// What the central editor panel is currently showing
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub enum EditorMode {
    /// Normal template editing mode
    #[default]
    Template,
    /// Editing an existing variable
    VariableEditor { variable_name: String },
    /// Creating a new variable
    NewVariable,
}

/// Active confirmation dialog
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ConfirmDialog {
    /// Confirm discarding unsaved variable editor changes
    DiscardVariableChanges,
    /// Confirm deleting a variable
    DeleteVariable { variable_name: String },
}

/// Autocomplete mode - what kind of completions to show
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AutocompleteMode {
    /// Completing variable names (@Var...)
    Variables,
    /// Completing options within a specific variable (@Var/opt...)
    Options { variable_name: String },
}

/// Autocomplete state
#[derive(Debug, Clone, Default)]
pub struct AutocompleteState {
    /// Whether autocomplete popup is active
    pub active: bool,
    /// The query being completed (text after @)
    pub query: String,
    /// The mode (variables or options within a variable)
    pub mode: Option<AutocompleteMode>,
    /// Currently selected completion index
    pub selected_index: usize,
    /// Byte position where the @ symbol starts in the editor
    pub trigger_position: usize,
    /// The response ID of the text editor for popup positioning
    pub editor_response_id: Option<egui::Id>,
}

/// Main application state (not serialized - rebuilt on startup)
pub struct AppState {
    // Workspace
    pub workspace: Workspace,
    pub libraries: Vec<Library>,
    pub library_paths: HashMap<String, std::path::PathBuf>, // library_id -> file_path
    pub selected_library_id: Option<String>,

    // Editor
    pub editor_content: String,
    pub selected_template_id: Option<String>,
    pub parse_result: Option<ParseResult>,

    // Preview
    pub preview_output: String,
    pub preview_seed: Option<u64>,
    pub slot_values: HashMap<String, Vec<String>>,
    pub auto_randomize_seed: bool,
    pub auto_render: bool,
    pub preview_dirty: bool,

    // UI State
    pub sidebar_view_mode: SidebarViewMode,
    pub sidebar_mode: SidebarMode,
    pub search_query: String,
    pub editor_focus: EditorFocus,

    // Variable Editor State
    pub editor_mode: EditorMode,
    pub variable_editor_name: String,
    pub variable_editor_content: String,
    pub variable_editor_original_name: Option<String>,
    pub variable_editor_dirty: bool,
    pub confirm_dialog: Option<ConfirmDialog>,

    // Autocomplete State (per-editor, keyed by editor ID)
    pub autocomplete_states: HashMap<String, AutocompleteState>,

    // Pending cursor positions (per-editor, keyed by editor ID)
    pub pending_cursor_positions: HashMap<String, usize>,
}

impl Default for AppState {
    fn default() -> Self {
        Self {
            workspace: Workspace::new(),
            libraries: Vec::new(),
            library_paths: HashMap::new(),
            selected_library_id: None,
            editor_content: String::new(),
            selected_template_id: None,
            parse_result: None,
            preview_output: String::new(),
            preview_seed: None,
            slot_values: HashMap::new(),
            auto_randomize_seed: true,
            auto_render: true,
            preview_dirty: false,
            sidebar_view_mode: SidebarViewMode::default(),
            sidebar_mode: SidebarMode::default(),
            search_query: String::new(),
            editor_focus: EditorFocus::default(),
            editor_mode: EditorMode::default(),
            variable_editor_name: String::new(),
            variable_editor_content: String::new(),
            variable_editor_original_name: None,
            variable_editor_dirty: false,
            confirm_dialog: None,
            autocomplete_states: HashMap::new(),
            pending_cursor_positions: HashMap::new(),
        }
    }
}

impl AppState {
    /// Get the currently selected library, if any
    pub fn selected_library(&self) -> Option<&Library> {
        self.selected_library_id
            .as_ref()
            .and_then(|id| self.libraries.iter().find(|lib| lib.id == *id))
    }

    /// Rebuild the workspace from loaded libraries
    pub fn rebuild_workspace(&mut self) {
        let mut workspace = Workspace::new();
        for lib in &self.libraries {
            workspace = workspace.with_library(lib.clone());
        }
        self.workspace = workspace;
    }

    /// Update parse result when editor content changes
    pub fn update_parse_result(&mut self) {
        self.parse_result = Some(self.workspace.parse_template(&self.editor_content));
        // Update slot values map - add new slots, keep existing values
        if let Some(result) = &self.parse_result
            && let Some(ast) = &result.ast
        {
            let current_slots = self.workspace.get_slots(ast);
            // Remove slots that no longer exist
            self.slot_values
                .retain(|name, _| current_slots.contains(name));
            // Add new slots with empty values
            for slot in current_slots {
                self.slot_values.entry(slot).or_default();
            }
            // Clear focused slot if it no longer exists
            if let EditorFocus::PickSlot { ref label } | EditorFocus::TextareaSlot { ref label } =
                self.editor_focus
                && !self.slot_values.contains_key(label)
            {
                self.editor_focus = EditorFocus::None;
                self.sidebar_mode = SidebarMode::Normal;
            }
        }
    }

    /// Render the current template with the given seed
    pub fn render_template(&mut self) -> Result<(), RenderError> {
        if let Some(result) = &self.parse_result
            && let Some(ast) = &result.ast
        {
            // Use seed if provided, otherwise generate a random one
            let seed = self.preview_seed.unwrap_or_else(|| {
                use std::time::{SystemTime, UNIX_EPOCH};
                SystemTime::now()
                    .duration_since(UNIX_EPOCH)
                    .map(|d| d.as_nanos() as u64)
                    .unwrap_or(42)
            });

            let mut ctx = EvalContext::with_seed(&self.workspace, seed);

            // Set slot overrides (multi-value)
            for (name, values) in &self.slot_values {
                if !values.is_empty() {
                    ctx.set_slot_values(name.clone(), values.clone());
                }
            }

            let render_result = render(ast, &mut ctx)?;
            self.preview_output = render_result.text;

            // Update the seed to what we actually used
            self.preview_seed = Some(seed);

            return Ok(());
        }
        self.preview_output.clear();
        Ok(())
    }

    /// Generate a new random seed
    pub fn randomize_seed(&mut self) {
        use std::time::{SystemTime, UNIX_EPOCH};
        self.preview_seed = Some(
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .map(|d| d.as_nanos() as u64)
                .unwrap_or(42),
        );
    }

    /// Request a preview render (if auto_render is enabled).
    /// Call this from any component that changes render-affecting state.
    pub fn request_render(&mut self) {
        if self.auto_render {
            self.preview_dirty = true;
        }
    }

    /// Process any pending render request.
    /// Call this from PreviewPanel at the start of show().
    pub fn process_pending_render(&mut self) {
        if self.preview_dirty {
            if self.auto_randomize_seed {
                self.randomize_seed();
            }
            let _ = self.render_template();
            self.preview_dirty = false;
        }
    }

    /// Get slot definitions from the current template
    pub fn get_slot_definitions(&self) -> Vec<SlotDefinition> {
        if let Some(result) = &self.parse_result
            && let Some(ast) = &result.ast
        {
            return self.workspace.get_slot_definitions(ast);
        }
        Vec::new()
    }

    /// Focus the main editor (and unfocus any slots, returning sidebar to normal)
    pub fn focus_main_editor(&mut self) {
        self.editor_focus = EditorFocus::MainEditor;
        self.sidebar_mode = SidebarMode::Normal;
    }

    /// Focus a textarea slot (and unfocus any pick slots, returning sidebar to normal)
    pub fn focus_textarea_slot(&mut self, slot_label: &str) {
        self.editor_focus = EditorFocus::TextareaSlot {
            label: slot_label.to_string(),
        };
        self.sidebar_mode = SidebarMode::Normal;
    }

    /// Focus a pick slot and switch sidebar to picker mode
    pub fn focus_slot(&mut self, slot_label: &str) {
        self.editor_focus = EditorFocus::PickSlot {
            label: slot_label.to_string(),
        };
        self.sidebar_mode = SidebarMode::SlotPicker {
            slot_label: slot_label.to_string(),
        };
    }

    /// Unfocus the current editor/slot and return sidebar to normal mode
    pub fn unfocus_slot(&mut self) {
        self.editor_focus = EditorFocus::None;
        self.sidebar_mode = SidebarMode::Normal;
    }

    /// Check if a specific slot is focused (pick or textarea)
    pub fn is_slot_focused(&self, slot_label: &str) -> bool {
        matches!(
            &self.editor_focus,
            EditorFocus::PickSlot { label } | EditorFocus::TextareaSlot { label } if label == slot_label
        )
    }

    /// Check if the main editor is focused
    pub fn is_main_editor_focused(&self) -> bool {
        matches!(self.editor_focus, EditorFocus::MainEditor)
    }

    /// Get expanded options for a pick slot, resolving variable references
    pub fn get_pick_options(&self, slot_label: &str) -> Vec<String> {
        let definitions = self.get_slot_definitions();
        if let Some(def) = definitions.iter().find(|d| d.label == slot_label)
            && let SlotDefKind::Pick { sources, .. } = &def.kind
        {
            let mut options = Vec::new();
            for source in sources {
                match source {
                    PickSource::VariableRef(lib_ref) => {
                        // Resolve variable reference
                        let matches = if let Some(lib_name) = &lib_ref.library {
                            self.workspace
                                .find_variable_in_library(lib_name, &lib_ref.variable)
                                .into_iter()
                                .collect::<Vec<_>>()
                        } else {
                            self.workspace.find_variables(&lib_ref.variable)
                        };
                        // Add all options from matched variables
                        for (_lib, variable) in matches {
                            options.extend(variable.options.iter().cloned());
                        }
                    }
                    PickSource::Literal { value, .. } => {
                        options.push(value.clone());
                    }
                }
            }
            return options;
        }
        Vec::new()
    }

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

    /// Add a value to a slot (for pick slots)
    pub fn add_slot_value(&mut self, slot_label: &str, value: String) {
        // Get cardinality first to avoid borrow issues
        let cardinality = self.get_slot_cardinality(slot_label);

        if let Some(values) = self.slot_values.get_mut(slot_label) {
            // Check cardinality limits
            if let Some(Cardinality::One) = cardinality {
                // For single-select, replace the value
                values.clear();
            } else if let Some(Cardinality::Many { max: Some(max) }) = cardinality {
                // Check if at max
                if values.len() >= max as usize {
                    return;
                }
            }
            if !values.contains(&value) {
                values.push(value);
            }
        }
    }

    /// Remove a value from a slot
    pub fn remove_slot_value(&mut self, slot_label: &str, value: &str) {
        if let Some(values) = self.slot_values.get_mut(slot_label) {
            values.retain(|v| v != value);
        }
    }

    /// Set all values for a slot (used for reordering)
    pub fn set_slot_values(&mut self, slot_label: &str, new_values: Vec<String>) {
        if let Some(values) = self.slot_values.get_mut(slot_label) {
            *values = new_values;
        }
    }

    /// Set the single value for a textarea slot
    pub fn set_textarea_value(&mut self, slot_label: &str, value: String) {
        if let Some(values) = self.slot_values.get_mut(slot_label) {
            values.clear();
            if !value.is_empty() {
                values.push(value);
            }
        }
    }

    /// Get the textarea value for a slot (first value or empty string)
    pub fn get_textarea_value(&self, slot_label: &str) -> String {
        self.slot_values
            .get(slot_label)
            .and_then(|v| v.first())
            .cloned()
            .unwrap_or_default()
    }

    // ==================== Variable Editor Methods ====================

    /// Enter variable editor mode for an existing variable
    pub fn enter_variable_editor(&mut self, variable_name: &str) {
        // Find the variable in the current library and extract data
        let variable_data = self.selected_library().and_then(|library| {
            library
                .variables
                .iter()
                .find(|g| g.name == variable_name)
                .map(|variable| (variable.name.clone(), variable.options.clone()))
        });

        if let Some((name, options)) = variable_data {
            self.variable_editor_name = name.clone();
            self.variable_editor_content = Self::options_to_text(&options);
            self.variable_editor_original_name = Some(name);
            self.variable_editor_dirty = false;
            self.editor_mode = EditorMode::VariableEditor {
                variable_name: variable_name.to_string(),
            };
            // Switch sidebar to variables view
            self.sidebar_view_mode = SidebarViewMode::Variables;
            self.sidebar_mode = SidebarMode::Normal;
        }
    }

    /// Enter variable editor mode for creating a new variable
    pub fn enter_new_variable_editor(&mut self) {
        self.variable_editor_name = String::new();
        self.variable_editor_content = String::new();
        self.variable_editor_original_name = None;
        self.variable_editor_dirty = false;
        self.editor_mode = EditorMode::NewVariable;
        // Switch sidebar to variables view
        self.sidebar_view_mode = SidebarViewMode::Variables;
        self.sidebar_mode = SidebarMode::Normal;
    }

    /// Exit variable editor mode and return to template editor
    /// Returns false if there are unsaved changes (caller should show confirmation)
    pub fn try_exit_variable_editor(&mut self) -> bool {
        if self.variable_editor_dirty {
            self.confirm_dialog = Some(ConfirmDialog::DiscardVariableChanges);
            return false;
        }
        self.exit_variable_editor_force();
        true
    }

    /// Force exit variable editor mode (discards any unsaved changes)
    pub fn exit_variable_editor_force(&mut self) {
        self.editor_mode = EditorMode::Template;
        self.variable_editor_name.clear();
        self.variable_editor_content.clear();
        self.variable_editor_original_name = None;
        self.variable_editor_dirty = false;
        self.confirm_dialog = None;
    }

    /// Mark the variable editor as having changes
    pub fn mark_variable_editor_dirty(&mut self) {
        self.variable_editor_dirty = true;
    }

    /// Parse options text into a Vec of options.
    ///
    /// Format:
    /// - Each line is a separate option by default
    /// - `---` on its own line marks the START of a multiline option
    /// - The multiline option continues until the next `---` or end of text
    ///
    /// Example:
    /// ```text
    /// option 1
    /// option 2
    /// ---
    /// some
    ///
    /// multiline option
    /// ---
    /// option 3
    /// ```
    /// Produces: ["option 1", "option 2", "some\n\nmultiline option", "option 3"]
    pub fn parse_options(text: &str) -> Vec<String> {
        let mut options = Vec::new();
        let mut in_multiline = false;
        let mut multiline_buffer = String::new();

        for line in text.lines() {
            if line.trim() == "---" {
                if in_multiline {
                    // End of multiline option
                    let trimmed = multiline_buffer.trim().to_string();
                    if !trimmed.is_empty() {
                        options.push(trimmed);
                    }
                    multiline_buffer.clear();
                    in_multiline = false;
                } else {
                    // Start of multiline option
                    in_multiline = true;
                }
            } else if in_multiline {
                // Inside a multiline option - preserve newlines
                if !multiline_buffer.is_empty() {
                    multiline_buffer.push('\n');
                }
                multiline_buffer.push_str(line);
            } else {
                // Single-line option
                let trimmed = line.trim();
                if !trimmed.is_empty() {
                    options.push(trimmed.to_string());
                }
            }
        }

        // Handle unclosed multiline block
        if in_multiline && !multiline_buffer.trim().is_empty() {
            options.push(multiline_buffer.trim().to_string());
        }

        options
    }

    /// Convert options Vec to text format.
    ///
    /// Single-line options are output as-is (one per line).
    /// Multi-line options are wrapped with `---` delimiters.
    pub fn options_to_text(options: &[String]) -> String {
        options
            .iter()
            .map(|opt| {
                if opt.contains('\n') {
                    // Multiline option - wrap with ---
                    format!("---\n{}\n---", opt)
                } else {
                    opt.clone()
                }
            })
            .collect::<Vec<_>>()
            .join("\n")
    }

    /// Get the current options count from the editor content
    pub fn get_variable_editor_option_count(&self) -> usize {
        Self::parse_options(&self.variable_editor_content).len()
    }

    /// Validate variable name (returns error message if invalid)
    pub fn validate_variable_name(&self) -> Option<String> {
        let name = self.variable_editor_name.trim();

        if name.is_empty() {
            return Some("Variable name cannot be empty".to_string());
        }

        // Check for duplicate names (excluding the original name if editing)
        if let Some(library) = self.selected_library() {
            let is_duplicate = library.variables.iter().any(|g| {
                g.name == name && Some(&g.name) != self.variable_editor_original_name.as_ref()
            });
            if is_duplicate {
                return Some(format!("A variable named \"{}\" already exists", name));
            }
        }

        None
    }

    /// Request to delete a variable (shows confirmation dialog)
    pub fn request_delete_variable(&mut self, variable_name: &str) {
        self.confirm_dialog = Some(ConfirmDialog::DeleteVariable {
            variable_name: variable_name.to_string(),
        });
    }

    /// Cancel any active confirmation dialog
    pub fn cancel_confirm_dialog(&mut self) {
        self.confirm_dialog = None;
    }

    // ==================== Autocomplete Methods (per-editor) ====================

    /// Get the autocomplete state for a specific editor
    pub fn get_autocomplete(&self, editor_id: &str) -> Option<&AutocompleteState> {
        self.autocomplete_states.get(editor_id)
    }

    /// Get mutable autocomplete state for a specific editor, creating if needed
    pub fn get_autocomplete_mut(&mut self, editor_id: &str) -> &mut AutocompleteState {
        self.autocomplete_states
            .entry(editor_id.to_string())
            .or_default()
    }

    /// Check if autocomplete is active for a specific editor
    pub fn is_autocomplete_active(&self, editor_id: &str) -> bool {
        self.autocomplete_states
            .get(editor_id)
            .is_some_and(|s| s.active)
    }

    /// Activate autocomplete with the given trigger position for a specific editor
    pub fn activate_autocomplete(&mut self, editor_id: &str, trigger_position: usize) {
        let state = self.get_autocomplete_mut(editor_id);
        state.active = true;
        state.trigger_position = trigger_position;
        state.query.clear();
        state.mode = Some(AutocompleteMode::Variables);
        state.selected_index = 0;
    }

    /// Deactivate autocomplete for a specific editor
    pub fn deactivate_autocomplete(&mut self, editor_id: &str) {
        if let Some(state) = self.autocomplete_states.get_mut(editor_id) {
            state.active = false;
            state.query.clear();
            state.mode = None;
            state.selected_index = 0;
            state.trigger_position = 0;
            state.editor_response_id = None;
        }
    }

    /// Deactivate autocomplete for all editors except the specified one
    pub fn deactivate_autocomplete_except(&mut self, editor_id: &str) {
        for (id, state) in &mut self.autocomplete_states {
            if id != editor_id {
                state.active = false;
                state.query.clear();
                state.mode = None;
                state.selected_index = 0;
                state.trigger_position = 0;
                state.editor_response_id = None;
            }
        }
    }

    /// Update autocomplete query based on cursor position and text content for a specific editor
    pub fn update_autocomplete_query(&mut self, editor_id: &str, content: &str, cursor_pos: usize) {
        let Some(state) = self.autocomplete_states.get_mut(editor_id) else {
            return;
        };
        if !state.active {
            return;
        }

        // Extract text from trigger position to cursor
        let trigger = state.trigger_position;
        if cursor_pos <= trigger || cursor_pos > content.len() {
            // Cursor moved before the @, deactivate
            state.active = false;
            state.query.clear();
            state.mode = None;
            state.selected_index = 0;
            state.trigger_position = 0;
            return;
        }

        // Get the text after @ up to cursor
        let query_text = &content[trigger + 1..cursor_pos]; // +1 to skip the @

        // Check if query contains whitespace or invalid chars (cancel autocomplete)
        if query_text.contains(char::is_whitespace) {
            state.active = false;
            state.query.clear();
            state.mode = None;
            state.selected_index = 0;
            state.trigger_position = 0;
            return;
        }

        // Parse the query to determine mode
        if let Some(slash_pos) = query_text.find('/') {
            // @Variable/option syntax - switch to options mode
            let variable_part = &query_text[..slash_pos];
            let option_part = &query_text[slash_pos + 1..];

            // Only reset selection if the query actually changed
            let new_query = option_part.to_string();
            let query_changed = state.query != new_query;

            state.mode = Some(AutocompleteMode::Options {
                variable_name: variable_part.to_string(),
            });
            state.query = new_query;

            if query_changed {
                state.selected_index = 0;
            }
        } else {
            // @Variable syntax - stay in variables mode
            let new_query = query_text.to_string();
            let query_changed = state.query != new_query;

            state.mode = Some(AutocompleteMode::Variables);
            state.query = new_query;

            if query_changed {
                state.selected_index = 0;
            }
        }
    }

    /// Move autocomplete selection up for a specific editor
    pub fn autocomplete_move_up(&mut self, editor_id: &str, total_items: usize) {
        if total_items == 0 {
            return;
        }
        if let Some(state) = self.autocomplete_states.get_mut(editor_id) {
            if state.selected_index == 0 {
                state.selected_index = total_items - 1;
            } else {
                state.selected_index -= 1;
            }
        }
    }

    /// Move autocomplete selection down for a specific editor
    pub fn autocomplete_move_down(&mut self, editor_id: &str, total_items: usize) {
        if total_items == 0 {
            return;
        }
        if let Some(state) = self.autocomplete_states.get_mut(editor_id) {
            state.selected_index = (state.selected_index + 1) % total_items;
        }
    }

    /// Set pending cursor position for a specific editor
    pub fn set_pending_cursor_position(&mut self, editor_id: &str, position: usize) {
        self.pending_cursor_positions
            .insert(editor_id.to_string(), position);
    }

    /// Take pending cursor position for a specific editor (returns and clears it)
    pub fn take_pending_cursor_position(&mut self, editor_id: &str) -> Option<usize> {
        self.pending_cursor_positions.remove(editor_id)
    }
}
