//! Unified style system for consistent, beautiful UI across the application.
//!
//! This module provides:
//! - Typography helpers for consistent text hierarchy
//! - Spacing constants for visual rhythm
//! - Component builders for common UI patterns
//! - Icon helpers with proper sizing

use egui::{Color32, RichText, Stroke, Vec2};
use egui_material_icons::icons::ICON_ASTERISK;

use crate::theme::{self, Theme};

// ============================================================================
// Spacing System
// ============================================================================

/// Spacing constants for consistent visual rhythm
pub mod spacing {
    /// Extra small spacing (2px) - tight internal padding
    pub const XS: f32 = 2.0;
    /// Small spacing (4px) - between related elements
    pub const SM: f32 = 4.0;
    /// Medium spacing (8px) - standard gap
    pub const MD: f32 = 8.0;
    /// Large spacing (12px) - section separation
    pub const LG: f32 = 12.0;
    /// Extra large spacing (16px) - major section breaks
    pub const XL: f32 = 16.0;
    /// 2x extra large spacing (24px) - panel separation
    pub const XXL: f32 = 24.0;
}

// ============================================================================
// Typography
// ============================================================================

/// Typography helpers for consistent text hierarchy
pub struct Typography;

impl Typography {
    /// Large heading - panel titles (e.g., "Preview", "Prompt")
    /// Sans-serif, bold, larger size
    pub fn heading(text: impl Into<String>) -> RichText {
        RichText::new(text).size(18.0).strong()
    }

    /// Section title - subsection headers (e.g., "Slots", "Output")
    /// Sans-serif, semi-bold, medium size
    pub fn section_title(text: impl Into<String>) -> RichText {
        RichText::new(text).size(14.0).strong()
    }

    /// Body text - regular content
    pub fn body(text: impl Into<String>) -> RichText {
        RichText::new(text).size(14.0)
    }

    /// Small text - secondary information
    pub fn small(text: impl Into<String>) -> RichText {
        RichText::new(text).size(12.0)
    }

    /// Caption text - labels, hints, metadata
    pub fn caption(text: impl Into<String>, theme: &Theme) -> RichText {
        RichText::new(text).size(12.0).color(theme.subtext0)
    }

    /// Muted text - de-emphasized content
    pub fn muted(text: impl Into<String>, theme: &Theme) -> RichText {
        RichText::new(text).size(14.0).color(theme.overlay0)
    }

    /// Monospace text - code, technical values
    pub fn mono(text: impl Into<String>) -> RichText {
        RichText::new(text)
            .family(egui::FontFamily::Monospace)
            .size(13.0)
    }

    /// Monospace small - line numbers, technical metadata
    pub fn mono_small(text: impl Into<String>, theme: &Theme) -> RichText {
        RichText::new(text)
            .family(egui::FontFamily::Monospace)
            .size(12.0)
            .color(theme.overlay0)
    }

    /// Error text - validation errors
    pub fn error(text: impl Into<String>, theme: &Theme) -> RichText {
        RichText::new(text).size(13.0).color(theme.syntax_error())
    }

    /// Success/positive text
    pub fn success(text: impl Into<String>, theme: &Theme) -> RichText {
        RichText::new(text).size(14.0).color(theme.accent_two)
    }

    /// Hint/placeholder style text
    pub fn hint(text: impl Into<String>, theme: &Theme) -> RichText {
        RichText::new(text)
            .size(13.0)
            .color(theme.overlay0)
            .italics()
    }
}

// ============================================================================
// Component Builders
// ============================================================================

/// Builders for common UI component patterns
pub struct Components;

impl Components {
    /// A subtle section frame - light background, no border
    /// Use for grouping related content
    pub fn section_frame(theme: &Theme) -> egui::Frame {
        egui::Frame::new()
            .fill(theme.surface0.gamma_multiply(0.5))
            .inner_margin(spacing::MD)
            .corner_radius(6.0)
    }

    /// A card frame - elevated with subtle shadow
    /// Use for distinct, clickable items
    pub fn card_frame(theme: &Theme) -> egui::Frame {
        egui::Frame::new()
            .fill(theme.surface0)
            .inner_margin(spacing::MD)
            .corner_radius(8.0)
    }

    /// An input field frame - for text inputs, textareas
    pub fn input_frame(theme: &Theme) -> egui::Frame {
        egui::Frame::new()
            .fill(theme.crust)
            .inner_margin(spacing::MD)
            .corner_radius(6.0)
            .stroke(Stroke::new(1.0, theme.surface0))
    }

    /// A toolbar frame - horizontal strip for buttons
    pub fn toolbar_frame() -> egui::Frame {
        egui::Frame::new().inner_margin(spacing::SM)
    }

