//! Autocomplete popup component for the prompt editor.
//!
//! Shows variable and option completions when the user types `@` in the editor.

use egui::Key;

use crate::state::{AppState, AutocompleteMode};
use crate::theme;
use promptgen_core::Library;
use promptgen_core::search::VariableSearchResult;

/// Safely get a substring from start to end of string.
/// Returns None if start is not a valid UTF-8 character boundary.
fn safe_slice_from(s: &str, start: usize) -> Option<&str> {
    if start <= s.len() && s.is_char_boundary(start) {
        Some(&s[start..])
    } else {
        None
    }
}

/// Safely get a substring from start of string to end position.
/// Returns None if end is not a valid UTF-8 character boundary.
fn safe_slice_to(s: &str, end: usize) -> Option<&str> {
    if end <= s.len() && s.is_char_boundary(end) {
        Some(&s[..end])
    } else {
        None
    }
}

/// Truncate a string to approximately n characters, adding "..." if truncated.
/// This is safe for multi-byte UTF-8 characters.
fn truncate_chars(s: &str, max_chars: usize) -> String {
    let char_count = s.chars().count();
    if char_count <= max_chars {
        s.to_string()
    } else {
        let truncated: String = s.chars().take(max_chars.saturating_sub(3)).collect();
        format!("{}...", truncated)
    }
}

/// Maximum number of completions to show in the popup
const MAX_COMPLETIONS: usize = 10;

/// A single completion item to display
#[derive(Debug, Clone)]
pub enum CompletionItem {
    /// A variable name completion
    Variable {
        name: String,
        option_count: usize,
        match_indices: Vec<usize>,
    },
    /// An option completion
    Option {
        text: String,
        variable_name: String,
        match_indices: Vec<usize>,
    },
}

impl CompletionItem {
    /// Get the text to insert when this completion is selected
    pub fn insert_text(&self) -> String {
        match self {
            CompletionItem::Variable { name, .. } => {
                // Check if variable name needs quotes
                let needs_quotes = name.contains(' ') || name.contains(':');
                if needs_quotes {
                    format!("@\"{}\"", name)
                } else {
                    format!("@{}", name)
                }
            }
            CompletionItem::Option { text, .. } => text.clone(),
        }
    }
}

/// Get completions based on current autocomplete state for a specific editor
pub fn get_completions(
    library: &Library,
    state: &AppState,
    editor_id: &str,
) -> Vec<CompletionItem> {
    let Some(autocomplete) = state.get_autocomplete(editor_id) else {
        return Vec::new();
    };
    let query = &autocomplete.query;

    match &autocomplete.mode {
        Some(AutocompleteMode::Variables) => {
            // Search for variable names
            let results = library.search_variables(query);

            // If the query exactly matches a variable name (case-insensitive), don't show completions.
            // This allows Enter to work normally when the user has typed a complete variable reference.
            let has_exact_match = results
                .iter()
                .any(|r| r.variable_name.eq_ignore_ascii_case(query));
            if has_exact_match {
                return Vec::new();
            }

            results
                .into_iter()
                .take(MAX_COMPLETIONS)
                .map(|r: VariableSearchResult| CompletionItem::Variable {
                    name: r.variable_name,
                    option_count: r.options.len(),
                    match_indices: r.match_indices,
                })
                .collect()
        }
        Some(AutocompleteMode::Options { variable_name }) => {
            // Search for options within matching variables
            let results = library.search_options_in_matching_variables(variable_name, query);
            let mut completions = Vec::new();
            for result in results {
                for opt in result.matches {
                    completions.push(CompletionItem::Option {
                        text: opt.text,
                        variable_name: result.variable_name.clone(),
                        match_indices: opt.match_indices,
                    });
                    if completions.len() >= MAX_COMPLETIONS {
                        break;
                    }
                }
                if completions.len() >= MAX_COMPLETIONS {
                    break;
                }
            }
            completions
        }
        None => Vec::new(),
    }
}

/// Result from showing the autocomplete popup
pub struct AutocompletePopupResult {
    /// The selected completion text, if any
    pub selected: Option<String>,
    /// Whether the pointer is hovering over the popup
    pub hovered: bool,
}

