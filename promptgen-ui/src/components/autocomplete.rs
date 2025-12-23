//! Autocomplete popup component for the prompt editor.
//!
//! Shows variable and option completions when the user types `@` in the editor.

use egui::Key;

use crate::state::{AppState, AutocompleteMode};
use crate::theme::syntax;
use promptgen_core::Library;
use promptgen_core::search::VariableSearchResult;

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

/// Autocomplete popup component
pub struct AutocompletePopup;

impl AutocompletePopup {
    /// Show the autocomplete popup below the editor widget.
    ///
    /// Returns `Some(completion_text)` if a completion was selected and should be inserted,
    /// or `None` if no action needed.
    #[allow(deprecated)]
    pub fn show(
        ui: &mut egui::Ui,
        state: &mut AppState,
        editor_id: &str,
        editor_response: &egui::Response,
        completions: &[CompletionItem],
    ) -> Option<String> {
        if !state.is_autocomplete_active(editor_id) || completions.is_empty() {
            return None;
        }

        let selected_index = state
            .get_autocomplete(editor_id)
            .map(|s| s.selected_index)
            .unwrap_or(0);

        let mut selected_completion: Option<String> = None;
        let editor_id_owned = editor_id.to_string();

        // NOTE: Keyboard handling is done in handle_autocomplete_keyboard() which must be
        // called BEFORE the TextEdit widget. This function only handles mouse clicks.

        // Show popup below the editor (unique ID per editor)
        let popup_id = ui.make_persistent_id(format!("autocomplete_popup_{}", editor_id));

        // IMPORTANT: Open the popup BEFORE calling popup_below_widget, otherwise
        // popup_below_widget will check if it's open and skip rendering since it wasn't open yet.
        ui.memory_mut(|mem| mem.open_popup(popup_id));

        egui::popup_below_widget(
            ui,
            popup_id,
            editor_response,
            egui::PopupCloseBehavior::CloseOnClick,
            |ui| {
                ui.set_min_width(300.0);

                let scroll_id =
                    ui.make_persistent_id(format!("autocomplete_scroll_{}", editor_id_owned));
                egui::ScrollArea::vertical()
                    .id_salt(scroll_id)
                    .max_height(250.0)
                    .show(ui, |ui| {
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

                                    // Add @ prefix
                                    job.append(
                                        "@",
                                        0.0,
                                        egui::TextFormat {
                                            color: syntax::VARIABLE_REF,
                                            ..Default::default()
                                        },
                                    );

                                    // Add variable name with match highlighting
                                    for (i, c) in name.chars().enumerate() {
                                        let color = if match_indices.contains(&i) {
                                            syntax::MATCH_HIGHLIGHT
                                        } else {
                                            syntax::VARIABLE_REF
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
                                            color: egui::Color32::from_rgb(108, 112, 134), // overlay0
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

                                    // Truncate long options
                                    let display_text = if text.len() > 50 {
                                        format!("{}...", &text[..47])
                                    } else {
                                        text.clone()
                                    };

                                    // Add option text with match highlighting
                                    for (i, c) in display_text.chars().enumerate() {
                                        let color = if match_indices.contains(&i) {
                                            syntax::MATCH_HIGHLIGHT
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
                                            color: egui::Color32::from_rgb(108, 112, 134), // overlay0
                                            ..Default::default()
                                        },
                                    );

                                    job
                                }
                            };

                            let response = ui.selectable_label(is_selected, label);

                            // Handle click - we'll return the completion text, caller handles deactivation
                            if response.clicked() {
                                selected_completion = Some(item.insert_text());
                            }

                            // Scroll to selected item
                            if is_selected {
                                response.scroll_to_me(Some(egui::Align::Center));
                            }
                        }
                    });
            },
        );

        selected_completion
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
    let before = &content[..trigger_pos];
    let after = if query_end <= content.len() {
        &content[query_end..]
    } else {
        ""
    };

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

    // Look at the character just typed (before cursor)
    let before_cursor = &content[..cursor_byte_pos];

    // Check if the last character is @
    if before_cursor.ends_with('@') {
        // Make sure it's not escaped or inside quotes - for now, simple check
        let at_pos = cursor_byte_pos - 1;

        // Check if there's a space or start of line before the @
        if at_pos == 0 {
            return Some(at_pos);
        }

        let prev_char = before_cursor[..at_pos].chars().last();
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

    let before_cursor = &content[..cursor_pos];

    // Scan backwards to find @ that could start an autocomplete context
    // Stop at whitespace or certain delimiters
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

    let prev_char = before_cursor[..at_pos].chars().last();
    match prev_char {
        None => Some(at_pos),
        Some(c) if c.is_whitespace() || c == '{' || c == '|' || c == '(' || c == ',' => {
            Some(at_pos)
        }
        _ => None, // @ is in the middle of a word, not valid
    }
}
