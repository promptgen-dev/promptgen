//! Sidebar panel component for library/prompt/variable navigation.

use std::path::PathBuf;

use egui_flex::{Flex, FlexItem};
use promptgen_core::Cardinality;

use egui_material_icons::icons::{
    ICON_ARROW_DOWNWARD, ICON_ARROW_UPWARD, ICON_CLOSE, ICON_DESCRIPTION, ICON_SORT_BY_ALPHA,
};

use super::prompt_export_list::{PromptExportAction, PromptExportList};
use super::prompt_menu::{PromptMenu, PromptMenuAction};
use super::prompts_menu::{PromptsMenu, PromptsMenuAction};
use super::variable_export_list::{VariableExportAction, VariableExportList};
use super::variable_list::{VariableList, VariableListConfig};
use crate::state::{AppState, OptionGroup, SidebarMode, SidebarViewMode, VariableSortOrder};
use crate::styles::{Buttons, Components, Icons, Layout, Patterns, Typography};
use crate::theme;
use crate::utils::truncate;

/// Sidebar panel for navigating libraries, prompts, and variables.
pub struct SidebarPanel;

impl SidebarPanel {
    /// Render the sidebar panel.
    #[cfg(not(target_arch = "wasm32"))]
    pub fn show(
        ui: &mut egui::Ui,
        state: &mut AppState,
        library_file_path: &Option<PathBuf>,
    ) -> bool {
        Self::render_content(ui, state, library_file_path);

        // Footer
        ui.with_layout(egui::Layout::bottom_up(egui::Align::LEFT), |ui| {
            egui::warn_if_debug_build(ui);
        });

        false // Open dialog is now only via File menu
    }

    /// Render the sidebar panel (WASM version).
    #[cfg(target_arch = "wasm32")]
    pub fn show(
        ui: &mut egui::Ui,
        state: &mut AppState,
        library_file_path: &Option<PathBuf>,
    ) -> bool {
        Self::render_content(ui, state, library_file_path);

        // Footer
        ui.with_layout(egui::Layout::bottom_up(egui::Align::LEFT), |ui| {
            egui::warn_if_debug_build(ui);
        });

        false
    }

    /// Render the main sidebar content (shared between native and WASM).
    fn render_content(
        ui: &mut egui::Ui,
        state: &mut AppState,
        library_file_path: &Option<PathBuf>,
    ) {
        // Check if we're in slot picker mode
        if let SidebarMode::SlotPicker { slot_label } = &state.sidebar.mode {
            Self::render_slot_picker(ui, state, slot_label.clone());
            return;
        }

        // Check if we're in variable export mode
        if state.variable_export.active {
            Self::render_variable_export_mode(ui, state);
            return;
        }

        // Check if we're in prompt export mode
        if state.prompt_export.active {
            Self::render_prompt_export_mode(ui, state);
            return;
        }

        // Check if we have a library loaded
        if let Some(lib_path) = &state.library_path {
            let theme = theme::current(ui.ctx());

            // Library header: icon + name as main heading
            ui.horizontal(|ui| {
                ui.label(Icons::button_icon(ICON_DESCRIPTION));
                let name = if state.library.name.is_empty() {
                    "Untitled Library"
                } else {
                    &state.library.name
                };
                ui.label(Typography::heading(name));
            });

            // File name in smaller, lighter text below
            let file_name = lib_path
                .file_name()
                .map(|n| n.to_string_lossy().to_string())
                .unwrap_or_else(|| lib_path.display().to_string());
            ui.label(Typography::caption(file_name, &theme));

            Layout::gap(ui);

            // View mode toggle (Prompts / Variables)
            ui.horizontal(|ui| {
                let prompts_btn = Buttons::tab(
                    "Prompts",
                    state.sidebar.view_mode == SidebarViewMode::Prompts,
                    &theme,
                );
                if ui.add(prompts_btn).clicked() {
                    state.sidebar.view_mode = SidebarViewMode::Prompts;
                }

                let variables_btn = Buttons::tab(
                    "Variables",
                    state.sidebar.view_mode == SidebarViewMode::Variables,
                    &theme,
                );
                if ui.add(variables_btn).clicked() {
                    state.sidebar.view_mode = SidebarViewMode::Variables;
                }
            });

            Layout::small_gap(ui);

            // Search input with clear button
            VariableList::show_search_bar(ui, &mut state.sidebar.search_query);

            // Toolbar (only in Variables view) - includes new variable button and import/export menu
            if state.sidebar.view_mode == SidebarViewMode::Variables {
                let toolbar_result = VariableList::show_toolbar(
                    ui,
                    &mut state.sidebar.variable_sort_order,
                    &mut state.sidebar.option_sort_order,
                    &mut state.sidebar.expand_all_variables,
                    true, // show new variable button
                    true, // show import/export menu
                );
                if toolbar_result.new_variable_clicked {
                    state.enter_new_variable_editor();
                }
                if toolbar_result.export_clicked {
                    state.variable_export.enter();
                }
                if toolbar_result.import_clicked {
                    state.variable_import.open();
                }
            }

            // Toolbar (only in Prompts view) - includes import/export menu
            if state.sidebar.view_mode == SidebarViewMode::Prompts {
                Self::show_prompts_toolbar(ui, state);
            }

            ui.separator();

            // Content list based on view mode
            Self::render_sidebar_content(ui, state);
        } else if library_file_path.is_some() {
            Patterns::empty_state(
                ui,
                ICON_DESCRIPTION,
                "Failed to load library",
                Some("Check the file format and try again"),
            );
        } else {
            Patterns::empty_state(
                ui,
                ICON_DESCRIPTION,
                "No library loaded",
                Some("Use File → Open Library to get started"),
            );
        }
    }

