//! Variable export list component for selecting variables to export.

use egui_flex::{Flex, FlexItem};
use egui_material_icons::icons::{
    ICON_ARROW_DOWNWARD, ICON_ARROW_UPWARD, ICON_CHECK_BOX, ICON_CHECK_BOX_OUTLINE_BLANK,
    ICON_CLOSE, ICON_DESELECT, ICON_FILE_UPLOAD, ICON_SEARCH, ICON_SELECT_ALL, ICON_SORT_BY_ALPHA,
};

use crate::state::{AppState, VariableSortOrder};
use crate::theme;

/// Actions that can result from the export list
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum VariableExportAction {
    None,
    Cancel,
    Export,
}

/// Variable export list component.
pub struct VariableExportList;

impl VariableExportList {
    /// Render the complete export panel and return any action triggered.
    pub fn show(ui: &mut egui::Ui, state: &mut AppState) -> VariableExportAction {
        let mut action = VariableExportAction::None;

        // Header
        ui.heading("Export Variables");
        ui.label(
            egui::RichText::new("Select variables to export to clipboard")
                .small()
                .color(ui.visuals().weak_text_color()),
        );

        ui.add_space(8.0);

        // Search bar
        Self::show_search_bar(ui, &mut state.variable_export.search_query);

        // Toolbar
        let toolbar_action = Self::show_toolbar(ui, state);
        if toolbar_action != VariableExportAction::None {
            action = toolbar_action;
        }

        ui.separator();

        // Variable list with checkboxes
        Self::show_variable_list(ui, state);

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
                        .hint_text("Search variables...")
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

    /// Render the toolbar with Cancel, Export, sort, select all, deselect all.
    fn show_toolbar(ui: &mut egui::Ui, state: &mut AppState) -> VariableExportAction {
        let mut action = VariableExportAction::None;

        ui.add_space(4.0);
        Flex::horizontal().w_full().wrap(false).show(ui, |flex| {
            // Cancel button (left)
            flex.add_ui(FlexItem::default(), |ui| {
                if ui.button("Cancel").clicked() {
                    action = VariableExportAction::Cancel;
                }
            });

            // Export button (left, next to Cancel)
            flex.add_ui(FlexItem::default(), |ui| {
                let selected_count = state.variable_export.selected.len();
                let can_export = selected_count > 0;
                let label = if selected_count > 0 {
                    format!("{} Export ({})", ICON_FILE_UPLOAD, selected_count)
                } else {
                    format!("{} Export", ICON_FILE_UPLOAD)
                };

                if ui
                    .add_enabled(can_export, egui::Button::new(label))
                    .on_hover_text("Export selected variables to clipboard")
                    .clicked()
                {
                    action = VariableExportAction::Export;
                }
            });

            // Spacer
            flex.add_ui(FlexItem::default().grow(1.0), |_ui| {});

            // Variable sort button
            flex.add_ui(FlexItem::default(), |ui| {
                let current_theme = theme::current(ui.ctx());
                let is_active = state.variable_export.sort_order != VariableSortOrder::None;
                let (icon, tooltip) = match state.variable_export.sort_order {
                    VariableSortOrder::None => {
                        (ICON_SORT_BY_ALPHA.to_string(), "Sort variables A-Z")
                    }
                    VariableSortOrder::Ascending => (
                        format!("{}{}", ICON_SORT_BY_ALPHA, ICON_ARROW_UPWARD),
                        "Sort variables Z-A",
                    ),
                    VariableSortOrder::Descending => (
                        format!("{}{}", ICON_SORT_BY_ALPHA, ICON_ARROW_DOWNWARD),
                        "Clear sort",
                    ),
                };
                let fill = if is_active {
                    current_theme.active_selected_bg()
                } else {
                    egui::Color32::TRANSPARENT
                };
                if ui
                    .add(egui::Button::new(&icon).small().fill(fill))
                    .on_hover_text(tooltip)
                    .clicked()
                {
                    state.variable_export.sort_order = match state.variable_export.sort_order {
                        VariableSortOrder::None => VariableSortOrder::Ascending,
                        VariableSortOrder::Ascending => VariableSortOrder::Descending,
                        VariableSortOrder::Descending => VariableSortOrder::None,
                    };
                }
            });

            // Select all button
            flex.add_ui(FlexItem::default(), |ui| {
                if ui
                    .small_button(ICON_SELECT_ALL)
                    .on_hover_text("Select all")
                    .clicked()
                {
                    state.variable_export.select_all(&state.library);
                }
            });

            // Deselect all button
            flex.add_ui(FlexItem::default(), |ui| {
                if ui
                    .small_button(ICON_DESELECT)
                    .on_hover_text("Deselect all")
                    .clicked()
                {
                    state.variable_export.deselect_all();
                }
            });
        });

        action
    }

    /// Render the variable list with checkboxes.
    fn show_variable_list(ui: &mut egui::Ui, state: &mut AppState) {
        let search_query = state.variable_export.search_query.to_lowercase();
        let sort_order = state.variable_export.sort_order;

        // Collect variable data upfront to avoid borrow issues
        let mut variables: Vec<(String, usize)> = state
            .library
            .variables
            .iter()
            .filter(|v| search_query.is_empty() || v.name.to_lowercase().contains(&search_query))
            .map(|v| (v.name.clone(), v.options.len()))
            .collect();

        // Apply sorting
        match sort_order {
            VariableSortOrder::None => {}
            VariableSortOrder::Ascending => {
                variables.sort_by(|a, b| a.0.to_lowercase().cmp(&b.0.to_lowercase()));
            }
            VariableSortOrder::Descending => {
                variables.sort_by(|a, b| b.0.to_lowercase().cmp(&a.0.to_lowercase()));
            }
        }

        if variables.is_empty() {
            if search_query.is_empty() {
                ui.label("No variables in this library");
            } else {
                ui.label("No matching variables");
            }
            return;
        }

        // Track which variable to toggle (collected after UI pass)
        let mut toggle_variable: Option<String> = None;

        egui::ScrollArea::vertical()
            .auto_shrink([false, false])
            .show(ui, |ui| {
                for (var_name, option_count) in &variables {
                    let is_selected = state.variable_export.selected.contains(var_name);

                    // Row with checkbox and variable name
                    Flex::horizontal().w_full().wrap(false).show(ui, |flex| {
                        // Checkbox
                        let name_for_checkbox = var_name.clone();
                        flex.add_ui(FlexItem::default(), |ui| {
                            let icon = if is_selected {
                                ICON_CHECK_BOX
                            } else {
                                ICON_CHECK_BOX_OUTLINE_BLANK
                            };

                            if ui.small_button(icon).clicked() {
                                toggle_variable = Some(name_for_checkbox);
                            }
                        });

                        // Variable name with option count (clickable to toggle)
                        let name_for_label = var_name.clone();
                        flex.add_ui(FlexItem::default().grow(1.0).shrink(), |ui| {
                            ui.with_layout(
                                egui::Layout::top_down_justified(egui::Align::LEFT),
                                |ui| {
                                    let label = format!("{} ({})", var_name, option_count);
                                    let button = egui::Button::new(&label)
                                        .fill(egui::Color32::TRANSPARENT)
                                        .wrap();
                                    if ui.add(button).clicked() {
                                        toggle_variable = Some(name_for_label);
                                    }
                                },
                            );
                        });
                    });
                }
            });

        // Apply toggle after UI pass
        if let Some(name) = toggle_variable {
            state.variable_export.toggle(&name);
        }
    }
}
