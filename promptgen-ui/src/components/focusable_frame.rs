//! Reusable focusable frame component for consistent highlighting and click behavior.

use egui::{Color32, Response, Ui};

/// A frame that provides consistent focus highlighting and click-to-focus behavior.
///
/// This component wraps content in a styled frame that:
/// - Shows a highlighted background when focused
/// - Detects clicks anywhere in the frame to trigger focus
/// - Provides consistent margin and padding across all editors
pub struct FocusableFrame {
    is_focused: bool,
    inner_margin: f32,
    corner_radius: f32,
}

/// Response from rendering a FocusableFrame
pub struct FocusableFrameResponse<T> {
    /// The inner content returned by the closure
    pub inner: T,
    /// The full rectangle of the frame (for click detection)
    pub rect: egui::Rect,
    /// Whether the frame was clicked (outside of inner widget interactions)
    pub clicked: bool,
    /// The frame's response
    pub response: Response,
}

impl FocusableFrame {
    /// Create a new focusable frame
    pub fn new(is_focused: bool) -> Self {
        Self {
            is_focused,
            inner_margin: 8.0,
            corner_radius: 4.0,
        }
    }

    /// Set the inner margin (default: 8.0)
    #[allow(dead_code)]
    pub fn inner_margin(mut self, margin: f32) -> Self {
        self.inner_margin = margin;
        self
    }

    /// Set the corner radius (default: 4.0)
    #[allow(dead_code)]
    pub fn corner_radius(mut self, radius: f32) -> Self {
        self.corner_radius = radius;
        self
    }

    /// Show the focusable frame with the given content
    ///
    /// Returns a FocusableFrameResponse containing the inner content result,
    /// the frame rect, and whether the frame was clicked.
    pub fn show<R>(
        self,
        ui: &mut Ui,
        add_contents: impl FnOnce(&mut Ui) -> R,
    ) -> FocusableFrameResponse<R> {
        let fill_color = if self.is_focused {
            Color32::from_rgb(49, 50, 68) // Catppuccin surface1
        } else {
            Color32::TRANSPARENT
        };

        let frame = egui::Frame::NONE
            .inner_margin(self.inner_margin)
            .corner_radius(self.corner_radius)
            .fill(fill_color);

        let frame_response = frame.show(ui, add_contents);
        let rect = frame_response.response.rect;

        // Check for clicks on the frame area
        let clicked = ui.rect_contains_pointer(rect)
            && ui.input(|i| i.pointer.primary_clicked());

        FocusableFrameResponse {
            inner: frame_response.inner,
            rect,
            clicked,
            response: frame_response.response,
        }
    }
}
