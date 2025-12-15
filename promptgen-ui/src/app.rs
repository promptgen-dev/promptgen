use crate::state::AppState;
use crate::theme;

/// Main application struct - implements eframe::App
#[derive(serde::Deserialize, serde::Serialize)]
#[serde(default)]
pub struct PromptGenApp {
    #[serde(skip)]
    state: AppState,
}

impl Default for PromptGenApp {
    fn default() -> Self {
        Self {
            state: AppState::default(),
        }
    }
}

impl PromptGenApp {
    /// Called once before the first frame.
    pub fn new(cc: &eframe::CreationContext<'_>) -> Self {
        // Apply Catppuccin Mocha theme
        theme::apply_theme(&cc.egui_ctx);

        // Load previous app state (if any).
        if let Some(storage) = cc.storage {
            return eframe::get_value(storage, eframe::APP_KEY).unwrap_or_default();
        }

        Self::default()
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
                let is_web = cfg!(target_arch = "wasm32");
                if !is_web {
                    ui.menu_button("File", |ui| {
                        if ui.button("Open Workspace...").clicked() {
                            // TODO: Open folder picker
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
                ui.heading("Workspace");
                ui.separator();

                ui.label("No workspace selected");
                ui.add_space(8.0);

                #[cfg(not(target_arch = "wasm32"))]
                if ui.button("Select Folder...").clicked() {
                    // TODO: Open folder picker
                }

                ui.with_layout(egui::Layout::bottom_up(egui::Align::LEFT), |ui| {
                    ui.horizontal(|ui| {
                        ui.spacing_mut().item_spacing.x = 0.0;
                        ui.label("Powered by ");
                        ui.hyperlink_to("egui", "https://github.com/emilk/egui");
                    });
                    egui::warn_if_debug_build(ui);
                });
            });

        // Right preview panel
        egui::SidePanel::right("preview")
            .resizable(true)
            .default_width(300.0)
            .width_range(200.0..=500.0)
            .show(ctx, |ui| {
                ui.heading("Preview");
                ui.separator();

                ui.label("Select a template to preview");
            });

        // Central editor panel
        egui::CentralPanel::default().show(ctx, |ui| {
            ui.heading("Editor");
            ui.separator();

            ui.add(
                egui::TextEdit::multiline(&mut self.state.editor_content)
                    .desired_width(f32::INFINITY)
                    .desired_rows(20)
                    .font(egui::TextStyle::Monospace)
                    .hint_text("Enter your prompt template here...\n\nUse @GroupName to reference variables.\nUse {option1|option2|option3} for inline choices.\nUse {{ slot_name }} for user-filled slots."),
            );
        });
    }
}