/// Autocomplete popup component
pub struct AutocompletePopup;

impl AutocompletePopup {
    /// Show the autocomplete popup below the editor widget.
    ///
    /// Returns `AutocompletePopupResult` with the selected completion and hover state.
    #[allow(deprecated)]
    pub fn show(
        ui: &mut egui::Ui,
        state: &mut AppState,
        editor_id: &str,
        editor_response: &egui::Response,
        completions: &[CompletionItem],
    ) -> AutocompletePopupResult {
        if !state.is_autocomplete_active(editor_id) || completions.is_empty() {
            return AutocompletePopupResult {
                selected: None,
                hovered: false,
            };
        }

        let selected_index = state
            .get_autocomplete(editor_id)
            .map(|s| s.selected_index)
            .unwrap_or(0);

        // Take the scroll flag - only scroll when selection changed via keyboard
        let should_scroll = state.take_autocomplete_scroll_flag(editor_id);

        let mut selected_completion: Option<String> = None;
        let editor_id_owned = editor_id.to_string();

        // NOTE: Keyboard handling is done in handle_autocomplete_keyboard() which must be
        // called BEFORE the TextEdit widget. This function only handles mouse clicks.

        // Position the popup below the editor
        let popup_pos = editor_response.rect.left_bottom() + egui::vec2(0.0, 4.0);
        let area_id = egui::Id::new(format!("autocomplete_area_{}", editor_id));

        // Use Area instead of popup_below_widget to have full control over click handling
        egui::Area::new(area_id)
            .order(egui::Order::Foreground)
            .fixed_pos(popup_pos)
            .show(ui.ctx(), |ui| {
                egui::Frame::popup(ui.style()).show(ui, |ui| {
                    // Set explicit width to prevent resize when filtering changes item count
                    let popup_width = 350.0;
                    ui.set_width(popup_width);

                    let scroll_id =
                        ui.make_persistent_id(format!("autocomplete_scroll_{}", editor_id_owned));
                    egui::ScrollArea::vertical()
                        .id_salt(scroll_id)
                        .min_scrolled_height(150.0) // Minimum height to prevent jarring size changes
                        .max_height(250.0)
                        .show(ui, |ui| {
                            // Ensure content fills the width
                            ui.set_min_width(popup_width - 16.0); // Account for scrollbar
                            for (idx, item) in completions.iter().enumerate() {
                                let is_selected = idx == selected_index;

                                // Build the label with highlighted characters
                                let label = match item {
                                    CompletionItem::Variable {
                                        name,
                                        option_count,
                                        match_indices,
                                    } => {
                                        let mut job = egui::text::LayoutJob::default();
                                        let current_theme = theme::current(ui.ctx());

                                        // Add @ prefix
                                        job.append(
                                            "@",
                                            0.0,
                                            egui::TextFormat {
                                                color: current_theme.syntax_reference(),
                                                ..Default::default()
                                            },
                                        );

                                        // Add variable name with match highlighting
                                        for (i, c) in name.chars().enumerate() {
                                            let color = if match_indices.contains(&i) {
                                                current_theme.match_highlight()
                                            } else {
                                                current_theme.syntax_reference()
                                            };
                                            job.append(
                                                &c.to_string(),
                                                0.0,
                                                egui::TextFormat {
                                                    color,
                                                    ..Default::default()
                                                },
                                            );
                                        }

                                        // Add option count
                                        job.append(
                                            &format!(" ({} options)", option_count),
                                            0.0,
                                            egui::TextFormat {
                                                color: current_theme.muted(),
                                                ..Default::default()
                                            },
                                        );

                                        job
                                    }
                                    CompletionItem::Option {
                                        text,
                                        variable_name,
                                        match_indices,
                                    } => {
                                        let mut job = egui::text::LayoutJob::default();
                                        let current_theme = theme::current(ui.ctx());

                                        // Truncate long options (safe for multi-byte UTF-8)
                                        let display_text = truncate_chars(text, 50);

                                        // Add option text with match highlighting
                                        for (i, c) in display_text.chars().enumerate() {
                                            let color = if match_indices.contains(&i) {
                                                current_theme.match_highlight()
                                            } else {
                                                ui.visuals().text_color()
                                            };
                                            job.append(
                                                &c.to_string(),
                                                0.0,
                                                egui::TextFormat {
                                                    color,
                                                    ..Default::default()
                                                },
                                            );
                                        }

                                        // Add variable name context
                                        job.append(
                                            &format!(" (@{})", variable_name),
                                            0.0,
                                            egui::TextFormat {
                                                color: current_theme.muted(),
                                                ..Default::default()
                                            },
                                        );

                                        job
                                    }
                                };

                                let response = ui.selectable_label(is_selected, label);

                                // Handle click
                                if response.clicked() {
                                    selected_completion = Some(item.insert_text());
                                }

                                // Scroll to selected item only when selection changed via keyboard
                                if is_selected && should_scroll {
                                    response.scroll_to_me(Some(egui::Align::Center));
                                }
                            }
                        });
                });
            });

        // Check if pointer is hovering over the popup area
        let hovered = ui.ctx().pointer_hover_pos().is_some_and(|pos| {
            // Get the Area's rect from memory
            ui.ctx().memory(|mem| {
                mem.area_rect(area_id)
                    .is_some_and(|rect| rect.contains(pos))
            })
        });

        AutocompletePopupResult {
            selected: selected_completion,
            hovered,
        }
    }
}

