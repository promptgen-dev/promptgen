use std::collections::HashMap;
use std::path::PathBuf;

use promptgen_core::{Library, ParseResult, Workspace};
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
    }
}
