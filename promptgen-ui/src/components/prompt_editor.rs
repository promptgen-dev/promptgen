//! Reusable template editor widget with syntax highlighting, line numbers, and autocomplete.

use egui::{FontId, TextBuffer};

use crate::components::autocomplete::{autocomplete_after_editor, autocomplete_before_editor};
use crate::highlighting::highlight_prompt;
use crate::state::AppState;
use crate::theme;
use promptgen_core::ParseResult;

/// Calculate the cursor screen position from a TextEditOutput.
/// Returns the position where the autocomplete popup should appear.
fn get_cursor_screen_pos(output: &egui::text_edit::TextEditOutput) -> Option<egui::Pos2> {
    // Get the cursor range from the output
    let cursor = output.cursor_range?;
    // Use the primary cursor position (where the caret is)
    // cursor.primary is already a CCursor
    let ccursor = cursor.primary;

    // Get the position from the galley (relative to galley origin)
    // pos_from_cursor returns a Rect representing the cursor position
    let cursor_rect_in_galley = output.galley.pos_from_cursor(ccursor);

    // Translate to screen coordinates using the galley position
    // We want the bottom-left corner for popup positioning (below the cursor line)
    let screen_pos = cursor_rect_in_galley
        .translate(output.galley_pos.to_vec2())
        .left_bottom();

    Some(screen_pos)
}

/// Paint line numbers at the Y position of each logical line's first visual row.
/// This correctly handles text wrapping - line numbers only appear at the start of each logical line.
fn paint_line_numbers(
    ui: &egui::Ui,
    galley: &std::sync::Arc<egui::Galley>,
    galley_pos: egui::Pos2,
    line_number_rect: egui::Rect,
    content: &str,
) {
    let painter = ui.painter();
    let font_id = FontId::monospace(14.0);
    let line_number_color = theme::current(ui.ctx()).muted();

    // Count logical lines to determine max digits needed
    let logical_line_count = content.lines().count().max(1);
    let max_digits = logical_line_count.to_string().len();

    // Track which logical line we're on
    let mut logical_line = 1;
    let mut char_index: usize = 0;

    for row in galley.rows.iter() {
        // Check if this row starts a new logical line
        // A row starts a new logical line if:
        // 1. It's the first row (char_index == 0), or
        // 2. The previous character was a newline
        let is_new_logical_line = char_index == 0
            || content
                .get(..char_index)
                .and_then(|s| s.chars().last())
                .is_some_and(|c| c == '\n');

        if is_new_logical_line {
            // Format line number right-aligned
            let line_num_str = format!("{:>width$}", logical_line, width = max_digits);

            // Calculate Y position from the row rect, translated to screen coordinates
            let row_y = galley_pos.y + row.rect().top();

            // Paint the line number right-aligned within the line number column
            let text_pos = egui::pos2(
                line_number_rect.right() - 4.0, // Small margin from right edge
                row_y,
            );

            painter.text(
                text_pos,
                egui::Align2::RIGHT_TOP,
                line_num_str,
                font_id.clone(),
                line_number_color,
            );

            logical_line += 1;
        }

        // Advance char_index by the number of characters in this row
        char_index += row.char_count_including_newline();
    }
}

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

        // Handle autocomplete keyboard input BEFORE the text editor processes input
        if let Some(new_content) = autocomplete_before_editor(ui, state, editor_id, content) {
            *content = new_content;
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

        // Calculate line number column width
        let logical_line_count = content.lines().count().max(1);
        let max_digits = logical_line_count.max(config.min_lines).to_string().len();
        let line_number_width = if config.show_line_numbers {
            (max_digits as f32) * 8.0 + 8.0 // ~8px per digit + margin
        } else {
            0.0
        };

        // Horizontal layout for line numbers + editor
        let layout_response = ui.horizontal_top(|ui| {
            // Reserve space for line numbers (we'll paint them after getting the galley)
            let line_number_rect = if config.show_line_numbers {
                let (rect, _) =
                    ui.allocate_exact_size(egui::vec2(line_number_width, 0.0), egui::Sense::hover());
                Some(rect)
            } else {
                None
            };

            if config.show_line_numbers {
                ui.add_space(4.0);
            }

            // Main editor - use show() instead of add() to get TextEditOutput with galley
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

            // Use show() to get full TextEditOutput including galley for cursor positioning
            let output = text_edit.show(ui);

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
                output.response.request_focus();
            }

            // Get cursor position (character index) for autocomplete query
            // cursor.primary is a CCursor which has an index field directly
            let cursor_char_pos = output
                .cursor_range
                .map(|range| range.primary.index)
                .unwrap_or(content.len());

            // Get cursor screen position for popup positioning
            let cursor_screen_pos = get_cursor_screen_pos(&output);

            (
                output.response,
                cursor_char_pos,
                cursor_screen_pos,
                line_number_rect,
                output.galley,
                output.galley_pos,
            )
        });

        let response = layout_response.inner.0;
        let cursor_pos = layout_response.inner.1;
        let cursor_screen_pos = layout_response.inner.2;
        let line_number_rect = layout_response.inner.3;
        let galley = layout_response.inner.4;
        let galley_pos = layout_response.inner.5;

        // Paint line numbers at correct Y positions using the galley
        if let Some(line_number_rect) = line_number_rect {
            // Extend the line number rect to match the galley height
            let extended_rect = egui::Rect::from_min_size(
                egui::pos2(line_number_rect.left(), galley_pos.y),
                egui::vec2(line_number_rect.width(), galley.rect.height()),
            );
            paint_line_numbers(ui, &galley, galley_pos, extended_rect, content);
        }

        // Handle autocomplete activation, popup display, and focus loss
        if let Some(new_content) = autocomplete_after_editor(
            ui,
            state,
            editor_id,
            content,
            &response,
            cursor_pos,
            cursor_screen_pos,
        )
        {
            *content = new_content;
        }

        PromptEditorResponse {
            response,
            parse_result,
        }
    }

    /// Show parse errors below the editor (call after show())
    pub fn show_errors(ui: &mut egui::Ui, parse_result: &ParseResult) {
        let theme = theme::current(ui.ctx());

        if !parse_result.errors.is_empty() {
            ui.add_space(8.0);
            ui.separator();

            for error in &parse_result.errors {
                ui.horizontal(|ui| {
                    ui.colored_label(theme.syntax_error(), "error:");
                    ui.label(&error.message);
                });

                // Show span info
                let span = &error.span;
                if !span.is_empty() {
                    ui.horizontal(|ui| {
                        ui.add_space(20.0);
                        ui.colored_label(
                            theme.muted(),
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
                    ui.colored_label(theme.accent_five, "warning:");
                    ui.label(&warning.message);
                });
            }
        }
    }
}