/// Handle autocomplete keyboard input BEFORE the text editor processes it.
/// This must be called before the TextEdit widget to consume arrow/enter/tab/escape keys.
/// Returns Some(completion_text) if a selection was made.
pub fn handle_autocomplete_keyboard(
    ui: &mut egui::Ui,
    state: &mut AppState,
    editor_id: &str,
    completions: &[CompletionItem],
) -> Option<String> {
    if !state.is_autocomplete_active(editor_id) || completions.is_empty() {
        return None;
    }

    // Consume keyboard events so they don't go to the text editor
    let (up, down, enter, tab, escape) = ui.ctx().input_mut(|i| {
        let up = i.consume_key(egui::Modifiers::NONE, Key::ArrowUp);
        let down = i.consume_key(egui::Modifiers::NONE, Key::ArrowDown);
        let enter = i.consume_key(egui::Modifiers::NONE, Key::Enter);
        let tab = i.consume_key(egui::Modifiers::NONE, Key::Tab);
        let escape = i.consume_key(egui::Modifiers::NONE, Key::Escape);
        (up, down, enter, tab, escape)
    });

    if escape {
        state.deactivate_autocomplete(editor_id);
        return None;
    }

    if up {
        state.autocomplete_move_up(editor_id, completions.len());
    }
    if down {
        state.autocomplete_move_down(editor_id, completions.len());
    }
    if enter || tab {
        let selected_index = state
            .get_autocomplete(editor_id)
            .map(|s| s.selected_index)
            .unwrap_or(0);
        if let Some(item) = completions.get(selected_index) {
            let text = item.insert_text();
            // Don't deactivate here - apply_completion needs the autocomplete state
            // to calculate the correct replacement range
            return Some(text);
        }
        state.deactivate_autocomplete(editor_id);
    }

    None
}

