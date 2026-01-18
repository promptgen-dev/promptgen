//! Slot panel component for editing prompt slots.

use egui::{Align, Id, Label, Layout, UiBuilder, Vec2};
use egui_dnd::dnd;
use egui_flex::{Flex, FlexItem};
use egui_material_icons::icons::{
    ICON_CHECK, ICON_CHEVRON_RIGHT, ICON_CLOSE, ICON_DONE_ALL, ICON_EDIT, ICON_EXPAND_MORE,
    ICON_INPUT, ICON_INVENTORY_2, ICON_TEXT_AD,
};
use promptgen_core::{Cardinality, Node, ParseResult, SlotDefKind, SlotDefinition};

use crate::components::autocomplete::{
    apply_completion, get_completions, handle_autocomplete_keyboard,
};
use crate::components::focusable_frame::FocusableFrame;
use crate::components::prompt_editor::{PromptEditor, PromptEditorConfig};
use crate::state::AppState;
use crate::styles::{Buttons, Components, Patterns, Typography, spacing};
use crate::theme;
use crate::utils::truncate;

// ============================================================================
// Hierarchical Slot Structure
// ============================================================================

/// A hierarchical slot entry - either a leaf slot or a reference group with children.
#[derive(Debug, Clone)]
enum SlotEntry {
    /// A regular slot (textarea or pick) - the leaf node.
    Slot(SlotDefinition),
    /// A reference group containing nested slots.
    ReferenceGroup {
        /// The prefix label for this group (e.g., "Style" from "Style - Color").
        label: String,
        /// Child entries (can be more reference groups or slots).
        children: Vec<SlotEntry>,
    },
}

/// Build a hierarchical slot structure from flat slot definitions.
///
/// Slots with prefixed names like "Style - Color" are grouped under a
/// ReferenceGroup with label "Style". Nested prefixes create nested groups.
fn build_slot_hierarchy(definitions: &[SlotDefinition]) -> Vec<SlotEntry> {
    let mut root: Vec<SlotEntry> = Vec::new();

    for def in definitions {
        insert_into_hierarchy(&mut root, def, &def.label);
    }

    root
}

/// Extract the local (display) name from a slot label.
///
/// For nested slots like "Style - Color", this returns "Color".
/// For non-nested slots like "Name", this returns "Name".
fn local_slot_name(label: &str) -> &str {
    label.rsplit(" - ").next().unwrap_or(label)
}

