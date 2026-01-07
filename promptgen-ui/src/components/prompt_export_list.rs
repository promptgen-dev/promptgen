//! Prompt export list component for selecting prompts to export.

use egui_flex::{Flex, FlexItem};
use egui_material_icons::icons::{
    ICON_CHECK_BOX, ICON_CHECK_BOX_OUTLINE_BLANK, ICON_CLOSE, ICON_DESELECT, ICON_FILE_UPLOAD,
    ICON_SEARCH, ICON_SELECT_ALL,
};

use crate::state::AppState;

/// Actions that can result from the export list
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PromptExportAction {
    None,
    Cancel,
    Export,
}

/// Prompt export list component.
pub struct PromptExportList;

impl PromptExportList {
    /// Render the complete export panel and return any action triggered.
    pub fn show(ui: &mut egui::Ui, state: &mut AppState) -> PromptExportAction {
        let mut action = PromptExportAction::None;

        // Header
        ui.heading("Export Prompts");
        ui.label(
            egui::RichText::new("Select prompts to export to clipboard")
                .small()
                .color(ui.visuals().weak_text_color()),
        );

        ui.add_space(8.0);

        // Search bar
        Self::show_search_bar(ui, &mut state.prompt_export_search_query);

        // Toolbar
        let toolbar_action = Self::show_toolbar(ui, state);
        if toolbar_action != PromptExportAction::None {
            action = toolbar_action;
        }

        ui.separator();

        // Prompt list with checkboxes
        Self::show_prompt_list(ui, state);

        action
    }

    /// Render the search bar.
    fn show_search_bar(ui: &mut egui::Ui, search_query: &mut String) {
        let has_query = !search_query.is_empty();

        Flex::horizontal().w_full().wrap(false).show(ui, |flex| {
            flex.add_ui(FlexItem::default(), |ui| {
                ui.label(ICON_SEARCH);
            });

            flex.add_ui(FlexItem::default().grow(1.0).shrink(), |ui| {
                ui.set_width(ui.available_width());
                ui.add(
                    egui::TextEdit::singleline(search_query)
                        .hint_text("Search prompts...")
                        .desired_width(ui.available_width()),
                );
            });

            if has_query {
                flex.add_ui(FlexItem::default(), |ui| {
                    if ui
                        .small_button(ICON_CLOSE)
                        .on_hover_text("Clear search")
                        .clicked()
                    {
                        search_query.clear();
                    }
                });
            }
        });
    }

    /// Render the toolbar with Cancel, Export, select all, deselect all.
    fn show_toolbar(ui: &mut egui::Ui, state: &mut AppState) -> PromptExportAction {
        let mut action = PromptExportAction::None;

        ui.add_space(4.0);
        Flex::horizontal().w_full().wrap(false).show(ui, |flex| {
            // Cancel button (left)
            flex.add_ui(FlexItem::default(), |ui| {
                if ui.button("Cancel").clicked() {
                    action = PromptExportAction::Cancel;
                }
            });

            // Export button (left, next to Cancel)
            flex.add_ui(FlexItem::default(), |ui| {
                let selected_count = state.prompt_export_selected.len();
                let can_export = selected_count > 0;
                let label = if selected_count > 0 {
                    format!("{} Export ({})", ICON_FILE_UPLOAD, selected_count)
                } else {
                    format!("{} Export", ICON_FILE_UPLOAD)
                };

                if ui
                    .add_enabled(can_export, egui::Button::new(label))
                    .on_hover_text("Export selected prompts to clipboard")
                    .clicked()
                {
                    action = PromptExportAction::Export;
                }
            });

            // Spacer
            flex.add_ui(FlexItem::default().grow(1.0), |_ui| {});

            // Select all button
            flex.add_ui(FlexItem::default(), |ui| {
                if ui
                    .small_button(ICON_SELECT_ALL)
                    .on_hover_text("Select all")
                    .clicked()
                {
                    state.select_all_prompt_export();
                }
            });

            // Deselect all button
            flex.add_ui(FlexItem::default(), |ui| {
                if ui
                    .small_button(ICON_DESELECT)
                    .on_hover_text("Deselect all")
                    .clicked()
                {
                    state.deselect_all_prompt_export();
                }
            });
        });

        action
    }

    /// Render the prompt list with checkboxes.
    fn show_prompt_list(ui: &mut egui::Ui, state: &mut AppState) {
        let search_query = state.prompt_export_search_query.to_lowercase();

        // Collect prompt data upfront to avoid borrow issues
        let prompts: Vec<String> = state
            .library
            .prompts
            .iter()
            .filter(|p| search_query.is_empty() || p.name.to_lowercase().contains(&search_query))
            .map(|p| p.name.clone())
            .collect();

        if prompts.is_empty() {
            if search_query.is_empty() {
                ui.label("No prompts in this library");
            } else {
                ui.label("No matching prompts");
            }
            return;
        }

        // Track which prompt to toggle (collected after UI pass)
        let mut toggle_prompt: Option<String> = None;

        egui::ScrollArea::vertical()
            .auto_shrink([false, false])
            .show(ui, |ui| {
                for prompt_name in &prompts {
                    let is_selected = state.prompt_export_selected.contains(prompt_name);

                    // Row with checkbox and prompt name
                    Flex::horizontal().w_full().wrap(false).show(ui, |flex| {
                        // Checkbox
                        let name_for_checkbox = prompt_name.clone();
                        flex.add_ui(FlexItem::default(), |ui| {
                            let icon = if is_selected {
                                ICON_CHECK_BOX
                            } else {
                                ICON_CHECK_BOX_OUTLINE_BLANK
                            };

                            if ui.small_button(icon).clicked() {
                                toggle_prompt = Some(name_for_checkbox);
                            }
                        });

                        // Prompt name (clickable to toggle)
                        let name_for_label = prompt_name.clone();
                        flex.add_ui(FlexItem::default().grow(1.0).shrink(), |ui| {
                            ui.with_layout(
                                egui::Layout::top_down_justified(egui::Align::LEFT),
                                |ui| {
                                    let button = egui::Button::new(prompt_name)
                                        .fill(egui::Color32::TRANSPARENT)
                                        .wrap();
                                    if ui.add(button).clicked() {
                                        toggle_prompt = Some(name_for_label);
                                    }
                                },
                            );
                        });
                    });
                }
            });

        // Apply toggle after UI pass
        if let Some(name) = toggle_prompt {
            state.toggle_prompt_export(&name);
        }
    }
}
