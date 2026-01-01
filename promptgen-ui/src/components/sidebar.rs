//! Sidebar panel component for library/prompt/variable navigation.

use std::path::PathBuf;

use egui_flex::{Flex, FlexItem};
use promptgen_core::Cardinality;

use egui_material_icons::icons::{
    ICON_CHEVRON_RIGHT, ICON_CLOSE, ICON_COLLAPSE_ALL, ICON_DELETE, ICON_DESCRIPTION, ICON_EDIT,
    ICON_EXPAND_ALL, ICON_EXPAND_MORE, ICON_MORE_VERT, ICON_SEARCH,
};

use crate::state::{AppState, SidebarMode, SidebarViewMode};

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
        if let SidebarMode::SlotPicker { slot_label } = &state.sidebar_mode {
            Self::render_slot_picker(ui, state, slot_label.clone());
            return;
        }

        // Check if we have a library loaded
        if let Some(lib_path) = &state.library_path {
            // Library header: icon + name as main heading
            ui.horizontal(|ui| {
                ui.label(ICON_DESCRIPTION);
                let name = if state.library.name.is_empty() {
                    "Untitled Library"
                } else {
                    &state.library.name
                };
                ui.heading(name);
            });

            // File name in smaller, lighter text below
            let file_name = lib_path
                .file_name()
                .map(|n| n.to_string_lossy().to_string())
                .unwrap_or_else(|| lib_path.display().to_string());
            ui.label(
                egui::RichText::new(file_name)
                    .small()
                    .color(ui.visuals().weak_text_color()),
            );

            ui.add_space(8.0);

            // View mode toggle (Prompts / Variables)
            ui.horizontal(|ui| {
                if ui
                    .selectable_label(
                        state.sidebar_view_mode == SidebarViewMode::Prompts,
                        "Prompts",
                    )
                    .clicked()
                {
                    state.sidebar_view_mode = SidebarViewMode::Prompts;
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

            // Search input - icon changes to clear button when text exists
            ui.horizontal(|ui| {
                if state.search_query.is_empty() {
                    ui.label(ICON_SEARCH);
                } else if ui
                    .small_button(ICON_CLOSE)
                    .on_hover_text("Clear search")
                    .clicked()
                {
                    state.search_query.clear();
                }

                ui.add(
                    egui::TextEdit::singleline(&mut state.search_query)
                        .hint_text("Search...")
                        .desired_width(f32::INFINITY),
                );
            });

            // Expand/Collapse all buttons (only in Variables view)
            if state.sidebar_view_mode == SidebarViewMode::Variables {
                ui.add_space(4.0);
                Flex::horizontal().w_full().wrap(false).show(ui, |flex| {
                    // Spacer to push buttons to the right
                    flex.add_ui(FlexItem::default().grow(1.0), |_ui| {});

                    // Expand all button
                    flex.add_ui(FlexItem::default(), |ui| {
                        if ui
                            .small_button(ICON_EXPAND_ALL)
                            .on_hover_text("Expand all")
                            .clicked()
                        {
                            state.expand_all_variables = Some(true);
                        }
                    });

                    // Collapse all button
                    flex.add_ui(FlexItem::default(), |ui| {
                        if ui
                            .small_button(ICON_COLLAPSE_ALL)
                            .on_hover_text("Collapse all")
                            .clicked()
                        {
                            state.expand_all_variables = Some(false);
                        }
                    });
                });
            }

            ui.separator();

            // Content list based on view mode
            Self::render_sidebar_content(ui, state);
        } else if library_file_path.is_some() {
            ui.add_space(16.0);
            ui.label("Failed to load library");
            ui.add_space(4.0);
            ui.label("Check the file format and try again");
        } else {
            ui.add_space(16.0);
            ui.label("No library loaded");
            ui.add_space(4.0);
            ui.label(
                egui::RichText::new("Use File → Open Library to get started")
                    .small()
                    .color(ui.visuals().weak_text_color()),
            );
        }
    }

    /// Render the sidebar content (prompts or variables list).
    fn render_sidebar_content(ui: &mut egui::Ui, state: &mut AppState) {
        egui::ScrollArea::vertical()
            .auto_shrink([false, false])
            .show(ui, |ui| match state.sidebar_view_mode {
                SidebarViewMode::Prompts => Self::render_prompt_list(ui, state),
                SidebarViewMode::Variables => Self::render_variable_list(ui, state),
            });
    }

    /// Render the prompt list.
    fn render_prompt_list(ui: &mut egui::Ui, state: &mut AppState) {
        let search_query = state.search_query.to_lowercase();

        // Collect prompt info we need
        let prompts: Vec<_> = state
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

        let mut prompt_to_open: Option<String> = None;
        let mut prompt_to_delete: Option<String> = None;

        // Get active tab info for highlighting
        let active_tab_name = state
            .active_tab_index
            .and_then(|idx| state.prompt_tabs.get(idx))
            .and_then(|tab| {
                if let crate::state::PromptSource::FromLibrary { original_name } = &tab.source {
                    Some(original_name.clone())
                } else {
                    None
                }
            });

        for name in &prompts {
            // Check if this prompt is open in a tab (for visual indication)
            let tab_index = state.find_tab_by_library_prompt(name);
            let is_open_in_tab = tab_index.is_some();
            let is_active_tab = active_tab_name.as_ref() == Some(name);

            // Determine background color based on state
            let (bg_fill, stroke) = if is_active_tab {
                // Active tab - bright blue (selection color)
                (ui.visuals().selection.bg_fill, egui::Stroke::NONE)
            } else if is_open_in_tab {
                // Open but not active - duller blue like inactive tabs
                (
                    egui::Color32::from_rgb(30, 30, 46),
                    egui::Stroke::new(1.0, egui::Color32::from_rgb(69, 71, 90)),
                )
            } else {
                // Not open - transparent
                (egui::Color32::TRANSPARENT, egui::Stroke::NONE)
            };

            // Wrap the row in a frame with background color
            egui::Frame::new()
                .fill(bg_fill)
                .stroke(stroke)
                .corner_radius(4.0)
                .inner_margin(egui::Margin::symmetric(4, 2))
                .show(ui, |ui| {
                    // Use flex layout for prompt row: label (grows/truncates) + menu button (fixed)
                    Flex::horizontal().w_full().wrap(false).show(ui, |flex| {
                        // Clickable label area (grows and truncates)
                        let prompt_name = name.clone();
                        flex.add_ui(FlexItem::default().grow(1.0).shrink(), |ui| {
                            // Allocate full width with click sense
                            let available_width = ui.available_width();
                            let text_height = ui.text_style_height(&egui::TextStyle::Body);
                            let (rect, response) = ui.allocate_exact_size(
                                egui::vec2(available_width, text_height),
                                egui::Sense::click(),
                            );

                            // Draw text left-aligned and vertically centered
                            ui.painter().text(
                                rect.left_center(),
                                egui::Align2::LEFT_CENTER,
                                name,
                                egui::TextStyle::Body.resolve(ui.style()),
                                ui.visuals().text_color(),
                            );

                            // Handle click
                            if response.clicked() {
                                prompt_to_open = Some(prompt_name.clone());
                            }

                            // Show tooltip on hover
                            response.on_hover_text(&prompt_name);
                        });

                        // Menu button (fixed size)
                        let prompt_name_for_delete = name.clone();
                        flex.add_ui(FlexItem::default(), |ui| {
                            ui.menu_button(ICON_MORE_VERT, |ui| {
                                ui.set_min_width(120.0);
                                if ui.button(format!("{} Delete", ICON_DELETE)).clicked() {
                                    prompt_to_delete = Some(prompt_name_for_delete.clone());
                                    ui.close();
                                }
                            })
                            .response
                            .on_hover_text("More options");
                        });
                    });
                });
        }

        // Open the clicked prompt in a tab
        if let Some(name) = prompt_to_open {
            state.open_library_prompt(&name);
            // Also ensure we're in Prompt editing mode
            state.editor_mode = crate::state::EditorMode::Prompt;
        }

        // Request delete confirmation
        if let Some(name) = prompt_to_delete {
            state.request_delete_prompt(&name);
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
        if state.library.variables.is_empty() {
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
            Some(state.library.search(search_query))
        } else {
            None
        };

        // Build the display data: for each variable, determine if it should be shown
        // and what highlighting to apply
        #[derive(Clone)]
        struct VariableDisplay {
            name: String,
            options: Vec<String>,
            /// For each option, the match indices (for option searches)
            option_matches: Vec<(String, Vec<usize>)>,
            /// Whether this is an option-based search result (affects display)
            is_option_search: bool,
        }

        let variables_display: Vec<VariableDisplay> = match &search_result {
            None => {
                // No search - show all variables
                state
                    .library
                    .variables
                    .iter()
                    .map(|v| VariableDisplay {
                        name: v.name.clone(),
                        options: v.options.clone(),
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

        // Take expand_all_variables state (consumed once per render)
        let expand_all = state.expand_all_variables.take();

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

            // Apply expand/collapse all if requested
            if let Some(expand) = expand_all {
                collapsing_state.set_open(expand);
            }

            // Header row: collapse toggle + label + edit button using flex layout
            Flex::horizontal().w_full().wrap(false).show(ui, |flex| {
                // Toggle icon (fixed size, no grow)
                let icon = if collapsing_state.is_open() {
                    ICON_EXPAND_MORE
                } else {
                    ICON_CHEVRON_RIGHT
                };
                flex.add_ui(FlexItem::default(), |ui| {
                    if ui.small_button(icon).clicked() {
                        collapsing_state.toggle(ui);
                    }
                });

                // Variable name label (shrinks to fit, truncates text)
                let header_text = Self::build_variable_header_text(
                    &var_display.name,
                    var_display.options.len(),
                    var_display.is_option_search,
                );
                let var_name = var_display.name.clone();
                flex.add_ui(FlexItem::default().grow(1.0).shrink(), |ui| {
                    // Left-align and truncate text to available width
                    ui.set_width(ui.available_width());
                    let label = egui::Label::new(&header_text).truncate();
                    let response = ui.add(label);
                    response.on_hover_text(format!("@{}", var_name));
                });

                // Edit button (fixed size, no grow)
                flex.add_ui(FlexItem::default(), |ui| {
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
                            let option_job = Self::build_option_button_job(
                                option_text,
                                match_indices,
                                default_color,
                            );
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

    /// Build a simple text string for a variable header (for use with truncation).
    fn build_variable_header_text(
        name: &str,
        option_count: usize,
        is_option_search: bool,
    ) -> String {
        let suffix = if is_option_search {
            let match_word = if option_count == 1 {
                "match"
            } else {
                "matches"
            };
            format!(" ({} {})", option_count, match_word)
        } else {
            format!(" ({})", option_count)
        };

        format!("@{}{}", name, suffix)
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
                if ui
                    .button(ICON_CLOSE)
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
                        let response = ui.add(egui::Button::new(display_text).fill(fill).wrap());

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
