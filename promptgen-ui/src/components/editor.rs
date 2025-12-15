//! Editor panel component for template editing.

use crate::state::AppState;

/// Editor panel for editing prompt templates.
pub struct EditorPanel;

impl EditorPanel {
    /// Render the editor panel.
    pub fn show(ui: &mut egui::Ui, state: &mut AppState) {
        ui.heading("Editor");
        ui.separator();

        let response = ui.add(
            egui::TextEdit::multiline(&mut state.editor_content)
                .desired_width(f32::INFINITY)
                .desired_rows(20)
                .font(egui::TextStyle::Monospace)
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
    }
}
