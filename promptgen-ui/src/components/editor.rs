//! Editor panel component for template editing.

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

        let config = TemplateEditorConfig {
            min_lines: 5,
            hint_text: Some(
                "Enter your prompt template here...\n\n\
                 Use @VariableName to reference variables.\n\
                 Use {option1|option2|option3} for inline choices.\n\
                 Use {{ slot_name }} for user-filled slots."
                    .to_string(),
            ),
            show_line_numbers: true,
        };

        let is_focused = state.is_main_editor_focused();

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

        // Track focus - either from TextEdit gaining focus or clicking anywhere in frame
        if (result.response.has_focus() || frame_response.clicked) && !is_focused {
            state.focus_main_editor();
        }

        // Error display below editor
        TemplateEditor::show_errors(ui, &result.parse_result);
    }
}
