use eframe::egui::{self, Color32, Pos2, Shape, Stroke, Vec2};
use serde::{Deserialize, Serialize};
use crate::scene::camera::{Camera, CameraSettings};
use crate::scene::canvas::CanvasRenderer;
use crate::shapes::any_shape::AnyShape;
use crate::shapes::polygon::Polygon;
use crate::shapes::free_linear::FreeLinearShape;
use crate::shapes::shape::Shape as ShapeTrait;
use crate::types::{EditorState, FigureType, Line};
use crate::file_io;
use crate::gizmo::{self, GizmoHit};
use super::shared;
use super::undo::UndoStack;

#[derive(Clone)]
struct FigureUndoState {
    shape: Option<AnyShape>,
    figure_type: FigureType,
    selected_point: Option<usize>,
    equilateral_mode: bool,
    state: EditorState,
}

pub struct FigureEditor {
    pub file_path: Option<String>,
    pub transfer_shape: Option<AnyShape>,
    pub transfer_to_pattern: Option<(Vec<Pos2>, Vec<Line>)>,
    pub transfer_to_initial: Option<(Vec<Pos2>, Vec<Line>)>,

    camera: Camera,
    canvas_renderer: CanvasRenderer,
    shape: Option<AnyShape>,
    state: EditorState,
    figure_type: FigureType,
    selected_point: Option<usize>,
    gizmo_hit: GizmoHit,
    gizmo_dragging: bool,
    show_gizmo: bool,
    equilateral_mode: bool,
    message: Option<shared::StatusMessage>,
    undo_stack: UndoStack<FigureUndoState>,
}

/// Réglages du menu `Options`, mémorisés entre deux lancements.
#[derive(Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct FigureSettings {
    pub show_gizmo: bool,
    pub camera: CameraSettings,
}

impl Default for FigureSettings {
    fn default() -> Self {
        FigureEditor::default().settings()
    }
}

impl Default for FigureEditor {
    fn default() -> Self {
        Self {
            file_path: None,
            transfer_shape: None,
            transfer_to_pattern: None,
            transfer_to_initial: None,
            camera: Camera::default(),
            canvas_renderer: CanvasRenderer::new(),
            shape: None,
            state: EditorState::Mouse,
            figure_type: FigureType::Polygon,
            selected_point: None,
            gizmo_hit: GizmoHit::None,
            gizmo_dragging: false,
            show_gizmo: true,
            equilateral_mode: false,
            message: None,
            undo_stack: UndoStack::new(100),
        }
    }
}

impl FigureEditor {
    pub fn settings(&self) -> FigureSettings {
        FigureSettings {
            show_gizmo: self.show_gizmo,
            camera: self.camera.settings(),
        }
    }

    pub fn apply_settings(&mut self, settings: &FigureSettings) {
        self.show_gizmo = settings.show_gizmo;
        self.camera.apply_settings(&settings.camera);
    }

    pub fn render(&mut self, ctx: &egui::Context) {
        egui::TopBottomPanel::top("figure_editor_menu").show(ctx, |ui| {
            self.render_menu(ui);
        });

        egui::CentralPanel::default().show(ctx, |ui| {
            self.render_canvas(ui);
        });
    }

    fn snapshot(&self) -> FigureUndoState {
        FigureUndoState {
            shape: self.shape.clone(),
            figure_type: self.figure_type,
            selected_point: self.selected_point,
            equilateral_mode: self.equilateral_mode,
            state: self.state,
        }
    }

