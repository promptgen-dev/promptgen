use std::path::PathBuf;

use crate::state::AppState;
use crate::theme;

#[cfg(not(target_arch = "wasm32"))]
use crate::storage::{NativeStorage, StorageBackend};

/// Main application struct - implements eframe::App
#[derive(serde::Deserialize, serde::Serialize)]
#[serde(default)]
pub struct PromptGenApp {
    /// Persisted workspace path
    workspace_path: Option<PathBuf>,

    #[serde(skip)]
    state: AppState,

    #[cfg(not(target_arch = "wasm32"))]
    #[serde(skip)]
    storage: NativeStorage,
}

impl Default for PromptGenApp {
    fn default() -> Self {
        Self {
            workspace_path: None,
            state: AppState::default(),
            #[cfg(not(target_arch = "wasm32"))]
            storage: NativeStorage::new(),
        }
    }
}

impl PromptGenApp {
    /// Called once before the first frame.
    pub fn new(cc: &eframe::CreationContext<'_>) -> Self {
        // Apply Catppuccin Mocha theme
        theme::apply_theme(&cc.egui_ctx);

        // Load previous app state (if any).
        let mut app: Self = if let Some(storage) = cc.storage {
            eframe::get_value(storage, eframe::APP_KEY).unwrap_or_default()
        } else {
            Self::default()
        };

        // If we have a saved workspace path, try to load it
        #[cfg(not(target_arch = "wasm32"))]
        if let Some(path) = &app.workspace_path {
            app.storage.set_workspace_path(path.clone());
            app.load_libraries();
        }

        app
    }

    /// Open a folder picker dialog and load the selected workspace
    #[cfg(not(target_arch = "wasm32"))]
    fn open_workspace_dialog(&mut self) {
        if let Some(path) = rfd::FileDialog::new()
            .set_title("Select Workspace Folder")
            .pick_folder()
        {
            self.set_workspace_path(path);
        }
    }

    /// Set the workspace path and load libraries
    #[cfg(not(target_arch = "wasm32"))]
    fn set_workspace_path(&mut self, path: PathBuf) {
        self.workspace_path = Some(path.clone());
        self.storage.set_workspace_path(path);
        self.load_libraries();
    }

    /// Load all libraries from the current workspace
    #[cfg(not(target_arch = "wasm32"))]
    fn load_libraries(&mut self) {
        match self.storage.load_all_libraries() {
            Ok(libraries) => {
                self.state.libraries = libraries;
                self.state.rebuild_workspace();

                // Auto-select first library if none selected
                if self.state.selected_library_id.is_none() && !self.state.libraries.is_empty() {
                    self.state.selected_library_id = Some(self.state.libraries[0].id.clone());
                }
            }
            Err(e) => {
                log::error!("Failed to load libraries: {}", e);
                self.state.libraries.clear();
                self.state.rebuild_workspace();
            }
        }
    }

    /// Render the sidebar panel
    fn render_sidebar(&mut self, ui: &mut egui::Ui) {
        ui.heading("Workspace");
        ui.separator();

        // Show workspace path or "no workspace" message
        #[cfg(not(target_arch = "wasm32"))]
        {
            if let Some(path) = &self.workspace_path {
                ui.horizontal(|ui| {
                    ui.label("📁");
                    let folder_name = path
                        .file_name()
                        .map(|n| n.to_string_lossy().to_string())
                        .unwrap_or_else(|| path.display().to_string());
                    ui.label(&folder_name);
                });

                if ui.button("Change...").clicked() {
                    self.open_workspace_dialog();
                }
            } else {
                ui.label("No workspace selected");
                ui.add_space(8.0);
                if ui.button("Select Folder...").clicked() {
                    self.open_workspace_dialog();
                }
            }
        }

        #[cfg(target_arch = "wasm32")]
        {
            ui.label("Web version - workspace selection coming soon");
        }

        ui.add_space(16.0);
        ui.separator();

        // Library list
        if !self.state.libraries.is_empty() {
            ui.heading("Libraries");
            ui.add_space(4.0);

            egui::ScrollArea::vertical()
                .auto_shrink([false, false])
                .show(ui, |ui| {
                    let mut selected_id = self.state.selected_library_id.clone();

                    for lib in &self.state.libraries {
                        let is_selected = selected_id.as_ref() == Some(&lib.id);
                        let response = ui.selectable_label(is_selected, &lib.name);

                        if response.clicked() {
                            selected_id = Some(lib.id.clone());
                        }

                        // Show library info on hover
                        response.on_hover_ui(|ui| {
                            ui.label(format!("ID: {}", lib.id));
                            if !lib.description.is_empty() {
                                ui.label(&lib.description);
                            }
                            ui.label(format!("{} groups", lib.groups.len()));
                            ui.label(format!("{} templates", lib.templates.len()));
                        });
                    }

                    self.state.selected_library_id = selected_id;
                });
        } else if self.workspace_path.is_some() {
            ui.label("No libraries found in workspace");
            ui.add_space(4.0);
            ui.label("Add .yaml library files to your workspace folder");
        }

        // Footer
        ui.with_layout(egui::Layout::bottom_up(egui::Align::LEFT), |ui| {
            ui.horizontal(|ui| {
                ui.spacing_mut().item_spacing.x = 0.0;
                ui.label("Powered by ");
                ui.hyperlink_to("egui", "https://github.com/emilk/egui");
            });
            egui::warn_if_debug_build(ui);
        });
    }

