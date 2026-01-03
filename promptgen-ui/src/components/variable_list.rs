//! Reusable variable list component for displaying option groups.
//!
//! Used by both the Variables tab and the slot picker.

use egui_flex::{Flex, FlexItem};
use egui_material_icons::icons::{
    ICON_ADD, ICON_ARROW_DOWNWARD, ICON_ARROW_UPWARD, ICON_CHEVRON_RIGHT, ICON_COLLAPSE_ALL,
    ICON_EDIT, ICON_EXPAND_ALL, ICON_EXPAND_MORE, ICON_MENU, ICON_SEARCH, ICON_SORT_BY_ALPHA,
};

use crate::state::{OptionGroup, VariableSortOrder};
use crate::theme;

/// Configuration for the variable list component.
pub struct VariableListConfig<'a> {
    /// ID prefix for egui widgets (to avoid ID conflicts)
    pub id_prefix: &'a str,
    /// Whether options are selectable (slot picker mode)
    pub selectable: bool,
    /// Currently selected values (for slot picker mode)
    pub selected_values: &'a [String],
    /// Whether more selections can be added (for cardinality limits)
    pub can_add_selection: bool,
    /// Whether to show edit buttons on variable groups
    pub show_edit_buttons: bool,
}

impl Default for VariableListConfig<'_> {
    fn default() -> Self {
        Self {
            id_prefix: "varlist",
            selectable: false,
            selected_values: &[],
            can_add_selection: true,
            show_edit_buttons: true,
        }
    }
}

/// Result from rendering the variable list.
#[derive(Default)]
pub struct VariableListResult {
    /// Option that was clicked for selection/deselection (option_value)
    pub option_clicked: Option<String>,
    /// Edit button clicked for a group (group_name)
    pub edit_clicked: Option<String>,
}

/// Internal display data for a group after search filtering
#[derive(Clone)]
struct GroupDisplay {
    name: String,
    options: Vec<String>,
    /// For each option, the match indices (for option searches)
    option_matches: Vec<(String, Vec<usize>)>,
    /// Whether this is an option-based search result (affects display)
    is_option_search: bool,
    /// Whether this group is editable
    is_editable: bool,
}

/// Reusable variable list component.
pub struct VariableList;

impl VariableList {
    /// Render the search bar.
    pub fn show_search_bar(ui: &mut egui::Ui, search_query: &mut String) {
        ui.horizontal(|ui| {
            ui.label(ICON_SEARCH);
            ui.add(
                egui::TextEdit::singleline(search_query)
                    .hint_text("Search...")
                    .desired_width(f32::INFINITY),
            );
        });
    }

