//! Reusable template editor widget with syntax highlighting and line numbers.

use egui::TextBuffer;

use crate::highlighting::highlight_template;
use crate::theme::syntax;
use promptgen_core::{ParseResult, Workspace};

/// Configuration for the template editor widget
#[derive(Clone)]
pub struct TemplateEditorConfig {
    /// Minimum number of lines to display (main editor: 5, slots: 3)
    pub min_lines: usize,
    /// Hint text to show when editor is empty
    pub hint_text: Option<String>,
    /// Whether to show line numbers (default: true)
    pub show_line_numbers: bool,
}

impl Default for TemplateEditorConfig {
    fn default() -> Self {
        Self {
            min_lines: 5,
            hint_text: None,
            show_line_numbers: true,
        }
    }
}

/// Response from the template editor widget
pub struct TemplateEditorResponse {
    /// The egui Response for the text edit widget
    pub response: egui::Response,
    /// The full rect of the editor layout (including line numbers)
    pub full_rect: egui::Rect,
    /// Parse result for the content (updated each frame)
    pub parse_result: ParseResult,
}

/// Reusable template editor widget with syntax highlighting and line numbers
pub struct TemplateEditor;

impl TemplateEditor {
    /// Show the editor widget
    ///
    /// Returns TemplateEditorResponse with the response and parse result
    pub fn show(
        ui: &mut egui::Ui,
        content: &mut String,
        workspace: &Workspace,
        config: &TemplateEditorConfig,
    ) -> TemplateEditorResponse {
        // Parse content for syntax highlighting
        let parse_result = workspace.parse_template(content);

        // Clone parse result for the layouter closure
        let parse_result_clone = parse_result.clone();

        // Create the text editor with custom syntax highlighting
        let mut layouter = |ui: &egui::Ui, text: &dyn TextBuffer, wrap_width: f32| {
            let text_str = text.as_str();
            let mut job = highlight_template(ui.ctx(), text_str, Some(&parse_result_clone));
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
            let mut text_edit = egui::TextEdit::multiline(content)
                .desired_width(f32::INFINITY)
                .desired_rows(desired_rows)
                .font(egui::TextStyle::Monospace)
                .layouter(&mut layouter);

            // Add hint text if provided
            if let Some(hint) = &config.hint_text {
                text_edit = text_edit.hint_text(hint.as_str());
            }

            ui.add(text_edit)
        });

        TemplateEditorResponse {
            response: layout_response.inner,
            full_rect: layout_response.response.rect,
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
