//! Tab bar component for multi-tab prompt editing

use crate::state::AppState;
use egui_material_icons::icons::{ICON_ADD, ICON_CLOSE, ICON_SAVE};

/// Actions that can be triggered by the tab bar
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TabBarAction {
    /// No action
    None,
    /// Switch to a different tab
    SwitchTab(usize),
    /// Create a new tab
    NewTab,
    /// Close a tab (with index)
    CloseTab(usize),
    /// Request to rename a tab (with index)
    StartRename(usize),
    /// Save the active tab to library
    SaveTab,
}

/// Result of showing the tab bar
pub struct TabBarResult {
    /// The action that was triggered
    pub action: TabBarAction,
    /// Whether the library was modified and needs saving
    pub library_modified: bool,
}

/// Tab bar panel for prompt tabs
pub struct TabBarPanel;

impl TabBarPanel {
    /// Show the tab bar and return the result including any action triggered
    pub fn show(ui: &mut egui::Ui, state: &mut AppState) -> TabBarResult {
        let mut action = TabBarAction::None;

        ui.horizontal(|ui| {
            // [+ New] button
            if ui
                .button(format!("{} New", ICON_ADD))
                .on_hover_text("Create new prompt tab")
                .clicked()
            {
                action = TabBarAction::NewTab;
            }

            // [Save] button - enabled when active tab is dirty
            let can_save = state
                .get_active_tab()
                .is_some_and(|tab| tab.dirty);

            if ui
                .add_enabled(can_save, egui::Button::new(format!("{} Save", ICON_SAVE)))
                .on_hover_text("Save prompt to library")
                .clicked()
            {
                action = TabBarAction::SaveTab;
            }

            ui.separator();

            // Render each tab
            let active_index = state.active_tab_index;
            let tab_count = state.prompt_tabs.len();

            for i in 0..tab_count {
                let tab = &state.prompt_tabs[i];
                let is_active = active_index == Some(i);

                // Build tab label (name + dirty indicator)
                let label = if tab.dirty {
                    format!("{}*", tab.name)
                } else {
                    tab.name.clone()
                };

                // Create a selectable label for the tab
                let response = ui.selectable_label(is_active, &label);

                if response.clicked() && !is_active {
                    action = TabBarAction::SwitchTab(i);
                }

                // Double-click to rename (future feature)
                if response.double_clicked() {
                    action = TabBarAction::StartRename(i);
                }

                // Close button (only show if more than one tab)
                if tab_count > 1 {
                    let close_response = ui
                        .small_button(ICON_CLOSE)
                        .on_hover_text("Close tab");
                    if close_response.clicked() {
                        action = TabBarAction::CloseTab(i);
                    }
                }

                // Add separator between tabs
                if i < tab_count - 1 {
                    ui.separator();
                }
            }
        });

        // Process the action
        let mut library_modified = false;

        match &action {
            TabBarAction::NewTab => {
                state.create_new_tab();
            }
            TabBarAction::SwitchTab(index) => {
                state.switch_to_tab(*index);
            }
            TabBarAction::CloseTab(index) => {
                // TODO: Check for unsaved changes first
                state.close_tab(*index);
            }
            TabBarAction::StartRename(_) => {
                // TODO: Implement inline rename
            }
            TabBarAction::SaveTab => {
                state.save_active_tab_to_library();
                library_modified = true;
            }
            TabBarAction::None => {}
        }

        TabBarResult {
            action,
            library_modified,
        }
    }
}
