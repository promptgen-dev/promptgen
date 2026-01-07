//! Variable menu component for Import/Export options.

use egui_material_icons::icons::{ICON_FILE_DOWNLOAD, ICON_FILE_UPLOAD, ICON_MORE_VERT};

/// Actions that can result from the variable menu
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum VariableMenuAction {
    None,
    Export,
    Import,
}

pub struct VariableMenu;

impl VariableMenu {
    /// Show the variable menu button and return the selected action.
    pub fn show(ui: &mut egui::Ui) -> VariableMenuAction {
        let mut action = VariableMenuAction::None;

        ui.menu_button(ICON_MORE_VERT, |ui| {
            ui.set_min_width(120.0);

            if ui
                .button(format!("{} Export", ICON_FILE_UPLOAD))
                .clicked()
            {
                action = VariableMenuAction::Export;
                ui.close();
            }

            if ui
                .button(format!("{} Import", ICON_FILE_DOWNLOAD))
                .clicked()
            {
                action = VariableMenuAction::Import;
                ui.close();
            }
        })
        .response
        .on_hover_text("Import/Export variables");

        action
    }
}
