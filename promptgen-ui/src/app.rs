use std::path::PathBuf;

use crate::components::{EditorPanel, PreviewPanel, SidebarPanel, SlotPanel};
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
        // Apply custom font sizes
        theme::apply_font_sizes(&cc.egui_ctx);

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
                let open_dialog = SidebarPanel::show(ui, &mut self.state, &self.workspace_path);

                #[cfg(not(target_arch = "wasm32"))]
                if open_dialog {
                    self.open_workspace_dialog();
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

        // Bottom slot panel (only show if there are slots)
        let has_slots = !self.state.get_slot_definitions().is_empty();
        if has_slots {
            egui::TopBottomPanel::bottom("slots")
                .resizable(true)
                .default_height(150.0)
                .height_range(80.0..=400.0)
                .show(ctx, |ui| {
                    ui.heading("Slots");
                    ui.separator();
                    SlotPanel::show(ui, &mut self.state);
                });
        }

        // Central editor panel
        egui::CentralPanel::default().show(ctx, |ui| {
            EditorPanel::show(ui, &mut self.state);
        });
    }
}
