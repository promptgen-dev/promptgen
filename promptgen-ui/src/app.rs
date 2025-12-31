use std::path::PathBuf;

use crate::components::{EditorPanel, PreviewPanel, SidebarPanel, SlotPanel, VariableEditorPanel};
use crate::state::{AppState, EditorMode};
use crate::theme;

#[cfg(not(target_arch = "wasm32"))]
use crate::storage::{NativeStorage, StorageBackend};

/// Main application struct - implements eframe::App
#[derive(Default, serde::Deserialize, serde::Serialize)]
#[serde(default)]
pub struct PromptGenApp {
    /// Persisted library file path
    library_file_path: Option<PathBuf>,

    #[serde(skip)]
    state: AppState,

    #[cfg(not(target_arch = "wasm32"))]
    #[serde(skip)]
    storage: NativeStorage,

    // Create Library dialog state (ephemeral)
    #[cfg(not(target_arch = "wasm32"))]
    #[serde(skip)]
    show_create_library_dialog: bool,

    #[cfg(not(target_arch = "wasm32"))]
    #[serde(skip)]
    create_library_name: String,

    #[cfg(not(target_arch = "wasm32"))]
    #[serde(skip)]
    create_library_path: Option<PathBuf>,
}

impl PromptGenApp {
    /// Called once before the first frame.
    pub fn new(cc: &eframe::CreationContext<'_>) -> Self {
        // Initialize material icons font
        egui_material_icons::initialize(&cc.egui_ctx);

        // Apply custom font sizes
        theme::apply_font_sizes(&cc.egui_ctx);

        // Load previous app state (if any).
        let mut app: Self = if let Some(storage) = cc.storage {
            eframe::get_value(storage, eframe::APP_KEY).unwrap_or_default()
        } else {
            Self::default()
        };

        // If we have a saved library path, try to load it
        #[cfg(not(target_arch = "wasm32"))]
        if let Some(path) = &app.library_file_path {
            app.storage.set_library_path(path.clone());
            app.load_library();
        }

        app
    }

    /// Open a file picker dialog and load the selected library
    #[cfg(not(target_arch = "wasm32"))]
    fn open_library_dialog(&mut self) {
        if let Some(path) = rfd::FileDialog::new()
            .set_title("Open Library File")
            .add_filter("YAML files", &["yaml", "yml"])
            .pick_file()
        {
            self.set_library_path(path);
        }
    }

    /// Set the library path and load it
    #[cfg(not(target_arch = "wasm32"))]
    fn set_library_path(&mut self, path: PathBuf) {
        self.library_file_path = Some(path.clone());
        self.storage.set_library_path(path);
        self.load_library();
    }

    /// Load the library from the current file path
    #[cfg(not(target_arch = "wasm32"))]
    fn load_library(&mut self) {
        match self.storage.load_library() {
            Ok((library, path)) => {
                self.state.library = library;
                self.state.library_path = Some(path);
            }
            Err(e) => {
                log::error!("Failed to load library: {}", e);
                self.state.library = promptgen_core::Library::default();
                self.state.library_path = None;
            }
        }
    }

    /// Show the create library dialog
    #[cfg(not(target_arch = "wasm32"))]
    fn render_create_library_dialog(&mut self, ctx: &egui::Context) {
        if !self.show_create_library_dialog {
            return;
        }

        egui::Window::new("Create New Library")
            .collapsible(false)
            .resizable(false)
            .anchor(egui::Align2::CENTER_CENTER, [0.0, 0.0])
            .show(ctx, |ui| {
                ui.label("Library Name:");
                ui.text_edit_singleline(&mut self.create_library_name);

                ui.add_space(8.0);

                ui.label("File Location:");
                ui.horizontal(|ui| {
                    let path_display = self
                        .create_library_path
                        .as_ref()
                        .map(|p| p.display().to_string())
                        .unwrap_or_else(|| "No location selected".to_string());
                    ui.add(
                        egui::TextEdit::singleline(&mut path_display.as_str())
                            .interactive(false)
                            .desired_width(300.0),
                    );

                    if ui.button("...").clicked() {
                        if let Some(path) = rfd::FileDialog::new()
                            .set_title("Save Library File")
                            .add_filter("YAML files", &["yaml", "yml"])
                            .set_file_name(&format!("{}.yaml", self.create_library_name))
                            .save_file()
                        {
                            self.create_library_path = Some(path);
                        }
                    }
                });

                ui.separator();

                ui.horizontal(|ui| {
                    if ui.button("Cancel").clicked() {
                        self.show_create_library_dialog = false;
                        self.create_library_name.clear();
                        self.create_library_path = None;
                    }

                    let can_create = !self.create_library_name.trim().is_empty()
                        && self.create_library_path.is_some();

                    if ui
                        .add_enabled(can_create, egui::Button::new("Create Library"))
                        .clicked()
                    {
                        self.create_library();
                    }
                });
            });
    }

