//! Editor panel component for template editing.

use egui::TextBuffer;

use crate::highlighting::highlight_template;
use crate::state::AppState;
use crate::theme::syntax;
use promptgen_core::ParseResult;

/// Editor panel for editing prompt templates.
pub struct EditorPanel;

impl EditorPanel {
    /// Render the editor panel.
    pub fn show(ui: &mut egui::Ui, state: &mut AppState) {
        ui.heading("Editor");
        ui.separator();

        // Clone parse result for the layouter closure
        let parse_result = state.parse_result.clone();

        // Create the text editor with custom syntax highlighting
        let mut layouter = |ui: &egui::Ui, text: &dyn TextBuffer, wrap_width: f32| {
            let text_str = text.as_str();
            let mut job = highlight_template(ui.ctx(), text_str, parse_result.as_ref());
            job.wrap.max_width = wrap_width;
            ui.ctx().fonts_mut(|f| f.layout_job(job))
        };

        // Calculate rows based on content, minimum 5 lines
        let line_count = state.editor_content.lines().count().max(1);
        let desired_rows = line_count.max(5);

        // Horizontal layout for line numbers + editor (no internal scroll)
        ui.horizontal_top(|ui| {
            // Line numbers column - match the number of lines in content (min 5)
            let line_numbers: String = (1..=desired_rows)
                .map(|n| format!("{:>4}", n))
                .collect::<Vec<_>>()
                .join("\n");

            ui.add(
                egui::TextEdit::multiline(&mut line_numbers.as_str())
                    .font(egui::TextStyle::Monospace)
                    .desired_width(40.0)
                    .frame(false)
                    .interactive(false)
                    .text_color(egui::Color32::from_rgb(108, 112, 134)), // Catppuccin overlay0
            );

            ui.add_space(4.0);

            // Main editor - auto-size to content
            let response = ui.add(
                egui::TextEdit::multiline(&mut state.editor_content)
                    .desired_width(f32::INFINITY)
                    .desired_rows(desired_rows)
                    .font(egui::TextStyle::Monospace)
                    .layouter(&mut layouter)
                    .hint_text(
                        "Enter your prompt template here...\n\n\
                         Use @GroupName to reference variables.\n\
                         Use {option1|option2|option3} for inline choices.\n\
                         Use {{ slot_name }} for user-filled slots.",
                    ),
            );

            // Update parse result when editor content changes
            if response.changed() {
                state.update_parse_result();

                // Auto-render if enabled
                if state.auto_render {
                    if state.auto_randomize_seed {
                        state.randomize_seed();
                    }
                    let _ = state.render_template();
                }
            }
        });

        // Error display below editor
        Self::show_errors(ui, &state.parse_result);
    }

    /// Display parse errors below the editor
    fn show_errors(ui: &mut egui::Ui, parse_result: &Option<ParseResult>) {
        if let Some(result) = parse_result {
            if !result.errors.is_empty() {
                ui.add_space(8.0);
                ui.separator();

                for error in &result.errors {
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
            if !result.warnings.is_empty() {
                ui.add_space(4.0);
                for warning in &result.warnings {
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
}
