//! Sidebar panel component for library/template/variable navigation.

use std::path::PathBuf;

use promptgen_core::Cardinality;

use egui_material_icons::icons::{
    ICON_CHEVRON_RIGHT, ICON_CLOSE, ICON_EDIT, ICON_EXPAND_MORE, ICON_FOLDER, ICON_SEARCH,
};

use crate::state::{AppState, SidebarMode, SidebarViewMode};

/// Sidebar panel for navigating libraries, templates, and variables.
pub struct SidebarPanel;

impl SidebarPanel {
    /// Render the sidebar panel.
    ///
    /// Returns `true` if the workspace dialog should be opened.
    #[cfg(not(target_arch = "wasm32"))]
    pub fn show(ui: &mut egui::Ui, state: &mut AppState, workspace_path: &Option<PathBuf>) -> bool {
        let mut open_dialog = false;

        // Workspace header with folder picker
        if let Some(path) = workspace_path {
            ui.horizontal(|ui| {
                ui.label(ICON_FOLDER);
                let folder_name = path
                    .file_name()
                    .map(|n| n.to_string_lossy().to_string())
                    .unwrap_or_else(|| path.display().to_string());
                ui.label(&folder_name);
                if ui.small_button("...").clicked() {
                    open_dialog = true;
                }
            });
        } else if ui.button("Select Workspace...").clicked() {
            open_dialog = true;
        }

        ui.add_space(8.0);

        Self::render_content(ui, state, workspace_path);

        // Footer
        ui.with_layout(egui::Layout::bottom_up(egui::Align::LEFT), |ui| {
            egui::warn_if_debug_build(ui);
        });

        open_dialog
    }

    /// Render the sidebar panel (WASM version - no workspace dialog).
    #[cfg(target_arch = "wasm32")]
    pub fn show(ui: &mut egui::Ui, state: &mut AppState, workspace_path: &Option<PathBuf>) -> bool {
        ui.label("Web version");
        ui.add_space(8.0);

        Self::render_content(ui, state, workspace_path);

        // Footer
        ui.with_layout(egui::Layout::bottom_up(egui::Align::LEFT), |ui| {
            egui::warn_if_debug_build(ui);
        });

        false
    }

    /// Render the main sidebar content (shared between native and WASM).
    fn render_content(ui: &mut egui::Ui, state: &mut AppState, workspace_path: &Option<PathBuf>) {
        // Check if we're in slot picker mode
        if let SidebarMode::SlotPicker { slot_label } = &state.sidebar_mode {
            Self::render_slot_picker(ui, state, slot_label.clone());
            return;
        }

        // Library selector (ComboBox)
        if !state.libraries.is_empty() {
            let selected_name = state
                .selected_library()
                .map(|lib| lib.name.clone())
                .unwrap_or_else(|| "Select library...".to_string());

            ui.horizontal(|ui| {
                ui.label("Library:");
                egui::ComboBox::from_id_salt("library_selector")
                    .selected_text(&selected_name)
                    .width(ui.available_width() - 8.0)
                    .show_ui(ui, |ui| {
                        for lib in &state.libraries {
                            let is_selected = state.selected_library_id.as_ref() == Some(&lib.id);
                            if ui.selectable_label(is_selected, &lib.name).clicked() {
                                state.selected_library_id = Some(lib.id.clone());
                            }
                        }
                    });
            });

            ui.add_space(8.0);

            // View mode toggle (Templates / Variables)
            ui.horizontal(|ui| {
                if ui
                    .selectable_label(
                        state.sidebar_view_mode == SidebarViewMode::Templates,
                        "Templates",
                    )
                    .clicked()
                {
                    state.sidebar_view_mode = SidebarViewMode::Templates;
                }
                if ui
                    .selectable_label(
                        state.sidebar_view_mode == SidebarViewMode::Variables,
                        "Variables",
                    )
                    .clicked()
                {
                    state.sidebar_view_mode = SidebarViewMode::Variables;
                }
            });

            ui.add_space(4.0);

            // Search input
            ui.horizontal(|ui| {
                ui.label(ICON_SEARCH);
                ui.add(
                    egui::TextEdit::singleline(&mut state.search_query)
                        .hint_text("Search...")
                        .desired_width(ui.available_width() - 24.0),
                );
                if !state.search_query.is_empty() && ui.small_button(ICON_CLOSE).clicked() {
                    state.search_query.clear();
                }
            });

            ui.separator();

            // Content list based on view mode
            Self::render_sidebar_content(ui, state);
        } else if workspace_path.is_some() {
            ui.add_space(16.0);
            ui.label("No libraries found");
            ui.add_space(4.0);
            ui.label("Add .yaml library files to your workspace folder");
        } else {
            ui.add_space(16.0);
            ui.label("Select a workspace folder to get started");
        }
    }