/// Insert a slot definition into the hierarchy at the appropriate position.
fn insert_into_hierarchy(entries: &mut Vec<SlotEntry>, def: &SlotDefinition, remaining_path: &str) {
    // Check if this path has a prefix (contains " - ")
    if let Some(sep_pos) = remaining_path.find(" - ") {
        let prefix = &remaining_path[..sep_pos];
        let rest = &remaining_path[sep_pos + 3..]; // Skip " - "

        // Find or create the reference group for this prefix
        let group_idx = entries.iter().position(|e| {
            matches!(e, SlotEntry::ReferenceGroup { label, .. } if label == prefix)
        });

        if let Some(idx) = group_idx {
            // Group exists, insert into it
            if let SlotEntry::ReferenceGroup { children, .. } = &mut entries[idx] {
                insert_into_hierarchy(children, def, rest);
            }
        } else {
            // Create new group
            let mut children = Vec::new();
            insert_into_hierarchy(&mut children, def, rest);
            entries.push(SlotEntry::ReferenceGroup {
                label: prefix.to_string(),
                children,
            });
        }
    } else {
        // No more prefixes - this is a leaf slot
        entries.push(SlotEntry::Slot(def.clone()));
    }
}

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
            Patterns::empty_state(
                ui,
                ICON_INVENTORY_2,
                "No slots in prompt",
                Some("Add slots with {label:type} syntax"),
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
                SlotDefKind::Reference { .. } => false, // References don't have editors
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

        // Build hierarchical slot structure
        let hierarchy = build_slot_hierarchy(&definitions);

        // Take expand_all_slots state (consumed once per render)
        let expand_all = state.preview.expand_all_slots.take();

        // Render the hierarchy (depth 0 = root level)
        Self::render_slot_entries(ui, state, &hierarchy, 0, &slot_autocomplete_selection, expand_all);

        // Add scroll padding at the bottom so autocomplete popups have room to display
        ui.add_space(300.0);
    }

    /// Render a list of slot entries at a given depth level.
    fn render_slot_entries(
        ui: &mut egui::Ui,
        state: &mut AppState,
        entries: &[SlotEntry],
        depth: usize,
        autocomplete_selection: &Option<(String, String)>,
        expand_all: Option<bool>,
    ) {
        for entry in entries {
            match entry {
                SlotEntry::Slot(def) => {
                    let is_focused = state.is_slot_focused(&def.label);

                    match &def.kind {
                        SlotDefKind::Textarea => {
                            let pending_completion = autocomplete_selection
                                .as_ref()
                                .filter(|(id, _)| *id == format!("slot_editor_{}", def.label))
                                .map(|(_, text)| text.clone());
                            Self::show_textarea_slot(
                                ui,
                                state,
                                &def.label,
                                is_focused,
                                pending_completion,
                            );
                        }
                        SlotDefKind::Pick {
                            cardinality, sep, ..
                        } => {
                            let pending_completion = autocomplete_selection
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
                        SlotDefKind::Reference { .. } => {
                            // Reference slots are expanded into groups, not rendered directly
                        }
                    }

                    ui.add_space(spacing::XS);
                }
                SlotEntry::ReferenceGroup { label, children } => {
                    Self::show_reference_group(
                        ui,
                        state,
                        label,
                        children,
                        depth,
                        autocomplete_selection,
                        expand_all,
                    );
                    ui.add_space(spacing::XS);
                }
            }
        }
    }

    /// Render a reference group as a collapsible section.
    fn show_reference_group(
        ui: &mut egui::Ui,
        state: &mut AppState,
        label: &str,
        children: &[SlotEntry],
        depth: usize,
        autocomplete_selection: &Option<(String, String)>,
        expand_all: Option<bool>,
    ) {
        let theme = theme::current(ui.ctx());
        let id = ui.make_persistent_id(format!("slot_ref_group_{}", label));

        // Use CollapsingState for custom header layout - default to open
        let mut collapsing_state =
            egui::collapsing_header::CollapsingState::load_with_default_open(ui.ctx(), id, true);

        // Apply expand/collapse all if requested
        if let Some(should_expand) = expand_all {
            collapsing_state.set_open(should_expand);
        }

        // Calculate background color based on nesting depth
        let bg_color = if theme.is_light {
            // For light themes, darken by 5% per level
            let darken_factor = 0.95_f32.powi(depth as i32 + 1);
            theme.base.gamma_multiply(darken_factor)
        } else {
            // For dark themes, lighten by 5% per level
            let lighten_factor = 1.05_f32.powi(depth as i32 + 1);
            theme.surface0.gamma_multiply(lighten_factor)
        };

        // Check if any slots in this group have values
        let has_values = state.has_slots_with_prefix_values(label);

        // Track if clear button was clicked
        let clear_clicked = std::cell::Cell::new(false);
        let label_for_clear = label.to_string();

        // Frame for the entire reference group
        egui::Frame::new()
            .fill(bg_color)
            .inner_margin(egui::Margin {
                left: 8,
                right: 8,
                top: 6,
                bottom: 6,
            })
            .corner_radius(6.0)
            .show(ui, |ui| {
                ui.set_width(ui.available_width());

                // Header row with clickable toggle area, chevron, and clear button
                Flex::horizontal().w_full().wrap(false).show(ui, |flex| {
                    // Clickable header area (icon + label) - grows to fill space
                    flex.add_ui(FlexItem::default().grow(1.0).shrink(), |ui| {
                        ui.set_width(ui.available_width());

                        // Make the entire header area clickable
                        let (rect, response) = ui.allocate_exact_size(
                            egui::vec2(ui.available_width(), 20.0),
                            egui::Sense::click(),
                        );

                        // Draw the header content (without chevron - it's on the right now)
                        let header_text = format!("{} {}", ICON_INPUT, label);

                        // Paint the text
                        ui.painter().text(
                            rect.left_center(),
                            egui::Align2::LEFT_CENTER,
                            header_text,
                            egui::FontId::proportional(15.0),
                            theme.text,
                        );

                        // Toggle on click
                        if response.clicked() {
                            collapsing_state.toggle(ui);
                        }

                        // Show hover cursor
                        if response.hovered() {
                            ui.ctx().set_cursor_icon(egui::CursorIcon::PointingHand);
                        }
                    });

                    // Chevron button (fixed size, on the right)
                    flex.add_ui(FlexItem::default(), |ui| {
                        let icon = if collapsing_state.is_open() {
                            ICON_EXPAND_MORE
                        } else {
                            ICON_CHEVRON_RIGHT
                        };
                        if ui
                            .add(Buttons::icon_small(icon))
                            .on_hover_text(if collapsing_state.is_open() {
                                "Collapse"
                            } else {
                                "Expand"
                            })
                            .clicked()
                        {
                            collapsing_state.toggle(ui);
                        }
                    });

                    // Clear button (fixed size, only shown if there are values)
                    if has_values {
                        flex.add_ui(FlexItem::default(), |ui| {
                            if ui
                                .add(Buttons::icon_small(ICON_CLOSE))
                                .on_hover_text("Clear all values in this group")
                                .clicked()
                            {
                                clear_clicked.set(true);
                            }
                        });
                    }
                });

                // Handle clear after UI is done
                if clear_clicked.get() {
                    state.clear_slots_with_prefix(&label_for_clear);
                }

                // Body content (only shown when expanded)
                collapsing_state.show_body_unindented(ui, |ui| {
                    ui.add_space(spacing::SM);
                    // Render children at the next depth level
                    Self::render_slot_entries(
                        ui,
                        state,
                        children,
                        depth + 1,
                        autocomplete_selection,
                        expand_all,
                    );
                });
            });
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
                    let display_name = local_slot_name(&label_owned);
                    let header_text = format!("{} {}", ICON_TEXT_AD, display_name);
                    ui.add(Label::new(header_text).truncate());
                });

                // Clear button (fixed size, only shown if there's content)
                if has_content {
                    flex.add_ui(FlexItem::default(), |ui| {
                        if ui
                            .add(Buttons::icon_small(ICON_CLOSE))
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
                    if ui
                        .add(Buttons::icon_small(icon))
                        .on_hover_text(tooltip)
                        .clicked()
                    {
                        toggle_edit_mode.set(true);
                    }
                });

                // Label and cardinality (grows and truncates)
                flex.add_ui(FlexItem::default().grow(1.0).shrink(), |ui| {
                    ui.set_width(ui.available_width());

                    // Build the header text (without icon, since toggle button has it)
                    let display_name = local_slot_name(&label_owned);
                    let header_text = match &cardinality_clone {
                        Cardinality::One => display_name.to_string(),
                        Cardinality::Many { max: None } => display_name.to_string(),
                        Cardinality::Many { max: Some(n) } => {
                            let count = items.len();
                            format!("{} ({}/{})", display_name, count, n)
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
                            .add(Buttons::icon_small(ICON_CLOSE))
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
                let theme = theme::current(ui.ctx());

                // Picker mode - display selected values as chips
                if !items.is_empty() {
                    // Container with input frame styling
                    Components::input_frame(&theme).show(ui, |ui| {
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
                                    let x_button_size = measure_text(ui, ICON_CLOSE);

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
                                    item_iter.next(ui, item_id, idx, true, |ui, item_handle| {
                                        item_handle.ui_sized(ui, chip_size, |ui, handle, _state| {
                                            // Chip with X button - entire chip is drag handle
                                            let chip_bg = theme::current(ui.ctx()).chip_bg();
                                            handle.ui_sized(ui, chip_size, |ui| {
                                                egui::Frame::NONE
                                                    .inner_margin(egui::Margin {
                                                        left: chip_padding as i8,
                                                        right: chip_padding as i8,
                                                        top: chip_vertical_padding as i8,
                                                        bottom: chip_vertical_padding as i8,
                                                    })
                                                    .corner_radius(6.0)
                                                    .fill(chip_bg)
                                                    .show(ui, |ui| {
                                                        ui.horizontal(|ui| {
                                                            ui.spacing_mut().item_spacing.x =
                                                                chip_spacing;
                                                            // Truncate long labels, show single-line version
                                                            let label_response = ui.add(
                                                                Label::new(display_value.as_ref())
                                                                    .truncate(),
                                                            );
                                                            // Show full original text on hover
                                                            label_response.on_hover_text(value);
                                                            if ui
                                                                .small_button(ICON_CLOSE)
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
                                        })
                                    });
                                });
                            });
                        });
                    });
                } else {
                    // Empty state - show placeholder in a clickable area
                    Components::input_frame(&theme).show(ui, |ui| {
                        ui.set_width(ui.available_width());
                        ui.label(Typography::hint("Click to select...", &theme));
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
