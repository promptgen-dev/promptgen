use std::collections::HashMap;

use promptgen_core::{
    Cardinality, EvalContext, Library, ParseResult, PickSource, RenderError, SlotDefKind,
    SlotDefaults, SlotDefinition, SlotValue, render,
};
use serde::{Deserialize, Serialize};

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

/// Sidebar view mode - what to show in the sidebar list
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
pub enum SidebarViewMode {
    #[default]
    Prompts,
    Variables,
}

/// Variable list sort order
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
pub enum VariableSortOrder {
    /// No sorting - show in library order
    #[default]
    None,
    /// Sort A-Z by variable name
    Ascending,
    /// Sort Z-A by variable name
    Descending,
}

/// Sidebar mode - normal navigation vs slot picker overlay
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub enum SidebarMode {
    /// Normal mode showing prompts/variables
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
    /// Main prompt editor is focused
    MainEditor,
    /// A textarea slot is focused
    TextareaSlot { label: String },
    /// A pick slot is focused (opens sidebar picker)
    PickSlot { label: String },
}

/// What the central editor panel is currently showing
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub enum EditorMode {
    /// Normal prompt editing mode
    #[default]
    Prompt,
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
    /// Confirm closing a tab with unsaved changes
    CloseUnsavedTab { tab_index: usize },
    /// Confirm deleting a prompt from the library
    DeletePrompt { prompt_name: String },
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
    /// Flag to scroll to selection (set on keyboard nav, cleared after scroll)
    pub scroll_to_automcomplete_selection: bool,
}

// ==================== Tab State ====================

/// Origin of a prompt tab - tracks where it came from
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
pub enum PromptSource {
    /// Created via [+ New] button - not yet saved to library
    #[default]
    New,
    /// Opened from a saved prompt in the library
    FromLibrary {
        /// Original name when opened (for tracking renames)
        original_name: String,
    },
}

/// A prompt tab being edited
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PromptTab {
    /// Tab/prompt name (must be unique across tabs and library)
    pub name: String,
    /// Prompt content (the text being edited)
    pub content: String,
    /// Slot values for this tab (independent per tab)
    pub slots: HashMap<String, SlotValue>,
    /// Default separator and suffix for slots
    #[serde(default)]
    pub slot_defaults: SlotDefaults,
    /// Where this tab originated from
    pub source: PromptSource,
    /// Whether this tab has unsaved changes
    #[serde(default)]
    pub dirty: bool,
}

impl Default for PromptTab {
    fn default() -> Self {
        Self {
            name: "Prompt 1".to_string(),
            content: String::new(),
            slots: HashMap::new(),
            slot_defaults: SlotDefaults::default(),
            source: PromptSource::New,
            dirty: false,
        }
    }
}

impl PromptTab {
    /// Create a new tab with the given name
    pub fn new(name: String) -> Self {
        Self {
            name,
            content: String::new(),
            slots: HashMap::new(),
            slot_defaults: SlotDefaults::default(),
            source: PromptSource::New,
            dirty: false,
        }
    }

    /// Create a tab from a saved library prompt
    pub fn from_library_prompt(
        name: String,
        content: String,
        slots: HashMap<String, SlotValue>,
        slot_defaults: SlotDefaults,
    ) -> Self {
        Self {
            name: name.clone(),
            content,
            slots,
            slot_defaults,
            source: PromptSource::FromLibrary {
                original_name: name,
            },
            dirty: false,
        }
    }
}

/// Main application state (not serialized - rebuilt on startup)
pub struct AppState {
    // Library
    pub library: Library,
    pub library_path: Option<std::path::PathBuf>,

    // Prompt Tabs
    pub prompt_tabs: Vec<PromptTab>,
    pub active_tab_index: Option<usize>,

    // Tab Rename State (ephemeral)
    pub tab_rename_index: Option<usize>,
    pub tab_rename_text: String,

    // Editor (legacy - will be replaced by active tab)
    pub editor_content: String,
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
    pub variable_sort_order: VariableSortOrder,
    pub option_sort_order: VariableSortOrder,
    pub sidebar_mode: SidebarMode,
    pub search_query: String,
    pub slot_picker_search_query: String,
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

    // Variable list expand/collapse all (consumed on next render)
    pub expand_all_variables: Option<bool>,
}