    /// Render the sidebar content (templates or variables list).
    fn render_sidebar_content(ui: &mut egui::Ui, state: &mut AppState) {
        egui::ScrollArea::vertical()
            .auto_shrink([false, false])
            .show(ui, |ui| match state.sidebar_view_mode {
                SidebarViewMode::Templates => Self::render_template_list(ui, state),
                SidebarViewMode::Variables => Self::render_variable_list(ui, state),
            });
    }

    /// Render the template list.
    fn render_template_list(ui: &mut egui::Ui, state: &mut AppState) {
        let Some(library) = state.selected_library() else {
            ui.label("No library selected");
            return;
        };

        let search_query = state.search_query.to_lowercase();

        // Collect template info we need, releasing the borrow on state
        let templates: Vec<_> = library
            .templates
            .iter()
            .filter(|t| {
                search_query.is_empty()
                    || t.name.to_lowercase().contains(&search_query)
                    || t.description.to_lowercase().contains(&search_query)
            })
            .map(|t| {
                (
                    t.id.clone(),
                    t.name.clone(),
                    t.description.clone(),
                    promptgen_core::template_to_source(&t.ast),
                )
            })
            .collect();

        if templates.is_empty() {
            if search_query.is_empty() {
                ui.label("No templates in this library");
            } else {
                ui.label("No matching templates");
            }
            return;
        }

        let mut new_selected_id = state.selected_template_id.clone();
        let mut load_template_source: Option<String> = None;

        for (id, name, description, source) in &templates {
            let is_selected = new_selected_id.as_ref() == Some(id);
            let response = ui.selectable_label(is_selected, name);

            if response.clicked() {
                new_selected_id = Some(id.clone());
                load_template_source = Some(source.clone());
            }

            if !description.is_empty() {
                response.on_hover_text(description);
            }
        }

        state.selected_template_id = new_selected_id;

        // Apply template source after the loop (outside the borrow)
        if let Some(source) = load_template_source {
            state.editor_content = source;
            state.update_parse_result();
        }
    }

    /// Render the variable list with expandable options.
    ///
    /// Uses a unified rendering path that:
    /// - Filters variables based on search query
    /// - Highlights matched characters
    /// - Maintains edit buttons and collapse controls in all cases
    ///
    /// Supports advanced search syntax:
    /// - `blue` - search all options across all variables
    /// - `@Ey` - search variable names only, show all options for matches
    /// - `@Ey/bl` - search variables matching "Ey" that have options matching "bl"
    /// - `@/bl` - search all options (same as plain search)
    fn render_variable_list(ui: &mut egui::Ui, state: &mut AppState) {
        let Some(library) = state.selected_library() else {
            ui.label("No library selected");
            return;
        };

        if library.variables.is_empty() {
            ui.label("No variables in this library");
            ui.add_space(8.0);
            if ui.button("+ New Variable").clicked() {
                state.enter_new_variable_editor();
            }
            return;
        }

        let search_query = state.search_query.trim();
        let is_searching = !search_query.is_empty();

        // Get search results for highlighting if we have a search query
        let search_result = if is_searching {
            Some(state.workspace.search(search_query))
        } else {
            None
        };

        // Build the display data: for each variable, determine if it should be shown
        // and what highlighting to apply
        #[derive(Clone)]
        struct VariableDisplay {
            name: String,
            options: Vec<String>,
            /// Match indices for the variable name (for @-prefix searches)
            name_match_indices: Vec<usize>,
            /// For each option, the match indices (for option searches)
            option_matches: Vec<(String, Vec<usize>)>,
            /// Whether this is an option-based search result (affects display)
            is_option_search: bool,
        }

        let variables_display: Vec<VariableDisplay> = match &search_result {
            None => {
                // No search - show all variables
                library
                    .variables
                    .iter()
                    .map(|v| VariableDisplay {
                        name: v.name.clone(),
                        options: v.options.clone(),
                        name_match_indices: vec![],
                        option_matches: vec![],
                        is_option_search: false,
                    })
                    .collect()
            }
            Some(promptgen_core::SearchResult::Variables(var_results)) => {
                // Variable name search - show matched variables with their full options
                var_results
                    .iter()
                    .map(|vr| VariableDisplay {
                        name: vr.variable_name.clone(),
                        options: vr.options.clone(),
                        name_match_indices: vr.match_indices.clone(),
                        option_matches: vec![],
                        is_option_search: false,
                    })
                    .collect()
            }
            Some(promptgen_core::SearchResult::Options(opt_results)) => {
                // Option search - show variables with matching options only
                opt_results
                    .iter()
                    .map(|or| VariableDisplay {
                        name: or.variable_name.clone(),
                        options: or.matches.iter().map(|m| m.text.clone()).collect(),
                        name_match_indices: vec![],
                        option_matches: or
                            .matches
                            .iter()
                            .map(|m| (m.text.clone(), m.match_indices.clone()))
                            .collect(),
                        is_option_search: true,
                    })
                    .collect()
            }
        };

        if variables_display.is_empty() && is_searching {
            ui.label("No matching variables");
            ui.add_space(8.0);
            if ui.button("+ New Variable").clicked() {
                state.enter_new_variable_editor();
            }
            return;
        }

        let default_color = ui.visuals().text_color();

        // Track which variable to edit (to avoid borrow issues)
        let mut variable_to_edit: Option<String> = None;

        for var_display in &variables_display {
            let id = ui.make_persistent_id(&var_display.name);

            // Use CollapsingState for custom header layout
            let mut collapsing_state =
                egui::collapsing_header::CollapsingState::load_with_default_open(
                    ui.ctx(),
                    id,
                    is_searching, // Auto-expand when searching
                );

            // Header row: collapse toggle + label + edit button
            ui.horizontal(|ui| {
                // Toggle icon
                let icon = if collapsing_state.is_open() {
                    ICON_EXPAND_MORE
                } else {
                    ICON_CHEVRON_RIGHT
                };
                if ui.small_button(icon).clicked() {
                    collapsing_state.toggle(ui);
                }

                // Variable name label with optional highlighting
                let header_job = Self::build_variable_header_job(
                    &var_display.name,
                    var_display.options.len(),
                    &var_display.name_match_indices,
                    var_display.is_option_search,
                    default_color,
                );
                ui.label(header_job);

                // Edit button aligned right
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    if ui
                        .small_button(ICON_EDIT)
                        .on_hover_text("Edit variable")
                        .clicked()
                    {
                        variable_to_edit = Some(var_display.name.clone());
                    }
                });
            });

