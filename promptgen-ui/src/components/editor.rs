//! Editor panel component for template editing.

use crate::components::autocomplete::{
    check_autocomplete_trigger, find_autocomplete_context, get_completions,
    handle_autocomplete_keyboard, AutocompletePopup,
};
use crate::components::focusable_frame::FocusableFrame;
use crate::components::template_editor::{TemplateEditor, TemplateEditorConfig};
use crate::state::AppState;

/// Editor panel for editing prompt templates.
pub struct EditorPanel;

impl EditorPanel {
    /// Render the editor panel.
    pub fn show(ui: &mut egui::Ui, state: &mut AppState) {
        ui.heading("Editor");
        ui.separator();

        // Take pending cursor position (will be cleared after use)
        let cursor_position = state.pending_cursor_position.take();

        let config = TemplateEditorConfig {
            id: "main_editor".to_string(),
            min_lines: 5,
            hint_text: Some(
                "Enter your prompt template here...\n\n\
                 Use @VariableName to reference variables.\n\
                 Use {option1|option2|option3} for inline choices.\n\
                 Use {{ slot_name }} for user-filled slots."
                    .to_string(),
            ),
            show_line_numbers: true,
            cursor_position,
        };

        let is_focused = state.is_main_editor_focused();

        // IMPORTANT: Handle autocomplete keyboard BEFORE the text editor processes input
        // This prevents Enter/Tab/Arrow keys from being handled by the text editor
        let mut autocomplete_selection: Option<String> = None;
        if state.autocomplete.active {
            let completions = get_completions(&state.workspace, state);
            if !completions.is_empty() {
                autocomplete_selection = handle_autocomplete_keyboard(ui, state, &completions);
            }
        }

        // If we got a selection from keyboard, apply it before rendering
        if let Some(completion_text) = autocomplete_selection {
            Self::apply_completion(state, &completion_text);
        }

        let frame_response = FocusableFrame::new(is_focused).show(ui, |ui| {
            TemplateEditor::show(ui, &mut state.editor_content, &state.workspace, &config)
        });

        let result = frame_response.inner;

        // Update parse result when editor content changes
        if result.response.changed() {
            state.parse_result = Some(result.parse_result.clone());
            state.update_parse_result();
            state.request_render();
        }

        // Get cursor position from the editor
        let cursor_pos = result.cursor_position.unwrap_or(state.editor_content.len());

        // Handle autocomplete activation/update based on cursor position
        if !state.autocomplete.active {
            // Check if we're in an autocomplete context (either just typed @ or cursor is after @)
            if let Some(trigger_pos) = check_autocomplete_trigger(&state.editor_content, cursor_pos)
                .or_else(|| find_autocomplete_context(&state.editor_content, cursor_pos))
            {
                state.activate_autocomplete(trigger_pos);
                // Update the query immediately
                let content_clone = state.editor_content.clone();
                state.update_autocomplete_query(&content_clone, cursor_pos);
            }
        } else {
            // Autocomplete is active, update the query with actual cursor position
            let content_clone = state.editor_content.clone();
            state.update_autocomplete_query(&content_clone, cursor_pos);
        }

        // Track focus - either from TextEdit gaining focus or clicking anywhere in frame
        if (result.response.has_focus() || frame_response.clicked) && !is_focused {
            state.focus_main_editor();
        }

        // Deactivate autocomplete if editor loses focus
        if !result.response.has_focus() && state.autocomplete.active {
            state.deactivate_autocomplete();
        }

        // Show autocomplete popup if active (visual only, keyboard already handled above)
        if state.autocomplete.active {
            let completions = get_completions(&state.workspace, state);

            if completions.is_empty() {
                // No completions, deactivate
                state.deactivate_autocomplete();
            } else {
                // Show popup and handle mouse clicks
                if let Some(completion_text) =
                    AutocompletePopup::show(ui, state, &result.response, &completions)
                {
                    Self::apply_completion(state, &completion_text);
                }
            }
        }

        // Error display below editor
        TemplateEditor::show_errors(ui, &result.parse_result);
    }

    /// Apply a completion to the editor content
    fn apply_completion(state: &mut AppState, completion_text: &str) {
        use crate::state::AutocompleteMode;

        // Replace from trigger position to end of the autocomplete query
        let trigger_pos = state.autocomplete.trigger_position;
        let query_len = state.autocomplete.query.len();

        // Calculate where the @query ends based on mode:
        // - Variables mode: @{query} -> trigger_pos + 1 + query_len
        // - Options mode: @{variable_name}/{query} -> trigger_pos + 1 + var_len + 1 + query_len
        let query_end = match &state.autocomplete.mode {
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
        let before = state.editor_content[..trigger_pos].to_string();
        let after = if query_end <= state.editor_content.len() {
            state.editor_content[query_end..].to_string()
        } else {
            String::new()
        };

        state.editor_content = format!("{}{}{}", before, completion_text, after);

        // Set cursor position to end of inserted text
        let new_cursor_pos = trigger_pos + completion_text.len();
        state.pending_cursor_position = Some(new_cursor_pos);

        // Deactivate autocomplete now that we've used the state
        state.deactivate_autocomplete();

        // Update parse result after insertion
        state.parse_result = Some(state.workspace.parse_template(&state.editor_content));
        state.update_parse_result();
        state.request_render();
    }
}
