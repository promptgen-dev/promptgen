use std::collections::HashMap;
use std::path::PathBuf;

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

/// Persisted application configuration
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct AppConfig {
    pub workspace_path: Option<PathBuf>,
    pub selected_library_id: Option<String>,
    pub sidebar_width: f32,
    pub sidebar_view_mode: SidebarViewMode,
}

/// Main application state (not serialized - rebuilt on startup)
pub struct AppState {
    // Workspace
    pub workspace: Workspace,
    pub libraries: Vec<Library>,
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
}

impl Default for AppState {
    fn default() -> Self {
        Self {
            workspace: Workspace::new(),
            libraries: Vec::new(),
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
                && !self.slot_values.contains_key(label) {
                    self.editor_focus = EditorFocus::None;
                    self.sidebar_mode = SidebarMode::Normal;
                }
        }
    }

    /// Get the list of slot names from the current template
    pub fn get_current_slots(&self) -> Vec<String> {
        if let Some(result) = &self.parse_result
            && let Some(ast) = &result.ast
        {
            return self.workspace.get_slots(ast);
        }
        Vec::new()
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

    /// Get expanded options for a pick slot, resolving group references
    pub fn get_pick_options(&self, slot_label: &str) -> Vec<String> {
        let definitions = self.get_slot_definitions();
        if let Some(def) = definitions.iter().find(|d| d.label == slot_label)
            && let SlotDefKind::Pick { sources, .. } = &def.kind {
                let mut options = Vec::new();
                for source in sources {
                    match source {
                        PickSource::GroupRef(lib_ref) => {
                            // Resolve group reference
                            let matches = if let Some(lib_name) = &lib_ref.library {
                                self.workspace
                                    .find_group_in_library(lib_name, &lib_ref.group)
                                    .into_iter()
                                    .collect::<Vec<_>>()
                            } else {
                                self.workspace.find_groups(&lib_ref.group)
                            };
                            // Add all options from matched groups
                            for (_lib, group) in matches {
                                options.extend(group.options.iter().cloned());
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
}
