use eframe::egui::{self, Color32, Pos2};
use serde::{Deserialize, Serialize};
use crate::scene::camera::CameraSettings;
use crate::types::{Line, ShapePatternData};
use crate::file_io;
use super::shared;
use super::shape_list::{ShapeListEditor, ShapeListStyle};

const STYLE: ShapeListStyle = ShapeListStyle {
    item_label: "Initial",
    outliner_title: "Figures initiales",
    empty_label: "Aucune figure initiale",
    new_label: "Nouveau",
    nothing_to_send: "Aucune figure initiale à envoyer",
    color: Color32::LIGHT_BLUE,
};

pub struct InitialEditor {
    pub transfer_shapes: Option<Vec<ShapePatternData>>,
    pub receive_figure: Option<(Vec<Pos2>, Vec<Line>)>,

    list: ShapeListEditor<()>,
}

/// Réglages du menu `Options`, mémorisés entre deux lancements.
#[derive(Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct InitialSettings {
    pub show_gizmo: bool,
    pub camera: CameraSettings,
}

impl Default for InitialSettings {
    fn default() -> Self {
        InitialEditor::default().settings()
    }
}

impl Default for InitialEditor {
    fn default() -> Self {
        Self {
            transfer_shapes: None,
            receive_figure: None,
            list: ShapeListEditor::new(STYLE, ()),
        }
    }
}

impl InitialEditor {
    pub fn settings(&self) -> InitialSettings {
        InitialSettings {
            show_gizmo: self.list.show_gizmo,
            camera: self.list.camera.settings(),
        }
    }

    pub fn apply_settings(&mut self, settings: &InitialSettings) {
        self.list.show_gizmo = settings.show_gizmo;
        self.list.camera.apply_settings(&settings.camera);
    }

    pub fn render(&mut self, ctx: &egui::Context) {
        if let Some((pts, lns)) = self.receive_figure.take() {
            self.list.push_undo();
            self.list.model_points = pts;
            self.list.model_lines = lns;
            self.list.shapes = vec![ShapePatternData::default()];
        }

        self.list.handle_delete_key(ctx);

        egui::TopBottomPanel::top("initial_editor_menu").show(ctx, |ui| {
            self.render_menu(ui);
        });

        egui::SidePanel::left("initial_outliner")
            .resizable(true)
            .default_width(200.0)
            .show(ctx, |ui| {
                self.list.render_outliner(ui);
            });

        egui::CentralPanel::default().show(ctx, |ui| {
            self.list.render_canvas(ui, false);
        });

        egui::SidePanel::right("initial_properties")
            .resizable(true)
            .default_width(200.0)
            .show(ctx, |ui| {
                ui.heading("Propriétés");
                self.list.model_info_label(ui);
                self.list.render_selection_properties(ui);
            });
    }

    fn render_menu(&mut self, ui: &mut egui::Ui) {
        ui.horizontal(|ui| {
            ui.menu_button("Fichier", |ui| {
                if ui.button("Ouvrir (tilfw)").clicked() {
                    self.list.push_undo();
                    if let Some((_path, content)) = file_io::open_json("Ouvrir un fichier initial", "filfw") {
                        match serde_json::from_str::<Vec<ShapePatternData>>(&content) {
                            Ok(data) => {
                                self.list.shapes = data;
                                self.list.info("Fichier initial chargé");
                            }
                            Err(e) => self.list.error(e.to_string()),
                        }
                    }
                    ui.close_menu();
                }
                if ui.button("Enregistrer (tilfw)").clicked() {
                    let json = serde_json::to_string_pretty(&self.list.shapes).unwrap();
                    if file_io::save_json("Enregistrer le fichier initial", "filfw", &json) {
                        self.list.info("Fichier initial enregistré");
                    }
                    ui.close_menu();
                }
            });

            self.list.model_menu(ui);

            ui.menu_button("Options", |ui| {
                shared::view_options(ui, &mut self.list.camera);
                shared::edit_options(ui, &mut self.list.show_gizmo, &mut self.list.camera);
            });

            if self.list.send_button(ui) {
                self.transfer_shapes = Some(self.list.shapes.clone());
            }

            self.list.edit_buttons(ui);

            shared::render_status_message(ui, &mut self.list.message);
        });
    }
}
