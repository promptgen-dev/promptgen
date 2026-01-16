//! Tab bar component for multi-tab prompt editing

use super::prompt_menu::{PromptMenu, PromptMenuAction};
use crate::state::AppState;
use crate::styles::{Buttons, Components, Icons, Layout, Patterns};
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
        let theme = theme::current(ui.ctx());

        ui.horizontal(|ui| {
            // [+ New] button
            let new_btn = Buttons::secondary(Icons::with_label(ICON_ADD, "New"), &theme);
            if ui
                .add(new_btn)
                .on_hover_text("Create new prompt tab")
                .clicked()
            {
                action = TabBarAction::NewTab;
            }

            // [Save] button - enabled when active tab is dirty
            let can_save = state.get_active_tab().is_some_and(|tab| tab.dirty);
            let save_btn = Buttons::primary(Icons::with_label(ICON_SAVE, "Save"), &theme);

            if ui
                .add_enabled(can_save, save_btn)
                .on_hover_text("Save prompt to library")
                .clicked()
            {
                action = TabBarAction::SaveTab;
            }

            Layout::gap(ui);

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
                                    let theme = theme::current(ui.ctx());

                                    // Use list_item_frame for consistent tab styling
                                    // Wrap handle around the entire frame so dragging works anywhere
                                    handle.ui(ui, |ui| {
                                        Components::list_item_frame(&theme, is_active, !is_active)
                                            .show(ui, |ui| {
                                                ui.horizontal_centered(|ui| {
                                                    let tab = &state.tabs.tabs[i];

                                                    // Clickable label
                                                    let response = ui.add(
                                                        egui::Label::new(&tab.name)
                                                            .selectable(false)
                                                            .sense(egui::Sense::click()),
                                                    );

                                                    if response.clicked() && !is_active {
                                                        action = TabBarAction::SwitchTab(i);
                                                    }

                                                    // Dirty indicator (separate from name)
                                                    if tab.dirty {
                                                        Patterns::dirty_indicator(ui, &theme);
                                                    }

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
                                                    if ui
                                                        .add(Buttons::icon_small(ICON_CLOSE))
                                                        .on_hover_text("Close tab")
                                                        .clicked()
                                                    {
                                                        action = TabBarAction::CloseTab(i);
                                                    }
                                                });
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
