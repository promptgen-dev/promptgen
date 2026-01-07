//! Modal dialogs for library management

use std::path::PathBuf;

use egui_material_icons::icons::ICON_CLOSE;

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

/// Actions that can result from the Delete Prompt dialog
pub enum DeletePromptAction {
    None,
    Delete,
    Cancel,
}

/// Actions that can result from the Rename Prompt dialog
pub enum RenamePromptAction {
    None,
    Rename { new_name: String },
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
            ui.label(format!("\"{}\" has unsaved changes.", tab_name));
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

/// Render the Delete Prompt confirmation dialog
pub fn render_delete_prompt_dialog(ctx: &egui::Context, prompt_name: &str) -> DeletePromptAction {
    let mut action = DeletePromptAction::None;

    egui::Window::new("Delete Prompt")
        .collapsible(false)
        .resizable(false)
        .anchor(egui::Align2::CENTER_CENTER, [0.0, 0.0])
        .show(ctx, |ui| {
            ui.label(format!("Delete \"{}\"?", prompt_name));
            ui.label("This cannot be undone.");

            ui.add_space(8.0);
            ui.separator();

            ui.horizontal(|ui| {
                if ui.button("Cancel").clicked() {
                    action = DeletePromptAction::Cancel;
                }

                if ui
                    .button(
                        egui::RichText::new("Delete").color(egui::Color32::from_rgb(243, 139, 168)), // Catppuccin red
                    )
                    .clicked()
                {
                    action = DeletePromptAction::Delete;
                }
            });
        });

    action
}

/// Render the Rename Prompt dialog
pub fn render_rename_prompt_dialog(
    ctx: &egui::Context,
    current_name: &str,
    new_name: &mut String,
    is_valid: bool,
) -> RenamePromptAction {
    let mut action = RenamePromptAction::None;

    // Check validity before entering the closure to avoid borrow issues
    let has_text = !new_name.trim().is_empty();
    let show_error = !is_valid && has_text;
    let can_rename = is_valid && has_text;

    egui::Window::new("Rename Prompt")
        .collapsible(false)
        .resizable(false)
        .anchor(egui::Align2::CENTER_CENTER, [0.0, 0.0])
        .show(ctx, |ui| {
            ui.label(format!("Rename \"{}\":", current_name));

            ui.add_space(4.0);

            // Text input with validation styling
            let mut text_edit = egui::TextEdit::singleline(new_name).desired_width(250.0);

            if show_error {
                text_edit = text_edit.text_color(egui::Color32::from_rgb(243, 139, 168));
            }

            let response = if show_error {
                egui::Frame::new()
                    .stroke(egui::Stroke::new(
                        1.0,
                        egui::Color32::from_rgb(243, 139, 168),
                    ))
                    .corner_radius(4.0)
                    .show(ui, |ui| ui.add(text_edit))
                    .inner
            } else {
                ui.add(text_edit)
            };

            // Auto-focus the text field
            if response.gained_focus() || !response.has_focus() {
                response.request_focus();
            }

            // Show error message if invalid
            if show_error {
                ui.label(
                    egui::RichText::new("Name already exists or is invalid")
                        .small()
                        .color(egui::Color32::from_rgb(243, 139, 168)),
                );
            }

            ui.add_space(8.0);
            ui.separator();

            ui.horizontal(|ui| {
                if ui.button("Cancel").clicked() {
                    action = RenamePromptAction::Cancel;
                }

                if ui
                    .add_enabled(can_rename, egui::Button::new("Rename"))
                    .clicked()
                {
                    action = RenamePromptAction::Rename {
                        new_name: new_name.trim().to_string(),
                    };
                }
            });

            // Handle Enter to confirm
            if ui.input(|i| i.key_pressed(egui::Key::Enter)) && can_rename {
                action = RenamePromptAction::Rename {
                    new_name: new_name.trim().to_string(),
                };
            }

            // Handle Escape to cancel
            if ui.input(|i| i.key_pressed(egui::Key::Escape)) {
                action = RenamePromptAction::Cancel;
            }
        });

    action
}

/// Actions that can result from the Import Variables dialog
pub enum ImportVariablesAction {
    None,
    Import,
    Cancel,
    /// YAML text changed - needs reparsing
    YamlChanged,
    /// Renamed name changed for a specific variable
    RenamedChanged {
        index: usize,
        new_name: String,
    },
}

/// Render the Import Variables dialog
pub fn render_import_variables_dialog(
    ctx: &egui::Context,
    yaml_text: &mut String,
    parsed_variables: &[crate::state::ImportedVariable],
    parse_error: Option<&str>,
    can_import: bool,
    originally_conflicting: &[bool],
    currently_conflicting: &[bool],
) -> ImportVariablesAction {
    let mut action = ImportVariablesAction::None;

    egui::Window::new("Import Variables")
        .collapsible(false)
        .resizable(true)
        .default_width(450.0)
        .default_height(400.0)
        .anchor(egui::Align2::CENTER_CENTER, [0.0, 0.0])
        .show(ctx, |ui| {
            // Header with Import and Close buttons
            ui.horizontal(|ui| {
                ui.heading("Import Variables");

                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    // Close button
                    if ui.button(ICON_CLOSE).on_hover_text("Close").clicked() {
                        action = ImportVariablesAction::Cancel;
                    }

                    // Import button
                    if ui
                        .add_enabled(can_import, egui::Button::new("Import"))
                        .on_hover_text(if can_import {
                            "Import variables"
                        } else {
                            "Resolve all conflicts first"
                        })
                        .clicked()
                    {
                        action = ImportVariablesAction::Import;
                    }
                });
            });

            ui.add_space(8.0);

            // Conflicts section - show variables that originally had conflicts
            // (keep visible until Import, even if renamed to resolve)
            let conflicts: Vec<_> = parsed_variables
                .iter()
                .enumerate()
                .filter(|(idx, _)| originally_conflicting.get(*idx).copied().unwrap_or(false))
                .collect();

            // Status header (replaces static "Conflicts" label)
            if parse_error.is_none() && !parsed_variables.is_empty() {
                let unresolved_count = currently_conflicting.iter().filter(|&&c| c).count();
                let total = parsed_variables.len();
                let error_color = egui::Color32::from_rgb(243, 139, 168);
                let success_color = egui::Color32::from_rgb(166, 227, 161);

                if unresolved_count > 0 {
                    ui.label(
                        egui::RichText::new(format!(
                            "{} conflict{} remaining",
                            unresolved_count,
                            if unresolved_count == 1 { "" } else { "s" }
                        ))
                        .strong()
                        .color(error_color),
                    );
                } else {
                    ui.label(
                        egui::RichText::new(format!(
                            "{} variable{} ready to import",
                            total,
                            if total == 1 { "" } else { "s" }
                        ))
                        .strong()
                        .color(success_color),
                    );
                }
                ui.add_space(4.0);
            }

            if !conflicts.is_empty() {

                for (idx, var) in &conflicts {
                    let is_still_conflicting =
                        currently_conflicting.get(*idx).copied().unwrap_or(false);
                    let error_color = egui::Color32::from_rgb(243, 139, 168);
                    let success_color = egui::Color32::from_rgb(166, 227, 161); // Catppuccin green

                    // Vertical stack: Ours on top, Imported below
                    egui::Frame::new()
                        .fill(ui.visuals().faint_bg_color)
                        .corner_radius(4.0)
                        .inner_margin(8.0)
                        .show(ui, |ui| {
                            // Ours row
                            ui.horizontal(|ui| {
                                ui.label(
                                    egui::RichText::new("Ours:")
                                        .small()
                                        .color(ui.visuals().weak_text_color()),
                                );
                                ui.label(&var.original_name);
                            });

                            ui.add_space(4.0);

                            // Imported row with editable field
                            ui.horizontal(|ui| {
                                ui.label(
                                    egui::RichText::new("Import as:")
                                        .small()
                                        .color(ui.visuals().weak_text_color()),
                                );

                                let mut renamed = var.renamed_name.clone();

                                // Style based on current conflict status
                                let text_edit = if is_still_conflicting {
                                    egui::TextEdit::singleline(&mut renamed)
                                        .desired_width(200.0)
                                        .text_color(error_color)
                                } else {
                                    egui::TextEdit::singleline(&mut renamed)
                                        .desired_width(200.0)
                                        .text_color(success_color)
                                };

                                let stroke_color = if is_still_conflicting {
                                    error_color
                                } else {
                                    success_color
                                };

                                let response = egui::Frame::new()
                                    .stroke(egui::Stroke::new(1.0, stroke_color))
                                    .corner_radius(4.0)
                                    .show(ui, |ui| ui.add(text_edit))
                                    .inner;

                                if response.changed() {
                                    action = ImportVariablesAction::RenamedChanged {
                                        index: *idx,
                                        new_name: renamed,
                                    };
                                }
                            });
                        });

                    ui.add_space(4.0);
                }

                ui.add_space(4.0);
            }

            // Parse error display
            if let Some(error) = parse_error {
                ui.label(
                    egui::RichText::new(error)
                        .small()
                        .color(egui::Color32::from_rgb(243, 139, 168)),
                );
                ui.add_space(4.0);
            }

            // Paste Variables section
            ui.label(egui::RichText::new("Paste Variables").strong());
            ui.add_space(4.0);

            // YAML textarea
            egui::ScrollArea::vertical()
                .max_height(200.0)
                .show(ui, |ui| {
                    let response = ui.add(
                        egui::TextEdit::multiline(yaml_text)
                            .desired_width(f32::INFINITY)
                            .desired_rows(10)
                            .font(egui::TextStyle::Monospace)
                            .hint_text("Paste exported variables YAML here..."),
                    );

                    if response.changed() {
                        action = ImportVariablesAction::YamlChanged;
                    }
                });

            // Handle Escape to cancel
            if ui.input(|i| i.key_pressed(egui::Key::Escape)) {
                action = ImportVariablesAction::Cancel;
            }
        });

    action
}