    /// Create a new library file
    #[cfg(not(target_arch = "wasm32"))]
    fn create_library(&mut self) {
        let name = self.create_library_name.trim().to_string();
        let path = self.create_library_path.clone().unwrap();

        // TODO: Check if current library has unsaved changes and prompt user

        // Create empty library
        let library = promptgen_core::Library {
            name: name.clone(),
            description: String::new(),
            variables: Vec::new(),
            prompts: Vec::new(),
        };

        // Save to disk
        match promptgen_core::save_library(&library, &path) {
            Ok(()) => {
                // Load the new library
                self.state.library = library;
                self.state.library_path = Some(path.clone());
                self.library_file_path = Some(path);

                // Close dialog
                self.show_create_library_dialog = false;
                self.create_library_name.clear();
                self.create_library_path = None;
            }
            Err(e) => {
                log::error!("Failed to create library: {}", e);
                // TODO: Show error in dialog instead of just logging
            }
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
        // Ensure custom font sizes are applied (theme switches may reset them)
        theme::apply_font_sizes(ctx);

        // Ensure text cursor is visible (theme switches may change this)
        theme::ensure_cursor_visible(ctx);

        // Top menu bar
        egui::TopBottomPanel::top("top_panel").show(ctx, |ui| {
            egui::MenuBar::new().ui(ui, |ui| {
                #[cfg(not(target_arch = "wasm32"))]
                {
                    ui.menu_button("File", |ui| {
                        if ui.button("Create Library...").clicked() {
                            ui.close();
                            self.show_create_library_dialog = true;
                        }
                        if ui.button("Open Library...").clicked() {
                            ui.close();
                            self.open_library_dialog();
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
                let open_dialog = SidebarPanel::show(ui, &mut self.state, &self.library_file_path);

                #[cfg(not(target_arch = "wasm32"))]
                if open_dialog {
                    self.open_library_dialog();
                }

                #[cfg(target_arch = "wasm32")]
                let _ = open_dialog;
            });

        // Right preview panel
        egui::SidePanel::right("preview")
            .resizable(true)
            .default_width(300.0)
            .width_range(200.0..=500.0)
            .show(ctx, |ui| {
                PreviewPanel::show(ui, &mut self.state);
            });

        // Handle Escape key to close slot picker
        if ctx.input(|i| i.key_pressed(egui::Key::Escape)) {
            self.state.unfocus_slot();
        }

        // Central panel with unified scroll area for editor + slots
        egui::CentralPanel::default().show(ctx, |ui| {
            egui::ScrollArea::vertical()
                .id_salt("main_scroll")
                .auto_shrink([false, false])
                .show(ui, |ui| {
                    // Choose which editor to show based on editor mode
                    match &self.state.editor_mode {
                        EditorMode::Prompt => {
                            // Prompt editor section
                            EditorPanel::show(ui, &mut self.state);

                            // Slots section (only show if there are slots)
                            let has_slots = !self.state.get_slot_definitions().is_empty();
                            if has_slots {
                                ui.separator();
                                ui.heading("Slots");
                                SlotPanel::show(ui, &mut self.state);
                            }
                        }
                        EditorMode::VariableEditor { .. } | EditorMode::NewVariable => {
                            // Variable editor section
                            VariableEditorPanel::show(ui, &mut self.state);
                        }
                    }
                });
        });

        // Render create library dialog (if active)
        #[cfg(not(target_arch = "wasm32"))]
        self.render_create_library_dialog(ctx);
    }
}
