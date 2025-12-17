//! Editor panel component for template editing.

use crate::components::template_editor::{TemplateEditor, TemplateEditorConfig};
use crate::state::AppState;

/// Editor panel for editing prompt templates.
pub struct EditorPanel;

impl EditorPanel {
    /// Render the editor panel.
    pub fn show(ui: &mut egui::Ui, state: &mut AppState) {
        ui.heading("Editor");
        ui.separator();

        let config = TemplateEditorConfig {
            min_lines: 5,
            hint_text: Some(
                "Enter your prompt template here...\n\n\
                 Use @GroupName to reference variables.\n\
                 Use {option1|option2|option3} for inline choices.\n\
                 Use {{ slot_name }} for user-filled slots."
                    .to_string(),
            ),
            show_line_numbers: true,
        };

        // Use frame for focus highlight background
        let is_focused = state.is_main_editor_focused();
        let frame = egui::Frame::NONE
            .inner_margin(8.0)
            .corner_radius(4.0)
            .fill(if is_focused {
                egui::Color32::from_rgb(49, 50, 68) // Catppuccin surface1
            } else {
                egui::Color32::TRANSPARENT
            });

        let result = frame
            .show(ui, |ui| {
                TemplateEditor::show(ui, &mut state.editor_content, &state.workspace, &config)
            })
            .inner;

        // Update parse result when editor content changes
        if result.response.changed() {
            state.parse_result = Some(result.parse_result.clone());
            state.update_parse_result();
            state.request_render();
        }

        // Track focus on main editor - unfocus pick slots when editor gains focus
        if result.response.has_focus() && !state.is_main_editor_focused() {
            state.focus_main_editor();
        }

        // Error display below editor
        TemplateEditor::show_errors(ui, &result.parse_result);
    }
}
