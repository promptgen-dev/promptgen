//! Variable editor component for editing variable variables.

use egui::{Color32, RichText, Vec2};

use egui_material_icons::icons::ICON_ARROW_BACK;

use crate::highlighting::highlight_template;
use crate::state::{AppState, ConfirmDialog};
use crate::theme::syntax;

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
            {
                if !state.try_exit_variable_editor() {
                    // Will show confirmation dialog
                }
            }

            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                // Save button
                let can_save = state.validate_variable_name().is_none()
                    && !state.variable_editor_content.trim().is_empty();

                let save_button = ui.add_enabled(can_save, egui::Button::new("Save"));
                if save_button.clicked() {
                    if Self::save_variable(state) {
                        should_close = true;
                    }
                }

                // Variable name display
                let variable_display_name = if state.variable_editor_name.is_empty() {
                    "New Variable".to_string()
                } else {
                    format!("@{}", state.variable_editor_name)
                };
                ui.heading(variable_display_name);

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
                ui.label(RichText::new("error:").color(syntax::ERROR));
                ui.label(error);
            });
        }

        ui.add_space(8.0);

        // Options section header
        ui.horizontal(|ui| {
            ui.label("Options (separate with ---):");
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

        // Options textarea with syntax highlighting
        Self::show_options_editor(ui, state);

        // Show option parse errors
        Self::show_option_errors(ui, state);

        ui.add_space(16.0);

        // Delete button (only for existing variables)
        if let Some(original_name) = state.variable_editor_original_name.clone() {
            ui.separator();
            ui.add_space(8.0);

            if ui
                .button(RichText::new("Delete Variable").color(syntax::ERROR))
                .clicked()
            {
                state.request_delete_variable(&original_name);
            }
        }

        // Handle confirmation dialogs
        Self::show_confirmation_dialogs(ui, state, &mut should_close);

        should_close
    }

    /// Render the options editor with syntax highlighting and option-based line numbers
    fn show_options_editor(ui: &mut egui::Ui, state: &mut AppState) {
        let editor_bg = ui.visuals().extreme_bg_color;
        let ctx = ui.ctx().clone();

        egui::Frame::NONE
            .fill(editor_bg)
            .inner_margin(8.0)
            .corner_radius(4.0)
            .show(ui, |ui| {
                ui.set_width(ui.available_width());

                // Calculate option numbers for each line
                let option_numbers = Self::calculate_option_numbers(&state.variable_editor_content);
                let line_count = state.variable_editor_content.lines().count().max(5);

                ui.horizontal(|ui| {
                    // Option numbers column
                    let max_option_num = option_numbers.iter().filter_map(|n| *n).max().unwrap_or(1);
                    let max_digits = max_option_num.to_string().len();
                    let number_width = (max_digits as f32) * 8.0 + 12.0;

                    ui.allocate_ui(Vec2::new(number_width, 0.0), |ui| {
                        let numbers_text: String = option_numbers
                            .iter()
                            .take(line_count.max(option_numbers.len()))
                            .map(|n| match n {
                                Some(num) => format!("{:>width$}", num, width = max_digits),
                                None => " ".repeat(max_digits), // Blank for delimiter lines
                            })
                            .collect::<Vec<_>>()
                            .join("\n");

                        ui.add(
                            egui::TextEdit::multiline(&mut numbers_text.as_str())
                                .font(egui::TextStyle::Monospace)
                                .interactive(false)
                                .desired_width(number_width)
                                .frame(false)
                                .text_color(Color32::from_rgb(108, 112, 134)),
                        );
                    });

                    // Main editor with syntax highlighting
                    let mut layouter =
                        |ui: &egui::Ui, text: &dyn egui::TextBuffer, wrap_width: f32| {
                            // Highlight each option segment independently
                            let mut job = Self::highlight_options_text(&ctx, text.as_str());
                            job.wrap.max_width = wrap_width;
                            ui.ctx().fonts_mut(|f| f.layout_job(job))
                        };

                    let response = ui.add(
                        egui::TextEdit::multiline(&mut state.variable_editor_content)
                            .font(egui::TextStyle::Monospace)
                            .desired_width(f32::INFINITY)
                            .desired_rows(line_count)
                            .layouter(&mut layouter),
                    );

                    if response.changed() {
                        state.mark_variable_editor_dirty();
                    }
                });
            });
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

    /// Create a LayoutJob with syntax highlighting for options text
    fn highlight_options_text(ctx: &egui::Context, text: &str) -> egui::text::LayoutJob {
        use egui::text::{LayoutJob, TextFormat};
        use egui::FontId;

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
                // Highlight this line as template syntax (no parse result, use fallback)
                let line_job = highlight_template(ctx, line_trimmed, None);

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
            let parse_result = state.workspace.parse_template(option);
            for error in &parse_result.errors {
                errors.push((idx + 1, error.message.clone()));
            }
        }

        if !errors.is_empty() {
            ui.add_space(8.0);
            for (option_num, error_msg) in errors {
                ui.horizontal(|ui| {
                    ui.label(RichText::new("error:").color(syntax::ERROR));
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
                        ui.label(format!(
                            "Delete @{}? This cannot be undone.",
                            variable_name
                        ));
                        ui.add_space(8.0);
                        ui.horizontal(|ui| {
                            if ui
                                .button(RichText::new("Delete").color(syntax::ERROR))
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

        // Get the library ID to update
        let library_id = match &state.selected_library_id {
            Some(id) => id.clone(),
            None => return false,
        };

        // Find and update the library
        if let Some(library) = state.libraries.iter_mut().find(|lib| lib.id == library_id) {
            if let Some(original_name) = &state.variable_editor_original_name {
                // Editing existing variable - find and update it
                if let Some(variable) = library.variables.iter_mut().find(|g| g.name == *original_name) {
                    variable.name = name;
                    variable.options = options;
                }
            } else {
                // Creating new variable
                library.variables.push(promptgen_core::PromptVariable::new(name, options));
            }
        }

        // Save to disk
        #[cfg(not(target_arch = "wasm32"))]
        {
            if let Some(path) = state.library_paths.get(&library_id) {
                if let Some(library) = state.libraries.iter().find(|lib| lib.id == library_id) {
                    if let Err(e) = promptgen_core::save_library(library, path) {
                        log::error!("Failed to save library: {}", e);
                        // Still continue - the in-memory state is updated
                    }
                }
            }
        }

        // Rebuild workspace to pick up changes
        state.rebuild_workspace();

        // Clear editor state
        state.exit_variable_editor_force();

        true
    }

    /// Delete a variable from the library
    fn delete_variable(state: &mut AppState, variable_name: &str) {
        let library_id = match &state.selected_library_id {
            Some(id) => id.clone(),
            None => return,
        };

        // Find and remove the variable
        if let Some(library) = state.libraries.iter_mut().find(|lib| lib.id == library_id) {
            library.variables.retain(|g| g.name != variable_name);
        }

        // Save to disk
        #[cfg(not(target_arch = "wasm32"))]
        {
            if let Some(path) = state.library_paths.get(&library_id) {
                if let Some(library) = state.libraries.iter().find(|lib| lib.id == library_id) {
                    if let Err(e) = promptgen_core::save_library(library, path) {
                        log::error!("Failed to save library after delete: {}", e);
                    }
                }
            }
        }

        // Rebuild workspace
        state.rebuild_workspace();

        // Clear editor state
        state.exit_variable_editor_force();
    }
}