impl Default for AppState {
    fn default() -> Self {
        // Create a default first tab
        let first_tab = PromptTab::default();
        Self {
            library: Library::default(),
            library_path: None,
            prompt_tabs: vec![first_tab],
            active_tab_index: Some(0),
            tab_rename_index: None,
            tab_rename_text: String::new(),
            editor_content: String::new(),
            parse_result: None,
            preview_output: String::new(),
            preview_seed: None,
            slot_values: HashMap::new(),
            auto_randomize_seed: true,
            auto_render: true,
            preview_dirty: false,
            sidebar_view_mode: SidebarViewMode::default(),
            variable_sort_order: VariableSortOrder::default(),
            option_sort_order: VariableSortOrder::default(),
            sidebar_mode: SidebarMode::default(),
            search_query: String::new(),
            slot_picker_search_query: String::new(),
            editor_focus: EditorFocus::default(),
            editor_mode: EditorMode::default(),
            variable_editor_name: String::new(),
            variable_editor_content: String::new(),
            variable_editor_original_name: None,
            variable_editor_dirty: false,
            confirm_dialog: None,
            autocomplete_states: HashMap::new(),
            pending_cursor_positions: HashMap::new(),
            expand_all_variables: None,
        }
    }
}

impl AppState {
    /// Update parse result when editor content changes
    pub fn update_parse_result(&mut self) {
        self.parse_result = Some(self.library.parse_prompt(&self.editor_content));
        // Update slot values map - add new slots, keep existing values
        if let Some(result) = &self.parse_result
            && let Some(ast) = &result.ast
        {
            let current_slots = self.library.get_slots(ast);
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

    /// Render the current prompt with the given seed
    pub fn render_prompt(&mut self) -> Result<(), RenderError> {
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

            let mut ctx = EvalContext::with_seed(&self.library, seed);

            // Set slot defaults from active tab
            if let Some(tab) = self.get_active_tab() {
                ctx.set_slot_defaults(tab.slot_defaults.clone());
            }

            // Set slot overrides (multi-value)
            for (name, values) in &self.slot_values {
                if !values.is_empty() {
                    ctx.set_slot_values(name.clone(), values.clone());
                }
            }

            let render_result = render(ast, &mut ctx)?;
            self.preview_output = render_result.text.trim().to_string();

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
            let _ = self.render_prompt();
            self.preview_dirty = false;
        }
    }

    /// Get slot definitions from the current prompt
    pub fn get_slot_definitions(&self) -> Vec<SlotDefinition> {
        if let Some(result) = &self.parse_result
            && let Some(ast) = &result.ast
        {
            let defaults = self
                .get_active_tab()
                .map(|t| t.slot_defaults.clone())
                .unwrap_or_default();
            return self.library.get_slot_definitions(ast, &defaults);
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
        // Reset slot picker state - clear search and expand all groups
        self.slot_picker_search_query.clear();
        self.expand_all_variables = Some(true);
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

    /// Get option groups for a pick slot, separating variables from free-form literals.
    /// Returns groups in source order, with literals grouped under "Options" (shown first unless sorted).
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
                    // Resolve variable reference
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

        // Add literals as "Options" group at the beginning (unless sorted)
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
                self.mark_active_tab_dirty();
            }
        }
    }

    /// Remove a value from a slot
    pub fn remove_slot_value(&mut self, slot_label: &str, value: &str) {
        if let Some(values) = self.slot_values.get_mut(slot_label) {
            let len_before = values.len();
            values.retain(|v| v != value);
            if values.len() != len_before {
                self.mark_active_tab_dirty();
            }
        }
    }

    /// Set all values for a slot (used for reordering)
    pub fn set_slot_values(&mut self, slot_label: &str, new_values: Vec<String>) {
        if let Some(values) = self.slot_values.get_mut(slot_label)
            && *values != new_values
        {
            *values = new_values;
            self.mark_active_tab_dirty();
        }
    }

