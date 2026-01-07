//! Shared prompt menu component for sidebar and tab bar.

use egui_material_icons::icons::{ICON_DELETE, ICON_EDIT, ICON_MORE_VERT};

/// Actions that can result from the prompt menu
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PromptMenuAction {
    /// No action
    None,
    /// Rename the prompt
    Rename,
    /// Delete the prompt
    Delete,
}

/// Shared prompt menu component.
/// Renders a "..." button that opens a menu with Rename and Delete options.
pub struct PromptMenu;

impl PromptMenu {
    /// Show the prompt menu button.
    /// Returns the action selected by the user.
    pub fn show(ui: &mut egui::Ui, can_delete: bool) -> PromptMenuAction {
        let mut action = PromptMenuAction::None;

        ui.menu_button(ICON_MORE_VERT, |ui| {
            ui.set_min_width(120.0);

            if ui.button(format!("{} Rename", ICON_EDIT)).clicked() {
                action = PromptMenuAction::Rename;
                ui.close();
            }

            if can_delete
                && ui
                    .button(
                        egui::RichText::new(format!("{} Delete", ICON_DELETE))
                            .color(egui::Color32::from_rgb(243, 139, 168)), // Catppuccin red
                    )
                    .clicked()
                {
                    action = PromptMenuAction::Delete;
                    ui.close();
                }
        })
        .response
        .on_hover_text("More options");

        action
    }
}
