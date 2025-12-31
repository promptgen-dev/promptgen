use egui::{Color32, Stroke};

/// All theme colors in one place
#[derive(Clone, Copy)]
pub struct Theme {
    // Syntax highlighting
    pub text: Color32,
    pub comment: Color32,
    pub reference: Color32,
    pub slot: Color32,
    pub option: Color32,
    pub brace: Color32,
    pub error: Color32,

    // UI elements
    pub chip_bg: Color32,
    pub focus_bg: Color32,
    pub cursor: Color32,
    pub match_highlight: Color32,
    pub muted: Color32,
}

/// Dark theme (Catppuccin Mocha inspired)
pub const DARK: Theme = Theme {
    // Syntax
    text: Color32::from_rgb(205, 214, 244),     // Mocha Text
    comment: Color32::from_rgb(108, 112, 134),  // Mocha Overlay0
    reference: Color32::from_rgb(137, 180, 250), // Mocha Blue
    slot: Color32::from_rgb(166, 227, 161),     // Mocha Green
    option: Color32::from_rgb(250, 179, 135),   // Mocha Peach
    brace: Color32::from_rgb(147, 153, 178),    // Mocha Overlay2
    error: Color32::from_rgb(243, 139, 168),    // Mocha Red

    // UI
    chip_bg: Color32::from_rgb(69, 71, 90),     // Mocha Surface2
    focus_bg: Color32::from_rgb(49, 50, 68),    // Mocha Surface1
    cursor: Color32::from_rgb(205, 214, 244),   // Mocha Text
    match_highlight: Color32::from_rgb(249, 226, 175), // Mocha Yellow
    muted: Color32::from_rgb(108, 112, 134),    // Mocha Overlay0
};

/// Light theme (Catppuccin Latte inspired, with contrast adjustments)
pub const LIGHT: Theme = Theme {
    // Syntax
    text: Color32::from_rgb(32, 32, 32),        // Dark gray for contrast
    comment: Color32::from_rgb(140, 143, 161),  // Latte Overlay0
    reference: Color32::from_rgb(30, 102, 245), // Latte Blue
    slot: Color32::from_rgb(64, 160, 43),       // Latte Green
    option: Color32::from_rgb(254, 100, 11),    // Latte Peach
    brace: Color32::from_rgb(124, 127, 147),    // Latte Overlay2
    error: Color32::from_rgb(210, 15, 57),      // Latte Red

    // UI
    chip_bg: Color32::from_rgb(188, 192, 204),  // Latte Surface2
    focus_bg: Color32::from_rgb(220, 224, 232), // Latte Surface1
    cursor: Color32::from_rgb(76, 79, 105),     // Latte Text
    match_highlight: Color32::from_rgb(223, 142, 29), // Latte Yellow (darker for contrast)
    muted: Color32::from_rgb(140, 143, 161),    // Latte Overlay0
};

/// Get the current theme based on dark/light mode
pub fn current(ctx: &egui::Context) -> Theme {
    if ctx.style().visuals.dark_mode {
        DARK
    } else {
        LIGHT
    }
}

// ============================================================================
// Font and cursor setup
// ============================================================================

/// Our custom font size additions (added to egui defaults)
const FONT_SIZE_INCREASE: f32 = 2.0;

/// Check if our font size customization has been applied
fn has_custom_font_sizes(ctx: &egui::Context) -> bool {
    let style = ctx.style();
    if let Some(font_id) = style.text_styles.get(&egui::TextStyle::Body) {
        font_id.size > 14.5 // Default is 14.0, we add 2.0
    } else {
        false
    }
}

/// Apply custom font sizes
pub fn apply_font_sizes(ctx: &egui::Context) {
    if !has_custom_font_sizes(ctx) {
        let mut style = (*ctx.style()).clone();
        for (_text_style, font_id) in style.text_styles.iter_mut() {
            font_id.size += FONT_SIZE_INCREASE;
        }
        ctx.set_style(style);
    }
}

/// Ensure the text cursor is visible with proper color contrast
pub fn ensure_cursor_visible(ctx: &egui::Context) {
    let theme = current(ctx);
    let current_stroke = ctx.style().visuals.text_cursor.stroke;
    let desired_stroke = Stroke::new(2.0, theme.cursor);

    // Only update if the stroke color doesn't match (avoid unnecessary style updates)
    if current_stroke.color != desired_stroke.color || current_stroke.width != desired_stroke.width {
        let mut style = (*ctx.style()).clone();
        style.visuals.text_cursor.stroke = desired_stroke;
        ctx.set_style(style);
    }
}