    /// Render the toolbar with optional new variable button, sort, and expand/collapse buttons.
    /// Returns true if the new variable button was clicked.
    pub fn show_toolbar(
        ui: &mut egui::Ui,
        sort_order: &mut VariableSortOrder,
        option_sort_order: &mut VariableSortOrder,
        expand_all: &mut Option<bool>,
        show_new_variable_button: bool,
    ) -> bool {
        let mut new_variable_clicked = false;

        ui.add_space(4.0);
        Flex::horizontal().w_full().wrap(false).show(ui, |flex| {
            // New variable button (left-aligned)
            if show_new_variable_button {
                flex.add_ui(FlexItem::default(), |ui| {
                    if ui
                        .small_button(format!("{} Add", ICON_ADD))
                        .on_hover_text("New variable")
                        .clicked()
                    {
                        new_variable_clicked = true;
                    }
                });
            }

            // Spacer to push remaining buttons to the right
            flex.add_ui(FlexItem::default().grow(1.0), |_ui| {});

            // Variable sort button - cycles through None -> Ascending -> Descending -> None
            flex.add_ui(FlexItem::default(), |ui| {
                let current_theme = theme::current(ui.ctx());
                let is_active = *sort_order != VariableSortOrder::None;
                let (icon, tooltip) = match sort_order {
                    VariableSortOrder::None => {
                        (ICON_SORT_BY_ALPHA.to_string(), "Sort variables A-Z")
                    }
                    VariableSortOrder::Ascending => (
                        format!("{}{}", ICON_SORT_BY_ALPHA, ICON_ARROW_UPWARD),
                        "Sort variables Z-A",
                    ),
                    VariableSortOrder::Descending => (
                        format!("{}{}", ICON_SORT_BY_ALPHA, ICON_ARROW_DOWNWARD),
                        "Clear variable sort",
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
                    *sort_order = match sort_order {
                        VariableSortOrder::None => VariableSortOrder::Ascending,
                        VariableSortOrder::Ascending => VariableSortOrder::Descending,
                        VariableSortOrder::Descending => VariableSortOrder::None,
                    };
                }
            });

            // Option sort button - cycles through None -> Ascending -> Descending -> None
            flex.add_ui(FlexItem::default(), |ui| {
                let current_theme = theme::current(ui.ctx());
                let is_active = *option_sort_order != VariableSortOrder::None;
                let (icon, tooltip) = match option_sort_order {
                    VariableSortOrder::None => (
                        format!("{}{}", ICON_MENU, ICON_SORT_BY_ALPHA),
                        "Sort options A-Z",
                    ),
                    VariableSortOrder::Ascending => (
                        format!("{}{}{}", ICON_MENU, ICON_SORT_BY_ALPHA, ICON_ARROW_UPWARD),
                        "Sort options Z-A",
                    ),
                    VariableSortOrder::Descending => (
                        format!("{}{}{}", ICON_MENU, ICON_SORT_BY_ALPHA, ICON_ARROW_DOWNWARD),
                        "Clear option sort",
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
                    *option_sort_order = match option_sort_order {
                        VariableSortOrder::None => VariableSortOrder::Ascending,
                        VariableSortOrder::Ascending => VariableSortOrder::Descending,
                        VariableSortOrder::Descending => VariableSortOrder::None,
                    };
                }
            });

            // Expand all button
            flex.add_ui(FlexItem::default(), |ui| {
                if ui
                    .small_button(ICON_EXPAND_ALL)
                    .on_hover_text("Expand all")
                    .clicked()
                {
                    *expand_all = Some(true);
                }
            });

            // Collapse all button
            flex.add_ui(FlexItem::default(), |ui| {
                if ui
                    .small_button(ICON_COLLAPSE_ALL)
                    .on_hover_text("Collapse all")
                    .clicked()
                {
                    *expand_all = Some(false);
                }
            });
        });

        new_variable_clicked
    }

    /// Render the variable/option group list.
    ///
    /// # Arguments
    /// * `ui` - The egui UI context
    /// * `groups` - The option groups to display
    /// * `search_query` - Current search query (for filtering and highlighting)
    /// * `sort_order` - Current sort order for variable groups
    /// * `option_sort_order` - Current sort order for options within groups
    /// * `expand_all` - If Some, expand or collapse all groups
    /// * `config` - Configuration for the component
    /// * `library` - The library for search functionality (optional, for variable search)
    #[allow(clippy::too_many_arguments)]
    pub fn show_groups(
        ui: &mut egui::Ui,
        groups: &[OptionGroup],
        search_query: &str,
        sort_order: VariableSortOrder,
        option_sort_order: VariableSortOrder,
        expand_all: Option<bool>,
        config: &VariableListConfig<'_>,
        library: Option<&promptgen_core::Library>,
    ) -> VariableListResult {
        let mut result = VariableListResult::default();

        if groups.is_empty() {
            ui.label("No options available");
            return result;
        }

        let search_query = search_query.trim();
        let is_searching = !search_query.is_empty();

        // Build display data with search filtering
        let groups_display = Self::build_display_data(groups, search_query, library);

        // Apply group sorting
        let mut groups_display = groups_display;
        match sort_order {
            VariableSortOrder::None => {} // Keep original order
            VariableSortOrder::Ascending => {
                groups_display.sort_by(|a, b| a.name.to_lowercase().cmp(&b.name.to_lowercase()));
            }
            VariableSortOrder::Descending => {
                groups_display.sort_by(|a, b| b.name.to_lowercase().cmp(&a.name.to_lowercase()));
            }
        }

        // Apply option sorting within each group
        if option_sort_order != VariableSortOrder::None {
            for group in &mut groups_display {
                match option_sort_order {
                    VariableSortOrder::None => {} // Keep original order
                    VariableSortOrder::Ascending => {
                        group.options.sort_by_key(|a| a.to_lowercase());
                        group
                            .option_matches
                            .sort_by(|(a, _), (b, _)| a.to_lowercase().cmp(&b.to_lowercase()));
                    }
                    VariableSortOrder::Descending => {
                        group
                            .options
                            .sort_by_key(|b| std::cmp::Reverse(b.to_lowercase()));
                        // Also sort option_matches if present
                        group
                            .option_matches
                            .sort_by(|(a, _), (b, _)| b.to_lowercase().cmp(&a.to_lowercase()));
                    }
                }
            }
        }

        if groups_display.is_empty() && is_searching {
            ui.label("No matching options");
            return result;
        }

        let default_color = ui.visuals().text_color();
        let theme = theme::current(ui.ctx());

        for group_display in &groups_display {
            let id = ui.make_persistent_id(format!("{}_{}", config.id_prefix, &group_display.name));

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

                // Group name label (shrinks to fit, truncates text)
                let header_text = Self::build_header_text(
                    &group_display.name,
                    group_display.options.len(),
                    group_display.is_option_search,
                    group_display.is_editable,
                );
                let group_name = group_display.name.clone();
                flex.add_ui(FlexItem::default().grow(1.0).shrink(), |ui| {
                    ui.set_width(ui.available_width());
                    let label = egui::Label::new(&header_text).truncate();
                    let response = ui.add(label);
                    if group_display.is_editable {
                        response.on_hover_text(format!("@{}", group_name));
                    }
                });

                // Edit button (only for editable groups)
                if config.show_edit_buttons && group_display.is_editable {
                    flex.add_ui(FlexItem::default(), |ui| {
                        if ui
                            .small_button(ICON_EDIT)
                            .on_hover_text("Edit variable")
                            .clicked()
                        {
                            result.edit_clicked = Some(group_display.name.clone());
                        }
                    });
                }
            });

            // Body content (only shown when expanded)
            collapsing_state.show_body_unindented(ui, |ui| {
                ui.with_layout(egui::Layout::top_down_justified(egui::Align::LEFT), |ui| {
                    let options_to_show: Vec<(&str, Vec<usize>)> = if group_display.is_option_search
                        && !group_display.option_matches.is_empty()
                    {
                        group_display
                            .option_matches
                            .iter()
                            .map(|(text, indices)| (text.as_str(), indices.clone()))
                            .collect()
                    } else {
                        group_display
                            .options
                            .iter()
                            .map(|opt| (opt.as_str(), vec![]))
                            .collect()
                    };

                    for (option_text, match_indices) in options_to_show {
                        let is_selected = config.selected_values.contains(&option_text.to_string());

                        // Build the button content
                        let button_content = if !match_indices.is_empty() {
                            Self::build_option_button_job(
                                option_text,
                                &match_indices,
                                default_color,
                            )
                        } else {
                            let mut job = egui::text::LayoutJob::default();
                            job.append(
                                &format!("• {}", option_text),
                                0.0,
                                egui::text::TextFormat {
                                    color: default_color,
                                    ..Default::default()
                                },
                            );
                            job
                        };

                        // Determine button fill color
                        let fill = if config.selectable && is_selected {
                            theme.active_selected_bg()
                        } else {
                            egui::Color32::TRANSPARENT
                        };

                        let response = ui.add(egui::Button::new(button_content).fill(fill).wrap());

                        // Hover text
                        if config.selectable {
                            response.clone().on_hover_text(option_text);
                        } else {
                            response.clone().on_hover_text("Click to copy");
                        }

                        if response.clicked() {
                            if config.selectable {
                                // In selectable mode, clicking toggles selection
                                // Only allow selection if can_add_selection or if already selected (for deselect)
                                if config.can_add_selection || is_selected {
                                    result.option_clicked = Some(option_text.to_string());
                                }
                            } else {
                                // In non-selectable mode, clicking copies to clipboard
                                ui.ctx().copy_text(option_text.to_string());
                            }
                        }
                    }
                });
            });
        }

        result
    }

