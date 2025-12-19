//! Slot panel component for editing template slots.

use egui::{Align, Id, Label, Layout, UiBuilder, Vec2};
use egui_dnd::dnd;
use promptgen_core::{Cardinality, Node, ParseResult, SlotDefKind};

use crate::components::autocomplete::{
    apply_completion, get_completions, handle_autocomplete_keyboard,
};
use crate::components::focusable_frame::FocusableFrame;
use crate::components::template_editor::{TemplateEditor, TemplateEditorConfig};
use crate::state::AppState;
use crate::theme::syntax;

/// Measure text size in the UI (based on hello_egui_utils::measure_text)
fn measure_text(ui: &mut egui::Ui, text: impl Into<egui::WidgetText>) -> Vec2 {
    let res = Label::new(text).layout_in_ui(
        &mut ui.new_child(
            UiBuilder::new()
                .max_rect(ui.available_rect_before_wrap())
                .layout(Layout::left_to_right(Align::Center)),
        ),
    );
    // Add small padding to avoid rounding errors
    res.2.rect.size() + Vec2::new(0.1, 0.0)
}

/// Slot panel for editing template slot values.
pub struct SlotPanel;

/// Check if a ParseResult contains any slot blocks (which are invalid in slot values).
/// Returns the label of the first slot block found, if any.
fn find_slot_block_in_parse_result(parse_result: &ParseResult) -> Option<String> {
    if let Some(ast) = &parse_result.ast {
        for (node, _span) in &ast.nodes {
            if let Node::SlotBlock(slot_block) = node {
                return Some(slot_block.label.0.clone());
            }
        }
    }
    None
}

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

        // IMPORTANT: Handle autocomplete keyboard for any active slot editor BEFORE rendering.
        // This must happen at the SlotPanel level, before the FocusableFrame creates nested UIs,
        // to ensure keyboard events are consumed before any TextEdit widget processes them.
        let mut slot_autocomplete_selection: Option<(String, String)> = None; // (editor_id, completion_text)
        for def in &definitions {
            if matches!(def.kind, SlotDefKind::Textarea) {
                let editor_id = format!("slot_editor_{}", def.label);
                if state.is_autocomplete_active(&editor_id) {
                    let completions = get_completions(&state.workspace, state, &editor_id);
                    if !completions.is_empty()
                        && let Some(completion_text) =
                            handle_autocomplete_keyboard(ui, state, &editor_id, &completions)
                    {
                        slot_autocomplete_selection = Some((editor_id, completion_text));
                        break;
                    }
                }
            }
        }

        // No internal scroll - parent handles scrolling
        for def in &definitions {
            let is_focused = state.is_slot_focused(&def.label);

            match &def.kind {
                SlotDefKind::Textarea => {
                    // Check if we have a pending autocomplete selection for this slot
                    let pending_completion = slot_autocomplete_selection
                        .as_ref()
                        .filter(|(id, _)| *id == format!("slot_editor_{}", def.label))
                        .map(|(_, text)| text.clone());
                    Self::show_textarea_slot(ui, state, &def.label, is_focused, pending_completion);
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
    fn show_textarea_slot(
        ui: &mut egui::Ui,
        state: &mut AppState,
        label: &str,
        is_focused: bool,
        pending_completion: Option<String>,
    ) {
        let label_owned = label.to_string();
        let editor_id = format!("slot_editor_{}", label_owned);

        // Apply pending completion from keyboard handling (done at SlotPanel level)
        if let Some(completion_text) = pending_completion {
            let current_value = state.get_textarea_value(&label_owned);
            let new_value = apply_completion(state, &current_value, &editor_id, &completion_text);
            state.set_textarea_value(&label_owned, new_value);
            state.request_render();
        }

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
                id: editor_id.clone(),
                min_lines: 3,
                hint_text: Some("Enter text...".to_string()),
                show_line_numbers: true,
            };

            let original_value = state.get_textarea_value(&label_owned);
            let mut value = original_value.clone();
            let result = TemplateEditor::show(ui, &mut value, state, &config);

            // Update if changed by user typing OR by autocomplete completion
            if value != original_value {
                state.set_textarea_value(&label_owned, value.clone());
                state.request_render();
            }

            // Show parse errors below the editor
            TemplateEditor::show_errors(ui, &result.parse_result);

            // Check for slot blocks in the parsed AST (slots cannot reference other slots)
            if let Some(nested_label) = find_slot_block_in_parse_result(&result.parse_result) {
                ui.add_space(4.0);
                ui.horizontal(|ui| {
                    ui.colored_label(syntax::ERROR, "error:");
                    ui.label(format!(
                        "Slot values cannot contain other slots (found \"{}\")",
                        nested_label
                    ));
                });
            }

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

        // Get current values as mutable vec with indices for DnD
        let mut items: Vec<(usize, String)> = state
            .slot_values
            .get(label)
            .cloned()
            .unwrap_or_default()
            .into_iter()
            .enumerate()
            .collect();

        let original_order: Vec<String> = items.iter().map(|(_, s)| s.clone()).collect();

        // For single-select, we can always open the picker to change selection
        // For multi-select, check if we're at max
        let can_open_picker = match cardinality {
            Cardinality::One => true, // Always allow opening to change selection
            Cardinality::Many { max: None } => true,
            Cardinality::Many { max: Some(n) } => items.len() < *n as usize,
        };

        // Track if chip X button was clicked (need to use Cell for interior mutability)
        let chip_removed = std::cell::Cell::new(false);

        let label_owned = label.to_string();
        let cardinality_clone = cardinality.clone();

        // Track value to remove
        let to_remove = std::cell::RefCell::new(None::<String>);

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
                        let count = items.len();
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
            if !items.is_empty() {
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
                        // Make item spacing equal for horizontal wrapped layout
                        ui.spacing_mut().item_spacing.x = ui.spacing().item_spacing.y;

                        // Use horizontal_wrapped with egui_dnd for drag-and-drop
                        ui.horizontal_wrapped(|ui| {
                            let dnd_id = format!("slot_dnd_{}", label_owned);
                            // Calculate max chip width (leave room for spacing)
                            let max_chip_width = (ui.available_width() - 16.0).max(100.0);

                            dnd(ui, dnd_id).show_custom_vec(&mut items, |ui, items, item_iter| {
                                items.iter().enumerate().for_each(|(idx, item)| {
                                    let (_original_idx, value) = item;

                                    // For display, replace newlines with spaces to keep chips single-line
                                    let display_value: String = value
                                        .chars()
                                        .map(|c| if c == '\n' { ' ' } else { c })
                                        .collect::<String>()
                                        .split_whitespace()
                                        .collect::<Vec<_>>()
                                        .join(" ");

                                    // Measure the chip content size: value text + "x" button + spacing
                                    let text_size = measure_text(ui, &display_value);
                                    let x_button_size = measure_text(ui, "x");

                                    // Chip padding and internal spacing
                                    let chip_padding = 6.0; // left + right inner margin
                                    let chip_spacing = 4.0; // space between label and X button
                                    let chip_vertical_padding = 2.0; // top + bottom

                                    let raw_chip_width = text_size.x
                                        + x_button_size.x
                                        + chip_padding * 2.0
                                        + chip_spacing
                                        + 8.0; // extra for button frame

                                    let chip_size = Vec2::new(
                                        raw_chip_width.min(max_chip_width),
                                        text_size.y.max(x_button_size.y)
                                            + chip_vertical_padding * 2.0
                                            + 4.0,
                                    );

                                    // Use the value string as a stable ID (combined with slot label for uniqueness)
                                    let item_id = Id::new((&label_owned, value));
                                    item_iter.next(
                                        ui,
                                        item_id,
                                        idx,
                                        true,
                                        |ui, item_handle| {
                                            item_handle.ui_sized(
                                                ui,
                                                chip_size,
                                                |ui, handle, _state| {
                                                    // Chip with X button - entire chip is drag handle
                                                    handle.ui_sized(ui, chip_size, |ui| {
                                                        egui::Frame::NONE
                                                            .inner_margin(egui::Margin {
                                                                left: chip_padding as i8,
                                                                right: chip_padding as i8,
                                                                top: chip_vertical_padding as i8,
                                                                bottom: chip_vertical_padding as i8,
                                                            })
                                                            .corner_radius(12.0)
                                                            .fill(egui::Color32::from_rgb(
                                                                69, 71, 90,
                                                            )) // Catppuccin surface2
                                                            .show(ui, |ui| {
                                                                ui.horizontal(|ui| {
                                                                    ui.spacing_mut().item_spacing.x =
                                                                        chip_spacing;
                                                                    // Truncate long labels, show single-line version
                                                                    let label_response = ui.add(
                                                                        Label::new(&display_value).truncate(),
                                                                    );
                                                                    // Show full original text on hover
                                                                    label_response.on_hover_text(value);
                                                                    if ui
                                                                        .small_button("x")
                                                                        .on_hover_text("Remove")
                                                                        .clicked()
                                                                    {
                                                                        *to_remove.borrow_mut() =
                                                                            Some(value.clone());
                                                                        chip_removed.set(true);
                                                                    }
                                                                });
                                                            });
                                                    });
                                                },
                                            )
                                        },
                                    );
                                });
                            });
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

        // Handle removal
        if let Some(value) = to_remove.borrow().as_ref() {
            state.remove_slot_value(&label_owned, value);
            state.request_render();
        } else {
            // Check if order changed via drag-and-drop
            let new_order: Vec<String> = items.iter().map(|(_, s)| s.clone()).collect();
            if new_order != original_order {
                state.set_slot_values(&label_owned, new_order);
                state.request_render();
            }
        }

        // Focus slot when clicking anywhere in frame (except on chip X buttons)
        if frame_response.clicked && can_open_picker && !chip_removed.get() {
            state.focus_slot(label);
        }
    }
}