    /// Render the sidebar content (prompts or variables list).
    fn render_sidebar_content(ui: &mut egui::Ui, state: &mut AppState) {
        egui::ScrollArea::vertical()
            .auto_shrink([false, false])
            .show(ui, |ui| match state.sidebar.view_mode {
                SidebarViewMode::Prompts => Self::render_prompt_list(ui, state),
                SidebarViewMode::Variables => Self::render_variable_list(ui, state),
            });
    }

    /// Render the prompt list.
    fn render_prompt_list(ui: &mut egui::Ui, state: &mut AppState) {
        let search_query = state.sidebar.search_query.to_lowercase();
        let sort_order = state.sidebar.prompt_sort_order;

        // Collect prompt info we need
        let mut prompts: Vec<_> = state
            .library
            .prompts
            .iter()
            .filter(|p| search_query.is_empty() || p.name.to_lowercase().contains(&search_query))
            .map(|p| p.name.clone())
            .collect();

        // Apply sorting
        match sort_order {
            VariableSortOrder::None => {}
            VariableSortOrder::Ascending => {
                prompts.sort_by_key(|a| a.to_lowercase());
            }
            VariableSortOrder::Descending => {
                prompts.sort_by_key(|b| std::cmp::Reverse(b.to_lowercase()));
            }
        }

        if prompts.is_empty() {
            if search_query.is_empty() {
                ui.label("No prompts in this library");
            } else {
                ui.label("No matching prompts");
            }
            return;
        }

        let mut prompt_to_open: Option<String> = None;
        let mut prompt_to_delete: Option<String> = None;
        let mut prompt_to_rename: Option<String> = None;

        // Get active tab info for highlighting
        let active_tab_name = state
            .tabs
            .active_index
            .and_then(|idx| state.tabs.tabs.get(idx))
            .and_then(|tab| {
                if let crate::state::PromptSource::FromLibrary { original_name } = &tab.source {
                    Some(original_name.clone())
                } else {
                    None
                }
            });

        for name in &prompts {
            // Check if this prompt is open in a tab (for visual indication)
            let tab_index = state.tabs.find_by_library_prompt(name);
            let is_open_in_tab = tab_index.is_some();
            let is_active_tab = active_tab_name.as_ref() == Some(name);

            let theme = theme::current(ui.ctx());

            // Use list_item_frame for consistent styling
            Components::list_item_frame(&theme, is_active_tab, is_open_in_tab)
                .show(ui, |ui| {
                    // Use flex layout for prompt row: label (wraps) + menu button (fixed)
                    Flex::horizontal().w_full().wrap(false).show(ui, |flex| {
                        // Clickable label area (grows and wraps like variable list)
                        let prompt_name = name.clone();
                        flex.add_ui(FlexItem::default().grow(1.0).shrink(), |ui| {
                            ui.with_layout(
                                egui::Layout::top_down_justified(egui::Align::LEFT),
                                |ui| {
                                    // Use Button with wrap like variable list does
                                    let display_text = truncate(name);
                                    let button = egui::Button::new(display_text.as_ref())
                                        .fill(egui::Color32::TRANSPARENT)
                                        .wrap();
                                    let response = ui.add(button);

                                    // Handle click
                                    if response.clicked() {
                                        prompt_to_open = Some(prompt_name.clone());
                                    }

                                    // Show full name on hover
                                    response.on_hover_text(&prompt_name);
                                },
                            );
                        });

                        // Menu button (fixed size)
                        let prompt_name_for_menu = name.clone();
                        flex.add_ui(FlexItem::default(), |ui| {
                            let menu_action = PromptMenu::show(ui, true);
                            match menu_action {
                                PromptMenuAction::Rename => {
                                    prompt_to_rename = Some(prompt_name_for_menu.clone());
                                }
                                PromptMenuAction::Delete => {
                                    prompt_to_delete = Some(prompt_name_for_menu.clone());
                                }
                                PromptMenuAction::None => {}
                            }
                        });
                    });
                });
        }

        // Open the clicked prompt in a tab
        if let Some(name) = prompt_to_open {
            state.open_library_prompt(&name);
            // Also ensure we're in Prompt editing mode
            state.editor.mode = crate::state::EditorMode::Prompt;
        }

        // Request delete confirmation
        if let Some(name) = prompt_to_delete {
            state.dialogs.request_delete_prompt(&name);
        }

        // Start rename (opens the prompt first, then starts rename)
        if let Some(name) = prompt_to_rename {
            // Open the prompt in a tab if not already open
            state.open_library_prompt(&name);
            state.editor.mode = crate::state::EditorMode::Prompt;
            // Find the tab index for this prompt and start rename
            if let Some(idx) = state.tabs.find_by_library_prompt(&name) {
                state.start_tab_rename(idx);
            }
        }
    }

