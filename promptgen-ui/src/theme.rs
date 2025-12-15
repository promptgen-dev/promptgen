use catppuccin_egui::{set_theme, MOCHA};

/// Apply the Catppuccin Mocha theme to the egui context.
pub fn apply_theme(ctx: &egui::Context) {
    set_theme(ctx, MOCHA);
}

/// Get Catppuccin Mocha colors for syntax highlighting.
pub mod syntax {
    use egui::Color32;

    /// Colors from Catppuccin Mocha palette for syntax highlighting
    pub const TEXT: Color32 = Color32::from_rgb(205, 214, 244); // Text
    pub const COMMENT: Color32 = Color32::from_rgb(108, 112, 134); // Overlay0
    pub const REFERENCE: Color32 = Color32::from_rgb(137, 180, 250); // Blue
    pub const SLOT: Color32 = Color32::from_rgb(166, 227, 161); // Green
    pub const OPTION: Color32 = Color32::from_rgb(250, 179, 135); // Peach
    pub const BRACE: Color32 = Color32::from_rgb(147, 153, 178); // Overlay2
    pub const ERROR: Color32 = Color32::from_rgb(243, 139, 168); // Red
}
