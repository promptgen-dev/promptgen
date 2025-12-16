//! Preview panel component for showing template validation and output.

use crate::state::AppState;

/// Preview panel for showing template validation status and rendered output.
pub struct PreviewPanel;

impl PreviewPanel {
    /// Render the preview panel.
    pub fn show(ui: &mut egui::Ui, state: &mut AppState) {
        ui.heading("Preview");
        ui.separator();

        // Seed controls - always visible
        ui.horizontal(|ui| {
            ui.label("Seed:");

            // Convert seed to string for editing
            let mut seed_str = state
                .preview_seed
                .map(|s| s.to_string())
                .unwrap_or_default();

            let response = ui.add(
                egui::TextEdit::singleline(&mut seed_str)
                    .desired_width(120.0)
                    .hint_text("random"),
            );

            if response.changed() {
                // Parse seed when changed
                state.preview_seed = seed_str.parse().ok();
            }

            if ui.button("🎲").on_hover_text("Random seed").clicked() {
                state.randomize_seed();
            }
        });

        ui.add_space(8.0);

        // Auto-render checkbox
        ui.checkbox(&mut state.auto_render, "Live preview")
            .on_hover_text("Automatically render as you type");

        // Auto-randomize seed checkbox
        ui.checkbox(&mut state.auto_randomize_seed, "Randomize seed")
            .on_hover_text("Generate a new random seed on edit and when clicking Render");

        ui.add_space(8.0);

        // Render and copy buttons - always visible
        let can_render = state
            .parse_result
            .as_ref()
            .is_some_and(|r| r.errors.is_empty() && r.ast.is_some());

        ui.horizontal(|ui| {
            if ui
                .add_enabled(can_render, egui::Button::new("▶ Render"))
                .clicked()
            {
                // Randomize seed first if enabled
                if state.auto_randomize_seed {
                    state.randomize_seed();
                }
                if let Err(e) = state.render_template() {
                    state.preview_output = format!("Error: {}", e);
                }
            }

            // Copy button - always visible but disabled if no output
            if ui
                .add_enabled(
                    !state.preview_output.is_empty(),
                    egui::Button::new("📋 Copy"),
                )
                .on_hover_text("Copy to clipboard")
                .clicked()
            {
                ui.ctx().copy_text(state.preview_output.clone());
            }
        });

        ui.add_space(8.0);

        // Preview output
        ui.separator();
        ui.label("Output:");
        egui::ScrollArea::vertical()
            .max_height(300.0)
            .show(ui, |ui| {
                if state.preview_output.is_empty() {
                    ui.label(
                        egui::RichText::new("Click 'Render' to generate output")
                            .italics()
                            .color(egui::Color32::from_rgb(108, 112, 134)), // Catppuccin overlay0
                    );
                } else {
                    ui.add(
                        egui::TextEdit::multiline(&mut state.preview_output.as_str())
                            .desired_width(f32::INFINITY)
                            .font(egui::TextStyle::Monospace),
                    );
                }
            });
    }
}
