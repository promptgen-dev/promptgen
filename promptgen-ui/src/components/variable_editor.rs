//! Variable editor component for editing variable variables.

use egui::{Color32, FontId, RichText};

use egui_material_icons::icons::ICON_ARROW_BACK;

use crate::components::autocomplete::{
    autocomplete_after_editor, autocomplete_before_editor,
};
use crate::highlighting::highlight_prompt;
use crate::state::{AppState, ConfirmDialog};
use crate::theme;

/// The editor ID for the variable options editor
const VARIABLE_OPTIONS_EDITOR_ID: &str = "variable_options_editor";

/// Variable editor panel for editing variable variable names and options.
pub struct VariableEditorPanel;

impl VariableEditorPanel {
    /// Render the variable editor panel.
    /// Returns true if the editor should be closed (user confirmed exit).
    pub fn show(ui: &mut egui::Ui, state: &mut AppState) -> bool {
        let mut should_close = false;

        // Header bar
        ui.horizontal(|ui| {
            // Back button
            if ui
                .button(format!("{} Back to Editor", ICON_ARROW_BACK))
                .clicked()
                && !state.try_exit_variable_editor()
            {
                // Will show confirmation dialog
            }

            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                // Save button
                let can_save = state.validate_variable_name().is_none()
                    && !state.variable_editor_content.trim().is_empty();

                let save_button = ui.add_enabled(can_save, egui::Button::new("Save"));
                if save_button.clicked() && Self::save_variable(state) {
                    should_close = true;
                }

                // Delete button (only for existing variables)
                if let Some(original_name) = state.variable_editor_original_name.clone()
                    && ui
                        .button(RichText::new("Delete").color(theme::current(ui.ctx()).syntax_error()))
                        .clicked()
                    {
                        state.request_delete_variable(&original_name);
                    }

                // Dirty indicator
                if state.variable_editor_dirty {
                    ui.label(RichText::new("•").color(Color32::from_rgb(249, 226, 175))); // Yellow dot
                }
            });
        });

        ui.separator();

        // Variable name input
        ui.horizontal(|ui| {
            ui.label("Variable Name:");
            let name_response = ui.add(
                egui::TextEdit::singleline(&mut state.variable_editor_name)
                    .hint_text("Enter variable name...")
                    .desired_width(300.0),
            );
            if name_response.changed() {
                state.mark_variable_editor_dirty();
            }
        });

        // Show name validation error
        if let Some(error) = state.validate_variable_name() {
            ui.horizontal(|ui| {
                ui.label(RichText::new("error:").color(theme::current(ui.ctx()).syntax_error()));
                ui.label(error);
            });
        }

        ui.add_space(8.0);

        // Options section header
        ui.horizontal(|ui| {
            ui.label("Options:");
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                let count = state.get_variable_editor_option_count();
                ui.label(
                    RichText::new(format!(
                        "{} option{}",
                        count,
                        if count == 1 { "" } else { "s" }
                    ))
                    .small()
                    .color(Color32::from_rgb(108, 112, 134)),
                );
            });
        });

        // Options textarea with syntax highlighting and autocomplete
        Self::show_options_editor(ui, state);

        // Show option parse errors
        Self::show_option_errors(ui, state);

        ui.add_space(16.0);

        // Handle confirmation dialogs
        Self::show_confirmation_dialogs(ui, state, &mut should_close);

        should_close
    }

    /// Render the options editor with syntax highlighting, option-based line numbers, and autocomplete
    fn show_options_editor(ui: &mut egui::Ui, state: &mut AppState) {
        let editor_bg = ui.visuals().extreme_bg_color;
        let ctx = ui.ctx().clone();
        let editor_id = VARIABLE_OPTIONS_EDITOR_ID;

        // Take pending cursor position (will be cleared after use)
        let pending_cursor_position = state.take_pending_cursor_position(editor_id);

        // Clone content to avoid double mutable borrow
        let mut content = state.variable_editor_content.clone();

        // Handle autocomplete keyboard input BEFORE the text editor processes input
        if let Some(new_content) = autocomplete_before_editor(ui, state, editor_id, &content) {
            content = new_content;
            state.mark_variable_editor_dirty();
        }

        egui::Frame::NONE
            .fill(editor_bg)
            .inner_margin(8.0)
            .corner_radius(4.0)
            .show(ui, |ui| {
                ui.set_width(ui.available_width());

                // Calculate option numbers for each line
                let option_numbers = Self::calculate_option_numbers(&content);
                let line_count = content.lines().count().max(5);

                // Calculate option number column width
                let max_option_num =
                    option_numbers.iter().filter_map(|n| *n).max().unwrap_or(1);
                let max_digits = max_option_num.to_string().len();
                let number_width = (max_digits as f32) * 8.0 + 12.0;

                ui.horizontal(|ui| {
                    // Reserve space for option numbers (we'll paint them after getting the galley)
                    let (number_rect, _) =
                        ui.allocate_exact_size(egui::vec2(number_width, 0.0), egui::Sense::hover());

                    ui.add_space(4.0);

                    // Main editor with syntax highlighting
                    let mut layouter =
                        |ui: &egui::Ui, text: &dyn egui::TextBuffer, wrap_width: f32| {
                            // Highlight each option segment independently
                            let mut job = Self::highlight_options_text(&ctx, text.as_str());
                            job.wrap.max_width = wrap_width;
                            ui.ctx().fonts_mut(|f| f.layout_job(job))
                        };

                    let text_edit_id = ui.make_persistent_id(editor_id);
                    // Use show() to get full TextEditOutput including galley for cursor positioning
                    let output = egui::TextEdit::multiline(&mut content)
                        .id(text_edit_id)
                        .font(egui::TextStyle::Monospace)
                        .desired_width(f32::INFINITY)
                        .desired_rows(line_count)
                        .layouter(&mut layouter)
                        .show(ui);

                    let response = &output.response;
                    let galley = &output.galley;
                    let galley_pos = output.galley_pos;

                    // Paint option numbers at correct Y positions using the galley
                    Self::paint_option_numbers(
                        ui,
                        galley,
                        galley_pos,
                        number_rect,
                        &content,
                        &option_numbers,
                        max_digits,
                    );

                    // Apply pending cursor position if set
                    if let Some(cursor_pos) = pending_cursor_position
                        && let Some(mut text_state) =
                            egui::TextEdit::load_state(ui.ctx(), text_edit_id)
                    {
                        let ccursor = egui::text::CCursor::new(cursor_pos);
                        text_state
                            .cursor
                            .set_char_range(Some(egui::text::CCursorRange::one(ccursor)));
                        text_state.store(ui.ctx(), text_edit_id);
                        response.request_focus();
                    }

                    // Get cursor position (character index) for autocomplete query
                    // cursor.primary is a CCursor which has an index field directly
                    let cursor_pos = output
                        .cursor_range
                        .map(|range| range.primary.index)
                        .unwrap_or(content.len());

                    // Get cursor screen position for popup positioning
                    let cursor_screen_pos = Self::get_cursor_screen_pos(&output);

                    // Handle autocomplete activation, popup display, and focus loss
                    if let Some(new_content) = autocomplete_after_editor(
                        ui,
                        state,
                        editor_id,
                        &content,
                        response,
                        cursor_pos,
                        cursor_screen_pos,
                    ) {
                        content = new_content;
                        state.mark_variable_editor_dirty();
                    }

                    if response.changed() {
                        state.mark_variable_editor_dirty();
                    }
                });
            });

        // Update state content if it changed
        if content != state.variable_editor_content {
            state.variable_editor_content = content;
        }
    }

    /// Calculate option numbers for each line (None for delimiter lines)
    ///
    /// Format:
    /// - Each non-empty line outside of `---` blocks is a separate option
    /// - `---` marks start/end of a multiline option block
    /// - Lines inside a multiline block share the same option number
    fn calculate_option_numbers(text: &str) -> Vec<Option<usize>> {
        let mut numbers = Vec::new();
        let mut current_option = 1;
        let mut in_multiline = false;

        for line in text.lines() {
            if line.trim() == "---" {
                numbers.push(None); // Delimiter line - no number
                if in_multiline {
                    // Closing a multiline block - next line starts new option
                    in_multiline = false;
                    current_option += 1;
                } else {
                    // Opening a multiline block
                    in_multiline = true;
                }
            } else if in_multiline {
                // Inside multiline block - same option number
                numbers.push(Some(current_option));
            } else {
                // Single-line option (only count non-empty lines)
                if line.trim().is_empty() {
                    numbers.push(None); // Empty line between options
                } else {
                    numbers.push(Some(current_option));
                    current_option += 1;
                }
            }
        }

        // Ensure at least 5 lines for display
        let last_option = if numbers.is_empty() {
            1
        } else {
            current_option
        };
        while numbers.len() < 5 {
            numbers.push(Some(last_option));
        }

        numbers
    }

    /// Paint option numbers at the Y position of each logical line's first visual row.
    /// This correctly handles text wrapping - option numbers only appear at the start of each logical line.
    fn paint_option_numbers(
        ui: &egui::Ui,
        galley: &std::sync::Arc<egui::Galley>,
        galley_pos: egui::Pos2,
        number_rect: egui::Rect,
        content: &str,
        option_numbers: &[Option<usize>],
        max_digits: usize,
    ) {
        let painter = ui.painter();
        let font_id = FontId::monospace(14.0);
        let number_color = Color32::from_rgb(108, 112, 134); // Catppuccin overlay0

        // Track which logical line we're on
        let mut logical_line = 0;
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
                // Get the option number for this logical line (if any)
                if let Some(Some(num)) = option_numbers.get(logical_line) {
                    // Format option number right-aligned
                    let num_str = format!("{:>width$}", num, width = max_digits);

                    // Calculate Y position from the row rect, translated to screen coordinates
                    let row_y = galley_pos.y + row.rect().top();

                    // Paint the option number right-aligned within the number column
                    let text_pos = egui::pos2(
                        number_rect.right() - 4.0, // Small margin from right edge
                        row_y,
                    );

                    painter.text(
                        text_pos,
                        egui::Align2::RIGHT_TOP,
                        num_str,
                        font_id.clone(),
                        number_color,
                    );
                }
                // If option_numbers[logical_line] is None, we don't paint anything (delimiter lines)

                logical_line += 1;
            }

            // Advance char_index by the number of characters in this row
            char_index += row.char_count_including_newline();
        }
    }

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

    /// Create a LayoutJob with syntax highlighting for options text
    fn highlight_options_text(ctx: &egui::Context, text: &str) -> egui::text::LayoutJob {
        use egui::FontId;
        use egui::text::{LayoutJob, TextFormat};

        let mut job = LayoutJob::default();
        let font_id = FontId::monospace(14.0);

        // Color for delimiter
        let delimiter_color = Color32::from_rgb(108, 112, 134); // Subdued gray

        for line in text.split_inclusive('\n') {
            let line_trimmed = line.trim_end_matches('\n');

            if line_trimmed.trim() == "---" {
                // Render delimiter in subdued color
                job.append(
                    line,
                    0.0,
                    TextFormat {
                        font_id: font_id.clone(),
                        color: delimiter_color,
                        ..Default::default()
                    },
                );
            } else {
                // Highlight this line as prompt syntax (no parse result, use fallback)
                let line_job = highlight_prompt(ctx, line_trimmed, None);

                // Append each section from the highlighted job
                for section in &line_job.sections {
                    let section_text = &line_job.text[section.byte_range.clone()];
                    job.append(section_text, 0.0, section.format.clone());
                }

                // Add newline if present
                if line.ends_with('\n') {
                    job.append(
                        "\n",
                        0.0,
                        TextFormat {
                            font_id: font_id.clone(),
                            ..Default::default()
                        },
                    );
                }
            }
        }

        job
    }

    /// Show parse errors for individual options
    fn show_option_errors(ui: &mut egui::Ui, state: &AppState) {
        let options = AppState::parse_options(&state.variable_editor_content);

        let mut errors = Vec::new();
        for (idx, option) in options.iter().enumerate() {
            let parse_result = state.library.parse_prompt(option);
            for error in &parse_result.errors {
                errors.push((idx + 1, error.message.clone()));
            }
        }

        if !errors.is_empty() {
            ui.add_space(8.0);
            for (option_num, error_msg) in errors {
                ui.horizontal(|ui| {
                    ui.label(RichText::new("error:").color(theme::current(ui.ctx()).syntax_error()));
                    ui.label(format!("Option {}: {}", option_num, error_msg));
                });
            }
        }
    }

    /// Show confirmation dialogs
    fn show_confirmation_dialogs(ui: &mut egui::Ui, state: &mut AppState, should_close: &mut bool) {
        let dialog = state.confirm_dialog.clone();

        if let Some(dialog) = dialog {
            egui::Window::new("Confirm")
                .collapsible(false)
                .resizable(false)
                .anchor(egui::Align2::CENTER_CENTER, [0.0, 0.0])
                .show(ui.ctx(), |ui| match dialog {
                    ConfirmDialog::DiscardVariableChanges => {
                        ui.label("You have unsaved changes. Discard them?");
                        ui.add_space(8.0);
                        ui.horizontal(|ui| {
                            if ui.button("Discard").clicked() {
                                state.exit_variable_editor_force();
                                *should_close = true;
                            }
                            if ui.button("Cancel").clicked() {
                                state.cancel_confirm_dialog();
                            }
                        });
                    }
                    ConfirmDialog::DeleteVariable { variable_name } => {
                        ui.label(format!("Delete @{}? This cannot be undone.", variable_name));
                        ui.add_space(8.0);
                        ui.horizontal(|ui| {
                            if ui
                                .button(RichText::new("Delete").color(theme::current(ui.ctx()).syntax_error()))
                                .clicked()
                            {
                                Self::delete_variable(state, &variable_name);
                                *should_close = true;
                            }
                            if ui.button("Cancel").clicked() {
                                state.cancel_confirm_dialog();
                            }
                        });
                    }
                    // CloseUnsavedTab, DeletePrompt, and OpenNewLibrary are handled in app.rs, not here
                    ConfirmDialog::CloseUnsavedTab { .. }
                    | ConfirmDialog::DeletePrompt { .. }
                    | ConfirmDialog::OpenNewLibrary { .. } => {}
                });
        }
    }

    /// Save the current variable to the library
    fn save_variable(state: &mut AppState) -> bool {
        let name = state.variable_editor_name.trim().to_string();
        let options = AppState::parse_options(&state.variable_editor_content);

        if name.is_empty() || options.is_empty() {
            return false;
        }

        // Update the library
        if let Some(original_name) = &state.variable_editor_original_name {
            // Editing existing variable - find and update it
            if let Some(variable) = state
                .library
                .variables
                .iter_mut()
                .find(|g| g.name == *original_name)
            {
                variable.name = name;
                variable.options = options;
            }
        } else {
            // Creating new variable
            state
                .library
                .variables
                .push(promptgen_core::PromptVariable::new(name, options));
        }

        // Save to disk
        #[cfg(not(target_arch = "wasm32"))]
        {
            if let Some(path) = &state.library_path
                && let Err(e) = promptgen_core::save_library(&state.library, path)
            {
                log::error!("Failed to save library: {}", e);
                // Still continue - the in-memory state is updated
            }
        }

        // Clear editor state
        state.exit_variable_editor_force();

        true
    }

    /// Delete a variable from the library
    fn delete_variable(state: &mut AppState, variable_name: &str) {
        // Remove the variable
        state.library.variables.retain(|g| g.name != variable_name);

        // Save to disk
        #[cfg(not(target_arch = "wasm32"))]
        {
            if let Some(path) = &state.library_path
                && let Err(e) = promptgen_core::save_library(&state.library, path)
            {
                log::error!("Failed to save library after delete: {}", e);
            }
        }

        // Clear editor state
        state.exit_variable_editor_force();
    }
}
