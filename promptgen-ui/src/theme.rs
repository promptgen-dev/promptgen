use egui::{Color32, Stroke, style};

/// All theme colors in one place - Catppuccin color palette
#[derive(Clone, Copy, PartialEq)]
pub struct Theme {
    // Base colors (backgrounds)
    pub base: Color32,
    pub mantle: Color32,
    pub crust: Color32,

    // Surface colors (elevated backgrounds)
    pub surface0: Color32,
    pub surface1: Color32,
    pub surface2: Color32,

    // Overlay colors (borders, subtle elements)
    pub overlay0: Color32,
    pub overlay1: Color32,
    pub overlay2: Color32,

    // Text colors
    pub text: Color32,
    pub subtext0: Color32,
    pub subtext1: Color32,

    // Accent colors
    pub accent_one: Color32,
    pub accent_two: Color32,
    pub accent_three: Color32,
    pub accent_four: Color32,
    pub accent_five: Color32,
    pub accent_six: Color32,
    pub accent_seven: Color32,

    pub active_selection_fill: Color32,

    // Whether this is a light theme
    pub is_light: bool,
}

/// Dark theme (Catppuccin Mocha)
pub const DARK: Theme = Theme {
    // Base
    base: Color32::from_rgb(30, 30, 46),
    mantle: Color32::from_rgb(24, 24, 37),
    crust: Color32::from_rgb(17, 17, 27),

    // Surface
    surface0: Color32::from_rgb(49, 50, 68),
    surface1: Color32::from_rgb(69, 71, 90),
    surface2: Color32::from_rgb(88, 91, 112),

    // Overlay
    overlay0: Color32::from_rgb(108, 112, 134),
    overlay1: Color32::from_rgb(127, 132, 156),
    overlay2: Color32::from_rgb(147, 153, 178),

    // Text
    text: Color32::from_rgb(205, 214, 244),
    subtext0: Color32::from_rgb(166, 173, 200),
    subtext1: Color32::from_rgb(186, 194, 222),

    // Accents
    accent_one: Color32::from_rgb(137, 180, 250),
    accent_two: Color32::from_rgb(166, 227, 161),
    accent_three: Color32::from_rgb(250, 179, 135),
    accent_four: Color32::from_rgb(243, 139, 168),
    accent_five: Color32::from_rgb(249, 226, 175),
    accent_six: Color32::from_rgb(245, 224, 220),
    accent_seven: Color32::from_rgb(235, 160, 172),

    active_selection_fill: Color32::from_rgb(29, 49, 161),

    is_light: false,
};

/// Light theme (Catppuccin Latte)
pub const LIGHT: Theme = Theme {
    // Base
    base: Color32::from_rgb(239, 241, 245),
    mantle: Color32::from_rgb(230, 233, 239),
    crust: Color32::from_rgb(220, 224, 232),

    // Surface
    surface0: Color32::from_rgb(204, 208, 218),
    surface1: Color32::from_rgb(188, 192, 204),
    surface2: Color32::from_rgb(172, 176, 190),

    // Overlay
    overlay0: Color32::from_rgb(156, 160, 176),
    overlay1: Color32::from_rgb(140, 143, 161),
    overlay2: Color32::from_rgb(124, 127, 147),

    // Text
    // text: Color32::from_rgb(76, 79, 105),
    text: Color32::from_rgb(0, 0, 0),
    subtext0: Color32::from_rgb(108, 111, 133),
    subtext1: Color32::from_rgb(92, 95, 119),

    // Accents
    accent_one: Color32::from_rgb(30, 102, 245),
    accent_two: Color32::from_rgb(64, 160, 43),
    accent_three: Color32::from_rgb(254, 100, 11),
    accent_four: Color32::from_rgb(210, 15, 57),
    accent_five: Color32::from_rgb(223, 142, 29),
    accent_six: Color32::from_rgb(220, 138, 120),
    accent_seven: Color32::from_rgb(230, 69, 83),

    active_selection_fill: Color32::from_rgb(173, 199, 255),

    is_light: true,
};

