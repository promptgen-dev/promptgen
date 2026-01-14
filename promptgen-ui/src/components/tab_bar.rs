//! Tab bar component for multi-tab prompt editing

use super::prompt_menu::{PromptMenu, PromptMenuAction};
use crate::state::AppState;
use crate::theme;
use egui_dnd::dnd;
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
            let can_save = state.get_active_tab().is_some_and(|tab| tab.dirty);

            if ui
                .add_enabled(can_save, egui::Button::new(format!("{} Save", ICON_SAVE)))
                .on_hover_text("Save prompt to library")
                .clicked()
            {
                action = TabBarAction::SaveTab;
            }

            ui.separator();

            // Scrollable area for tabs with bottom padding for scrollbar
            egui::ScrollArea::horizontal()
                .id_salt("tab_bar_scroll")
                .show(ui, |ui| {
                    egui::Frame::new()
                        .inner_margin(egui::Margin {
                            bottom: 8,
                            ..Default::default()
                        })
                        .show(ui, |ui| {
                            // Create a list of tab indices for drag-and-drop
                            let active_index = state.tabs.active_index;
                            let tab_count = state.tabs.tabs.len();
                            let mut tab_indices: Vec<usize> = (0..tab_count).collect();

                            // Use egui_dnd for drag-and-drop reordering
                            let response = dnd(ui, "tab_bar_dnd").show_vec(
                                &mut tab_indices,
                                |ui, original_idx, handle, _dragging| {
                                    let i = *original_idx;
                                    let is_active = active_index == Some(i);

                                    // Tab styling - use theme selection colors
                                    let theme = theme::current(ui.ctx());
                                    let (tab_fill, tab_stroke) = if is_active {
                                        (
                                            theme.active_selected_bg(),
                                            egui::Stroke::new(1.0, theme.active_selected_stroke()),
                                        )
                                    } else {
                                        (
                                            theme.selected_bg(),
                                            egui::Stroke::new(1.0, theme.selected_stroke()),
                                        )
                                    };

                                    egui::Frame::new()
                                        .fill(tab_fill)
                                        .stroke(tab_stroke)
                                        .corner_radius(4.0)
                                        .inner_margin(egui::Margin::symmetric(6, 2))
                                        .show(ui, |ui| {
                                            ui.horizontal(|ui| {
                                                let tab = &state.tabs.tabs[i];

                                                // Build tab label (name + dirty indicator)
                                                let label = if tab.dirty {
                                                    format!("{}*", tab.name)
                                                } else {
                                                    tab.name.clone()
                                                };

                                                // Drag handle wraps the clickable label
                                                handle.ui(ui, |ui| {
                                                    let response = ui.add(
                                                        egui::Label::new(&label)
                                                            .selectable(false)
                                                            .sense(egui::Sense::click()),
                                                    );

                                                    if response.clicked() && !is_active {
                                                        action = TabBarAction::SwitchTab(i);
                                                    }
                                                });

                                                // Menu button (always allow delete since we have blank state)
                                                let menu_action = PromptMenu::show(ui, true);
                                                match menu_action {
                                                    PromptMenuAction::Rename => {
                                                        action = TabBarAction::StartRename(i);
                                                    }
                                                    PromptMenuAction::Delete => {
                                                        action = TabBarAction::CloseTab(i);
                                                    }
                                                    PromptMenuAction::None => {}
                                                }

                                                // Close button (always show since we have blank state)
                                                let close_response = ui
                                                    .small_button(ICON_CLOSE)
                                                    .on_hover_text("Close tab");
                                                if close_response.clicked() {
                                                    action = TabBarAction::CloseTab(i);
                                                }
                                            });
                                        });
                                },
                            );

                            // Apply reordering if tabs were dragged
                            if response.is_drag_finished() {
                                // The tab_indices Vec has been reordered by egui_dnd
                                // We need to reorder the actual tabs to match
                                let new_tabs: Vec<_> = tab_indices
                                    .iter()
                                    .filter_map(|&idx| state.tabs.tabs.get(idx).cloned())
                                    .collect();

                                if new_tabs.len() == state.tabs.tabs.len() {
                                    // Find the new position of the active tab
                                    if let Some(active) = active_index {
                                        let new_active =
                                            tab_indices.iter().position(|&idx| idx == active);
                                        state.tabs.active_index = new_active;
                                    }
                                    state.tabs.tabs = new_tabs;
                                }
                            }
                        });
                });
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
                state.try_close_tab(*index);
            }
            TabBarAction::StartRename(index) => {
                state.start_tab_rename(*index);
            }
            TabBarAction::SaveTab => {
                state.save_active_tab_to_library();
                library_modified = true;
            }
            TabBarAction::None => {}
        }

        TabBarResult { library_modified }
    }
}