    /// Set the single value for a textarea slot
    pub fn set_textarea_value(&mut self, slot_label: &str, value: String) {
        if let Some(values) = self.slot_values.get_mut(slot_label) {
            let old_value = values.first().cloned().unwrap_or_default();
            if old_value != value {
                values.clear();
                if !value.is_empty() {
                    values.push(value);
                }
                self.mark_active_tab_dirty();
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

    /// Clear all slot values
    pub fn clear_all_slot_values(&mut self) {
        let had_values = self.slot_values.values().any(|v| !v.is_empty());
        for values in self.slot_values.values_mut() {
            values.clear();
        }
        if had_values {
            self.mark_active_tab_dirty();
            self.request_render();
        }
    }

    /// Clear a single slot's values
    pub fn clear_slot(&mut self, slot_label: &str) {
        if let Some(values) = self.slot_values.get_mut(slot_label)
            && !values.is_empty()
        {
            values.clear();
            self.mark_active_tab_dirty();
            self.request_render();
        }
    }

    // ==================== Variable Editor Methods ====================

    /// Enter variable editor mode for an existing variable
    pub fn enter_variable_editor(&mut self, variable_name: &str) {
        // Find the variable in the library and extract data
        let variable_data = self
            .library
            .variables
            .iter()
            .find(|g| g.name == variable_name)
            .map(|variable| (variable.name.clone(), variable.options.clone()));

        if let Some((name, options)) = variable_data {
            self.variable_editor_name = name.clone();
            self.variable_editor_content = Self::options_to_text(&options);
            self.variable_editor_original_name = Some(name);
            self.variable_editor_dirty = false;
            self.editor_mode = EditorMode::VariableEditor {
                variable_name: variable_name.to_string(),
            };
            // Switch sidebar to variables view, but preserve slot picker mode
            if !matches!(self.sidebar_mode, SidebarMode::SlotPicker { .. }) {
                self.sidebar_view_mode = SidebarViewMode::Variables;
                self.sidebar_mode = SidebarMode::Normal;
            }
        }
    }

    /// Enter variable editor mode for creating a new variable
    pub fn enter_new_variable_editor(&mut self) {
        self.variable_editor_name = String::new();
        self.variable_editor_content = String::new();
        self.variable_editor_original_name = None;
        self.variable_editor_dirty = false;
        self.editor_mode = EditorMode::NewVariable;
        // Switch sidebar to variables view, but preserve slot picker mode
        if !matches!(self.sidebar_mode, SidebarMode::SlotPicker { .. }) {
            self.sidebar_view_mode = SidebarViewMode::Variables;
            self.sidebar_mode = SidebarMode::Normal;
        }
    }

    /// Exit variable editor mode and return to prompt editor
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
        self.editor_mode = EditorMode::Prompt;
        self.variable_editor_name.clear();
        self.variable_editor_content.clear();
        self.variable_editor_original_name = None;
        self.variable_editor_dirty = false;
        self.confirm_dialog = None;
        // Re-parse the prompt to pick up any new/changed variables
        self.update_parse_result();
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
        let is_duplicate = self.library.variables.iter().any(|g| {
            g.name == name && Some(&g.name) != self.variable_editor_original_name.as_ref()
        });
        if is_duplicate {
            return Some(format!("A variable named \"{}\" already exists", name));
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

    /// Take the scroll_to_selection flag (returns current value and clears it)
    pub fn take_autocomplete_scroll_flag(&mut self, editor_id: &str) -> bool {
        if let Some(state) = self.autocomplete_states.get_mut(editor_id) {
            let should_scroll = state.scroll_to_automcomplete_selection;
            state.scroll_to_automcomplete_selection = false;
            should_scroll
        } else {
            false
        }
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
            state.scroll_to_automcomplete_selection = true;
        }
    }

    /// Move autocomplete selection down for a specific editor
    pub fn autocomplete_move_down(&mut self, editor_id: &str, total_items: usize) {
        if total_items == 0 {
            return;
        }
        if let Some(state) = self.autocomplete_states.get_mut(editor_id) {
            state.selected_index = (state.selected_index + 1) % total_items;
            state.scroll_to_automcomplete_selection = true;
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

    // ==================== Tab Management Methods ====================

    /// Get the active tab (immutable reference)
    pub fn get_active_tab(&self) -> Option<&PromptTab> {
        self.active_tab_index
            .and_then(|idx| self.prompt_tabs.get(idx))
    }

    /// Get the active tab (mutable reference)
    pub fn get_active_tab_mut(&mut self) -> Option<&mut PromptTab> {
        self.active_tab_index
            .and_then(|idx| self.prompt_tabs.get_mut(idx))
    }

    /// Switch to a specific tab by index
    pub fn switch_to_tab(&mut self, index: usize) {
        if index < self.prompt_tabs.len() {
            // Save current slot values to the old active tab before switching
            self.save_slot_values_to_active_tab();

            self.active_tab_index = Some(index);
            // Sync editor_content and slot_values with the new active tab
            if let Some(tab) = self.prompt_tabs.get(index) {
                self.editor_content = tab.content.clone();
                // Convert tab's SlotValue HashMap to the working slot_values format
                self.slot_values = Self::slot_values_to_vec_map(&tab.slots);
                self.update_parse_result();
                // Render immediately so preview is updated when switching tabs
                self.request_render();
            }
        }
    }

    /// Convert SlotValue HashMap to Vec<String> HashMap (working format)
    pub fn slot_values_to_vec_map(
        slots: &HashMap<String, SlotValue>,
    ) -> HashMap<String, Vec<String>> {
        slots
            .iter()
            .map(|(k, v)| {
                let vec = match v {
                    SlotValue::Text(s) => {
                        if s.is_empty() {
                            Vec::new()
                        } else {
                            vec![s.clone()]
                        }
                    }
                    SlotValue::Pick(items) => items.clone(),
                };
                (k.clone(), vec)
            })
            .collect()
    }

    /// Convert Vec<String> HashMap back to SlotValue HashMap
    /// Uses slot definitions to determine whether each is Text or Pick
    fn vec_map_to_slot_values(
        vec_map: &HashMap<String, Vec<String>>,
        definitions: &[promptgen_core::SlotDefinition],
    ) -> HashMap<String, SlotValue> {
        vec_map
            .iter()
            .map(|(k, v)| {
                // Look up the slot definition to determine type
                let is_textarea = definitions
                    .iter()
                    .find(|d| d.label == *k)
                    .is_some_and(|d| matches!(d.kind, promptgen_core::SlotDefKind::Textarea));

                let slot_value = if is_textarea {
                    SlotValue::Text(v.first().cloned().unwrap_or_default())
                } else {
                    SlotValue::Pick(v.clone())
                };
                (k.clone(), slot_value)
            })
            .collect()
    }

    /// Save current slot_values to the active tab
    pub fn save_slot_values_to_active_tab(&mut self) {
        let definitions = self.get_slot_definitions();
        let slot_values = Self::vec_map_to_slot_values(&self.slot_values, &definitions);

        if let Some(tab) = self.get_active_tab_mut()
            && tab.slots != slot_values
        {
            tab.slots = slot_values;
            tab.dirty = true;
        }
    }

    /// Create a new tab with an auto-generated unique name
    pub fn create_new_tab(&mut self) -> usize {
        let name = self.find_next_prompt_name();
        let tab = PromptTab::new(name);
        self.prompt_tabs.push(tab);
        let new_index = self.prompt_tabs.len() - 1;
        self.switch_to_tab(new_index);
        new_index
    }

    /// Try to close a tab by index
    /// If the tab has unsaved changes, shows a confirmation dialog
    /// Returns true if the tab was closed immediately, false if confirmation is needed or cannot close
    pub fn try_close_tab(&mut self, index: usize) -> bool {
        if self.prompt_tabs.len() <= 1 {
            // Don't close the last tab
            return false;
        }

        if index >= self.prompt_tabs.len() {
            return false;
        }

        // Check if tab has unsaved changes
        if let Some(tab) = self.prompt_tabs.get(index)
            && tab.dirty
        {
            // Show confirmation dialog
            self.confirm_dialog = Some(ConfirmDialog::CloseUnsavedTab { tab_index: index });
            return false;
        }

        // Tab is clean, close immediately
        self.close_tab_force(index)
    }

    /// Close a tab by index without checking for unsaved changes
    /// Returns true if the tab was closed, false if it was the last tab
    pub fn close_tab_force(&mut self, index: usize) -> bool {
        if self.prompt_tabs.len() <= 1 {
            // Don't close the last tab
            return false;
        }

        if index >= self.prompt_tabs.len() {
            return false;
        }

        self.prompt_tabs.remove(index);

        // Adjust active tab index
        if let Some(active) = self.active_tab_index {
            if active >= self.prompt_tabs.len() {
                // Was pointing past end, move to last tab
                self.active_tab_index = Some(self.prompt_tabs.len() - 1);
            } else if active > index {
                // Was pointing after removed tab, shift down
                self.active_tab_index = Some(active - 1);
            }
            // If active was pointing before removed tab, no change needed
        }

        // Sync editor content and slot values with new active tab
        if let Some(idx) = self.active_tab_index {
            if let Some(tab) = self.prompt_tabs.get(idx) {
                self.editor_content = tab.content.clone();
                self.slot_values = Self::slot_values_to_vec_map(&tab.slots);
            }
            self.update_parse_result();
            // Render immediately so preview is updated after closing a tab
            self.request_render();
        }

        true
    }

    /// Find the next available sequential prompt name ("Prompt 1", "Prompt 2", etc.)
    pub fn find_next_prompt_name(&self) -> String {
        let mut n = 1;
        loop {
            let candidate = format!("Prompt {}", n);
            if self.is_name_available(&candidate) {
                return candidate;
            }
            n += 1;
        }
    }

    /// Check if a name is available (not used by any tab or library prompt)
    pub fn is_name_available(&self, name: &str) -> bool {
        // Check tabs
        let used_in_tabs = self.prompt_tabs.iter().any(|t| t.name == name);
        if used_in_tabs {
            return false;
        }

        // Check library prompts
        let used_in_library = self.library.prompts.iter().any(|p| p.name == name);
        !used_in_library
    }

    /// Check if a name is available for renaming a specific tab
    /// (excludes the tab's current name from the check)
    pub fn is_name_available_for_rename(&self, name: &str, tab_index: usize) -> bool {
        // Check other tabs (excluding the one being renamed)
        let used_in_other_tabs = self
            .prompt_tabs
            .iter()
            .enumerate()
            .any(|(i, t)| i != tab_index && t.name == name);
        if used_in_other_tabs {
            return false;
        }

        // Check library prompts (but allow if this tab came from that prompt)
        if let Some(tab) = self.prompt_tabs.get(tab_index)
            && let PromptSource::FromLibrary { original_name } = &tab.source
        {
            // Allow keeping/restoring the original name
            if name == original_name {
                return true;
            }
        }

        let used_in_library = self.library.prompts.iter().any(|p| p.name == name);
        !used_in_library
    }

    /// Find a tab by its source library prompt name
    pub fn find_tab_by_library_prompt(&self, prompt_name: &str) -> Option<usize> {
        self.prompt_tabs.iter().position(|tab| {
            matches!(&tab.source, PromptSource::FromLibrary { original_name } if original_name == prompt_name)
        })
    }

    /// Open a saved prompt from the library in a tab
    /// Returns the tab index (existing or new)
    pub fn open_library_prompt(&mut self, prompt_name: &str) -> Option<usize> {
        // Check if already open
        if let Some(index) = self.find_tab_by_library_prompt(prompt_name) {
            self.switch_to_tab(index);
            return Some(index);
        }

        // Find the prompt in the library
        let prompt = self
            .library
            .prompts
            .iter()
            .find(|p| p.name == prompt_name)?;

        // Create new tab from the library prompt
        let tab = PromptTab::from_library_prompt(
            prompt.name.clone(),
            prompt.content.clone(),
            prompt.slots.clone(),
            prompt.slot_defaults.clone(),
        );

        self.prompt_tabs.push(tab);
        let new_index = self.prompt_tabs.len() - 1;
        self.switch_to_tab(new_index);
        Some(new_index)
    }

    /// Mark the active tab as dirty (has unsaved changes)
    pub fn mark_active_tab_dirty(&mut self) {
        if let Some(tab) = self.get_active_tab_mut() {
            tab.dirty = true;
        }
    }

    /// Sync active tab content from editor_content
    /// Call this when editor_content changes
    pub fn sync_active_tab_content(&mut self) {
        let new_content = self.editor_content.clone();
        if let Some(tab) = self.get_active_tab_mut()
            && tab.content != new_content
        {
            tab.content = new_content;
            tab.dirty = true;
        }
    }

    /// Save the active tab to the library
    /// Returns true if successful, false if no active tab
    pub fn save_active_tab_to_library(&mut self) -> bool {
        // First sync current slot values to the tab
        self.save_slot_values_to_active_tab();

        // Get the active tab index and data we need
        let Some(idx) = self.active_tab_index else {
            return false;
        };
        let Some(tab) = self.prompt_tabs.get(idx) else {
            return false;
        };

        let tab_name = tab.name.clone();
        let tab_content = tab.content.clone();
        let tab_slots = tab.slots.clone();
        let tab_slot_defaults = tab.slot_defaults.clone();
        let tab_source = tab.source.clone();

        // Create or update the library prompt
        match &tab_source {
            PromptSource::New => {
                // New prompt - add to library
                let saved_prompt = promptgen_core::SavedPrompt {
                    name: tab_name.clone(),
                    content: tab_content,
                    slots: tab_slots,
                    slot_defaults: tab_slot_defaults,
                };
                self.library.prompts.push(saved_prompt);

                // Update tab source to reflect it's now from the library
                if let Some(tab) = self.prompt_tabs.get_mut(idx) {
                    tab.source = PromptSource::FromLibrary {
                        original_name: tab_name,
                    };
                    tab.dirty = false;
                }
            }
            PromptSource::FromLibrary { original_name } => {
                // Existing prompt - find and update it
                if let Some(prompt) = self
                    .library
                    .prompts
                    .iter_mut()
                    .find(|p| p.name == *original_name)
                {
                    // If name changed, also update the library prompt name
                    if prompt.name != tab_name {
                        prompt.name = tab_name.clone();
                    }
                    prompt.content = tab_content;
                    prompt.slots = tab_slots;
                    prompt.slot_defaults = tab_slot_defaults;

                    // Update the source to reflect any name change
                    if let Some(tab) = self.prompt_tabs.get_mut(idx) {
                        tab.source = PromptSource::FromLibrary {
                            original_name: tab_name,
                        };
                        tab.dirty = false;
                    }
                } else {
                    // Original prompt not found (maybe deleted), treat as new
                    let saved_prompt = promptgen_core::SavedPrompt {
                        name: tab_name.clone(),
                        content: tab_content,
                        slots: tab_slots,
                        slot_defaults: tab_slot_defaults,
                    };
                    self.library.prompts.push(saved_prompt);

                    if let Some(tab) = self.prompt_tabs.get_mut(idx) {
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

    // ==================== Tab Rename Methods ====================

    /// Start renaming a tab
    pub fn start_tab_rename(&mut self, index: usize) {
        if let Some(tab) = self.prompt_tabs.get(index) {
            self.tab_rename_index = Some(index);
            self.tab_rename_text = tab.name.clone();
        }
    }

    /// Cancel tab rename
    pub fn cancel_tab_rename(&mut self) {
        self.tab_rename_index = None;
        self.tab_rename_text.clear();
    }

    /// Commit tab rename if the new name is valid
    /// Returns true if rename was successful, false if name is invalid/duplicate
    pub fn commit_tab_rename(&mut self) -> bool {
        let Some(index) = self.tab_rename_index else {
            return false;
        };

        let new_name = self.tab_rename_text.trim().to_string();

        // Check if name is empty
        if new_name.is_empty() {
            return false;
        }

        // Check if name is available (excluding current tab)
        if !self.is_name_available_for_rename(&new_name, index) {
            return false;
        }

        // Apply the rename
        if let Some(tab) = self.prompt_tabs.get_mut(index)
            && tab.name != new_name
        {
            tab.name = new_name;
            tab.dirty = true;
        }

        // Clear rename state
        self.tab_rename_index = None;
        self.tab_rename_text.clear();

        true
    }

    /// Check if a tab is currently being renamed
    pub fn is_tab_renaming(&self, index: usize) -> bool {
        self.tab_rename_index == Some(index)
    }

    /// Validate the current tab rename text without committing
    /// Returns true if the name is valid and can be saved
    pub fn is_tab_rename_valid(&self) -> bool {
        let Some(index) = self.tab_rename_index else {
            return false;
        };

        let new_name = self.tab_rename_text.trim();

        // Check if name is empty
        if new_name.is_empty() {
            return false;
        }

        // Check if name is available (excluding current tab)
        self.is_name_available_for_rename(new_name, index)
    }

    // ==================== Prompt Management Methods ====================

    /// Request to delete a prompt (shows confirmation dialog)
    pub fn request_delete_prompt(&mut self, prompt_name: &str) {
        self.confirm_dialog = Some(ConfirmDialog::DeletePrompt {
            prompt_name: prompt_name.to_string(),
        });
    }

    /// Delete a prompt from the library
    /// Also closes any tabs that were opened from this prompt
    pub fn delete_prompt(&mut self, prompt_name: &str) {
        // Close any tabs that were opened from this prompt
        let tabs_to_close: Vec<usize> = self
            .prompt_tabs
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

        // Close tabs in reverse order to maintain correct indices
        for idx in tabs_to_close.into_iter().rev() {
            self.close_tab_force(idx);
        }

        // Remove the prompt from the library
        self.library.prompts.retain(|p| p.name != prompt_name);

        // Clear the confirmation dialog
        self.confirm_dialog = None;
    }
}