    /// Build display data from option groups, applying search filtering.
    fn build_display_data(
        groups: &[OptionGroup],
        search_query: &str,
        library: Option<&promptgen_core::Library>,
    ) -> Vec<GroupDisplay> {
        if search_query.is_empty() {
            // No search - show all groups with all options
            return groups
                .iter()
                .map(|g| GroupDisplay {
                    name: g.name.clone(),
                    options: g.options.clone(),
                    option_matches: vec![],
                    is_option_search: false,
                    is_editable: g.is_editable,
                })
                .collect();
        }

        let query_lower = search_query.to_lowercase();

        // Helper to do substring matching for a group's options
        let substring_match_group = |g: &OptionGroup| -> Option<GroupDisplay> {
            let matching_options: Vec<_> = g
                .options
                .iter()
                .filter(|opt| opt.to_lowercase().contains(&query_lower))
                .cloned()
                .collect();

            if matching_options.is_empty() {
                None
            } else {
                Some(GroupDisplay {
                    name: g.name.clone(),
                    options: matching_options,
                    option_matches: vec![],
                    is_option_search: false,
                    is_editable: g.is_editable,
                })
            }
        };

        // If we have a library, use its search functionality for library variables
        // Also do substring matching for non-library groups (like "Options")
        if let Some(lib) = library {
            let search_result = lib.search(search_query);

            match search_result {
                promptgen_core::SearchResult::Variables(var_results) => {
                    // Variable name search - filter to matching groups
                    let matching_names: std::collections::HashSet<_> =
                        var_results.iter().map(|vr| &vr.variable_name).collect();

                    let mut result: Vec<GroupDisplay> = groups
                        .iter()
                        .filter(|g| g.is_editable && matching_names.contains(&g.name))
                        .map(|g| GroupDisplay {
                            name: g.name.clone(),
                            options: g.options.clone(),
                            option_matches: vec![],
                            is_option_search: false,
                            is_editable: g.is_editable,
                        })
                        .collect();

                    // Also check non-editable groups with substring matching
                    for group in groups.iter().filter(|g| !g.is_editable) {
                        if let Some(display) = substring_match_group(group) {
                            result.push(display);
                        }
                    }

                    result
                }
                promptgen_core::SearchResult::Options(opt_results) => {
                    // Option search - show variables with matching options only
                    let mut result = Vec::new();

                    for group in groups {
                        if group.is_editable {
                            // Library variable - use library search results
                            if let Some(opt_result) =
                                opt_results.iter().find(|or| or.variable_name == group.name)
                            {
                                result.push(GroupDisplay {
                                    name: group.name.clone(),
                                    options: opt_result
                                        .matches
                                        .iter()
                                        .map(|m| m.text.clone())
                                        .collect(),
                                    option_matches: opt_result
                                        .matches
                                        .iter()
                                        .map(|m| (m.text.clone(), m.match_indices.clone()))
                                        .collect(),
                                    is_option_search: true,
                                    is_editable: group.is_editable,
                                });
                            }
                        } else {
                            // Non-library group - use substring matching
                            if let Some(display) = substring_match_group(group) {
                                result.push(display);
                            }
                        }
                    }

                    result
                }
            }
        } else {
            // No library - do simple substring matching on options
            groups
                .iter()
                .filter_map(|g| {
                    let matching_options: Vec<_> = g
                        .options
                        .iter()
                        .filter(|opt| opt.to_lowercase().contains(&query_lower))
                        .cloned()
                        .collect();

                    if matching_options.is_empty() {
                        None
                    } else {
                        Some(GroupDisplay {
                            name: g.name.clone(),
                            options: matching_options,
                            option_matches: vec![],
                            is_option_search: false,
                            is_editable: g.is_editable,
                        })
                    }
                })
                .collect()
        }
    }

    /// Build header text for a group.
    fn build_header_text(
        name: &str,
        option_count: usize,
        is_option_search: bool,
        is_editable: bool,
    ) -> String {
        let prefix = if is_editable { "@" } else { "" };
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

        format!("{}{}{}", prefix, name, suffix)
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

    /// Create a LayoutJob that highlights matched characters.
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
}
