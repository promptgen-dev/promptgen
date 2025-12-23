//! Editor panel component for prompt editing.

use crate::components::focusable_frame::FocusableFrame;
use crate::components::prompt_editor::{PromptEditor, PromptEditorConfig};
use crate::state::AppState;

/// The editor ID for the main prompt editor
const MAIN_EDITOR_ID: &str = "main_editor";

/// Editor panel for editing prompt prompts.
pub struct EditorPanel;

impl EditorPanel {
    /// Render the editor panel.
    pub fn show(ui: &mut egui::Ui, state: &mut AppState) {
        ui.heading("Editor");
        ui.separator();

        let config = PromptEditorConfig {
            id: MAIN_EDITOR_ID.to_string(),
            min_lines: 5,
            hint_text: Some(
                "Enter your prompt prompt here...\n\n\
                 Use @VariableName to reference variables.\n\
                 Use {option1|option2|option3} for inline choices.\n\
                 Use {{ slot_name }} for user-filled slots."
                    .to_string(),
            ),
            show_line_numbers: true,
        };

        let is_focused = state.is_main_editor_focused();

        // Clone content to avoid double mutable borrow
        let mut content = state.editor_content.clone();

        let frame_response = FocusableFrame::new(is_focused).show(ui, |ui| {
            PromptEditor::show(ui, &mut content, state, &config)
        });

        let result = frame_response.inner;

        // Update editor content if it changed
        if content != state.editor_content {
            state.editor_content = content;
        }

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
        PromptEditor::show_errors(ui, &result.parse_result);
    }
}
