/// Our custom font size additions (added to egui defaults)
const FONT_SIZE_INCREASE: f32 = 2.0;

/// Check if our font size customization has been applied
fn has_custom_font_sizes(ctx: &egui::Context) -> bool {
    // Check if Body font is larger than default (14.0 -> 16.0)
    let style = ctx.style();
    if let Some(font_id) = style.text_styles.get(&egui::TextStyle::Body) {
        font_id.size > 14.5 // Default is 14.0, we add 2.0
    } else {
        false
    }
}

/// Apply custom font sizes to the current style
pub fn apply_font_sizes(ctx: &egui::Context) {
    if !has_custom_font_sizes(ctx) {
        let mut style = (*ctx.style()).clone();
        for (_text_style, font_id) in style.text_styles.iter_mut() {
            font_id.size += FONT_SIZE_INCREASE;
        }
        ctx.set_style(style);
    }
}

/// Get colors for syntax highlighting that adapt to dark/light mode.
pub mod syntax {
    use egui::Color32;

    /// Get the text color based on dark/light mode
    pub fn text(ctx: &egui::Context) -> Color32 {
        if ctx.style().visuals.dark_mode {
            Color32::from_rgb(205, 214, 244) // Catppuccin Mocha Text
        } else {
            Color32::from_rgb(76, 79, 105) // Catppuccin Latte Text
        }
    }

    /// Get the comment color based on dark/light mode
    pub fn comment(ctx: &egui::Context) -> Color32 {
        if ctx.style().visuals.dark_mode {
            Color32::from_rgb(108, 112, 134) // Mocha Overlay0
        } else {
            Color32::from_rgb(140, 143, 161) // Latte Overlay0
        }
    }

    /// Get the reference color based on dark/light mode
    pub fn reference(ctx: &egui::Context) -> Color32 {
        if ctx.style().visuals.dark_mode {
            Color32::from_rgb(137, 180, 250) // Mocha Blue
        } else {
            Color32::from_rgb(30, 102, 245) // Latte Blue
        }
    }

    /// Get the slot color based on dark/light mode
    pub fn slot(ctx: &egui::Context) -> Color32 {
        if ctx.style().visuals.dark_mode {
            Color32::from_rgb(166, 227, 161) // Mocha Green
        } else {
            Color32::from_rgb(64, 160, 43) // Latte Green
        }
    }

    /// Get the option color based on dark/light mode
    pub fn option(ctx: &egui::Context) -> Color32 {
        if ctx.style().visuals.dark_mode {
            Color32::from_rgb(250, 179, 135) // Mocha Peach
        } else {
            Color32::from_rgb(254, 100, 11) // Latte Peach
        }
    }

    /// Get the brace color based on dark/light mode
    pub fn brace(ctx: &egui::Context) -> Color32 {
        if ctx.style().visuals.dark_mode {
            Color32::from_rgb(147, 153, 178) // Mocha Overlay2
        } else {
            Color32::from_rgb(124, 127, 147) // Latte Overlay2
        }
    }

    /// Error color (same red works for both modes)
    pub const ERROR: Color32 = Color32::from_rgb(210, 15, 57); // Latte Red (darker, visible in both)
}
