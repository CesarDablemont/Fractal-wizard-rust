use eframe::egui::{self, Color32, Pos2, Shape, Stroke, Vec2};
use serde::{Deserialize, Serialize};
use crate::scene::camera::Camera;
use crate::scene::canvas::CanvasRenderer;
use crate::shapes::shape::apply_transform;
use crate::types::{Line, ShapePatternData};
use crate::file_io;

#[derive(Serialize, Deserialize)]
struct ModelData {
    r#type: String,
    points: Vec<[f32; 2]>,
    #[serde(skip_serializing_if = "Vec::is_empty", default)]
    lines: Vec<Line>,
}

pub struct InitialEditor {
    pub shapes: Vec<ShapePatternData>,

    pub transfer_shapes: Option<Vec<ShapePatternData>>,
    pub receive_figure: Option<(Vec<Pos2>, Vec<Line>)>,

    model_points: Vec<Pos2>,
    model_lines: Vec<Line>,

    camera: Camera,
    canvas_renderer: CanvasRenderer,
    selected: Vec<usize>,
    message: Option<String>,
}

fn default_model() -> (Vec<Pos2>, Vec<Line>) {
    let pts = vec![
        Pos2::new(-0.5, -0.5),
        Pos2::new(0.5, -0.5),
        Pos2::new(0.5, 0.5),
        Pos2::new(-0.5, 0.5),
    ];
    let lines = vec![[0, 1], [1, 2], [2, 3], [3, 0]];
    (pts, lines)
}

impl Default for InitialEditor {
    fn default() -> Self {
        let (mp, ml) = default_model();
        Self {
            shapes: Vec::new(),
            transfer_shapes: None,
            receive_figure: None,
            model_points: mp,
            model_lines: ml,
            camera: Camera::default(),
            canvas_renderer: CanvasRenderer::new(),
            selected: Vec::new(),
            message: None,
        }
    }
}

impl InitialEditor {
    pub fn render(&mut self, ctx: &egui::Context) {
        if let Some((pts, lns)) = self.receive_figure.take() {
            self.model_points = pts.clone();
            self.model_lines = lns;
            self.shapes = pts.iter().map(|&p| ShapePatternData {
                translate: p,
                rotate: 0.0,
                scale: 1.0,
            }).collect();
        }

        egui::TopBottomPanel::top("initial_editor_menu").show(ctx, |ui| {
            self.render_menu(ui);
        });

        egui::SidePanel::left("initial_outliner")
            .resizable(true)
            .default_width(200.0)
            .show(ctx, |ui| {
                self.render_outliner(ui);
            });

        egui::CentralPanel::default().show(ctx, |ui| {
            self.render_canvas(ui);
        });

        egui::SidePanel::right("initial_properties")
            .resizable(true)
            .default_width(200.0)
            .show(ctx, |ui| {
                self.render_properties(ui);
            });
    }