    /// Render the variable list with expandable options.
    /// Uses the shared VariableList component.
    fn render_variable_list(ui: &mut egui::Ui, state: &mut AppState) {
        // Convert library variables to OptionGroups
        let groups: Vec<OptionGroup> = state
            .library
            .variables
            .iter()
            .map(|v| OptionGroup {
                name: v.name.clone(),
                options: v.options.clone(),
                is_editable: true,
            })
            .collect();

        // Take expand_all_variables state (consumed once per render)
        let expand_all = state.sidebar.expand_all_variables.take();

        let config = VariableListConfig {
            id_prefix: "variables",
            selectable: false,
            selected_values: &[],
            can_add_selection: true,
            show_edit_buttons: true,
        };

        let result = VariableList::show_groups(
            ui,
            &groups,
            &state.sidebar.search_query,
            state.sidebar.variable_sort_order,
            state.sidebar.option_sort_order,
            expand_all,
            &config,
            Some(&state.library),
        );

        // Handle edit button clicks
        if let Some(name) = result.edit_clicked {
            state.enter_variable_editor(&name);
        }
    }

    /// Render the slot picker overlay for selecting options for a pick slot.
    fn render_slot_picker(ui: &mut egui::Ui, state: &mut AppState, slot_label: String) {
        let theme = theme::current(ui.ctx());

        // Header with slot name and close button
        ui.horizontal(|ui| {
            ui.label(Typography::heading(&slot_label));
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                if ui
                    .add(Buttons::icon(ICON_CLOSE))
                    .on_hover_text("Close picker")
                    .clicked()
                {
                    state.unfocus_slot();
                }
            });
        });

        // Show cardinality info
        if let Some(cardinality) = state.get_slot_cardinality(&slot_label) {
            let cardinality_text = match &cardinality {
                Cardinality::One => "Select one".to_string(),
                Cardinality::Many { max: None } => "Select any".to_string(),
                Cardinality::Many { max: Some(n) } => {
                    let current = state
                        .preview
                        .slot_values
                        .get(&slot_label)
                        .map(|v| v.len())
                        .unwrap_or(0);
                    format!("Select up to {} ({}/{})", n, current, n)
                }
            };
            ui.label(Typography::caption(cardinality_text, &theme));
        }

        Layout::small_gap(ui);

        // Search bar
        VariableList::show_search_bar(ui, &mut state.sidebar.slot_picker.search_query);

        // Toolbar (sort, expand/collapse) - no new variable or export button in slot picker
        VariableList::show_toolbar(
            ui,
            &mut state.sidebar.slot_picker.sort_order,
            &mut state.sidebar.slot_picker.option_sort_order,
            &mut state.sidebar.expand_all_variables,
            false, // don't show new variable button
            false, // don't show export button
        );

        ui.separator();

        // Get option groups and selected values
        let groups = state.get_pick_option_groups(&slot_label);
        let selected_values: Vec<String> = state
            .preview
            .slot_values
            .get(&slot_label)
            .cloned()
            .unwrap_or_default();

        // Check if we can add more (single-select always allows as it replaces)
        let can_add = match state.get_slot_cardinality(&slot_label) {
            Some(Cardinality::One) => true, // Single-select always allows (replaces)
            Some(Cardinality::Many { max: Some(n) }) => selected_values.len() < n as usize,
            _ => true,
        };

        // Take expand_all_variables state (consumed once per render)
        let expand_all = state.sidebar.expand_all_variables.take();

        let config = VariableListConfig {
            id_prefix: "slot_picker",
            selectable: true,
            selected_values: &selected_values,
            can_add_selection: can_add,
            show_edit_buttons: true,
        };

        // Show the option groups in a scroll area
        egui::ScrollArea::vertical()
            .auto_shrink([false, false])
            .show(ui, |ui| {
                let result = VariableList::show_groups(
                    ui,
                    &groups,
                    &state.sidebar.slot_picker.search_query,
                    state.sidebar.slot_picker.sort_order,
                    state.sidebar.slot_picker.option_sort_order,
                    expand_all,
                    &config,
                    Some(&state.library),
                );

                // Handle option clicks - toggle selection
                if let Some(option) = result.option_clicked {
                    let is_selected = selected_values.contains(&option);
                    if is_selected {
                        state.remove_slot_value(&slot_label, &option);
                    } else if can_add {
                        state.add_slot_value(&slot_label, option);
                    }
                    state.request_render();
                }

                // Handle group name clicks - insert dynamic option @"GroupName"
                if let Some(group_name) = result.group_name_clicked {
                    let dynamic_option = format!("@\"{}\"", group_name);
                    // Check if already selected
                    let is_selected = selected_values.contains(&dynamic_option);
                    if is_selected {
                        state.remove_slot_value(&slot_label, &dynamic_option);
                    } else if can_add {
                        state.add_slot_value(&slot_label, dynamic_option);
                    }
                    state.request_render();
                }

                // Handle edit button clicks - navigate to variable editor
                if let Some(name) = result.edit_clicked {
                    state.enter_variable_editor(&name);
                }
            });
    }

    /// Render the prompts toolbar with Import/Export menu and sort button.
    fn show_prompts_toolbar(ui: &mut egui::Ui, state: &mut AppState) {
        Layout::small_gap(ui);
        Flex::horizontal().w_full().wrap(false).show(ui, |flex| {
            // Import/Export menu (left-aligned)
            flex.add_ui(FlexItem::default(), |ui| {
                let menu_action = PromptsMenu::show(ui);
                match menu_action {
                    PromptsMenuAction::Export => {
                        state.prompt_export.enter();
                    }
                    PromptsMenuAction::Import => {
                        state.prompt_import.open();
                    }
                    PromptsMenuAction::None => {}
                }
            });

            // Spacer to push remaining buttons to the right
            flex.add_ui(FlexItem::default().grow(1.0), |_ui| {});

            // Sort button - cycles through None -> Ascending -> Descending -> None
            flex.add_ui(FlexItem::default(), |ui| {
                let current_theme = theme::current(ui.ctx());
                let is_active = state.sidebar.prompt_sort_order != VariableSortOrder::None;
                let (icon, tooltip) = match state.sidebar.prompt_sort_order {
                    VariableSortOrder::None => (ICON_SORT_BY_ALPHA.to_string(), "Sort prompts A-Z"),
                    VariableSortOrder::Ascending => (
                        format!("{}{}", ICON_SORT_BY_ALPHA, ICON_ARROW_UPWARD),
                        "Sort prompts Z-A",
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
                    state.sidebar.prompt_sort_order = match state.sidebar.prompt_sort_order {
                        VariableSortOrder::None => VariableSortOrder::Ascending,
                        VariableSortOrder::Ascending => VariableSortOrder::Descending,
                        VariableSortOrder::Descending => VariableSortOrder::None,
                    };
                }
            });
        });
    }

    /// Render the variable export mode UI.
    fn render_variable_export_mode(ui: &mut egui::Ui, state: &mut AppState) {
        let action = VariableExportList::show(ui, state);

        match action {
            VariableExportAction::Cancel => {
                state.variable_export.exit();
            }
            VariableExportAction::Export => {
                // Export to clipboard
                if let Some(yaml) = state.variable_export.export_to_yaml(&state.library) {
                    ui.ctx().copy_text(yaml);
                }
                state.variable_export.exit();
            }
            VariableExportAction::None => {}
        }
    }

    /// Render the prompt export mode UI.
    fn render_prompt_export_mode(ui: &mut egui::Ui, state: &mut AppState) {
        let action = PromptExportList::show(ui, state);

        match action {
            PromptExportAction::Cancel => {
                state.prompt_export.exit();
            }
            PromptExportAction::Export => {
                // Export to clipboard
                if let Some(yaml) = state.prompt_export.export_to_yaml(&state.library) {
                    ui.ctx().copy_text(yaml);
                }
                state.prompt_export.exit();
            }
            PromptExportAction::None => {}
        }
    }
}