    /// Render the preview panel
    fn render_preview(&mut self, ui: &mut egui::Ui) {
        ui.heading("Preview");
        ui.separator();

        if self.state.editor_content.is_empty() {
            ui.label("Enter a template in the editor to preview");
        } else {
            // Show parse result info
            if let Some(result) = &self.state.parse_result {
                if result.errors.is_empty() {
                    ui.colored_label(egui::Color32::from_rgb(166, 227, 161), "✓ Valid template");
                } else {
                    ui.colored_label(
                        egui::Color32::from_rgb(243, 139, 168),
                        format!("✗ {} error(s)", result.errors.len()),
                    );
                    for error in &result.errors {
                        ui.label(format!("  • {}", error.message));
                    }
                }
            }

            ui.add_space(8.0);

            // Preview output
            if !self.state.preview_output.is_empty() {
                ui.separator();
                ui.label("Output:");
                egui::ScrollArea::vertical()
                    .max_height(200.0)
                    .show(ui, |ui| {
                        ui.add(
                            egui::TextEdit::multiline(&mut self.state.preview_output.as_str())
                                .desired_width(f32::INFINITY)
                                .font(egui::TextStyle::Monospace),
                        );
                    });
            }
        }
    }

    /// Render the editor panel
    fn render_editor(&mut self, ui: &mut egui::Ui) {
        ui.heading("Editor");
        ui.separator();

        let response = ui.add(
            egui::TextEdit::multiline(&mut self.state.editor_content)
                .desired_width(f32::INFINITY)
                .desired_rows(20)
                .font(egui::TextStyle::Monospace)
                .hint_text(
                    "Enter your prompt template here...\n\n\
                     Use @GroupName to reference variables.\n\
                     Use {option1|option2|option3} for inline choices.\n\
                     Use {{ slot_name }} for user-filled slots.",
                ),
        );

        // Update parse result when editor content changes
        if response.changed() {
            self.state.update_parse_result();
        }
    }
}

impl eframe::App for PromptGenApp {
    /// Called by the framework to save state before shutdown.
    fn save(&mut self, storage: &mut dyn eframe::Storage) {
        eframe::set_value(storage, eframe::APP_KEY, self);
    }

    /// Called each time the UI needs repainting.
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        // Top menu bar
        egui::TopBottomPanel::top("top_panel").show(ctx, |ui| {
            egui::menu::bar(ui, |ui| {
                #[cfg(not(target_arch = "wasm32"))]
                {
                    ui.menu_button("File", |ui| {
                        if ui.button("Open Workspace...").clicked() {
                            ui.close_menu();
                            self.open_workspace_dialog();
                        }
                        ui.separator();
                        if ui.button("Quit").clicked() {
                            ctx.send_viewport_cmd(egui::ViewportCommand::Close);
                        }
                    });
                    ui.add_space(16.0);
                }

                egui::widgets::global_theme_preference_buttons(ui);
            });
        });

        // Left sidebar
        egui::SidePanel::left("sidebar")
            .resizable(true)
            .default_width(250.0)
            .width_range(180.0..=400.0)
            .show(ctx, |ui| {
                self.render_sidebar(ui);
            });

        // Right preview panel
        egui::SidePanel::right("preview")
            .resizable(true)
            .default_width(300.0)
            .width_range(200.0..=500.0)
            .show(ctx, |ui| {
                self.render_preview(ui);
            });

        // Central editor panel
        egui::CentralPanel::default().show(ctx, |ui| {
            self.render_editor(ui);
        });
    }
}
