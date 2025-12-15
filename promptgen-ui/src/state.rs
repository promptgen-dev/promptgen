use std::collections::HashMap;
use std::path::PathBuf;

use promptgen_core::{EvalContext, Library, ParseResult, RenderError, Workspace, render};
use serde::{Deserialize, Serialize};

/// Sidebar view mode - what to show in the sidebar list
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
pub enum SidebarViewMode {
    #[default]
    Templates,
    Variables,
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
    pub slot_values: HashMap<String, String>,
    pub auto_randomize_seed: bool,
    pub auto_render: bool,

    // UI State
    pub sidebar_view_mode: SidebarViewMode,
    pub search_query: String,
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
            sidebar_view_mode: SidebarViewMode::default(),
            search_query: String::new(),
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

            // Set slot overrides
            for (name, value) in &self.slot_values {
                if !value.is_empty() {
                    ctx.set_slot(name.clone(), value.clone());
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
}