    fn load_model(&mut self, content: &str) -> Result<(), String> {
        let data: ModelData = serde_json::from_str(content).map_err(|e| e.to_string())?;
        self.model_points = data.points.iter().map(|&p| Pos2::new(p[0], p[1])).collect();
        self.model_lines = data.lines;
        if self.model_lines.is_empty() && self.model_points.len() >= 2 {
            if data.r#type == "Polygon" || data.r#type == "cPolygon" {
                self.model_lines = (0..self.model_points.len() - 1)
                    .map(|i| [i, i + 1])
                    .collect();
                if self.model_points.len() > 2 {
                    self.model_lines.push([self.model_points.len() - 1, 0]);
                }
            } else {
                self.model_lines = (0..self.model_points.len() - 1)
                    .map(|i| [i, i + 1])
                    .collect();
            }
        }
        Ok(())
    }

    fn render_menu(&mut self, ui: &mut egui::Ui) {
        ui.horizontal(|ui| {
            ui.menu_button("Fichier", |ui| {
                if ui.button("Ouvrir (tilfw)").clicked() {
                    if let Some((_path, content)) = file_io::open_json("Ouvrir un fichier initial", "filfw") {
                        match serde_json::from_str::<Vec<ShapePatternData>>(&content) {
                            Ok(data) => {
                                self.shapes = data;
                                self.message = Some("Fichier initial chargé".into());
                            }
                            Err(e) => self.message = Some(format!("Erreur: {}", e)),
                        }
                    }
                    ui.close_menu();
                }
                if ui.button("Enregistrer (tilfw)").clicked() {
                    let json = serde_json::to_string_pretty(&self.shapes).unwrap();
                    if file_io::save_json("Enregistrer le fichier initial", "filfw", &json) {
                        self.message = Some("Fichier initial enregistré".into());
                    }
                    ui.close_menu();
                }
            });

            ui.menu_button("Modèle", |ui| {
                if ui.button("Ouvrir un modèle (firfw)").clicked() {
                    if let Some((_path, content)) = file_io::open_json("Ouvrir un modèle", "firfw") {
                        match self.load_model(&content) {
                            Ok(()) => self.message = Some("Modèle chargé".into()),
                            Err(e) => self.message = Some(format!("Erreur: {}", e)),
                        }
                    }
                    ui.close_menu();
                }
            });

            if !self.shapes.is_empty() {
                if ui.button("➡ Envoyer à Fractale").clicked() {
                    self.transfer_shapes = Some(self.shapes.clone());
                }
            }

            if ui.button("Nouveau").clicked() {
                self.shapes.push(ShapePatternData::default());
            }
            if ui.button("Dupliquer sélection").clicked() {
                let to_dup: Vec<_> = self.selected.clone();
                for &i in to_dup.iter().rev() {
                    if i < self.shapes.len() {
                        let dup = self.shapes[i].clone();
                        self.shapes.insert(i + 1, dup);
                    }
                }
            }
            if ui.button("Supprimer sélection").clicked() {
                let mut to_remove: Vec<usize> = self.selected.clone();
                to_remove.sort_unstable_by(|a, b| b.cmp(a));
                for &i in &to_remove {
                    if i < self.shapes.len() {
                        self.shapes.remove(i);
                    }
                }
                self.selected.clear();
            }

            if let Some(ref msg) = self.message {
                ui.separator();
                ui.label(msg);
            }
        });
    }

    fn render_outliner(&mut self, ui: &mut egui::Ui) {
        ui.heading("Figures initiales");
        for (i, p) in self.shapes.iter().enumerate() {
            let label = format!(
                "Initial {} : T({:.1}, {:.1}) R({:.1}°) S({:.2})",
                i + 1, p.translate.x, p.translate.y, p.rotate.to_degrees(), p.scale
            );
            let selected = self.selected.contains(&i);
            if ui.selectable_label(selected, &label).clicked() {
                if ui.input(|i| i.modifiers.ctrl) {
                    if selected {
                        self.selected.retain(|&x| x != i);
                    } else {
                        self.selected.push(i);
                    }
                } else {
                    self.selected = vec![i];
                }
            }
        }
        if self.shapes.is_empty() {
            ui.label("Aucune figure initiale");
        }
    }

    fn render_shape_at(
        &self,
        camera: &Camera,
        canvas_center: Pos2,
        translate: Pos2,
        rotate: f32,
        scale: f32,
        color: Color32,
        shapes: &mut Vec<Shape>,
    ) {
        if self.model_points.is_empty() {
            return;
        }
        let stroke = Stroke::new(1.5, color);
        let transformed: Vec<Pos2> = self
            .model_points
            .iter()
            .map(|&p| apply_transform(p, translate, rotate, Vec2::new(scale, scale)))
            .collect();
        for &[a, b] in &self.model_lines {
            if a < transformed.len() && b < transformed.len() {
                let p1 = camera.world_to_screen(transformed[a], canvas_center);
                let p2 = camera.world_to_screen(transformed[b], canvas_center);
                shapes.push(Shape::line_segment([p1, p2], stroke));
            }
        }
    }

    fn render_canvas(&mut self, ui: &mut egui::Ui) {
        let (response, painter) = ui.allocate_painter(
            ui.available_size(),
            egui::Sense::click_and_drag(),
        );

        let canvas_rect = response.rect;
        let canvas_center = canvas_rect.center();
        let mut shapes: Vec<Shape> = Vec::new();

        if response.hovered() {
            let scroll = ui.input(|i| i.raw_scroll_delta);
            if scroll.y != 0.0 {
                let factor = 1.15f32.powf(scroll.y / 10.0);
                self.camera.zoom_at(factor, ui.input(|i| i.pointer.hover_pos().unwrap_or(canvas_center)), canvas_center);
            }
        }
        if response.dragged_by(egui::PointerButton::Middle) {
            self.camera.pan(ui.input(|i| i.pointer.delta()));
        }

        self.canvas_renderer.draw_grid(&self.camera, canvas_rect, &mut shapes);
        self.canvas_renderer.draw_origin(&self.camera, canvas_rect, &mut shapes);

        // Draw each initial figure as the transformed shape
        for (i, p) in self.shapes.iter().enumerate() {
            let is_selected = self.selected.contains(&i);
            let color = if is_selected { Color32::WHITE } else { Color32::LIGHT_BLUE };
            self.render_shape_at(
                &self.camera, canvas_center,
                p.translate, p.rotate, 1.0 / p.scale,
                color,
                &mut shapes,
            );
        }

        // Canvas interaction
        let half = self.camera.point_size;

        if response.clicked_by(egui::PointerButton::Primary) {
            if let Some(mouse) = ui.input(|i| i.pointer.interact_pos()) {
                let hit = self.shapes.iter().position(|p| {
                    let screen = self.camera.world_to_screen(p.translate, canvas_center);
                    let d = screen - mouse;
                    d.x.abs() <= half && d.y.abs() <= half
                });
                if let Some(idx) = hit {
                    self.selected = vec![idx];
                } else {
                    self.selected.clear();
                }
            }
        }

        if let Some(&idx) = self.selected.first() {
            if response.dragged_by(egui::PointerButton::Primary) && idx < self.shapes.len() {
                let delta = ui.input(|i| i.pointer.delta());
                if delta != Vec2::ZERO {
                    let world_delta = self.camera.screen_delta_to_world(delta);
                    self.shapes[idx].translate += world_delta;
                }
            }
        } else if response.dragged_by(egui::PointerButton::Primary) {
            self.camera.pan(ui.input(|i| i.pointer.delta()));
        }

        if response.clicked_by(egui::PointerButton::Secondary) {
            if let Some(mouse) = ui.input(|i| i.pointer.interact_pos()) {
                if let Some(idx) = self.shapes.iter().position(|p| {
                    let screen = self.camera.world_to_screen(p.translate, canvas_center);
                    let d = screen - mouse;
                    d.x.abs() <= half && d.y.abs() <= half
                }) {
                    self.shapes.remove(idx);
                    self.selected.retain(|&x| x != idx);
                }
            }
        }

        painter.extend(shapes);
    }

    fn render_properties(&mut self, ui: &mut egui::Ui) {
        ui.heading("Propriétés");
        ui.label(format!("Modèle: {} pts, {} lignes", self.model_points.len(), self.model_lines.len()));

        if let Some(&idx) = self.selected.first() {
            if idx < self.shapes.len() {
                let p = &mut self.shapes[idx];
                ui.separator();
                ui.label(format!("Initial {}", idx + 1));

                let mut changed = false;
                let mut tx = p.translate.x;
                let mut ty = p.translate.y;
                ui.horizontal(|ui| {
                    ui.label("X:");
                    changed |= ui.add(egui::DragValue::new(&mut tx).speed(1.0)).changed();
                    ui.label("Y:");
                    changed |= ui.add(egui::DragValue::new(&mut ty).speed(1.0)).changed();
                });
                let mut deg = p.rotate.to_degrees();
                ui.horizontal(|ui| {
                    ui.label("Rotation:");
                    changed |= ui.add(egui::DragValue::new(&mut deg).speed(1.0).suffix("°")).changed();
                });
                ui.horizontal(|ui| {
                    ui.label("Scale:");
                    changed |= ui.add(egui::DragValue::new(&mut p.scale).speed(0.1).range(0.01..=10.0)).changed();
                });

                if changed {
                    p.translate.x = tx;
                    p.translate.y = ty;
                    p.rotate = deg.to_radians();
                }
            }
        }
    }
}
