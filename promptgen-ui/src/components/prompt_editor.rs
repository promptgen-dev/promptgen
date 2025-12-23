//! Reusable template editor widget with syntax highlighting, line numbers, and autocomplete.

use egui::TextBuffer;

use crate::components::autocomplete::{
    AutocompletePopup, apply_completion, check_autocomplete_trigger, find_autocomplete_context,
    get_completions, handle_autocomplete_keyboard,
};
use crate::highlighting::highlight_prompt;
use crate::state::AppState;
use crate::theme::syntax;
use promptgen_core::ParseResult;

/// Configuration for the template editor widget
#[derive(Clone)]
pub struct PromptEditorConfig {
    /// Unique identifier for this editor instance (required for multiple editors)
    pub id: String,
    /// Minimum number of lines to display (main editor: 5, slots: 3)
    pub min_lines: usize,
    /// Hint text to show when editor is empty
    pub hint_text: Option<String>,
    /// Whether to show line numbers (default: true)
    pub show_line_numbers: bool,
}

impl Default for PromptEditorConfig {
    fn default() -> Self {
        Self {
            id: "template_editor".to_string(),
            min_lines: 5,
            hint_text: None,
            show_line_numbers: true,
        }
    }
}

/// Response from the template editor widget
pub struct PromptEditorResponse {
    /// The egui Response for the text edit widget
    pub response: egui::Response,
    /// Parse result for the content (updated each frame)
    pub parse_result: ParseResult,
}

/// Reusable template editor widget with syntax highlighting, line numbers, and autocomplete
pub struct PromptEditor;

