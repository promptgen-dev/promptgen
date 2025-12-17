//! Slot panel component for editing template slots.

use promptgen_core::{Cardinality, SlotDefKind};

use crate::components::focusable_frame::FocusableFrame;
use crate::components::template_editor::{TemplateEditor, TemplateEditorConfig};
use crate::state::AppState;

/// Slot panel for editing template slot values.
pub struct SlotPanel;

impl SlotPanel {
    /// Render the slot panel.
    pub fn show(ui: &mut egui::Ui, state: &mut AppState) {
        let definitions = state.get_slot_definitions();

        if definitions.is_empty() {
            ui.label(
                egui::RichText::new("No slots in template")
                    .italics()
                    .color(egui::Color32::from_rgb(108, 112, 134)),
            );
            return;
        }

        // No internal scroll - parent handles scrolling
        for def in &definitions {
            let is_focused = state.is_slot_focused(&def.label);

            match &def.kind {
                SlotDefKind::Textarea => {
                    Self::show_textarea_slot(ui, state, &def.label, is_focused);
                }
                SlotDefKind::Pick {
                    cardinality, sep, ..
                } => {
                    Self::show_pick_slot(ui, state, &def.label, cardinality, sep, is_focused);
                }
            }

            ui.add_space(4.0);
        }
    }

    /// Render a textarea slot.
    fn show_textarea_slot(ui: &mut egui::Ui, state: &mut AppState, label: &str, is_focused: bool) {
        let label_owned = label.to_string();

        let frame_response = FocusableFrame::new(is_focused).show(ui, |ui| {
            ui.set_width(ui.available_width());

            ui.horizontal(|ui| {
                ui.label(egui::RichText::new(&label_owned).strong());
                ui.label(
                    egui::RichText::new("(text)")
                        .small()
                        .color(egui::Color32::from_rgb(108, 112, 134)),
                );
            });

            let config = TemplateEditorConfig {
                min_lines: 3,
                hint_text: Some("Enter text...".to_string()),
                show_line_numbers: true,
            };

            let mut value = state.get_textarea_value(&label_owned);
            let result = TemplateEditor::show(ui, &mut value, &state.workspace, &config);

            if result.response.changed() {
                state.set_textarea_value(&label_owned, value);
                state.request_render();
            }

            // Show parse errors below the editor
            TemplateEditor::show_errors(ui, &result.parse_result);

            result
        });

        let result = frame_response.inner;

        // Track focus - either from TextEdit gaining focus or clicking anywhere in frame
        if (result.response.has_focus() || frame_response.clicked) && !is_focused {
            state.focus_textarea_slot(label);
        }
    }

    /// Render a pick slot.
    /// Clicking anywhere in the slot row (except chip X buttons) opens the picker.
    fn show_pick_slot(
        ui: &mut egui::Ui,
        state: &mut AppState,
        label: &str,
        cardinality: &Cardinality,
        _sep: &str,
        is_focused: bool,
    ) {
        // Get the editor background color from the current theme
        let editor_bg = ui.visuals().extreme_bg_color;

        // Get current values
        let values = state.slot_values.get(label).cloned().unwrap_or_default();

        // For single-select, we can always open the picker to change selection
        // For multi-select, check if we're at max
        let can_open_picker = match cardinality {
            Cardinality::One => true, // Always allow opening to change selection
            Cardinality::Many { max: None } => true,
            Cardinality::Many { max: Some(n) } => values.len() < *n as usize,
        };

        // Track if chip X button was clicked (need to use Cell for interior mutability)
        let chip_removed = std::cell::Cell::new(false);

        let label_owned = label.to_string();
        let cardinality_clone = cardinality.clone();

        let frame_response = FocusableFrame::new(is_focused).show(ui, |ui| {
            ui.set_width(ui.available_width());

            // Header with label and cardinality info
            ui.horizontal(|ui| {
                ui.label(egui::RichText::new(&label_owned).strong());

                let cardinality_text = match &cardinality_clone {
                    Cardinality::One => "(single)",
                    Cardinality::Many { max: None } => "(multi)",
                    Cardinality::Many { max: Some(n) } => {
                        // Show count/max
                        let count = values.len();
                        ui.label(
                            egui::RichText::new(format!("{}/{}", count, n))
                                .small()
                                .color(egui::Color32::from_rgb(108, 112, 134)),
                        );
                        ""
                    }
                };

                if !cardinality_text.is_empty() {
                    ui.label(
                        egui::RichText::new(cardinality_text)
                            .small()
                            .color(egui::Color32::from_rgb(108, 112, 134)),
                    );
                }
            });

            // Display selected values as chips inside a dark background container
            if !values.is_empty() {
                // Container with editor background color - full width
                egui::Frame::NONE
                    .inner_margin(egui::Margin {
                        left: 8,
                        right: 8,
                        top: 6,
                        bottom: 6,
                    })
                    .corner_radius(4.0)
                    .fill(editor_bg)
                    .show(ui, |ui| {
                        ui.set_width(ui.available_width());
                        ui.horizontal_wrapped(|ui| {
                            let mut to_remove = None;

                            for value in &values {
                                // Chip with X button
                                ui.horizontal(|ui| {
                                    ui.spacing_mut().item_spacing.x = 4.0;

                                    egui::Frame::NONE
                                        .inner_margin(egui::Margin {
                                            left: 6,
                                            right: 6,
                                            top: 2,
                                            bottom: 2,
                                        })
                                        .corner_radius(12.0)
                                        .fill(egui::Color32::from_rgb(69, 71, 90)) // Catppuccin surface2
                                        .show(ui, |ui| {
                                            ui.label(value);
                                            if ui
                                                .small_button("x")
                                                .on_hover_text("Remove")
                                                .clicked()
                                            {
                                                to_remove = Some(value.clone());
                                                chip_removed.set(true);
                                            }
                                        });
                                });
                            }

                            if let Some(value) = to_remove {
                                state.remove_slot_value(&label_owned, &value);
                                state.request_render();
                            }
                        });
                    });
            } else {
                // Empty state - show placeholder in a clickable area
                egui::Frame::NONE
                    .inner_margin(egui::Margin {
                        left: 8,
                        right: 8,
                        top: 6,
                        bottom: 6,
                    })
                    .corner_radius(4.0)
                    .fill(editor_bg)
                    .show(ui, |ui| {
                        ui.set_width(ui.available_width());
                        ui.label(
                            egui::RichText::new("Click to select...")
                                .italics()
                                .color(egui::Color32::from_rgb(108, 112, 134)),
                        );
                    });
            }
        });

        // Focus slot when clicking anywhere in frame (except on chip X buttons)
        if frame_response.clicked && can_open_picker && !chip_removed.get() {
            state.focus_slot(label);
        }
    }
}