            // Body content (only shown when expanded)
            collapsing_state.show_body_unindented(ui, |ui| {
                // Use justified layout to make buttons fill full width (like slot picker)
                ui.with_layout(egui::Layout::top_down_justified(egui::Align::LEFT), |ui| {
                    if var_display.is_option_search && !var_display.option_matches.is_empty() {
                        // Show options with highlighting as clickable buttons
                        for (option_text, match_indices) in &var_display.option_matches {
                            let option_job =
                                Self::build_option_button_job(option_text, match_indices, default_color);
                            let response = ui.add(
                                egui::Button::new(option_job)
                                    .fill(egui::Color32::TRANSPARENT)
                                    .wrap(),
                            );
                            if response.clicked() {
                                ui.ctx().copy_text(option_text.clone());
                            }
                            response.on_hover_text("Click to copy");
                        }
                    } else {
                        // Show plain options as clickable buttons
                        for option in &var_display.options {
                            let response = ui.add(
                                egui::Button::new(format!("• {}", option))
                                    .fill(egui::Color32::TRANSPARENT)
                                    .wrap(),
                            );
                            if response.clicked() {
                                ui.ctx().copy_text(option.clone());
                            }
                            response.on_hover_text("Click to copy");
                        }
                    }
                });
            });
        }

        // Handle edit action after the loop
        if let Some(name) = variable_to_edit {
            state.enter_variable_editor(&name);
        }

        // Add new variable button at the bottom
        ui.add_space(8.0);
        if ui.button("+ New Variable").clicked() {
            state.enter_new_variable_editor();
        }
    }

    /// Build a LayoutJob for a variable header with optional highlighting.
    fn build_variable_header_job(
        name: &str,
        option_count: usize,
        match_indices: &[usize],
        is_option_search: bool,
        default_color: egui::Color32,
    ) -> egui::text::LayoutJob {
        use egui::FontId;
        use egui::text::{LayoutJob, TextFormat};

        let mut job = LayoutJob::default();

        // Add "@" prefix
        job.append(
            "@",
            0.0,
            TextFormat {
                font_id: FontId::default(),
                color: default_color,
                ..Default::default()
            },
        );

        // Add variable name with highlighting if applicable
        if !match_indices.is_empty() {
            let name_job = Self::highlighted_text(name, match_indices, default_color);
            for section in name_job.sections {
                job.append(
                    &name_job.text[section.byte_range.clone()],
                    0.0,
                    section.format,
                );
            }
        } else {
            job.append(
                name,
                0.0,
                TextFormat {
                    font_id: FontId::default(),
                    color: default_color,
                    ..Default::default()
                },
            );
        }

        // Add count suffix - for option search, show match count instead of total
        let suffix = if is_option_search {
            let match_word = if option_count == 1 { "match" } else { "matches" };
            format!(" ({} {})", option_count, match_word)
        } else {
            format!(" ({})", option_count)
        };

        job.append(
            &suffix,
            0.0,
            TextFormat {
                font_id: FontId::default(),
                color: default_color,
                ..Default::default()
            },
        );

        job
    }

    /// Build a LayoutJob for an option button with highlighting.
    fn build_option_button_job(
        option_text: &str,
        match_indices: &[usize],
        default_color: egui::Color32,
    ) -> egui::text::LayoutJob {
        use egui::FontId;
        use egui::text::{LayoutJob, TextFormat};

        let mut job = LayoutJob::default();

        // Add bullet prefix
        job.append(
            "• ",
            0.0,
            TextFormat {
                font_id: FontId::default(),
                color: default_color,
                ..Default::default()
            },
        );

        // Add highlighted option text
        let text_job = Self::highlighted_text(option_text, match_indices, default_color);
        for section in text_job.sections {
            job.append(
                &text_job.text[section.byte_range.clone()],
                0.0,
                section.format,
            );
        }

        job
    }

    /// Create a LayoutJob that highlights matched characters in green.
    fn highlighted_text(
        text: &str,
        match_indices: &[usize],
        default_color: egui::Color32,
    ) -> egui::text::LayoutJob {
        use egui::FontId;
        use egui::text::{LayoutJob, TextFormat};

        let highlight_color = egui::Color32::from_rgb(166, 227, 161); // Catppuccin green
        let mut job = LayoutJob::default();

        let chars: Vec<char> = text.chars().collect();
        let match_set: std::collections::HashSet<usize> = match_indices.iter().copied().collect();

        let mut i = 0;
        while i < chars.len() {
            // Find a run of same-colored characters
            let is_highlighted = match_set.contains(&i);
            let start = i;

            while i < chars.len() && match_set.contains(&i) == is_highlighted {
                i += 1;
            }

            // Collect the substring
            let substring: String = chars[start..i].iter().collect();
            let color = if is_highlighted {
                highlight_color
            } else {
                default_color
            };

            job.append(
                &substring,
                0.0,
                TextFormat {
                    font_id: FontId::default(),
                    color,
                    ..Default::default()
                },
            );
        }

        job
    }

    /// Render the slot picker overlay for selecting options for a pick slot.
    fn render_slot_picker(ui: &mut egui::Ui, state: &mut AppState, slot_label: String) {
        // Header with slot name and close button
        ui.horizontal(|ui| {
            ui.heading(&slot_label);
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                if ui.button(ICON_CLOSE).on_hover_text("Close picker").clicked() {
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
                        .slot_values
                        .get(&slot_label)
                        .map(|v| v.len())
                        .unwrap_or(0);
                    format!("Select up to {} ({}/{})", n, current, n)
                }
            };
            ui.label(
                egui::RichText::new(cardinality_text)
                    .small()
                    .color(egui::Color32::from_rgb(108, 112, 134)),
            );
        }

        ui.separator();

        // Get available options
        let options = state.get_pick_options(&slot_label);
        let selected_values = state
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

        // Show options list
        egui::ScrollArea::vertical()
            .auto_shrink([false, false])
            .show(ui, |ui| {
                if options.is_empty() {
                    ui.label(
                        egui::RichText::new("No options available")
                            .italics()
                            .color(egui::Color32::from_rgb(108, 112, 134)),
                    );
                    return;
                }

                // Use justified layout to make buttons fill full width
                ui.with_layout(egui::Layout::top_down_justified(egui::Align::LEFT), |ui| {
                    for option in &options {
                        let is_selected = selected_values.contains(option);
                        let display_text = format!("• {}", option);

                        // Full-width selectable button - transparent when not selected, highlight when selected
                        let fill = if is_selected {
                            ui.visuals().selection.bg_fill
                        } else {
                            egui::Color32::TRANSPARENT
                        };
                        let response = ui.add(
                            egui::Button::new(display_text).fill(fill).wrap(),
                        );

                        // Show full text on hover for truncated options
                        response.clone().on_hover_text(option);

                        if response.clicked() {
                            if is_selected {
                                // Remove selection
                                state.remove_slot_value(&slot_label, option);
                                state.request_render();
                            } else if can_add {
                                // Add/replace selection (add_slot_value handles single-select replacement)
                                state.add_slot_value(&slot_label, option.clone());
                                state.request_render();
                            }
                        }
                    }
                });
            });
    }
}
