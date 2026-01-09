//! Help system for displaying markdown documentation in popup windows.
//!
//! This module provides a pattern for embedding static markdown files and
//! displaying them in egui popup windows.
//!
//! # Usage
//!
//! 1. Add a new help topic by creating a markdown file and embedding it:
//!    ```rust,ignore
//!    pub const VARIABLES_HELP: &str = include_str!("../../help/variables.md");
//!    ```
//!
//! 2. Display help using the HelpWindow component:
//!    ```rust,ignore
//!    HelpWindow::new("variables_help", "Variables")
//!        .show(ctx, &mut show_help, VARIABLES_HELP, &mut cache);
//!    ```

use egui_commonmark::{CommonMarkCache, CommonMarkViewer};

/// Help content embedded at compile time.
/// Add new help topics here as static strings.
pub mod content {
    /// Example help content for demonstration
    pub const GETTING_STARTED: &str = include_str!("../../help/getting_started.md");
}

/// A popup window for displaying markdown help content.
pub struct HelpWindow {
    /// Unique ID for this help window
    id: &'static str,
    /// Window title
    title: &'static str,
    /// Minimum window size
    min_size: egui::Vec2,
    /// Default window size
    default_size: egui::Vec2,
}

impl HelpWindow {
    /// Create a new help window with the given ID and title.
    pub fn new(id: &'static str, title: &'static str) -> Self {
        Self {
            id,
            title,
            min_size: egui::vec2(400.0, 300.0),
            default_size: egui::vec2(500.0, 400.0),
        }
    }

    /// Set the minimum window size.
    #[allow(dead_code)]
    pub fn min_size(mut self, size: egui::Vec2) -> Self {
        self.min_size = size;
        self
    }

    /// Set the default window size.
    #[allow(dead_code)]
    pub fn default_size(mut self, size: egui::Vec2) -> Self {
        self.default_size = size;
        self
    }

    /// Show the help window if `open` is true.
    ///
    /// # Arguments
    /// * `ctx` - The egui context
    /// * `open` - Mutable reference to the open state (window can be closed by user)
    /// * `markdown` - The markdown content to display
    /// * `cache` - The CommonMark cache for rendering (can be shared across windows)
    pub fn show(
        &self,
        ctx: &egui::Context,
        open: &mut bool,
        markdown: &str,
        cache: &mut CommonMarkCache,
    ) {
        if !*open {
            return;
        }

        egui::Window::new(self.title)
            .id(egui::Id::new(self.id))
            .open(open)
            .resizable(true)
            .collapsible(true)
            .min_size(self.min_size)
            .default_size(self.default_size)
            .show(ctx, |ui| {
                egui::ScrollArea::vertical().show(ui, |ui| {
                    CommonMarkViewer::new().show(ui, cache, markdown);
                });
            });
    }
}

/// Convenience function to show a simple help window.
///
/// This is a simpler API when you don't need to customize the window.
#[allow(dead_code)]
pub fn show_help_window(
    ctx: &egui::Context,
    id: &'static str,
    title: &'static str,
    open: &mut bool,
    markdown: &str,
    cache: &mut CommonMarkCache,
) {
    HelpWindow::new(id, title).show(ctx, open, markdown, cache);
}
