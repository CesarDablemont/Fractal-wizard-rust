use eframe::egui::{self, Color32, Pos2};
use serde::{Deserialize, Serialize};
use crate::scene::camera::CameraSettings;
use crate::types::{Line, ShapePatternData};
use crate::file_io;
use super::shared;
use super::shape_list::{ShapeListEditor, ShapeListStyle};

const STYLE: ShapeListStyle = ShapeListStyle {
    item_label: "Pattern",
    outliner_title: "Patterns",
    empty_label: "Aucun pattern",
    new_label: "Nouveau pattern",
    nothing_to_send: "Aucun pattern à envoyer",
    color: Color32::YELLOW,
};

#[derive(Serialize, Deserialize)]
struct PatternFile {
    display_parent: bool,
    patterns: Vec<ShapePatternData>,
}

pub struct PatternEditor {
    pub transfer_patterns: Option<Vec<ShapePatternData>>,
    pub receive_figure: Option<(Vec<Pos2>, Vec<Line>)>,

    /// `extra` contient le `display_parent` du fichier pattern.
    list: ShapeListEditor<bool>,
    show_origin_figure: bool,
}

/// Réglages du menu `Options`, mémorisés entre deux lancements.
#[derive(Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct PatternSettings {
    pub show_origin_figure: bool,
    pub show_gizmo: bool,
    pub camera: CameraSettings,
}

impl Default for PatternSettings {
    fn default() -> Self {
        PatternEditor::default().settings()
    }
}

impl Default for PatternEditor {
    fn default() -> Self {
        Self {
            transfer_patterns: None,
            receive_figure: None,
            list: ShapeListEditor::new(STYLE, false),
            show_origin_figure: true,
        }
    }
}

impl PatternEditor {
    pub fn settings(&self) -> PatternSettings {
        PatternSettings {
            show_origin_figure: self.show_origin_figure,
            show_gizmo: self.list.show_gizmo,
            camera: self.list.camera.settings(),
        }
    }

    pub fn apply_settings(&mut self, settings: &PatternSettings) {
        self.show_origin_figure = settings.show_origin_figure;
        self.list.show_gizmo = settings.show_gizmo;
        self.list.camera.apply_settings(&settings.camera);
    }

    pub fn render(&mut self, ctx: &egui::Context) {
        if let Some((pts, lns)) = self.receive_figure.take() {
            self.list.model_points = pts;
            self.list.model_lines = lns;
        }

        self.list.handle_delete_key(ctx);

        egui::TopBottomPanel::top("pattern_editor_menu").show(ctx, |ui| {
            self.render_menu(ui);
        });

        egui::SidePanel::left("pattern_outliner")
            .resizable(true)
            .default_width(200.0)
            .show(ctx, |ui| {
                self.list.render_outliner(ui);
            });

        egui::CentralPanel::default().show(ctx, |ui| {
            self.list.render_canvas(ui, self.show_origin_figure);
        });

        egui::SidePanel::right("pattern_properties")
            .resizable(true)
            .default_width(200.0)
            .show(ctx, |ui| {
                self.render_properties(ui);
            });
    }

    fn render_menu(&mut self, ui: &mut egui::Ui) {
        ui.horizontal(|ui| {
            ui.menu_button("Fichier", |ui| {
                if ui.button("Ouvrir (ptnfw)").clicked() {
                    self.list.push_undo();
                    if let Some((_path, content)) = file_io::open_json("Ouvrir un pattern", "ptnfw") {
                        match serde_json::from_str::<PatternFile>(&content) {
                            Ok(data) => {
                                self.list.shapes = data.patterns;
                                self.list.extra = data.display_parent;
                                self.list.info("Pattern chargé");
                            }
                            Err(e) => self.list.error(e.to_string()),
                        }
                    }
                    ui.close_menu();
                }
                if ui.button("Enregistrer (ptnfw)").clicked() {
                    let data = PatternFile {
                        display_parent: self.list.extra,
                        patterns: self.list.shapes.clone(),
                    };
                    let json = serde_json::to_string_pretty(&data).unwrap();
                    if file_io::save_json("Enregistrer le pattern", "ptnfw", &json) {
                        self.list.info("Pattern enregistré");
                    }
                    ui.close_menu();
                }
            });

            self.list.model_menu(ui);

            ui.menu_button("Options", |ui| {
                shared::view_options(ui, &mut self.list.camera);
                ui.checkbox(&mut self.show_origin_figure, "Figure d'origine");
                shared::edit_options(ui, &mut self.list.show_gizmo, &mut self.list.camera);
            });

            if self.list.send_button(ui) {
                self.transfer_patterns = Some(self.list.shapes.clone());
            }

            self.list.edit_buttons(ui);

            shared::render_status_message(ui, &mut self.list.message);
        });
    }

    fn render_properties(&mut self, ui: &mut egui::Ui) {
        ui.heading("Propriétés");

        if let Some(dimension) = self.dimension() {
            ui.label(format!("Dimension estimée: {:.3}", dimension));
        }
        if !self.list.shapes.is_empty() {
            self.list.model_info_label(ui);
        }

        self.list.render_selection_properties(ui);
    }

    /// Dimension d'auto-similarité : log(N) / log(h), h étant l'échelle du premier pattern.
    fn dimension(&self) -> Option<f32> {
        let h = self.list.shapes.first()?.scale;
        (h > 0.0).then(|| (self.list.shapes.len() as f32).log10() / h.log10())
    }
}