    /// A chip/tag frame - for selected values
    pub fn chip_frame(theme: &Theme, is_selected: bool) -> egui::Frame {
        let (fill, stroke_color) = if is_selected {
            (theme.active_selection_fill, theme.accent_one)
        } else {
            (theme.surface1, theme.surface2)
        };

        egui::Frame::new()
            .fill(fill)
            .inner_margin(egui::Margin {
                left: 8,
                right: 8,
                top: 4,
                bottom: 4,
            })
            .corner_radius(6.0)
            .stroke(Stroke::new(1.0, stroke_color))
    }

    /// A list item frame - for sidebar items, options
    pub fn list_item_frame(theme: &Theme, is_active: bool, is_hovered: bool) -> egui::Frame {
        let fill = if is_active {
            theme.active_selection_fill
        } else if is_hovered {
            theme.surface0
        } else {
            Color32::TRANSPARENT
        };

        egui::Frame::new()
            .fill(fill)
            .inner_margin(egui::Margin {
                left: 8,
                right: 8,
                top: 4,
                bottom: 4,
            })
            .corner_radius(6.0)
    }

    /// A panel header frame
    pub fn panel_header_frame(theme: &Theme) -> egui::Frame {
        egui::Frame::new()
            .fill(theme.mantle)
            .inner_margin(egui::Margin {
                left: 8,
                right: 8,
                top: 4,
                bottom: 4,
            })
    }

    /// Collapsible section header styling
    pub fn collapsible_header(theme: &Theme, is_expanded: bool) -> egui::Frame {
        let fill = if is_expanded {
            theme.surface0.gamma_multiply(0.3)
        } else {
            Color32::TRANSPARENT
        };

        egui::Frame::new()
            .fill(fill)
            .inner_margin(egui::Margin {
                left: 4,
                right: 4,
                top: 2,
                bottom: 2,
            })
            .corner_radius(4.0)
    }
}

// ============================================================================
// Button Helpers
// ============================================================================

/// Button style variants
pub struct Buttons;

impl Buttons {
    /// Primary action button - filled with accent color
    pub fn primary(text: impl Into<String>, theme: &Theme) -> egui::Button<'static> {
        egui::Button::new(RichText::new(text).color(if theme.is_light {
            Color32::WHITE
        } else {
            theme.crust
        }))
        .fill(theme.accent_one)
        .corner_radius(6.0)
        .min_size(Vec2::new(0.0, 32.0))
    }

    /// Secondary button - subtle fill
    pub fn secondary(text: impl Into<String>, theme: &Theme) -> egui::Button<'static> {
        egui::Button::new(RichText::new(text))
            .fill(theme.surface0)
            .stroke(Stroke::new(1.0, theme.surface1))
            .corner_radius(6.0)
            .min_size(Vec2::new(0.0, 32.0))
    }

    /// Ghost/subtle button - transparent until hovered
    pub fn ghost(text: impl Into<String>) -> egui::Button<'static> {
        egui::Button::new(RichText::new(text))
            .fill(Color32::TRANSPARENT)
            .corner_radius(6.0)
            .min_size(Vec2::new(0.0, 32.0))
    }

    /// Icon-only button - square, minimal
    pub fn icon(icon: &str) -> egui::Button<'static> {
        egui::Button::new(icon)
            .fill(Color32::TRANSPARENT)
            .corner_radius(4.0)
            .min_size(Vec2::splat(28.0))
    }

    /// Small icon button - for inline actions
    pub fn icon_small(icon: &str) -> egui::Button<'static> {
        egui::Button::new(RichText::new(icon).size(14.0))
            .fill(Color32::TRANSPARENT)
            .corner_radius(4.0)
            .min_size(Vec2::splat(22.0))
    }

    /// Danger/destructive button
    pub fn danger(text: impl Into<String>, theme: &Theme) -> egui::Button<'static> {
        egui::Button::new(RichText::new(text).color(theme.accent_four))
            .fill(Color32::TRANSPARENT)
            .stroke(Stroke::new(1.0, theme.accent_four.gamma_multiply(0.5)))
            .corner_radius(6.0)
            .min_size(Vec2::new(0.0, 32.0))
    }

    /// Tab button styling
    pub fn tab(text: impl Into<String>, is_active: bool, theme: &Theme) -> egui::Button<'static> {
        let (fill, text_color) = if is_active {
            (theme.active_selection_fill, theme.text)
        } else {
            (Color32::TRANSPARENT, theme.subtext0)
        };

        egui::Button::new(RichText::new(text).color(text_color))
            .fill(fill)
            .corner_radius(6.0)
    }
}

