use eframe::egui::{self, pos2, Color32, Pos2, Shape, Vec2};
use crate::scene::camera::Camera;
use crate::scene::canvas::CanvasRenderer;
use crate::types::{Line, ShapePatternData};
use crate::file_io;
use crate::gizmo::{self, GizmoHit};
use super::shared;
use super::undo::UndoStack;

#[derive(Clone)]
struct InitialUndoState {
    shapes: Vec<ShapePatternData>,
    selected: Vec<usize>,
}

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
    last_clicked: Option<usize>,
    message: Option<shared::StatusMessage>,
    undo_stack: UndoStack<InitialUndoState>,
    property_dragging: bool,
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
            last_clicked: None,
            message: None,
            undo_stack: UndoStack::new(100),
            property_dragging: false,
        }
    }
}

impl InitialEditor {
    pub fn render(&mut self, ctx: &egui::Context) {
        if let Some((pts, lns)) = self.receive_figure.take() {
            self.push_undo();
            self.model_points = pts;
            self.model_lines = lns;
            self.shapes = vec![ShapePatternData {
                translate: pos2(0.0, 0.0),
                rotate: 0.0,
                scale: 1.0,
            }];
        }

        if ctx.input(|i| i.key_pressed(egui::Key::Delete)) {
            if !self.selected.is_empty() {
                self.push_undo();
            }
            let mut to_remove: Vec<usize> = self.selected.clone();
            to_remove.sort_unstable_by(|a, b| b.cmp(a));
            for &i in &to_remove {
                if i < self.shapes.len() {
                    self.shapes.remove(i);
                }
            }
            self.selected.clear();
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

    fn snapshot(&self) -> InitialUndoState {
        InitialUndoState {
            shapes: self.shapes.clone(),
            selected: self.selected.clone(),
        }
    }

    fn restore(&mut self, state: InitialUndoState) {
        self.shapes = state.shapes;
        self.selected = state.selected;
    }

    fn push_undo(&mut self) {
        self.undo_stack.push(self.snapshot());
    }

    fn undo(&mut self) {
        if let Some(state) = self.undo_stack.undo(self.snapshot()) {
            self.restore(state);
        }
    }

    fn redo(&mut self) {
        if let Some(state) = self.undo_stack.redo(self.snapshot()) {
            self.restore(state);
        }
    }

    fn load_model(&mut self, content: &str) -> Result<(), String> {
        shared::load_model(content, &mut self.model_points, &mut self.model_lines)
    }

    fn render_menu(&mut self, ui: &mut egui::Ui) {
        ui.horizontal(|ui| {
            ui.menu_button("Fichier", |ui| {
                if ui.button("Ouvrir (tilfw)").clicked() {
                    self.push_undo();
                    if let Some((_path, content)) = file_io::open_json("Ouvrir un fichier initial", "filfw") {
                        match serde_json::from_str::<Vec<ShapePatternData>>(&content) {
                            Ok(data) => {
                                self.shapes = data;
                                shared::set_status_message(&mut self.message, shared::StatusMessage::info("Fichier initial chargé"));
                            }
                            Err(e) => shared::set_status_message(&mut self.message, shared::StatusMessage::error(e.to_string())),
                        }
                    }
                    ui.close_menu();
                }
                if ui.button("Enregistrer (tilfw)").clicked() {
                    let json = serde_json::to_string_pretty(&self.shapes).unwrap();
                    if file_io::save_json("Enregistrer le fichier initial", "filfw", &json) {
                        shared::set_status_message(&mut self.message, shared::StatusMessage::info("Fichier initial enregistré"));
                    }
                    ui.close_menu();
                }
            });

            ui.menu_button("Modèle", |ui| {
                if ui.button("Ouvrir un modèle (firfw)").clicked() {
                    if let Some((_path, content)) = file_io::open_json("Ouvrir un modèle", "firfw") {
                        match self.load_model(&content) {
                            Ok(()) => shared::set_status_message(&mut self.message, shared::StatusMessage::info("Modèle chargé")),
                            Err(e) => shared::set_status_message(&mut self.message, shared::StatusMessage::error(e.to_string())),
                        }
                    }
                    ui.close_menu();
                }
            });

            ui.menu_button("Options", |ui| {
                ui.checkbox(&mut self.show_gizmo, "Gizmo");
                ui.checkbox(&mut self.camera.magnetism, "Magnétisme");
            });

            if !self.shapes.is_empty()
                && ui.button("➡ Envoyer à Fractale").clicked() {
                    self.transfer_shapes = Some(self.shapes.clone());
                }

            if ui.button("Nouveau").clicked() {
                self.push_undo();
                self.shapes.push(ShapePatternData::default());
            }
            if ui.button("Dupliquer sélection").clicked() {
                self.push_undo();
                let to_dup: Vec<_> = self.selected.clone();
                for &i in to_dup.iter().rev() {
                    if i < self.shapes.len() {
                        let dup = self.shapes[i].clone();
                        self.shapes.insert(i + 1, dup);
                    }
                }
            }
            if ui.button("Supprimer sélection").clicked() {
                self.push_undo();
                let mut to_remove: Vec<usize> = self.selected.clone();
                to_remove.sort_unstable_by(|a, b| b.cmp(a));
                for &i in &to_remove {
                    if i < self.shapes.len() {
                        self.shapes.remove(i);
                    }
                }
                self.selected.clear();
            }

            shared::render_status_message(ui, &mut self.message);
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
                if ui.input(|i| i.modifiers.shift) {
                    if let Some(anchor) = self.last_clicked {
                        let start = anchor.min(i);
                        let end = anchor.max(i);
                        self.selected = (start..=end).collect();
                    } else {
                        self.selected = vec![i];
                    }
                } else if ui.input(|i| i.modifiers.ctrl) {
                    if selected {
                        self.selected.retain(|&x| x != i);
                    } else {
                        self.selected.push(i);
                    }
                } else {
                    self.selected = vec![i];
                }
                self.last_clicked = Some(i);
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
                &shared::ShapeTransform { translate: p.translate, rotate: p.rotate, scale: 1.0 / p.scale },
                color,
                &mut shapes,
            );
        }

        let translates: Vec<Pos2> = self.shapes.iter().map(|s| s.translate).collect();

        let gizmo_ctx = shared::GizmoContext {
            ui, camera: &self.camera, canvas_center,
            show_gizmo: self.show_gizmo,
            translates: &translates,
        };

        shared::handle_draw_gizmo(
            &gizmo_ctx, &self.selected, self.gizmo_dragging,
            &mut self.gizmo_hit, &mut shapes,
        );

        shared::handle_primary_click_selection(
            &gizmo_ctx, &response,
            self.gizmo_hit, self.camera.point_size,
            &mut self.selected,
        );

        let pointer_pressed = ui.input(|i| i.pointer.any_pressed());
        let pointer_released = ui.input(|i| i.pointer.any_released());
        let half = self.camera.point_size;

        if ui.input(|i| i.modifiers.command && i.key_pressed(egui::Key::Z) && !i.modifiers.shift) {
            self.undo();
        }
        if ui.input(|i| i.modifiers.command && i.key_pressed(egui::Key::Y)) {
            self.redo();
        }

        if self.gizmo_dragging {
            if pointer_released {
                self.gizmo_dragging = false;
                if self.camera.magnetism {
                    if let Some(&idx) = self.selected.first() {
                        if idx < self.shapes.len() {
                            let s = &self.shapes[idx];
                            let others: Vec<shared::OtherTransform> = self.shapes
                                .iter()
                                .enumerate()
                                .filter(|(i, _)| *i != idx)
                                .map(|(_, s)| shared::OtherTransform {
                                    translate: s.translate,
                                    rotate: s.rotate,
                                    scale: 1.0 / s.scale,
                                })
                                .collect();
                            let offset = shared::snap_translation(
                                &self.model_points, s.translate, s.rotate, 1.0 / s.scale,
                                self.camera.zoom,
                                &others,
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
            self.push_undo();
            self.gizmo_dragging = true;
        } else if let Some(&idx) = self.selected.first() {
            if response.dragged_by(egui::PointerButton::Primary) && idx < self.shapes.len() {
                if pointer_pressed {
                    self.push_undo();
                }
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
                    self.push_undo();
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
                let old_translate = self.shapes[idx].translate;
                let old_rotate = self.shapes[idx].rotate;
                let old_scale = self.shapes[idx].scale;

                let old_state = self.snapshot();

                let changed = {
                    let p = &mut self.shapes[idx];
                    shared::render_transform_properties(
                        ui,
                        &format!("Initial {}", idx + 1),
                        &mut p.translate,
                        &mut p.rotate,
                        &mut p.scale,
                    )
                };

                if changed {
                    if !self.property_dragging {
                        self.property_dragging = true;
                        self.undo_stack.push(old_state);
                    }

                    let d_translate = self.shapes[idx].translate - old_translate;
                    let d_rotate = self.shapes[idx].rotate - old_rotate;
                    let d_scale = self.shapes[idx].scale - old_scale;

                    for &sel in &self.selected {
                        if sel != idx && sel < self.shapes.len() {
                            self.shapes[sel].translate += d_translate;
                            self.shapes[sel].rotate += d_rotate;
                            self.shapes[sel].scale += d_scale;
                        }
                    }
                } else {
                    self.property_dragging = false;
                }
            }
        }
    }
}