impl Theme {
    /// Convert this theme to egui::Visuals
    pub fn visuals(&self, old: egui::Visuals) -> egui::Visuals {
        let shadow_color = if self.is_light {
            Color32::from_black_alpha(25)
        } else {
            Color32::from_black_alpha(96)
        };

        egui::Visuals {
            dark_mode: !self.is_light,
            override_text_color: None, // Let widgets control text color via fg_stroke
            weak_text_color: Some(self.overlay0), // More muted for hint/placeholder text
            hyperlink_color: self.accent_six,
            faint_bg_color: self.surface0,
            extreme_bg_color: self.crust,
            code_bg_color: self.mantle,
            warn_fg_color: self.accent_three,
            error_fg_color: self.accent_seven,
            window_fill: self.base,
            panel_fill: self.base,
            window_stroke: Stroke {
                color: self.overlay1,
                ..old.window_stroke
            },
            widgets: style::Widgets {
                noninteractive: self.make_widget_visual(old.widgets.noninteractive, self.base),
                inactive: self.make_widget_visual(old.widgets.inactive, self.surface0),
                hovered: self.make_widget_visual(old.widgets.hovered, self.surface2),
                active: self.make_widget_visual(old.widgets.active, self.surface1),
                open: self.make_widget_visual(old.widgets.open, self.surface0),
            },
            selection: style::Selection {
                bg_fill: self.active_selection_fill,
                stroke: Stroke {
                    color: self.text,
                    ..old.selection.stroke
                },
            },
            window_shadow: egui::Shadow {
                color: shadow_color,
                ..old.window_shadow
            },
            popup_shadow: egui::Shadow {
                color: shadow_color,
                ..old.popup_shadow
            },
            ..old
        }
    }

    fn make_widget_visual(
        &self,
        old: style::WidgetVisuals,
        bg_fill: Color32,
    ) -> style::WidgetVisuals {
        style::WidgetVisuals {
            bg_fill,
            weak_bg_fill: bg_fill,
            bg_stroke: Stroke {
                color: self.overlay1,
                ..old.bg_stroke
            },
            fg_stroke: Stroke {
                color: self.text,
                ..old.fg_stroke
            },
            ..old
        }
    }

    // Convenience accessors for syntax highlighting (maps to old field names)
    pub fn syntax_comment(&self) -> Color32 {
        self.overlay0
    }
    pub fn syntax_reference(&self) -> Color32 {
        self.accent_one
    }
    pub fn syntax_slot(&self) -> Color32 {
        self.accent_two
    }
    pub fn syntax_option(&self) -> Color32 {
        self.accent_three
    }
    pub fn syntax_brace(&self) -> Color32 {
        self.overlay2
    }
    pub fn syntax_error(&self) -> Color32 {
        self.accent_four
    }

    // UI element colors
    pub fn chip_bg(&self) -> Color32 {
        // Use accent color with transparency for better contrast
        self.accent_one.gamma_multiply(0.35)
    }
    pub fn focus_bg(&self) -> Color32 {
        self.surface1
    }
    pub fn cursor(&self) -> Color32 {
        self.text
    }
    pub fn match_highlight(&self) -> Color32 {
        self.accent_five
    }
    pub fn muted(&self) -> Color32 {
        self.overlay0
    }

    // Selection backgrounds (for tabs, sidebar items)
    pub fn active_selected_bg(&self) -> Color32 {
        self.active_selection_fill
    }
    pub fn active_selected_stroke(&self) -> Color32 {
        Color32::TRANSPARENT
    }
    pub fn selected_bg(&self) -> Color32 {
        self.surface0
    }
    pub fn selected_stroke(&self) -> Color32 {
        self.surface1
    }
}

/// Get the current theme based on dark/light mode
pub fn current(ctx: &egui::Context) -> Theme {
    if ctx.style().visuals.dark_mode {
        DARK
    } else {
        LIGHT
    }
}

/// Apply the theme visuals to the context
pub fn apply_theme(ctx: &egui::Context, theme: &Theme) {
    let old_visuals = ctx.style().visuals.clone();
    let new_visuals = theme.visuals(old_visuals);
    ctx.set_visuals(new_visuals);
}

/// Ensure the catppuccin theme is applied. Call this once at startup.
/// After initial application, egui's dark_mode toggle will work automatically
/// since the visuals are properly set up.
pub fn ensure_theme_applied(ctx: &egui::Context) {
    // Check if our theme has been applied by checking panel_fill color
    let current_visuals = &ctx.style().visuals;
    let expected_base = if current_visuals.dark_mode {
        DARK.base
    } else {
        LIGHT.base
    };

    // If panel_fill doesn't match our expected base, apply the theme
    if current_visuals.panel_fill != expected_base {
        let theme = current(ctx);
        apply_theme(ctx, &theme);
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
    let desired_stroke = Stroke::new(2.0, theme.cursor());

    // Only update if the stroke color doesn't match (avoid unnecessary style updates)
    if current_stroke.color != desired_stroke.color || current_stroke.width != desired_stroke.width
    {
        let mut style = (*ctx.style()).clone();
        style.visuals.text_cursor.stroke = desired_stroke;
        ctx.set_style(style);
    }
}
