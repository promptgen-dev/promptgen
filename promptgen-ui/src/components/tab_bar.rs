//! Tab bar component for multi-tab prompt editing

use crate::state::AppState;
use egui_dnd::dnd;
use egui_material_icons::icons::{ICON_ADD, ICON_CHECK, ICON_CLOSE, ICON_SAVE};

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
                            let active_index = state.active_tab_index;
                            let tab_count = state.prompt_tabs.len();
                            let mut tab_indices: Vec<usize> = (0..tab_count).collect();

                            // Use egui_dnd for drag-and-drop reordering
                            let response = dnd(ui, "tab_bar_dnd").show_vec(
                                &mut tab_indices,
                                |ui, original_idx, handle, _dragging| {
                                    let i = *original_idx;
                                    let is_active = active_index == Some(i);
                                    let is_renaming = state.is_tab_renaming(i);

                                    ui.horizontal(|ui| {
                                        if is_renaming {
                                            // Validate current rename text
                                            let is_valid = state.is_tab_rename_valid();

                                            // Inline text edit for renaming (not draggable while renaming)
                                            let mut text_edit = egui::TextEdit::singleline(
                                                &mut state.tab_rename_text,
                                            )
                                            .desired_width(100.0)
                                            .id(egui::Id::new(("tab_rename", i)));

                                            // Add red border if invalid
                                            if !is_valid {
                                                text_edit = text_edit.text_color(
                                                    egui::Color32::from_rgb(243, 139, 168), // Catppuccin red
                                                );
                                            }

                                            // Wrap in a frame with border for error state
                                            let response = if !is_valid {
                                                egui::Frame::new()
                                                    .stroke(egui::Stroke::new(
                                                        1.0,
                                                        egui::Color32::from_rgb(243, 139, 168),
                                                    ))
                                                    .corner_radius(4.0)
                                                    .show(ui, |ui| ui.add(text_edit))
                                                    .inner
                                            } else {
                                                ui.add(text_edit)
                                            };

                                            // Request focus on first frame
                                            if response.gained_focus() || !response.has_focus() {
                                                response.request_focus();
                                            }

                                            // Checkmark button to save (replaces X button)
                                            let check_response = ui
                                                .add_enabled(
                                                    is_valid,
                                                    egui::Button::new(ICON_CHECK).small(),
                                                )
                                                .on_hover_text(if is_valid {
                                                    "Save name"
                                                } else {
                                                    "Name already exists or is empty"
                                                });
                                            if check_response.clicked() {
                                                state.commit_tab_rename();
                                            }

                                            // Handle Enter to commit
                                            if ui.input(|i| i.key_pressed(egui::Key::Enter))
                                                && is_valid
                                            {
                                                state.commit_tab_rename();
                                            }

                                            // Handle Escape to cancel
                                            if ui.input(|i| i.key_pressed(egui::Key::Escape)) {
                                                state.cancel_tab_rename();
                                            }

                                            // Handle clicking away - cancel instead of forcing commit
                                            if response.lost_focus()
                                                && !ui.input(|i| i.key_pressed(egui::Key::Escape))
                                                && !check_response.clicked()
                                            {
                                                // Cancel on click away
                                                state.cancel_tab_rename();
                                            }
                                        } else {
                                            let tab = &state.prompt_tabs[i];

                                            // Build tab label (name + dirty indicator)
                                            let label = if tab.dirty {
                                                format!("{}*", tab.name)
                                            } else {
                                                tab.name.clone()
                                            };

                                            // Drag handle is the selectable label
                                            handle.ui(ui, |ui| {
                                                let response =
                                                    ui.selectable_label(is_active, &label);

                                                if response.clicked() && !is_active {
                                                    action = TabBarAction::SwitchTab(i);
                                                }

                                                // Double-click to rename
                                                if response.double_clicked() {
                                                    action = TabBarAction::StartRename(i);
                                                }
                                            });

                                            // Close button (only show if more than one tab)
                                            if tab_count > 1 {
                                                let close_response = ui
                                                    .small_button(ICON_CLOSE)
                                                    .on_hover_text("Close tab");
                                                if close_response.clicked() {
                                                    action = TabBarAction::CloseTab(i);
                                                }
                                            }
                                        }

                                        // Add separator after each tab (except the last)
                                        // Note: This might not look perfect during drag, but works for now
                                    });
                                },
                            );

                            // Apply reordering if tabs were dragged
                            if response.is_drag_finished() {
                                // The tab_indices Vec has been reordered by egui_dnd
                                // We need to reorder the actual tabs to match
                                let new_tabs: Vec<_> = tab_indices
                                    .iter()
                                    .filter_map(|&idx| state.prompt_tabs.get(idx).cloned())
                                    .collect();

                                if new_tabs.len() == state.prompt_tabs.len() {
                                    // Find the new position of the active tab
                                    if let Some(active) = active_index {
                                        let new_active =
                                            tab_indices.iter().position(|&idx| idx == active);
                                        state.active_tab_index = new_active;
                                    }
                                    state.prompt_tabs = new_tabs;
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
