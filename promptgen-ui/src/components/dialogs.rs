//! Modal dialogs for library management

use std::path::PathBuf;

/// Actions that can result from the Create Library dialog
pub enum CreateLibraryAction {
    None,
    Create { name: String, path: PathBuf },
    Cancel,
}

/// Actions that can result from the Edit Library dialog
pub enum EditLibraryAction {
    None,
    Save { new_name: String },
    Cancel,
}

/// Actions that can result from the Overwrite Confirmation dialog
pub enum OverwriteConfirmAction {
    None,
    Confirm,
    Cancel,
}

/// Actions that can result from the Close Unsaved Tab dialog
pub enum CloseUnsavedTabAction {
    None,
    Save,
    DontSave,
    Cancel,
}

/// Render the Create Library dialog
pub fn render_create_library_dialog(
    ctx: &egui::Context,
    show: &mut bool,
    name: &mut String,
    path: &mut Option<PathBuf>,
) -> CreateLibraryAction {
    if !*show {
        return CreateLibraryAction::None;
    }

    let mut action = CreateLibraryAction::None;

    egui::Window::new("Create New Library")
        .collapsible(false)
        .resizable(false)
        .anchor(egui::Align2::CENTER_CENTER, [0.0, 0.0])
        .show(ctx, |ui| {
            ui.label("Library Name:");
            ui.text_edit_singleline(name);

            ui.add_space(8.0);

            ui.label("File Location:");
            ui.horizontal(|ui| {
                let path_display = path
                    .as_ref()
                    .map(|p| p.display().to_string())
                    .unwrap_or_else(|| "No location selected".to_string());
                ui.add(
                    egui::TextEdit::singleline(&mut path_display.as_str())
                        .interactive(false)
                        .desired_width(300.0),
                );

                if ui.button("...").clicked()
                    && let Some(selected_path) = rfd::FileDialog::new()
                        .set_title("Save Library File")
                        .add_filter("YAML files", &["yaml", "yml"])
                        .set_file_name(format!("{}.yaml", name))
                        .save_file()
                {
                    *path = Some(selected_path);
                }
            });

            ui.separator();

            ui.horizontal(|ui| {
                if ui.button("Cancel").clicked() {
                    action = CreateLibraryAction::Cancel;
                }

                let can_create = !name.trim().is_empty() && path.is_some();

                if ui
                    .add_enabled(can_create, egui::Button::new("Create Library"))
                    .clicked()
                {
                    action = CreateLibraryAction::Create {
                        name: name.trim().to_string(),
                        path: path.clone().unwrap(),
                    };
                }
            });
        });

    action
}

/// Render the Edit Library dialog
pub fn render_edit_library_dialog(
    ctx: &egui::Context,
    show: &mut bool,
    name: &mut String,
    current_library_name: &str,
) -> EditLibraryAction {
    if !*show {
        return EditLibraryAction::None;
    }

    let mut action = EditLibraryAction::None;

    egui::Window::new("Edit Library")
        .collapsible(false)
        .resizable(false)
        .anchor(egui::Align2::CENTER_CENTER, [0.0, 0.0])
        .show(ctx, |ui| {
            ui.label("Library Name:");
            ui.text_edit_singleline(name);

            ui.add_space(8.0);

            ui.separator();

            ui.horizontal(|ui| {
                if ui.button("Cancel").clicked() {
                    action = EditLibraryAction::Cancel;
                }

                let can_save = !name.trim().is_empty() && name.trim() != current_library_name;

                if ui
                    .add_enabled(can_save, egui::Button::new("Save"))
                    .clicked()
                {
                    action = EditLibraryAction::Save {
                        new_name: name.trim().to_string(),
                    };
                }
            });
        });

    action
}

/// Render the Overwrite Confirmation dialog
pub fn render_overwrite_confirm_dialog(
    ctx: &egui::Context,
    show: &mut bool,
    filename: &str,
) -> OverwriteConfirmAction {
    if !*show {
        return OverwriteConfirmAction::None;
    }

    let mut action = OverwriteConfirmAction::None;

    egui::Window::new("Confirm Overwrite")
        .collapsible(false)
        .resizable(false)
        .anchor(egui::Align2::CENTER_CENTER, [0.0, 0.0])
        .show(ctx, |ui| {
            ui.label(format!(
                "The library \"{}\" already exists in this folder.",
                filename
            ));
            ui.label("Would you like to overwrite it?");

            ui.add_space(8.0);
            ui.separator();

            ui.horizontal(|ui| {
                if ui.button("Cancel").clicked() {
                    action = OverwriteConfirmAction::Cancel;
                }

                if ui.button("Overwrite").clicked() {
                    action = OverwriteConfirmAction::Confirm;
                }
            });
        });

    action
}

/// Render the Close Unsaved Tab confirmation dialog
pub fn render_close_unsaved_tab_dialog(
    ctx: &egui::Context,
    tab_name: &str,
) -> CloseUnsavedTabAction {
    let mut action = CloseUnsavedTabAction::None;

    egui::Window::new("Unsaved Changes")
        .collapsible(false)
        .resizable(false)
        .anchor(egui::Align2::CENTER_CENTER, [0.0, 0.0])
        .show(ctx, |ui| {
            ui.label(format!(
                "\"{}\" has unsaved changes.",
                tab_name
            ));
            ui.label("Do you want to save before closing?");

            ui.add_space(8.0);
            ui.separator();

            ui.horizontal(|ui| {
                if ui.button("Cancel").clicked() {
                    action = CloseUnsavedTabAction::Cancel;
                }

                if ui.button("Don't Save").clicked() {
                    action = CloseUnsavedTabAction::DontSave;
                }

                if ui.button("Save").clicked() {
                    action = CloseUnsavedTabAction::Save;
                }
            });
        });

    action
}