// ============================================================================
// Layout Helpers
// ============================================================================

/// Layout helpers for common patterns
pub struct Layout;

impl Layout {
    /// Add standard section spacing
    pub fn section_gap(ui: &mut egui::Ui) {
        ui.add_space(spacing::LG);
    }

    /// Add small gap between related elements
    pub fn small_gap(ui: &mut egui::Ui) {
        ui.add_space(spacing::SM);
    }

    /// Add medium gap
    pub fn gap(ui: &mut egui::Ui) {
        ui.add_space(spacing::MD);
    }

    /// A subtle horizontal divider (less prominent than separator)
    pub fn subtle_divider(ui: &mut egui::Ui, theme: &Theme) {
        ui.add_space(spacing::SM);
        let rect = ui.available_rect_before_wrap();
        let line_y = rect.top();
        ui.painter().line_segment(
            [
                egui::pos2(rect.left(), line_y),
                egui::pos2(rect.right(), line_y),
            ],
            Stroke::new(1.0, theme.surface0),
        );
        ui.add_space(spacing::SM);
    }

    /// Show a section with a title and content
    pub fn titled_section(
        ui: &mut egui::Ui,
        title: impl Into<String>,
        add_contents: impl FnOnce(&mut egui::Ui),
    ) {
        ui.label(Typography::section_title(title));
        ui.add_space(spacing::SM);
        add_contents(ui);
    }
}

// ============================================================================
// Icon Helpers
// ============================================================================

/// Icon styling helpers
pub struct Icons;

impl Icons {
    /// Standard icon size for buttons
    pub fn button_icon(icon: &str) -> RichText {
        RichText::new(icon).size(18.0)
    }

    /// Small inline icon
    pub fn inline(icon: &str) -> RichText {
        RichText::new(icon).size(14.0)
    }

    /// Large icon for empty states
    pub fn large(icon: &str, theme: &Theme) -> RichText {
        RichText::new(icon).size(32.0).color(theme.overlay0)
    }

    /// Icon with label - common pattern for buttons with text
    pub fn with_label(icon: &str, label: impl Into<String>) -> String {
        format!("{} {}", icon, label.into())
    }
}

// ============================================================================
// Semantic UI Patterns
// ============================================================================

/// Semantic UI patterns for specific use cases
pub struct Patterns;

impl Patterns {
    /// Empty state - shown when a list or area has no content
    pub fn empty_state(ui: &mut egui::Ui, icon: &str, message: &str, hint: Option<&str>) {
        let theme = theme::current(ui.ctx());

        ui.vertical_centered(|ui| {
            ui.add_space(spacing::XXL);
            ui.label(Icons::large(icon, &theme));
            ui.add_space(spacing::MD);
            ui.label(Typography::muted(message, &theme));
            if let Some(hint_text) = hint {
                ui.add_space(spacing::SM);
                ui.label(Typography::hint(hint_text, &theme));
            }
        });
    }

    /// Status badge - small colored indicator with text
    pub fn status_badge(ui: &mut egui::Ui, text: &str, color: Color32) {
        egui::Frame::new()
            .fill(color.gamma_multiply(0.2))
            .inner_margin(egui::Margin {
                left: 4,
                right: 4,
                top: 2,
                bottom: 2,
            })
            .corner_radius(4.0)
            .show(ui, |ui| {
                ui.label(RichText::new(text).size(11.0).color(color));
            });
    }

    /// Count badge - shows a number (e.g., "3 items")
    pub fn count_badge(ui: &mut egui::Ui, count: usize, label: &str, theme: &Theme) {
        let text = if count == 1 {
            format!("{} {}", count, label)
        } else {
            format!("{} {}s", count, label)
        };
        ui.label(Typography::caption(text, theme));
    }

    /// Dirty indicator dot
    pub fn dirty_indicator(ui: &mut egui::Ui, theme: &Theme) {
        ui.label(
            RichText::new(ICON_ASTERISK)
                .size(10.0)
                .color(theme.accent_five),
        );
    }

    /// Keyboard shortcut hint
    pub fn kbd_hint(ui: &mut egui::Ui, key: &str, theme: &Theme) {
        egui::Frame::new()
            .fill(theme.surface0)
            .inner_margin(egui::Margin {
                left: 4,
                right: 4,
                top: 2,
                bottom: 2,
            })
            .corner_radius(3.0)
            .stroke(Stroke::new(1.0, theme.surface1))
            .show(ui, |ui| {
                ui.label(
                    RichText::new(key)
                        .size(11.0)
                        .family(egui::FontFamily::Monospace)
                        .color(theme.subtext0),
                );
            });
    }
}