/// Actions that can result from the Import Prompts dialog
pub enum ImportPromptsAction {
    None,
    Import,
    Cancel,
    /// YAML text changed - needs reparsing
    YamlChanged,
    /// Renamed name changed for a specific prompt
    RenamedChanged {
        index: usize,
        new_name: String,
    },
}

/// Render the Import Prompts dialog
pub fn render_import_prompts_dialog(
    ctx: &egui::Context,
    yaml_text: &mut String,
    parsed_prompts: &[crate::state::ImportedPrompt],
    parse_error: Option<&str>,
    can_import: bool,
    originally_conflicting: &[bool],
    currently_conflicting: &[bool],
) -> ImportPromptsAction {
    let mut action = ImportPromptsAction::None;

    egui::Window::new("Import Prompts")
        .collapsible(false)
        .resizable(true)
        .default_width(450.0)
        .default_height(400.0)
        .anchor(egui::Align2::CENTER_CENTER, [0.0, 0.0])
        .show(ctx, |ui| {
            // Header with Import and Close buttons
            ui.horizontal(|ui| {
                ui.heading("Import Prompts");

                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    // Close button
                    if ui.button(ICON_CLOSE).on_hover_text("Close").clicked() {
                        action = ImportPromptsAction::Cancel;
                    }

                    // Import button
                    if ui
                        .add_enabled(can_import, egui::Button::new("Import"))
                        .on_hover_text(if can_import {
                            "Import prompts"
                        } else {
                            "Resolve all conflicts first"
                        })
                        .clicked()
                    {
                        action = ImportPromptsAction::Import;
                    }
                });
            });

            ui.add_space(8.0);

            // Conflicts section - show prompts that originally had conflicts
            // (keep visible until Import, even if renamed to resolve)
            let conflicts: Vec<_> = parsed_prompts
                .iter()
                .enumerate()
                .filter(|(idx, _)| originally_conflicting.get(*idx).copied().unwrap_or(false))
                .collect();

            // Status header (replaces static "Conflicts" label)
            if parse_error.is_none() && !parsed_prompts.is_empty() {
                let unresolved_count = currently_conflicting.iter().filter(|&&c| c).count();
                let total = parsed_prompts.len();
                let error_color = egui::Color32::from_rgb(243, 139, 168);
                let success_color = egui::Color32::from_rgb(166, 227, 161);

                if unresolved_count > 0 {
                    ui.label(
                        egui::RichText::new(format!(
                            "{} conflict{} remaining",
                            unresolved_count,
                            if unresolved_count == 1 { "" } else { "s" }
                        ))
                        .strong()
                        .color(error_color),
                    );
                } else {
                    ui.label(
                        egui::RichText::new(format!(
                            "{} prompt{} ready to import",
                            total,
                            if total == 1 { "" } else { "s" }
                        ))
                        .strong()
                        .color(success_color),
                    );
                }
                ui.add_space(4.0);
            }

            if !conflicts.is_empty() {
                for (idx, prompt) in &conflicts {
                    let is_still_conflicting =
                        currently_conflicting.get(*idx).copied().unwrap_or(false);
                    let error_color = egui::Color32::from_rgb(243, 139, 168);
                    let success_color = egui::Color32::from_rgb(166, 227, 161); // Catppuccin green

                    // Vertical stack: Ours on top, Imported below
                    egui::Frame::new()
                        .fill(ui.visuals().faint_bg_color)
                        .corner_radius(4.0)
                        .inner_margin(8.0)
                        .show(ui, |ui| {
                            // Ours row
                            ui.horizontal(|ui| {
                                ui.label(
                                    egui::RichText::new("Ours:")
                                        .small()
                                        .color(ui.visuals().weak_text_color()),
                                );
                                ui.label(&prompt.original_name);
                            });

                            ui.add_space(4.0);

                            // Imported row with editable field
                            ui.horizontal(|ui| {
                                ui.label(
                                    egui::RichText::new("Import as:")
                                        .small()
                                        .color(ui.visuals().weak_text_color()),
                                );

                                let mut renamed = prompt.renamed_name.clone();

                                // Style based on current conflict status
                                let text_edit = if is_still_conflicting {
                                    egui::TextEdit::singleline(&mut renamed)
                                        .desired_width(200.0)
                                        .text_color(error_color)
                                } else {
                                    egui::TextEdit::singleline(&mut renamed)
                                        .desired_width(200.0)
                                        .text_color(success_color)
                                };

                                let stroke_color = if is_still_conflicting {
                                    error_color
                                } else {
                                    success_color
                                };

                                let response = egui::Frame::new()
                                    .stroke(egui::Stroke::new(1.0, stroke_color))
                                    .corner_radius(4.0)
                                    .show(ui, |ui| ui.add(text_edit))
                                    .inner;

                                if response.changed() {
                                    action = ImportPromptsAction::RenamedChanged {
                                        index: *idx,
                                        new_name: renamed,
                                    };
                                }
                            });
                        });

                    ui.add_space(4.0);
                }

                ui.add_space(4.0);
            }

            // Parse error display
            if let Some(error) = parse_error {
                ui.label(
                    egui::RichText::new(error)
                        .small()
                        .color(egui::Color32::from_rgb(243, 139, 168)),
                );
                ui.add_space(4.0);
            }

            // Paste Prompts section
            ui.label(egui::RichText::new("Paste Prompts").strong());
            ui.add_space(4.0);

            // YAML textarea
            egui::ScrollArea::vertical()
                .max_height(200.0)
                .show(ui, |ui| {
                    let response = ui.add(
                        egui::TextEdit::multiline(yaml_text)
                            .desired_width(f32::INFINITY)
                            .desired_rows(10)
                            .font(egui::TextStyle::Monospace)
                            .hint_text("Paste exported prompts YAML here..."),
                    );

                    if response.changed() {
                        action = ImportPromptsAction::YamlChanged;
                    }
                });

            // Handle Escape to cancel
            if ui.input(|i| i.key_pressed(egui::Key::Escape)) {
                action = ImportPromptsAction::Cancel;
            }
        });

    action
}
