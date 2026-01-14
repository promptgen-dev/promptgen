//! Sidebar state for the sidebar panel.

use serde::{Deserialize, Serialize};

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

/// State for the slot picker (when selecting options for a pick slot)
#[derive(Debug, Clone, Default)]
pub struct SlotPickerState {
    /// Search query for filtering options
    pub search_query: String,
    /// Sort order for variables in the picker
    pub sort_order: VariableSortOrder,
    /// Sort order for options within variables
    pub option_sort_order: VariableSortOrder,
}

impl SlotPickerState {
    /// Clear slot picker state
    pub fn clear(&mut self) {
        self.search_query.clear();
    }
}

/// State for the sidebar panel
#[derive(Debug, Clone, Default)]
pub struct SidebarState {
    /// What view to show in the sidebar (Prompts vs Variables)
    pub view_mode: SidebarViewMode,
    /// Current sidebar mode (Normal vs SlotPicker)
    pub mode: SidebarMode,
    /// Search query for filtering the sidebar list
    pub search_query: String,
    /// Sort order for variables in the sidebar
    pub variable_sort_order: VariableSortOrder,
    /// Sort order for options within variables
    pub option_sort_order: VariableSortOrder,
    /// Sort order for prompts in the sidebar
    pub prompt_sort_order: VariableSortOrder,
    /// State for the slot picker overlay
    pub slot_picker: SlotPickerState,
    /// Variable list expand/collapse all (consumed on next render)
    pub expand_all_variables: Option<bool>,
}

impl SidebarState {
    /// Enter slot picker mode for a specific slot
    pub fn enter_slot_picker(&mut self, slot_label: &str) {
        self.mode = SidebarMode::SlotPicker {
            slot_label: slot_label.to_string(),
        };
        // Reset slot picker state - clear search and expand all groups
        self.slot_picker.search_query.clear();
        self.expand_all_variables = Some(true);
    }

    /// Exit slot picker mode and return to normal
    pub fn exit_slot_picker(&mut self) {
        self.mode = SidebarMode::Normal;
    }

    /// Clear sidebar state (used when resetting app state)
    pub fn clear(&mut self) {
        self.mode = SidebarMode::Normal;
        self.search_query.clear();
        self.slot_picker.clear();
        self.expand_all_variables = None;
    }
}