/// Apply a completion to content, updating cursor position and deactivating autocomplete.
///
/// This is the central function for applying autocomplete completions. It:
/// - Calculates the replacement range based on the autocomplete mode
/// - Replaces the @query or @variable/query with the completion text
/// - Sets the pending cursor position to after the inserted text
/// - Deactivates autocomplete for this editor
///
/// Returns the new content string.
pub fn apply_completion(
    state: &mut AppState,
    content: &str,
    editor_id: &str,
    completion_text: &str,
) -> String {
    let Some(autocomplete) = state.get_autocomplete(editor_id) else {
        return content.to_string();
    };

    // Replace from trigger position to end of the autocomplete query
    let trigger_pos = autocomplete.trigger_position;
    let query_len = autocomplete.query.len();

    // Calculate where the @query ends based on mode:
    // - Variables mode: @{query} -> trigger_pos + 1 + query_len
    // - Options mode: @{variable_name}/{query} -> trigger_pos + 1 + var_len + 1 + query_len
    let query_end = match &autocomplete.mode {
        Some(AutocompleteMode::Options { variable_name }) => {
            // @variable_name/query
            trigger_pos + 1 + variable_name.len() + 1 + query_len
        }
        _ => {
            // @query
            trigger_pos + 1 + query_len
        }
    };

    // Build the new content, preserving text before @ and after the query
    // Use safe slicing to handle multi-byte UTF-8 characters
    let before = safe_slice_to(content, trigger_pos).unwrap_or("");
    let after = safe_slice_from(content, query_end).unwrap_or("");

    let new_content = format!("{}{}{}", before, completion_text, after);

    // Set cursor position to end of inserted text
    let new_cursor_pos = trigger_pos + completion_text.len();
    state.set_pending_cursor_position(editor_id, new_cursor_pos);

    // Deactivate autocomplete now that we've used the state
    state.deactivate_autocomplete(editor_id);

    new_content
}

/// Check if we should trigger autocomplete based on the just-typed character
/// Returns the trigger position (byte offset of @) if autocomplete should be activated
pub fn check_autocomplete_trigger(content: &str, cursor_byte_pos: usize) -> Option<usize> {
    if cursor_byte_pos == 0 || cursor_byte_pos > content.len() {
        return None;
    }

    // Safely get the substring before cursor (handles multi-byte UTF-8)
    let before_cursor = safe_slice_to(content, cursor_byte_pos)?;

    // Check if the last character is @
    if before_cursor.ends_with('@') {
        // Make sure it's not escaped or inside quotes - for now, simple check
        let at_pos = cursor_byte_pos - 1;

        // Check if there's a space or start of line before the @
        if at_pos == 0 {
            return Some(at_pos);
        }

        // Get the character before @ safely
        let before_at = safe_slice_to(before_cursor, at_pos)?;
        let prev_char = before_at.chars().last();
        match prev_char {
            None => Some(at_pos),
            Some(c) if c.is_whitespace() || c == '{' || c == '|' || c == '(' || c == ',' => {
                Some(at_pos)
            }
            _ => None, // Don't trigger if @ is in the middle of a word
        }
    } else {
        None
    }
}

/// Find an autocomplete context at the given cursor position by looking backwards.
/// Returns the trigger position (byte offset of @) if cursor is in a valid autocomplete context.
/// This is used to detect autocomplete contexts when backspacing or moving cursor.
pub fn find_autocomplete_context(content: &str, cursor_pos: usize) -> Option<usize> {
    if cursor_pos == 0 || cursor_pos > content.len() {
        return None;
    }

    // Safely get the substring before cursor (handles multi-byte UTF-8)
    let before_cursor = safe_slice_to(content, cursor_pos)?;

    // Scan backwards to find @ that could start an autocomplete context
    // Stop at whitespace or certain delimiters
    // Note: char_indices() returns byte indices at character boundaries, so at_pos is always valid
    let mut at_pos = None;
    for (i, c) in before_cursor.char_indices().rev() {
        if c == '@' {
            at_pos = Some(i);
            break;
        }
        // Stop scanning if we hit whitespace or invalid chars
        if c.is_whitespace() {
            return None;
        }
        // Slash is valid (for @Var/opt syntax)
        if c == '/' {
            continue;
        }
        // Other special chars end the search
        if c == '{' || c == '}' || c == '|' || c == '(' || c == ')' {
            return None;
        }
    }

    let at_pos = at_pos?;

    // Check if the @ is at a valid position (start of line or after whitespace/delimiter)
    if at_pos == 0 {
        return Some(at_pos);
    }

    // at_pos came from char_indices() so it's a valid char boundary
    let before_at = safe_slice_to(before_cursor, at_pos)?;
    let prev_char = before_at.chars().last();
    match prev_char {
        None => Some(at_pos),
        Some(c) if c.is_whitespace() || c == '{' || c == '|' || c == '(' || c == ',' => {
            Some(at_pos)
        }
        _ => None, // @ is in the middle of a word, not valid
    }
}

