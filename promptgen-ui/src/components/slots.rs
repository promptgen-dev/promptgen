//! Slot panel component for editing template slots.

use promptgen_core::{Cardinality, SlotDefKind};

use crate::state::AppState;

/// Slot panel for editing template slot values.
pub struct SlotPanel;

impl SlotPanel {
    /// Render the slot panel.
    /// Returns the label of a slot that was clicked (for focus handling).
    pub fn show(ui: &mut egui::Ui, state: &mut AppState) -> Option<String> {
        let definitions = state.get_slot_definitions();

        if definitions.is_empty() {
            ui.label(
                egui::RichText::new("No slots in template")
                    .italics()
                    .color(egui::Color32::from_rgb(108, 112, 134)),
            );
            return None;
        }

        let mut clicked_slot = None;

        egui::ScrollArea::vertical()
            .auto_shrink([false, false])
            .show(ui, |ui| {
                for def in &definitions {
                    let is_focused = state.focused_slot.as_ref() == Some(&def.label);

                    // Create a frame for each slot
                    let frame = egui::Frame::NONE
                        .inner_margin(8)
                        .corner_radius(4.0)
                        .fill(if is_focused {
                            egui::Color32::from_rgb(49, 50, 68) // Catppuccin surface1
                        } else {
                            egui::Color32::TRANSPARENT
                        });

                    frame.show(ui, |ui| {
                        ui.set_width(ui.available_width());

                        match &def.kind {
                            SlotDefKind::Textarea => {
                                Self::show_textarea_slot(ui, state, &def.label, is_focused);
                            }
                            SlotDefKind::Pick {
                                cardinality, sep, ..
                            } => {
                                if Self::show_pick_slot(
                                    ui,
                                    state,
                                    &def.label,
                                    cardinality,
                                    sep,
                                    is_focused,
                                ) {
                                    clicked_slot = Some(def.label.clone());
                                }
                            }
                        }
                    });

                    ui.add_space(4.0);
                }
            });

        clicked_slot
    }

    /// Render a textarea slot.
    fn show_textarea_slot(ui: &mut egui::Ui, state: &mut AppState, label: &str, is_focused: bool) {
        ui.horizontal(|ui| {
            ui.label(egui::RichText::new(label).strong());
            ui.label(
                egui::RichText::new("(text)")
                    .small()
                    .color(egui::Color32::from_rgb(108, 112, 134)),
            );
        });

        let mut value = state.get_textarea_value(label);
        let response = ui.add(
            egui::TextEdit::multiline(&mut value)
                .desired_width(f32::INFINITY)
                .desired_rows(3)
                .hint_text("Enter text...")
                .frame(true),
        );

        if response.changed() {
            state.set_textarea_value(label, value);
        }

        // Highlight if focused
        if is_focused {
            ui.painter().rect_stroke(
                response.rect.expand(2.0),
                4.0,
                egui::Stroke::new(2.0, egui::Color32::from_rgb(137, 180, 250)), // Catppuccin blue
                egui::StrokeKind::Outside,
            );
        }
    }

    /// Render a pick slot.
    /// Returns true if the slot was clicked to focus it.
    fn show_pick_slot(
        ui: &mut egui::Ui,
        state: &mut AppState,
        label: &str,
        cardinality: &Cardinality,
        _sep: &str,
        is_focused: bool,
    ) -> bool {
        let mut clicked = false;

        // Header with label and cardinality info
        ui.horizontal(|ui| {
            ui.label(egui::RichText::new(label).strong());

            let cardinality_text = match cardinality {
                Cardinality::One => "(single)",
                Cardinality::Many { max: None } => "(multi)",
                Cardinality::Many { max: Some(n) } => {
                    // Show count/max
                    let count = state
                        .slot_values
                        .get(label)
                        .map(|v| v.len())
                        .unwrap_or(0);
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

        // Get current values
        let values = state
            .slot_values
            .get(label)
            .cloned()
            .unwrap_or_default();

        // Display selected values as chips
        if !values.is_empty() {
            ui.horizontal_wrapped(|ui| {
                let mut to_remove = None;

                for value in &values {
                    // Chip with X button
                    let chip = ui
                        .horizontal(|ui| {
                            ui.spacing_mut().item_spacing.x = 4.0;

                            egui::Frame::NONE
                                .inner_margin(egui::Margin { left: 6, right: 6, top: 2, bottom: 2 })
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
                                    }
                                });
                        })
                        .response;

                    if chip.clicked() {
                        // Clicking the chip itself could do something in future
                    }
                }

                if let Some(value) = to_remove {
                    state.remove_slot_value(label, &value);
                }
            });
        } else {
            // Empty state
            ui.label(
                egui::RichText::new("No selections")
                    .italics()
                    .color(egui::Color32::from_rgb(108, 112, 134)),
            );
        }

        // Add button (opens sidebar picker)
        let can_add = match cardinality {
            Cardinality::One => values.is_empty(),
            Cardinality::Many { max: None } => true,
            Cardinality::Many { max: Some(n) } => values.len() < *n as usize,
        };

        if can_add {
            if ui.button("+ Add").clicked() {
                state.focus_slot(label);
                clicked = true;
            }
        }

        // Highlight if focused
        if is_focused {
            let rect = ui.min_rect();
            ui.painter().rect_stroke(
                rect.expand(2.0),
                4.0,
                egui::Stroke::new(2.0, egui::Color32::from_rgb(137, 180, 250)), // Catppuccin blue
                egui::StrokeKind::Outside,
            );
        }

        clicked
    }
}
