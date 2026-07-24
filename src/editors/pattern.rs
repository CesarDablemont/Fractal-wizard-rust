use eframe::egui::{self, Color32, Pos2, Shape, Vec2};
use serde::{Deserialize, Serialize};
use crate::scene::camera::Camera;
use crate::scene::canvas::CanvasRenderer;
use crate::types::{Line, ShapePatternData};
use crate::file_io;
use crate::gizmo::{self, GizmoHit};
use super::shared;

#[derive(Serialize, Deserialize)]
struct PatternFile {
    display_parent: bool,
    patterns: Vec<ShapePatternData>,
}

pub struct PatternEditor {
    pub patterns: Vec<ShapePatternData>,
    pub display_parent: bool,
    pub dimension: f32,

    pub transfer_patterns: Option<Vec<ShapePatternData>>,
    pub receive_figure: Option<(Vec<Pos2>, Vec<Line>)>,

    model_points: Vec<Pos2>,
    model_lines: Vec<Line>,
    show_origin_figure: bool,

    camera: Camera,
    canvas_renderer: CanvasRenderer,
    gizmo_hit: GizmoHit,
    gizmo_dragging: bool,
    show_gizmo: bool,
    selected: Vec<usize>,
    message: Option<String>,
}

impl Default for PatternEditor {
    fn default() -> Self {
        let (mp, ml) = shared::default_model();
        Self {
            patterns: Vec::new(),
            display_parent: false,
            dimension: 0.0,
            transfer_patterns: None,
            receive_figure: None,
            model_points: mp,
            model_lines: ml,
            show_origin_figure: true,
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

impl PatternEditor {
    pub fn render(&mut self, ctx: &egui::Context) {
        if let Some((pts, lns)) = self.receive_figure.take() {
            self.model_points = pts;
            self.model_lines = lns;
        }

        if ctx.input(|i| i.key_pressed(egui::Key::Delete)) {
            let mut to_remove: Vec<usize> = self.selected.clone();
            to_remove.sort_unstable_by(|a, b| b.cmp(a));
            for &i in &to_remove {
                if i < self.patterns.len() {
                    self.patterns.remove(i);
                }
            }
            self.selected.clear();
        }

        egui::TopBottomPanel::top("pattern_editor_menu").show(ctx, |ui| {
            self.render_menu(ui);
        });

        egui::SidePanel::left("pattern_outliner")
            .resizable(true)
            .default_width(200.0)
            .show(ctx, |ui| {
                self.render_outliner(ui);
            });

        egui::CentralPanel::default().show(ctx, |ui| {
            self.render_canvas(ui);
        });

        egui::SidePanel::right("pattern_properties")
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
                if ui.button("Ouvrir (ptnfw)").clicked() {
                    if let Some((_path, content)) = file_io::open_json("Ouvrir un pattern", "ptnfw") {
                        match serde_json::from_str::<PatternFile>(&content) {
                            Ok(data) => {
                                self.patterns = data.patterns;
                                self.display_parent = data.display_parent;
                                self.recalculate_dimension();
                                self.message = Some("Pattern chargé".into());
                            }
                            Err(e) => self.message = Some(format!("Erreur: {e}")),
                        }
                    }
                    ui.close_menu();
                }
                if ui.button("Enregistrer (ptnfw)").clicked() {
                    let data = PatternFile {
                        display_parent: self.display_parent,
                        patterns: self.patterns.clone(),
                    };
                    let json = serde_json::to_string_pretty(&data).unwrap();
                    if file_io::save_json("Enregistrer le pattern", "ptnfw", &json) {
                        self.message = Some("Pattern enregistré".into());
                    }
                    ui.close_menu();
                }
            });

            ui.menu_button("Modèle", |ui| {
                if ui.button("Ouvrir un modèle (firfw)").clicked() {
                    if let Some((_path, content)) = file_io::open_json("Ouvrir un modèle", "firfw") {
                        match self.load_model(&content) {
                            Ok(()) => self.message = Some("Modèle chargé".into()),
                            Err(e) => self.message = Some(format!("Erreur: {e}")),
                        }
                    }
                    ui.close_menu();
                }
            });

            ui.menu_button("Options", |ui| {
                ui.checkbox(&mut self.show_origin_figure, "Afficher la figure d'origine");
                ui.checkbox(&mut self.show_gizmo, "Gizmo");
                ui.checkbox(&mut self.camera.magnetism, "Magnétisme");
            });

            if !self.patterns.is_empty()
                && ui.button("➡ Envoyer à Fractale").clicked() {
                    self.transfer_patterns = Some(self.patterns.clone());
                }

            if ui.button("Nouveau pattern").clicked() {
                self.patterns.push(ShapePatternData::default());
            }
            if ui.button("Dupliquer sélection").clicked() {
                let to_dup: Vec<_> = self.selected.clone();
                for &i in to_dup.iter().rev() {
                    if i < self.patterns.len() {
                        let dup = self.patterns[i].clone();
                        self.patterns.insert(i + 1, dup);
                    }
                }
            }
            if ui.button("Supprimer sélection").clicked() {
                let mut to_remove: Vec<usize> = self.selected.clone();
                to_remove.sort_unstable_by(|a, b| b.cmp(a));
                for &i in &to_remove {
                    if i < self.patterns.len() {
                        self.patterns.remove(i);
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
        ui.heading("Patterns");
        for (i, p) in self.patterns.iter().enumerate() {
            let label = format!(
                "Pattern {} : T({:.1}, {:.1}) R({:.1}°) S({:.2})",
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
        if self.patterns.is_empty() {
            ui.label("Aucun pattern");
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

        if self.show_origin_figure && !self.model_points.is_empty() {
            shared::render_shape_at(
                &self.model_points, &self.model_lines,
                &self.camera, canvas_center,
                Pos2::ZERO, 0.0, 1.0,
                Color32::from_rgba_premultiplied(180, 180, 180, 100),
                &mut shapes,
            );
        }

        for (i, p) in self.patterns.iter().enumerate() {
            let is_selected = self.selected.contains(&i);
            let color = if is_selected { Color32::WHITE } else { Color32::YELLOW };
            shared::render_shape_at(
                &self.model_points, &self.model_lines,
                &self.camera, canvas_center,
                p.translate, p.rotate, 1.0 / p.scale,
                color,
                &mut shapes,
            );
        }

        let translates: Vec<Pos2> = self.patterns.iter().map(|s| s.translate).collect();

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
                        if idx < self.patterns.len() {
                            let p = &self.patterns[idx];
                            let offset = shared::snap_translation(
                                &self.model_points, p.translate, p.rotate, 1.0 / p.scale,
                                self.camera.zoom,
                            );
                            self.patterns[idx].translate += offset;
                            self.recalculate_dimension();
                        }
                    }
                }
            } else {
                let delta = ui.input(|i| i.pointer.delta());
                if delta != Vec2::ZERO {
                    let world_delta = gizmo::Gizmo::handle_drag(self.gizmo_hit, delta, &self.camera);
                    if let Some(&idx) = self.selected.first() {
                        if idx < self.patterns.len() {
                            self.patterns[idx].translate += world_delta;
                            self.recalculate_dimension();
                        }
                    }
                }
            }
        } else if pointer_pressed && self.show_gizmo && self.gizmo_hit != GizmoHit::None {
            self.gizmo_dragging = true;
        } else if let Some(&idx) = self.selected.first() {
            if response.dragged_by(egui::PointerButton::Primary) && idx < self.patterns.len() {
                let delta = ui.input(|i| i.pointer.delta());
                if delta != Vec2::ZERO {
                    let world_delta = self.camera.screen_delta_to_world(delta);
                    self.patterns[idx].translate += world_delta;
                    self.recalculate_dimension();
                }
            }
        } else if response.dragged_by(egui::PointerButton::Primary) {
            self.camera.pan(ui.input(|i| i.pointer.delta()));
        }

        if response.clicked_by(egui::PointerButton::Secondary) {
            if let Some(mouse) = ui.input(|i| i.pointer.interact_pos()) {
                if let Some(idx) = shared::iter_hit_test(&translates, mouse, &self.camera, canvas_center, half) {
                    self.patterns.remove(idx);
                    self.selected.retain(|&x| x != idx);
                }
            }
        }

        painter.extend(shapes);
    }

    fn render_properties(&mut self, ui: &mut egui::Ui) {
        ui.heading("Propriétés");

        if !self.patterns.is_empty() {
            ui.label(format!("Dimension estimée: {:.3}", self.dimension));
            ui.label(format!("Modèle: {} pts, {} lignes", self.model_points.len(), self.model_lines.len()));
        }

        if let Some(&idx) = self.selected.first() {
            if idx < self.patterns.len() {
                let p = &mut self.patterns[idx];
                let changed = shared::render_transform_properties(
                    ui,
                    &format!("Pattern {}", idx + 1),
                    &mut p.translate,
                    &mut p.rotate,
                    &mut p.scale,
                );
                if changed {
                    self.recalculate_dimension();
                }
            }
        }
    }

    fn recalculate_dimension(&mut self) {
        if !self.patterns.is_empty() && self.patterns[0].scale > 0.0 {
            let n = self.patterns.len() as f32;
            let h = self.patterns[0].scale;
            if h > 0.0 {
                self.dimension = n.log10() / h.log10();
            }
        }
    }
}