// =============================================================================
// Autocomplete Orchestration Helpers
// =============================================================================
//
// These functions encapsulate the autocomplete workflow that is shared between
// the main prompt editor and the variable options editor. The workflow is:
//
// 1. BEFORE TextEdit: Call `autocomplete_before_editor()` to handle keyboard
//    input (arrow keys, Enter, Tab, Escape) before the TextEdit can consume them.
//
// 2. RENDER TextEdit: Show the egui::TextEdit widget normally.
//
// 3. AFTER TextEdit: Call `autocomplete_after_editor()` to handle activation,
//    popup display, mouse clicks, and focus loss.

/// Handle autocomplete keyboard input BEFORE the TextEdit widget.
///
/// This must be called before showing the TextEdit to intercept arrow keys,
/// Enter, Tab, and Escape before the TextEdit processes them.
///
/// Returns `Some(new_content)` if a completion was selected via keyboard,
/// `None` otherwise.
pub fn autocomplete_before_editor(
    ui: &mut egui::Ui,
    state: &mut AppState,
    editor_id: &str,
    content: &str,
) -> Option<String> {
    if !state.is_autocomplete_active(editor_id) {
        return None;
    }

    let completions = get_completions(&state.library, state, editor_id);
    if completions.is_empty() {
        return None;
    }

    // Handle keyboard input (arrows, enter, tab, escape)
    let selection = handle_autocomplete_keyboard(ui, state, editor_id, &completions);

    // If a completion was selected, apply it
    selection.map(|completion_text| apply_completion(state, content, editor_id, &completion_text))
}

/// Handle autocomplete activation, popup display, and focus loss AFTER the TextEdit widget.
///
/// This handles:
/// - Activating autocomplete when @ is typed or cursor moves into @ context
/// - Updating the autocomplete query based on cursor position
/// - Showing the popup and handling mouse clicks
/// - Deactivating autocomplete when editor loses focus (unless hovering popup)
///
/// Returns `Some(new_content)` if a completion was selected via mouse click,
/// `None` otherwise.
pub fn autocomplete_after_editor(
    ui: &mut egui::Ui,
    state: &mut AppState,
    editor_id: &str,
    content: &str,
    response: &egui::Response,
    cursor_pos: usize,
) -> Option<String> {
    // Handle autocomplete activation/update based on cursor position
    if !state.is_autocomplete_active(editor_id) {
        // Check if we're in an autocomplete context (either just typed @ or cursor is after @)
        if let Some(trigger_pos) = check_autocomplete_trigger(content, cursor_pos)
            .or_else(|| find_autocomplete_context(content, cursor_pos))
        {
            state.activate_autocomplete(editor_id, trigger_pos);
            // Deactivate autocomplete in other editors
            state.deactivate_autocomplete_except(editor_id);
            // Update the query immediately
            state.update_autocomplete_query(editor_id, content, cursor_pos);
        }
    } else {
        // Autocomplete is active, update the query with actual cursor position
        state.update_autocomplete_query(editor_id, content, cursor_pos);
    }

    // Show autocomplete popup if active
    // NOTE: We must show the popup BEFORE checking focus loss, because clicking the popup
    // causes the editor to lose focus. The popup click handler needs to run first.
    if !state.is_autocomplete_active(editor_id) {
        return None;
    }

    let completions = get_completions(&state.library, state, editor_id);

    if completions.is_empty() {
        state.deactivate_autocomplete(editor_id);
        return None;
    }

    // Show popup and handle mouse clicks
    let popup_result = AutocompletePopup::show(ui, state, editor_id, response, &completions);

    let new_content = popup_result
        .selected
        .map(|completion_text| apply_completion(state, content, editor_id, &completion_text));

    // Deactivate autocomplete if editor loses focus, unless pointer is over popup
    if !response.has_focus() && state.is_autocomplete_active(editor_id) && !popup_result.hovered {
        state.deactivate_autocomplete(editor_id);
    }

    new_content
}