    fn restore(&mut self, state: FigureUndoState) {
        self.shape = state.shape;
        self.figure_type = state.figure_type;
        self.selected_point = state.selected_point;
        self.equilateral_mode = state.equilateral_mode;
        self.state = state.state;
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

    fn render_menu(&mut self, ui: &mut egui::Ui) {
        ui.horizontal(|ui| {
            ui.menu_button("Fichier", |ui| {
                if ui.button("Nouveau Polygone").clicked() {
                    self.push_undo();
                    self.shape = Some(AnyShape::Polygon(Polygon::new()));
                    self.figure_type = FigureType::Polygon;
                    self.state = EditorState::Add;
                    ui.close_menu();
                }
                if ui.button("Nouveau Libre").clicked() {
                    self.push_undo();
                    self.shape = Some(AnyShape::FreeLinear(FreeLinearShape::new()));
                    self.figure_type = FigureType::FreeLinear;
                    self.state = EditorState::Add;
                    ui.close_menu();
                }
                if ui.button("Nouveau Triangle équilatéral").clicked() {
                    self.push_undo();
                    self.shape = Some(AnyShape::Polygon(Polygon::new()));
                    self.figure_type = FigureType::Polygon;
                    self.equilateral_mode = true;
                    self.state = EditorState::Add;
                    ui.close_menu();
                }
                ui.separator();
                if ui.button("Ouvrir").clicked() {
                    self.push_undo();
                    if let Some((path, content)) = file_io::open_json("Ouvrir une figure", "firfw") {
                        match serde_json::from_str::<shared::ModelData>(&content) {
                            Ok(data) => {
                                self.file_path = Some(path.display().to_string());
                                self.shape = Some(data.to_shape());
                                shared::set_status_message(&mut self.message, shared::StatusMessage::info("Figure chargée"));
                            }
                            Err(e) => shared::set_status_message(&mut self.message, shared::StatusMessage::error(e.to_string())),
                        }
                    }
                    ui.close_menu();
                }
                if ui.button("Enregistrer").clicked() {
                    if let Some(ref shape) = self.shape {
                        let data = shared::ModelData::from_shape(shape);
                        let json = serde_json::to_string_pretty(&data).unwrap();
                        let name = self.file_path.as_deref().and_then(|p| {
                            std::path::Path::new(p).file_stem().and_then(|s| s.to_str())
                        }).unwrap_or("figure");
                        if file_io::save_json_path("Enregistrer la figure", "firfw", &format!("{name}.firfw"), &json) {
                            shared::set_status_message(&mut self.message, shared::StatusMessage::info("Figure enregistrée"));
                        }
                    }
                    ui.close_menu();
                }
            });

            ui.separator();

            ui.menu_button("Options", |ui| {
                shared::view_options(ui, &mut self.camera);
                shared::points_option(ui, &mut self.camera);
                shared::edit_options(ui, &mut self.show_gizmo, &mut self.camera);
            });

            let has_shape = self.shape.is_some();
            ui.add_enabled_ui(has_shape, |ui| {
                if ui.button("Mode souris").on_disabled_hover_text("Aucune figure").clicked() {
                    self.state = EditorState::Mouse;
                }
                if ui.button("Mode point").on_disabled_hover_text("Aucune figure").clicked() {
                    self.state = EditorState::Point;
                }
            });
            ui.separator();
            if ui
                .add_enabled(has_shape, egui::Button::new("➡ Envoyer"))
                .on_disabled_hover_text("Aucune figure à envoyer")
                .clicked()
            {
                if let Some(ref shape) = self.shape {
                    self.transfer_shape = Some(shape.clone());
                    let pts = shape.points().to_vec();
                    let lns = shape.lines().to_vec();
                    self.transfer_to_pattern = Some((pts.clone(), lns.clone()));
                    self.transfer_to_initial = Some((pts, lns));
                }
            }

            shared::render_status_message(ui, &mut self.message);
        });
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

        self.canvas_renderer.draw_grid(&self.camera, canvas_rect, &mut shapes);
        self.canvas_renderer.draw_origin(&self.camera, canvas_rect, &mut shapes);

        if let Some(ref shape) = self.shape {
            let points = shape.points();
            if !points.is_empty() {
                let stroke = Stroke::new(2.0_f32, Color32::YELLOW);
                let mut prev_screen = None;
                for &p in points {
                    let screen = self.camera.world_to_screen(p, canvas_center);
                    if let Some(prev) = prev_screen {
                        shapes.push(Shape::line_segment([prev, screen], stroke));
                    }
                    prev_screen = Some(screen);
                }
                if matches!(shape, AnyShape::Polygon(_)) && points.len() > 2 {
                    if let (Some(&first), Some(&last)) = (points.first(), points.last()) {
                        let s1 = self.camera.world_to_screen(first, canvas_center);
                        let s2 = self.camera.world_to_screen(last, canvas_center);
                        shapes.push(Shape::line_segment([s1, s2], stroke));
                    }
                }
            }
        }

        if let Some(ref shape) = self.shape {
            if self.camera.display_points {
                for (i, &p) in shape.points().iter().enumerate() {
                    let screen = self.camera.world_to_screen(p, canvas_center);
                    let color = if self.selected_point == Some(i) {
                        Color32::WHITE
                    } else {
                        Color32::RED
                    };
                    let half = self.camera.point_size / 2.0;
                    shapes.push(Shape::rect_filled(
                        egui::Rect::from_min_max(screen - Vec2::splat(half), screen + Vec2::splat(half)),
                        0.0,
                        color,
                    ));
                }
            }
        }

        if self.show_gizmo && !self.gizmo_dragging {
            if let Some(ref shape) = self.shape {
                if let Some(center) = shape.center() {
                    if let Some(mouse) = ui.input(|i| i.pointer.hover_pos()) {
                        self.gizmo_hit = gizmo::Gizmo::hit_test(mouse, center, &self.camera, canvas_center);
                    }
                    gizmo::Gizmo::draw(center, &self.camera, canvas_center, self.gizmo_hit, &mut shapes);
                }
            }
        }

        let pointer_pressed = ui.input(|i| i.pointer.any_pressed());
        let pointer_released = ui.input(|i| i.pointer.any_released());

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
                    if let Some(ref mut shape) = self.shape {
                        let pts = shape.points();
                        if !pts.is_empty() {
                            let spacing = crate::scene::camera::Camera::choose_grid_spacing(self.camera.zoom);
                            let mut best_dist = f32::MAX;
                            let mut best_offset = Vec2::ZERO;
                            for &p in pts {
                                let sx = (p.x / spacing).round() * spacing;
                                let sy = (p.y / spacing).round() * spacing;
                                let dx = sx - p.x;
                                let dy = sy - p.y;
                                let d = dx * dx + dy * dy;
                                if d < best_dist {
                                    best_dist = d;
                                    best_offset = Vec2::new(dx, dy);
                                }
                            }
                            for p in shape.points_mut() {
                                *p += best_offset;
                            }
                        }
                    }
                }
            } else {
                let delta = ui.input(|i| i.pointer.delta());
                if delta != Vec2::ZERO {
                    let world_delta = gizmo::Gizmo::handle_drag(self.gizmo_hit, delta, &self.camera);
                    if let Some(ref mut shape) = self.shape {
                        shape.translate_all(world_delta);
                    }
                }
            }
        } else if pointer_pressed && self.show_gizmo && self.gizmo_hit != GizmoHit::None {
            self.push_undo();
            self.gizmo_dragging = true;
        } else if response.dragged_by(egui::PointerButton::Middle)
            || (response.dragged_by(egui::PointerButton::Primary) && self.state == EditorState::Mouse && self.selected_point.is_none())
        {
            self.camera.pan(ui.input(|i| i.pointer.delta()));
        }

        if response.clicked_by(egui::PointerButton::Primary) && !self.gizmo_dragging {
            if let Some(mouse_pos) = ui.input(|i| i.pointer.interact_pos()) {
                let world_pos = self.camera.screen_to_world(mouse_pos, canvas_center);
                match self.state {
                    EditorState::Add | EditorState::Point => {
                        if self.shape.is_some() {
                            self.push_undo();
                        }
                        if let Some(ref mut shape) = self.shape {
                            let snapped = self.camera.snap(world_pos);
                            shape.add_point(snapped);
                            self.selected_point = Some(shape.points().len() - 1);

                            if self.equilateral_mode && shape.points().len() == 2 {
                                let p1 = shape.points()[0];
                                let p2 = shape.points()[1];
                                let dx = p2.x - p1.x;
                                let dy = p2.y - p1.y;
                                let mid = Pos2::new((p1.x + p2.x) / 2.0, (p1.y + p2.y) / 2.0);
                                let h = (dx * dx + dy * dy).sqrt() * 1.7320508 / 2.0;
                                let nx = -dy / (dx * dx + dy * dy).sqrt();
                                let ny = dx / (dx * dx + dy * dy).sqrt();
                                let p3 = Pos2::new(mid.x + nx * h, mid.y + ny * h);
                                shape.add_point(p3);
                                self.selected_point = Some(2);
                                self.equilateral_mode = false;
                                self.state = EditorState::Mouse;
                            }
                        }
                    }
                    EditorState::Mouse => {
                        if let Some(ref shape) = self.shape {
                            self.selected_point = shape.hit_test(world_pos, self.camera.point_size);
                        }
                    }
                    EditorState::SelectPointSimulation => {}
                }
            }
        }

        if response.clicked_by(egui::PointerButton::Secondary) {
            if let Some(idx) = self.selected_point {
                if self.shape.is_some() {
                    self.push_undo();
                }
                if let Some(ref mut shape) = self.shape {
                    shape.remove_point(idx);
                    self.selected_point = None;
                }
            }
        }

        painter.extend(shapes);
    }
}
