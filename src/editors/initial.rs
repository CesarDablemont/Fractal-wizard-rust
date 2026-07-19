use eframe::egui::{self, pos2, Color32, Pos2, Shape, Vec2};
use crate::scene::camera::Camera;
use crate::scene::canvas::CanvasRenderer;
use crate::types::{Line, ShapePatternData};
use crate::file_io;
use crate::gizmo::{self, GizmoHit};
use super::shared;

pub struct InitialEditor {
    pub shapes: Vec<ShapePatternData>,

    pub transfer_shapes: Option<Vec<ShapePatternData>>,
    pub receive_figure: Option<(Vec<Pos2>, Vec<Line>)>,

    model_points: Vec<Pos2>,
    model_lines: Vec<Line>,

    camera: Camera,
    canvas_renderer: CanvasRenderer,
    gizmo_hit: GizmoHit,
    gizmo_dragging: bool,
    show_gizmo: bool,
    selected: Vec<usize>,
    message: Option<String>,
}

impl Default for InitialEditor {
    fn default() -> Self {
        let (mp, ml) = shared::default_model();
        Self {
            shapes: Vec::new(),
            transfer_shapes: None,
            receive_figure: None,
            model_points: mp,
            model_lines: ml,
            camera: Camera::default(),
            canvas_renderer: CanvasRenderer::new(),
            gizmo_hit: GizmoHit::None,
            gizmo_dragging: false,
            show_gizmo: true,
            selected: Vec::new(),
            message: None,
        }
    }
}

impl InitialEditor {
    pub fn render(&mut self, ctx: &egui::Context) {
        if let Some((pts, lns)) = self.receive_figure.take() {
            self.model_points = pts;
            self.model_lines = lns;
            self.shapes = vec![ShapePatternData {
                translate: pos2(0.0, 0.0),
                rotate: 0.0,
                scale: 1.0,
            }];
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
        shared::load_model(content, &mut self.model_points, &mut self.model_lines)
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

            ui.menu_button("Options", |ui| {
                ui.checkbox(&mut self.show_gizmo, "Gizmo");
                ui.checkbox(&mut self.camera.magnetism, "Magnétisme");
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

    fn render_canvas(&mut self, ui: &mut egui::Ui) {
        let (response, painter) = ui.allocate_painter(
            ui.available_size(),
            egui::Sense::click_and_drag(),
        );
        let canvas_rect = response.rect;
        let canvas_center = canvas_rect.center();
        let mut shapes: Vec<Shape> = Vec::new();

        shared::handle_zoom_scroll(&response, ui, &mut self.camera, canvas_center);
        shared::handle_middle_pan(&response, ui, &mut self.camera);

        self.canvas_renderer.draw_grid(&self.camera, canvas_rect, &mut shapes);
        self.canvas_renderer.draw_origin(&self.camera, canvas_rect, &mut shapes);

        for (i, p) in self.shapes.iter().enumerate() {
            let is_selected = self.selected.contains(&i);
            let color = if is_selected { Color32::WHITE } else { Color32::LIGHT_BLUE };
            shared::render_shape_at(
                &self.model_points, &self.model_lines,
                &self.camera, canvas_center,
                p.translate, p.rotate, 1.0 / p.scale,
                color,
                &mut shapes,
            );
        }

        let translates: Vec<Pos2> = self.shapes.iter().map(|s| s.translate).collect();

        shared::handle_draw_gizmo(
            ui, &self.camera, canvas_center,
            self.show_gizmo, self.gizmo_dragging,
            &self.selected, &translates,
            &mut self.gizmo_hit, &mut shapes,
        );

        shared::handle_primary_click_selection(
            &response, ui,
            self.show_gizmo, self.gizmo_hit,
            &translates, &self.camera, canvas_center,
            self.camera.point_size,
            &mut self.selected,
        );

        let pointer_pressed = ui.input(|i| i.pointer.any_pressed());
        let pointer_released = ui.input(|i| i.pointer.any_released());
        let half = self.camera.point_size;

        if self.gizmo_dragging {
            if pointer_released {
                self.gizmo_dragging = false;
                if self.camera.magnetism {
                    if let Some(&idx) = self.selected.first() {
                        if idx < self.shapes.len() {
                            let s = &self.shapes[idx];
                            let offset = shared::snap_translation(
                                &self.model_points, s.translate, s.rotate, 1.0 / s.scale,
                                self.camera.zoom,
                            );
                            self.shapes[idx].translate += offset;
                        }
                    }
                }
            } else {
                let delta = ui.input(|i| i.pointer.delta());
                if delta != Vec2::ZERO {
                    let world_delta = gizmo::Gizmo::handle_drag(self.gizmo_hit, delta, &self.camera);
                    if let Some(&idx) = self.selected.first() {
                        if idx < self.shapes.len() {
                            self.shapes[idx].translate += world_delta;
                        }
                    }
                }
            }
        } else if pointer_pressed && self.show_gizmo && self.gizmo_hit != GizmoHit::None {
            self.gizmo_dragging = true;
        } else if let Some(&idx) = self.selected.first() {
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
                if let Some(idx) = shared::iter_hit_test(&translates, mouse, &self.camera, canvas_center, half) {
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
                shared::render_transform_properties(
                    ui,
                    &format!("Initial {}", idx + 1),
                    &mut p.translate,
                    &mut p.rotate,
                    &mut p.scale,
                );
            }
        }
    }
}
