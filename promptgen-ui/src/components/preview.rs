//! Preview panel component for showing prompt validation and output.

use egui_material_icons::icons::{ICON_CASINO, ICON_CONTENT_COPY, ICON_PLAY_ARROW, ICON_VISIBILITY};

use crate::state::AppState;
use crate::styles::{spacing, Buttons, Components, Icons, Layout, Patterns, Typography};
use crate::theme;

/// Preview panel for showing prompt validation status and rendered output.
pub struct PreviewPanel;

impl PreviewPanel {
    /// Render the preview panel.
    pub fn show(ui: &mut egui::Ui, state: &mut AppState) {
        let theme = theme::current(ui.ctx());

        // Capture output before processing render
        let output_before = state.preview.output.clone();

        // Process any pending render requests from other components
        state.process_pending_render();

        // Auto-copy if enabled and output changed
        if state.preview.auto_copy
            && state.preview.output != output_before
            && !state.preview.output.is_empty()
        {
            ui.ctx().copy_text(state.preview.output.clone());
        }

        // Panel header
        ui.label(Typography::heading("Preview"));

        Layout::section_gap(ui);

        // Seed controls - collapsible for cleaner UI
        egui::CollapsingHeader::new(
            egui::RichText::new(format!("{} Seed", ICON_CASINO))
                .size(14.0)
                .color(theme.subtext0),
        )
        .default_open(false)
        .show(ui, |ui| {
            ui.horizontal(|ui| {
                // Convert seed to string for editing
                let mut seed_str = state
                    .preview
                    .seed
                    .map(|s| s.to_string())
                    .unwrap_or_default();

                let response = ui.add(
                    egui::TextEdit::singleline(&mut seed_str)
                        .desired_width(140.0)
                        .hint_text("random")
                        .font(egui::TextStyle::Monospace),
                );

                if response.changed() {
                    state.preview.seed = seed_str.parse().ok();
                }

                if ui
                    .add(Buttons::icon(ICON_CASINO))
                    .on_hover_text("Randomize seed")
                    .clicked()
                {
                    state.preview.randomize_seed();
                }
            });
        });

        Layout::gap(ui);

        // Options row - compact checkboxes
        ui.horizontal(|ui| {
            ui.checkbox(&mut state.preview.auto_render, "")
                .on_hover_text("Automatically render as you type");
            ui.label(Typography::body(format!("{} Live", ICON_VISIBILITY)));

            ui.add_space(spacing::LG);

            ui.checkbox(&mut state.preview.auto_randomize_seed, "")
                .on_hover_text("Generate new random seed on render");
            ui.label(Typography::body("Randomize"));

            ui.add_space(spacing::LG);

            ui.checkbox(&mut state.preview.auto_copy, "")
                .on_hover_text("Auto-copy output to clipboard");
            ui.label(Typography::body("Auto copy"));
        });

        Layout::gap(ui);

        // Action buttons - primary render, secondary copy
        let can_render = state
            .editor
            .parse_result
            .as_ref()
            .is_some_and(|r| r.errors.is_empty() && r.ast.is_some());

        ui.horizontal(|ui| {
            let render_btn = Buttons::primary(
                Icons::with_label(ICON_PLAY_ARROW, "Render"),
                &theme,
            );

            if ui
                .add_enabled(can_render, render_btn)
                .clicked()
            {
                if state.preview.auto_randomize_seed {
                    state.preview.randomize_seed();
                }
                if let Err(e) = state.render_prompt() {
                    state.preview.output = format!("Error: {}", e);
                } else if state.preview.auto_copy && !state.preview.output.is_empty() {
                    ui.ctx().copy_text(state.preview.output.clone());
                }
            }

            let copy_btn = Buttons::secondary(
                Icons::with_label(ICON_CONTENT_COPY, "Copy"),
                &theme,
            );

            if ui
                .add_enabled(!state.preview.output.is_empty(), copy_btn)
                .on_hover_text("Copy to clipboard")
                .clicked()
            {
                ui.ctx().copy_text(state.preview.output.clone());
            }
        });

        Layout::section_gap(ui);

        // Output section
        Layout::subtle_divider(ui, &theme);

        ui.label(Typography::section_title("Output"));
        Layout::small_gap(ui);

        // Output area with frame
        Components::input_frame(&theme).show(ui, |ui| {
            egui::ScrollArea::vertical()
                .auto_shrink([false, false])
                .max_height(ui.available_height() - spacing::MD)
                .show(ui, |ui| {
                    if state.preview.output.is_empty() {
                        Patterns::empty_state(
                            ui,
                            ICON_PLAY_ARROW,
                            "No output yet",
                            Some("Click Render to generate output"),
                        );
                    } else {
                        ui.add(
                            egui::TextEdit::multiline(&mut state.preview.output.as_str())
                                .desired_width(f32::INFINITY)
                                .font(egui::TextStyle::Monospace)
                                .frame(false),
                        );
                    }
                });
        });
    }
}
