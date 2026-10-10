use eframe::egui::{self, Color32, Pos2, Shape, Vec2};
use crate::scene::camera::Camera;
use crate::scene::canvas::CanvasRenderer;
use crate::types::{Line, ShapePatternData};
use crate::file_io;
use crate::gizmo::{self, GizmoHit};
use super::shared;
use super::undo::UndoStack;

/// Textes et couleur propres à chaque éditeur de liste de formes.
#[derive(Clone, Copy)]
pub struct ShapeListStyle {
    /// Préfixe d'un élément dans l'outliner et les propriétés (« Pattern », « Initial »).
    pub item_label: &'static str,
    pub outliner_title: &'static str,
    pub empty_label: &'static str,
    pub new_label: &'static str,
    pub nothing_to_send: &'static str,
    pub color: Color32,
}

#[derive(Clone)]
struct ShapeListUndoState<X> {
    shapes: Vec<ShapePatternData>,
    extra: X,
    selected: Vec<usize>,
}

/// Logique commune aux éditeurs qui placent des copies transformées d'un modèle
/// (`PatternEditor`, `InitialEditor`) : outliner, canvas, propriétés, undo.
///
/// `X` est l'état propre au document de l'éditeur, inclus dans l'historique d'undo.
pub struct ShapeListEditor<X: Clone> {
    pub shapes: Vec<ShapePatternData>,
    pub extra: X,

    pub model_points: Vec<Pos2>,
    pub model_lines: Vec<Line>,

    pub camera: Camera,
    pub show_gizmo: bool,
    pub message: Option<shared::StatusMessage>,

    style: ShapeListStyle,
    canvas_renderer: CanvasRenderer,
    gizmo_hit: GizmoHit,
    gizmo_dragging: bool,
    selected: Vec<usize>,
    last_clicked: Option<usize>,
    undo_stack: UndoStack<ShapeListUndoState<X>>,
    property_dragging: bool,
}

impl<X: Clone> ShapeListEditor<X> {
    pub fn new(style: ShapeListStyle, extra: X) -> Self {
        let (model_points, model_lines) = shared::default_model();
        Self {
            shapes: Vec::new(),
            extra,
            model_points,
            model_lines,
            camera: Camera::default(),
            show_gizmo: true,
            message: None,
            style,
            canvas_renderer: CanvasRenderer::new(),
            gizmo_hit: GizmoHit::None,
            gizmo_dragging: false,
            selected: Vec::new(),
            last_clicked: None,
            undo_stack: UndoStack::new(100),
            property_dragging: false,
        }
    }

    fn snapshot(&self) -> ShapeListUndoState<X> {
        ShapeListUndoState {
            shapes: self.shapes.clone(),
            extra: self.extra.clone(),
            selected: self.selected.clone(),
        }
    }

    fn restore(&mut self, state: ShapeListUndoState<X>) {
        self.shapes = state.shapes;
        self.extra = state.extra;
        self.selected = state.selected;
    }

    pub fn push_undo(&mut self) {
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

    pub fn info(&mut self, text: impl Into<String>) {
        shared::set_status_message(&mut self.message, shared::StatusMessage::info(text));
    }

    pub fn error(&mut self, text: impl Into<String>) {
        shared::set_status_message(&mut self.message, shared::StatusMessage::error(text));
    }

    fn remove_selected(&mut self) {
        let mut to_remove: Vec<usize> = self.selected.clone();
        to_remove.sort_unstable_by(|a, b| b.cmp(a));
        for &i in &to_remove {
            if i < self.shapes.len() {
                self.shapes.remove(i);
            }
        }
        self.selected.clear();
    }

    pub fn handle_delete_key(&mut self, ctx: &egui::Context) {
        if ctx.input(|i| i.key_pressed(egui::Key::Delete)) {
            if !self.selected.is_empty() {
                self.push_undo();
            }
            self.remove_selected();
        }
    }

    /// Menu « Modèle » : charge la figure dupliquée par l'éditeur.
    pub fn model_menu(&mut self, ui: &mut egui::Ui) {
        ui.menu_button("Modèle", |ui| {
            if ui.button("Ouvrir un modèle (firfw)").clicked() {
                if let Some((_path, content)) = file_io::open_json("Ouvrir un modèle", "firfw") {
                    match shared::load_model(&content, &mut self.model_points, &mut self.model_lines) {
                        Ok(()) => self.info("Modèle chargé"),
                        Err(e) => self.error(e),
                    }
                }
                ui.close_menu();
            }
        });
    }

    /// Bouton « Envoyer à Fractale » ; renvoie `true` s'il a été cliqué.
    pub fn send_button(&self, ui: &mut egui::Ui) -> bool {
        ui.add_enabled(!self.shapes.is_empty(), egui::Button::new("➡ Envoyer à Fractale"))
            .on_disabled_hover_text(self.style.nothing_to_send)
            .clicked()
    }

    /// Boutons « Nouveau », « Dupliquer sélection » et « Supprimer sélection ».
    pub fn edit_buttons(&mut self, ui: &mut egui::Ui) {
        if ui.button(self.style.new_label).clicked() {
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
            self.remove_selected();
        }
    }

    pub fn render_outliner(&mut self, ui: &mut egui::Ui) {
        ui.heading(self.style.outliner_title);
        for (i, p) in self.shapes.iter().enumerate() {
            let label = format!(
                "{} {} : T({:.1}, {:.1}) R({:.1}°) S({:.2})",
                self.style.item_label, i + 1, p.translate.x, p.translate.y, p.rotate.to_degrees(), p.scale
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
            ui.label(self.style.empty_label);
        }
    }

    /// Canvas interactif ; `show_model_at_origin` dessine le modèle en gris sous les formes.
    pub fn render_canvas(&mut self, ui: &mut egui::Ui, show_model_at_origin: bool) {
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

        if show_model_at_origin && !self.model_points.is_empty() {
            shared::render_shape_at(
                &self.model_points, &self.model_lines,
                &self.camera, canvas_center,
                &shared::ShapeTransform { translate: Pos2::ZERO, rotate: 0.0, scale: 1.0 },
                Color32::from_rgba_premultiplied(180, 180, 180, 100),
                &mut shapes,
            );
        }

        for (i, p) in self.shapes.iter().enumerate() {
            let is_selected = self.selected.contains(&i);
            let color = if is_selected { Color32::WHITE } else { self.style.color };
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

    pub fn model_info_label(&self, ui: &mut egui::Ui) {
        ui.label(format!("Modèle: {} pts, {} lignes", self.model_points.len(), self.model_lines.len()));
    }

    /// Propriétés de la forme sélectionnée ; le delta est appliqué à toute la sélection.
    pub fn render_selection_properties(&mut self, ui: &mut egui::Ui) {
        let Some(&idx) = self.selected.first() else { return };
        if idx >= self.shapes.len() {
            return;
        }
        let old_translate = self.shapes[idx].translate;
        let old_rotate = self.shapes[idx].rotate;
        let old_scale = self.shapes[idx].scale;

        let old_state = self.snapshot();

        let changed = {
            let p = &mut self.shapes[idx];
            shared::render_transform_properties(
                ui,
                &format!("{} {}", self.style.item_label, idx + 1),
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
