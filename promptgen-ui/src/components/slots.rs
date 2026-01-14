//! Slot panel component for editing prompt slots.

use egui::{Align, Id, Label, Layout, UiBuilder, Vec2};
use egui_dnd::dnd;
use egui_flex::{Flex, FlexItem};
use egui_material_icons::icons::{ICON_CHECK, ICON_CLOSE, ICON_DONE_ALL, ICON_EDIT, ICON_TEXT_AD};
use promptgen_core::{Cardinality, Node, ParseResult, SlotDefKind};

use crate::components::autocomplete::{
    apply_completion, get_completions, handle_autocomplete_keyboard,
};
use crate::components::focusable_frame::FocusableFrame;
use crate::components::prompt_editor::{PromptEditor, PromptEditorConfig};
use crate::state::AppState;
use crate::theme;
use crate::utils::truncate;

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

/// Slot panel for editing prompt slot values.
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
                egui::RichText::new("No slots in prompt")
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
            // Handle autocomplete for textarea slots and pick slots in manual edit mode
            let should_handle = match &def.kind {
                SlotDefKind::Textarea => true,
                SlotDefKind::Pick { .. } => state.is_slot_manual_edit(&def.label),
            };

            if should_handle {
                let editor_id = format!("slot_editor_{}", def.label);
                if state.is_autocomplete_active(&editor_id) {
                    let completions = get_completions(&state.library, state, &editor_id);
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
                    // Check if we have a pending autocomplete selection for this pick slot in manual edit mode
                    let pending_completion = slot_autocomplete_selection
                        .as_ref()
                        .filter(|(id, _)| *id == format!("slot_editor_{}", def.label))
                        .map(|(_, text)| text.clone());
                    Self::show_pick_slot(
                        ui,
                        state,
                        &def.label,
                        cardinality,
                        sep,
                        is_focused,
                        pending_completion,
                    );
                }
            }

            ui.add_space(4.0);
        }

        // Add scroll padding at the bottom so autocomplete popups have room to display
        ui.add_space(300.0);
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

        // Track if clear button was clicked
        let clear_clicked = std::cell::Cell::new(false);
        let has_content = !state.get_textarea_value(&label_owned).is_empty();

        let frame_response = FocusableFrame::new(is_focused).show(ui, |ui| {
            ui.set_width(ui.available_width());

            // Header with label, type indicator, and clear button using flex layout
            Flex::horizontal().w_full().wrap(false).show(ui, |flex| {
                // Label and type indicator (grows and truncates)
                flex.add_ui(FlexItem::default().grow(1.0).shrink(), |ui| {
                    ui.set_width(ui.available_width());
                    let header_text = format!("{} {}", ICON_TEXT_AD, label_owned);
                    ui.add(Label::new(header_text).truncate());
                });

                // Clear button (fixed size, only shown if there's content)
                if has_content {
                    flex.add_ui(FlexItem::default(), |ui| {
                        if ui
                            .small_button(ICON_CLOSE)
                            .on_hover_text("Clear slot")
                            .clicked()
                        {
                            clear_clicked.set(true);
                        }
                    });
                }
            });

            let config = PromptEditorConfig {
                id: editor_id.clone(),
                min_lines: 3,
                hint_text: Some("Enter text...".to_string()),
                show_line_numbers: true,
            };

            let original_value = state.get_textarea_value(&label_owned);
            let mut value = original_value.clone();
            let result = PromptEditor::show(ui, &mut value, state, &config);

            // Update if changed by user typing OR by autocomplete completion
            if value != original_value {
                state.set_textarea_value(&label_owned, value.clone());
                state.request_render();
            }

            // Show parse errors below the editor
            PromptEditor::show_errors(ui, &result.parse_result);

            // Check for slot blocks in the parsed AST (slots cannot reference other slots)
            if let Some(nested_label) = find_slot_block_in_parse_result(&result.parse_result) {
                ui.add_space(4.0);
                ui.horizontal(|ui| {
                    ui.colored_label(theme::current(ui.ctx()).syntax_error(), "error:");
                    ui.label(format!(
                        "Slot values cannot contain other slots (found \"{}\")",
                        nested_label
                    ));
                });
            }

            result
        });

        let result = frame_response.inner;

        // Handle clear button
        if clear_clicked.get() {
            state.clear_slot(&label_owned);
        }

        // Track focus - either from TextEdit gaining focus or clicking anywhere in frame
        if (result.response.has_focus() || frame_response.clicked) && !is_focused {
            state.focus_textarea_slot(label);
        }
    }

    /// Render a pick slot.
    /// Clicking anywhere in the slot row (except chip X buttons) opens the picker.
    /// Supports manual edit mode where values are edited as text.
    fn show_pick_slot(
        ui: &mut egui::Ui,
        state: &mut AppState,
        label: &str,
        cardinality: &Cardinality,
        sep: &str,
        is_focused: bool,
        pending_completion: Option<String>,
    ) {
        let label_owned = label.to_string();
        let editor_id = format!("slot_editor_{}", label_owned);
        let is_manual_edit = state.is_slot_manual_edit(label);
        let is_single_select = matches!(cardinality, Cardinality::One);
        let sep_owned = sep.to_string();

        // Apply pending completion from keyboard handling (only in manual edit mode)
        if is_manual_edit && let Some(completion_text) = pending_completion {
            let current_value = state.get_slot_manual_edit_text(&label_owned);
            let new_value = apply_completion(state, &current_value, &editor_id, &completion_text);
            state.set_slot_manual_edit_text(&label_owned, new_value);
            state.request_render();
        }

        // Get the editor background color from the current theme
        let editor_bg = ui.visuals().extreme_bg_color;

        // Get current values as mutable vec with indices for DnD
        let mut items: Vec<(usize, String)> = state
            .preview
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

        let cardinality_clone = cardinality.clone();

        // Track value to remove
        let to_remove = std::cell::RefCell::new(None::<String>);

        // Track if clear button was clicked
        let clear_clicked = std::cell::Cell::new(false);

        // Track if edit mode toggle was clicked
        let toggle_edit_mode = std::cell::Cell::new(false);

        let frame_response = FocusableFrame::new(is_focused).show(ui, |ui| {
            ui.set_width(ui.available_width());

            // Header with toggle button, label, cardinality info, and clear button
            Flex::horizontal().w_full().wrap(false).show(ui, |flex| {
                // Edit mode toggle button (fixed size)
                flex.add_ui(FlexItem::default(), |ui| {
                    let icon = if is_manual_edit {
                        // In manual edit mode - show checkmark icon to confirm/exit
                        match &cardinality_clone {
                            Cardinality::One => ICON_CHECK,
                            Cardinality::Many { .. } => ICON_DONE_ALL,
                        }
                    } else {
                        // In picker mode - show edit icon to enter manual edit
                        ICON_EDIT
                    };
                    let tooltip = if is_manual_edit {
                        "Exit manual edit mode"
                    } else {
                        "Edit as text"
                    };
                    if ui.small_button(icon).on_hover_text(tooltip).clicked() {
                        toggle_edit_mode.set(true);
                    }
                });

                // Label and cardinality (grows and truncates)
                flex.add_ui(FlexItem::default().grow(1.0).shrink(), |ui| {
                    ui.set_width(ui.available_width());

                    // Build the header text (without icon, since toggle button has it)
                    let header_text = match &cardinality_clone {
                        Cardinality::One => label_owned.clone(),
                        Cardinality::Many { max: None } => label_owned.clone(),
                        Cardinality::Many { max: Some(n) } => {
                            let count = items.len();
                            format!("{} ({}/{})", label_owned, count, n)
                        }
                    };

                    ui.add(Label::new(header_text).truncate());
                });

                // Clear button (fixed size, only shown if there are values or manual edit text)
                let has_content = if is_manual_edit {
                    !state.get_slot_manual_edit_text(&label_owned).is_empty()
                } else {
                    !items.is_empty()
                };
                if has_content {
                    flex.add_ui(FlexItem::default(), |ui| {
                        if ui
                            .small_button(ICON_CLOSE)
                            .on_hover_text("Clear slot")
                            .clicked()
                        {
                            clear_clicked.set(true);
                        }
                    });
                }
            });

            // Content area - either manual text editor or chip picker
            if is_manual_edit {
                // Manual edit mode - show text editor
                let config = PromptEditorConfig {
                    id: editor_id.clone(),
                    min_lines: 3,
                    hint_text: Some(if is_single_select {
                        "Enter value...".to_string()
                    } else {
                        format!("Enter values separated by \"{}\"...", sep)
                    }),
                    show_line_numbers: false,
                };

                let original_value = state.get_slot_manual_edit_text(&label_owned);
                let mut value = original_value.clone();
                let result = PromptEditor::show(ui, &mut value, state, &config);

                // Update if changed
                if value != original_value {
                    state.set_slot_manual_edit_text(&label_owned, value);
                    state.request_render();
                }

                // Show parse errors
                PromptEditor::show_errors(ui, &result.parse_result);

                // Check for slot blocks in the parsed AST
                if let Some(nested_label) = find_slot_block_in_parse_result(&result.parse_result) {
                    ui.add_space(4.0);
                    ui.horizontal(|ui| {
                        ui.colored_label(theme::current(ui.ctx()).syntax_error(), "error:");
                        ui.label(format!(
                            "Slot values cannot contain other slots (found \"{}\")",
                            nested_label
                        ));
                    });
                }
            } else {
                // Picker mode - display selected values as chips
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

                                dnd(ui, dnd_id).show_custom_vec(
                                    &mut items,
                                    |ui, items, item_iter| {
                                        items.iter().enumerate().for_each(|(idx, item)| {
                                            let (_original_idx, value) = item;

                                            // For display, replace newlines with spaces and truncate
                                            let collapsed: String = value
                                                .chars()
                                                .map(|c| if c == '\n' { ' ' } else { c })
                                                .collect::<String>()
                                                .split_whitespace()
                                                .collect::<Vec<_>>()
                                                .join(" ");
                                            let display_value = truncate(&collapsed);

                                            // Measure the chip content size: value text + "x" button + spacing
                                            let text_size = measure_text(ui, display_value.as_ref());
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
                                                            let chip_bg =
                                                                theme::current(ui.ctx()).chip_bg();
                                                            handle.ui_sized(ui, chip_size, |ui| {
                                                                egui::Frame::NONE
                                                                    .inner_margin(egui::Margin {
                                                                        left: chip_padding as i8,
                                                                        right: chip_padding as i8,
                                                                        top: chip_vertical_padding
                                                                            as i8,
                                                                        bottom: chip_vertical_padding
                                                                            as i8,
                                                                    })
                                                                    .corner_radius(12.0)
                                                                    .fill(chip_bg)
                                                                    .show(ui, |ui| {
                                                                        ui.horizontal(|ui| {
                                                                            ui.spacing_mut()
                                                                                .item_spacing
                                                                                .x = chip_spacing;
                                                                            // Truncate long labels, show single-line version
                                                                            let label_response =
                                                                                ui.add(
                                                                                    Label::new(
                                                                                        display_value.as_ref(),
                                                                                    )
                                                                                    .truncate(),
                                                                                );
                                                                            // Show full original text on hover
                                                                            label_response
                                                                                .on_hover_text(
                                                                                    value,
                                                                                );
                                                                            if ui
                                                                                .small_button("x")
                                                                                .on_hover_text(
                                                                                    "Remove",
                                                                                )
                                                                                .clicked()
                                                                            {
                                                                                *to_remove
                                                                                    .borrow_mut() =
                                                                                    Some(
                                                                                        value
                                                                                            .clone(),
                                                                                    );
                                                                                chip_removed
                                                                                    .set(true);
                                                                            }
                                                                        });
                                                                    });
                                                            });
                                                        },
                                                    )
                                                },
                                            );
                                        });
                                    },
                                );
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
            }
        });

        // Handle edit mode toggle
        if toggle_edit_mode.get() {
            if is_manual_edit {
                // Exit manual edit mode - convert text back to values
                state.exit_slot_manual_edit(&label_owned, &sep_owned, is_single_select);
            } else {
                // Enter manual edit mode - convert values to text
                state.enter_slot_manual_edit(&label_owned, &sep_owned);
            }
        } else if clear_clicked.get() {
            // Handle clear button
            if is_manual_edit {
                state.set_slot_manual_edit_text(&label_owned, String::new());
                state.request_render();
            } else {
                state.clear_slot(&label_owned);
            }
        } else if let Some(value) = to_remove.borrow().as_ref() {
            // Handle single chip removal (only in picker mode)
            state.remove_slot_value(&label_owned, value);
            state.request_render();
        } else if !is_manual_edit {
            // Check if order changed via drag-and-drop (only in picker mode)
            let new_order: Vec<String> = items.iter().map(|(_, s)| s.clone()).collect();
            if new_order != original_order {
                state.set_slot_values(&label_owned, new_order);
                state.request_render();
            }
        }

        // Focus slot when clicking anywhere in frame (except on chip X buttons or clear button)
        // Only in picker mode, not manual edit mode (which handles its own focus)
        if !is_manual_edit
            && frame_response.clicked
            && can_open_picker
            && !chip_removed.get()
            && !clear_clicked.get()
            && !toggle_edit_mode.get()
        {
            state.focus_slot(label);
        }
    }
}