impl PromptEditor {
    /// Show the editor widget with full autocomplete support.
    ///
    /// This is the main entry point that handles:
    /// - Syntax highlighting
    /// - Line numbers (optional)
    /// - Autocomplete activation, keyboard handling, and popup display
    ///
    /// Returns TemplateEditorResponse with the response and parse result
    pub fn show(
        ui: &mut egui::Ui,
        content: &mut String,
        state: &mut AppState,
        config: &PromptEditorConfig,
    ) -> PromptEditorResponse {
        let editor_id = &config.id;

        // Take pending cursor position (will be cleared after use)
        let cursor_position = state.take_pending_cursor_position(editor_id);

        // IMPORTANT: Handle autocomplete keyboard BEFORE the text editor processes input
        // This prevents Enter/Tab/Arrow keys from being handled by the text editor
        let mut autocomplete_selection: Option<String> = None;
        if state.is_autocomplete_active(editor_id) {
            let completions = get_completions(&state.library, state, editor_id);
            if !completions.is_empty() {
                autocomplete_selection =
                    handle_autocomplete_keyboard(ui, state, editor_id, &completions);
            }
        }

        // If we got a selection from keyboard, apply it before rendering
        if let Some(completion_text) = autocomplete_selection {
            *content = apply_completion(state, content, editor_id, &completion_text);
        }

        // Parse content for syntax highlighting
        let parse_result = state.library.parse_prompt(content);

        // Clone parse result for the layouter closure
        let parse_result_clone = parse_result.clone();

        // Create the text editor with custom syntax highlighting
        let mut layouter = |ui: &egui::Ui, text: &dyn TextBuffer, wrap_width: f32| {
            let text_str = text.as_str();
            let mut job = highlight_prompt(ui.ctx(), text_str, Some(&parse_result_clone));
            job.wrap.max_width = wrap_width;
            ui.ctx().fonts_mut(|f| f.layout_job(job))
        };

        // Calculate rows based on content, minimum from config
        let line_count = content.lines().count().max(1);
        let desired_rows = line_count.max(config.min_lines);

        // Horizontal layout for line numbers + editor (no internal scroll)
        let layout_response = ui.horizontal_top(|ui| {
            if config.show_line_numbers {
                // Line numbers column - match the number of lines in content
                // Right-align numbers with minimal width based on max line number
                let max_digits = desired_rows.to_string().len();
                let line_numbers: String = (1..=desired_rows)
                    .map(|n| format!("{:>width$}", n, width = max_digits))
                    .collect::<Vec<_>>()
                    .join("\n");

                // Calculate width: ~8px per digit + small margin
                let width = (max_digits as f32) * 8.0 + 4.0;

                ui.add(
                    egui::TextEdit::multiline(&mut line_numbers.as_str())
                        .font(egui::TextStyle::Monospace)
                        .desired_width(width)
                        .frame(false)
                        .interactive(false)
                        .text_color(egui::Color32::from_rgb(108, 112, 134)), // Catppuccin overlay0
                );

                ui.add_space(4.0);
            }

            // Main editor - auto-size to content
            let text_edit_id = ui.make_persistent_id(editor_id);
            let mut text_edit = egui::TextEdit::multiline(content)
                .id(text_edit_id)
                .desired_width(f32::INFINITY)
                .desired_rows(desired_rows)
                .font(egui::TextStyle::Monospace)
                .layouter(&mut layouter);

            // Add hint text if provided
            if let Some(hint) = &config.hint_text {
                text_edit = text_edit.hint_text(hint.as_str());
            }

            let response = ui.add(text_edit);

            // Apply pending cursor position if set
            if let Some(cursor_pos) = cursor_position
                && let Some(mut text_state) = egui::TextEdit::load_state(ui.ctx(), text_edit_id)
            {
                let ccursor = egui::text::CCursor::new(cursor_pos);
                text_state
                    .cursor
                    .set_char_range(Some(egui::text::CCursorRange::one(ccursor)));
                text_state.store(ui.ctx(), text_edit_id);
                // Request focus to make sure the cursor is visible
                response.request_focus();
            }

            // Read current cursor position
            let cursor_position = egui::TextEdit::load_state(ui.ctx(), text_edit_id)
                .and_then(|text_state| text_state.cursor.char_range())
                .map(|range| range.primary.index);

            (response, cursor_position)
        });

        let response = layout_response.inner.0;
        let cursor_pos = layout_response.inner.1.unwrap_or(content.len());

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

        // Deactivate autocomplete if editor loses focus
        if !response.has_focus() && state.is_autocomplete_active(editor_id) {
            state.deactivate_autocomplete(editor_id);
        }

        // Show autocomplete popup if active (visual only, keyboard already handled above)
        if state.is_autocomplete_active(editor_id) {
            let completions = get_completions(&state.library, state, editor_id);

            if completions.is_empty() {
                // No completions, deactivate
                state.deactivate_autocomplete(editor_id);
            } else {
                // Show popup and handle mouse clicks
                if let Some(completion_text) =
                    AutocompletePopup::show(ui, state, editor_id, &response, &completions)
                {
                    *content = apply_completion(state, content, editor_id, &completion_text);
                }
            }
        }

        PromptEditorResponse {
            response,
            parse_result,
        }
    }

    /// Show parse errors below the editor (call after show())
    pub fn show_errors(ui: &mut egui::Ui, parse_result: &ParseResult) {
        if !parse_result.errors.is_empty() {
            ui.add_space(8.0);
            ui.separator();

            for error in &parse_result.errors {
                ui.horizontal(|ui| {
                    ui.colored_label(syntax::ERROR, "error:");
                    ui.label(&error.message);
                });

                // Show span info
                let span = &error.span;
                if !span.is_empty() {
                    ui.horizontal(|ui| {
                        ui.add_space(20.0);
                        ui.colored_label(
                            egui::Color32::from_rgb(108, 112, 134),
                            format!("  at position {}..{}", span.start, span.end),
                        );
                    });
                }
            }
        }

        // Show warnings too
        if !parse_result.warnings.is_empty() {
            ui.add_space(4.0);
            for warning in &parse_result.warnings {
                ui.horizontal(|ui| {
                    ui.colored_label(
                        egui::Color32::from_rgb(249, 226, 175), // Catppuccin yellow
                        "warning:",
                    );
                    ui.label(&warning.message);
                });
            }
        }
    }
}
