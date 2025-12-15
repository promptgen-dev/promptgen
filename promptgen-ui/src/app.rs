use std::path::PathBuf;

use crate::state::AppState;
use crate::theme;

#[cfg(not(target_arch = "wasm32"))]
use crate::storage::{NativeStorage, StorageBackend};

/// Main application struct - implements eframe::App
#[derive(Default, serde::Deserialize, serde::Serialize)]
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
        // Workspace header with folder picker
        #[cfg(not(target_arch = "wasm32"))]
        let open_dialog = {
            if let Some(path) = &self.workspace_path {
                ui.horizontal(|ui| {
                    ui.label("📁");
                    let folder_name = path
                        .file_name()
                        .map(|n| n.to_string_lossy().to_string())
                        .unwrap_or_else(|| path.display().to_string());
                    ui.label(&folder_name);
                    ui.small_button("...").clicked()
                }).inner
            } else {
                ui.button("Select Workspace...").clicked()
            }
        };

        #[cfg(not(target_arch = "wasm32"))]
        if open_dialog {
            self.open_workspace_dialog();
        }

        #[cfg(target_arch = "wasm32")]
        {
            ui.label("Web version");
        }

        ui.add_space(8.0);

        // Library selector (ComboBox)
        if !self.state.libraries.is_empty() {
            let selected_name = self
                .state
                .selected_library()
                .map(|lib| lib.name.clone())
                .unwrap_or_else(|| "Select library...".to_string());

            ui.horizontal(|ui| {
                ui.label("Library:");
                egui::ComboBox::from_id_salt("library_selector")
                    .selected_text(&selected_name)
                    .width(ui.available_width() - 8.0)
                    .show_ui(ui, |ui| {
                        for lib in &self.state.libraries {
                            let is_selected = self.state.selected_library_id.as_ref() == Some(&lib.id);
                            if ui.selectable_label(is_selected, &lib.name).clicked() {
                                self.state.selected_library_id = Some(lib.id.clone());
                            }
                        }
                    });
            });

            ui.add_space(8.0);

            // View mode toggle (Templates / Variables)
            ui.horizontal(|ui| {
                use crate::state::SidebarViewMode;

                if ui
                    .selectable_label(
                        self.state.sidebar_view_mode == SidebarViewMode::Templates,
                        "Templates",
                    )
                    .clicked()
                {
                    self.state.sidebar_view_mode = SidebarViewMode::Templates;
                }
                if ui
                    .selectable_label(
                        self.state.sidebar_view_mode == SidebarViewMode::Variables,
                        "Variables",
                    )
                    .clicked()
                {
                    self.state.sidebar_view_mode = SidebarViewMode::Variables;
                }
            });

            ui.add_space(4.0);

            // Search input
            ui.horizontal(|ui| {
                ui.label("🔍");
                ui.add(
                    egui::TextEdit::singleline(&mut self.state.search_query)
                        .hint_text("Search...")
                        .desired_width(ui.available_width() - 24.0),
                );
                if !self.state.search_query.is_empty() && ui.small_button("✕").clicked() {
                    self.state.search_query.clear();
                }
            });

            ui.separator();

            // Content list based on view mode
            self.render_sidebar_content(ui);
        } else if self.workspace_path.is_some() {
            ui.add_space(16.0);
            ui.label("No libraries found");
            ui.add_space(4.0);
            ui.label("Add .yaml library files to your workspace folder");
        } else {
            ui.add_space(16.0);
            ui.label("Select a workspace folder to get started");
        }

        // Footer
        ui.with_layout(egui::Layout::bottom_up(egui::Align::LEFT), |ui| {
            egui::warn_if_debug_build(ui);
        });
    }

    /// Render the sidebar content (templates or variables list)
    fn render_sidebar_content(&mut self, ui: &mut egui::Ui) {
        use crate::state::SidebarViewMode;

        egui::ScrollArea::vertical()
            .auto_shrink([false, false])
            .show(ui, |ui| {
                match self.state.sidebar_view_mode {
                    SidebarViewMode::Templates => self.render_template_list(ui),
                    SidebarViewMode::Variables => self.render_variable_list(ui),
                }
            });
    }

    /// Render the template list
    fn render_template_list(&mut self, ui: &mut egui::Ui) {
        let Some(library) = self.state.selected_library() else {
            ui.label("No library selected");
            return;
        };

        let search_query = self.state.search_query.to_lowercase();

        // Collect template info we need, releasing the borrow on self.state
        let templates: Vec<_> = library
            .templates
            .iter()
            .filter(|t| {
                search_query.is_empty()
                    || t.name.to_lowercase().contains(&search_query)
                    || t.description.to_lowercase().contains(&search_query)
            })
            .map(|t| (t.id.clone(), t.name.clone(), t.description.clone(), promptgen_core::template_to_source(&t.ast)))
            .collect();

        if templates.is_empty() {
            if search_query.is_empty() {
                ui.label("No templates in this library");
            } else {
                ui.label("No matching templates");
            }
            return;
        }

        let mut new_selected_id = self.state.selected_template_id.clone();
        let mut load_template_source: Option<String> = None;

        for (id, name, description, source) in &templates {
            let is_selected = new_selected_id.as_ref() == Some(id);
            let response = ui.selectable_label(is_selected, name);

            if response.clicked() {
                new_selected_id = Some(id.clone());
                load_template_source = Some(source.clone());
            }

            if !description.is_empty() {
                response.on_hover_text(description);
            }
        }

        self.state.selected_template_id = new_selected_id;

        // Apply template source after the loop (outside the borrow)
        if let Some(source) = load_template_source {
            self.state.editor_content = source;
            self.state.update_parse_result();
        }
    }

    /// Render the variable (group) list with expandable options
    fn render_variable_list(&mut self, ui: &mut egui::Ui) {
        let Some(library) = self.state.selected_library() else {
            ui.label("No library selected");
            return;
        };

        let search_query = self.state.search_query.to_lowercase();
        let groups: Vec<_> = library
            .groups
            .iter()
            .filter(|g| {
                search_query.is_empty()
                    || g.name.to_lowercase().contains(&search_query)
                    || g.options.iter().any(|o| o.to_lowercase().contains(&search_query))
            })
            .collect();

        if groups.is_empty() {
            if search_query.is_empty() {
                ui.label("No variables in this library");
            } else {
                ui.label("No matching variables");
            }
            return;
        }

        for group in groups {
            let header_text = format!("@{} ({})", group.name, group.options.len());

            egui::CollapsingHeader::new(&header_text)
                .default_open(false)
                .show(ui, |ui| {
                    for option in &group.options {
                        // Highlight matching text in search
                        if !search_query.is_empty() && option.to_lowercase().contains(&search_query) {
                            ui.colored_label(egui::Color32::from_rgb(166, 227, 161), format!("  • {}", option));
                        } else {
                            ui.label(format!("  • {}", option));
                        }
                    }
                });
        }
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
            egui::MenuBar::new().ui(ui, |ui| {
                #[cfg(not(target_arch = "wasm32"))]
                {
                    ui.menu_button("File", |ui| {
                        if ui.button("Open Workspace...").clicked() {
                            ui.close();
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
