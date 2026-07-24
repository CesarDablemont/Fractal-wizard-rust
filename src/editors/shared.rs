use eframe::egui::{self, Color32, Pos2, Shape, Stroke, Vec2};
use serde::{Deserialize, Serialize};
use crate::scene::camera::Camera;
use crate::shapes::shape::apply_transform;
use crate::gizmo::{self, GizmoHit};
use crate::types::Line;

#[derive(Serialize, Deserialize)]
pub struct ModelData {
    pub r#type: String,
    pub points: Vec<[f32; 2]>,
    #[serde(skip_serializing_if = "Vec::is_empty", default)]
    pub lines: Vec<Line>,
}

pub struct ShapeTransform {
    pub translate: Pos2,
    pub rotate: f32,
    pub scale: f32,
}

pub struct GizmoContext<'a> {
    pub ui: &'a egui::Ui,
    pub camera: &'a Camera,
    pub canvas_center: Pos2,
    pub show_gizmo: bool,
    pub translates: &'a [Pos2],
}

pub fn default_model() -> (Vec<Pos2>, Vec<Line>) {
    let pts = vec![
        Pos2::new(-0.5, -0.5),
        Pos2::new(0.5, -0.5),
        Pos2::new(0.5, 0.5),
        Pos2::new(-0.5, 0.5),
    ];
    let lines = vec![[0, 1], [1, 2], [2, 3], [3, 0]];
    (pts, lines)
}

pub fn load_model(content: &str, model_points: &mut Vec<Pos2>, model_lines: &mut Vec<Line>) -> Result<(), String> {
    let data: ModelData = serde_json::from_str(content).map_err(|e| e.to_string())?;
    *model_points = data.points.iter().map(|&p| Pos2::new(p[0], p[1])).collect();
    *model_lines = data.lines;
    if model_lines.is_empty() && model_points.len() >= 2 {
        if data.r#type == "Polygon" || data.r#type == "cPolygon" {
            *model_lines = (0..model_points.len() - 1)
                .map(|i| [i, i + 1])
                .collect();
            if model_points.len() > 2 {
                model_lines.push([model_points.len() - 1, 0]);
            }
        } else {
            *model_lines = (0..model_points.len() - 1)
                .map(|i| [i, i + 1])
                .collect();
        }
    }
    Ok(())
}

pub fn render_shape_at(
    model_points: &[Pos2],
    model_lines: &[Line],
    camera: &Camera,
    canvas_center: Pos2,
    transform: &ShapeTransform,
    color: Color32,
    shapes: &mut Vec<Shape>,
) {
    if model_points.is_empty() {
        return;
    }
    let stroke = Stroke::new(1.5, color);
    let transformed: Vec<Pos2> = model_points
        .iter()
        .map(|&p| apply_transform(p, transform.translate, transform.rotate, Vec2::new(transform.scale, transform.scale)))
        .collect();
    for &[a, b] in model_lines {
        if a < transformed.len() && b < transformed.len() {
            let p1 = camera.world_to_screen(transformed[a], canvas_center);
            let p2 = camera.world_to_screen(transformed[b], canvas_center);
            shapes.push(Shape::line_segment([p1, p2], stroke));
        }
    }
}

pub fn snap_translation(
    model_points: &[Pos2],
    current_translate: Pos2,
    rotate: f32,
    scale: f32,
    zoom: f32,
) -> Vec2 {
    let spacing = Camera::choose_grid_spacing(zoom);
    let s = scale;
    let mut best_dist = f32::MAX;
    let mut best_offset = Vec2::ZERO;
    for &mp in model_points {
        let tp = apply_transform(mp, current_translate, rotate, Vec2::new(s, s));
        let sx = (tp.x / spacing).round() * spacing;
        let sy = (tp.y / spacing).round() * spacing;
        let dx = sx - tp.x;
        let dy = sy - tp.y;
        let d = dx * dx + dy * dy;
        if d < best_dist {
            best_dist = d;
            best_offset = Vec2::new(dx, dy);
        }
    }
    best_offset
}

pub fn iter_hit_test(
    shapes: &[Pos2],
    mouse: Pos2,
    camera: &Camera,
    canvas_center: Pos2,
    half: f32,
) -> Option<usize> {
    shapes.iter().position(|&p| {
        let screen = camera.world_to_screen(p, canvas_center);
        let d = screen - mouse;
        d.x.abs() <= half && d.y.abs() <= half
    })
}

pub fn handle_zoom_scroll(response: &egui::Response, ui: &egui::Ui, camera: &mut Camera, canvas_center: Pos2) {
    if response.hovered() {
        let scroll = ui.input(|i| i.raw_scroll_delta);
        if scroll.y != 0.0 {
            let factor = 1.15f32.powf(scroll.y / 10.0);
            let mouse = ui.input(|i| i.pointer.hover_pos()).unwrap_or(canvas_center);
            camera.zoom_at(factor, mouse, canvas_center);
        }
    }
}

pub fn handle_middle_pan(response: &egui::Response, ui: &egui::Ui, camera: &mut Camera) {
    if response.dragged_by(egui::PointerButton::Middle) {
        camera.pan(ui.input(|i| i.pointer.delta()));
    }
}

pub fn handle_draw_gizmo(
    ctx: &GizmoContext<'_>,
    selected: &[usize],
    gizmo_dragging: bool,
    gizmo_hit: &mut GizmoHit,
    shapes: &mut Vec<Shape>,
) {
    if ctx.show_gizmo && !gizmo_dragging {
        if let Some(&idx) = selected.first() {
            if idx < ctx.translates.len() {
                let pos = ctx.translates[idx];
                if let Some(mouse) = ctx.ui.input(|i| i.pointer.hover_pos()) {
                    *gizmo_hit = gizmo::Gizmo::hit_test(mouse, pos, ctx.camera, ctx.canvas_center);
                }
                gizmo::Gizmo::draw(pos, ctx.camera, ctx.canvas_center, *gizmo_hit, shapes);
            }
        }
    }
}

pub fn handle_primary_click_selection(
    ctx: &GizmoContext<'_>,
    response: &egui::Response,
    gizmo_hit: GizmoHit,
    half: f32,
    selected: &mut Vec<usize>,
) {
    if response.clicked_by(egui::PointerButton::Primary) {
        if let Some(mouse) = ctx.ui.input(|i| i.pointer.interact_pos()) {
            if ctx.show_gizmo && gizmo_hit != GizmoHit::None {
                // gizmo click handled via drag
            } else {
                let hit = iter_hit_test(ctx.translates, mouse, ctx.camera, ctx.canvas_center, half);
                if let Some(idx) = hit {
                    *selected = vec![idx];
                } else {
                    selected.clear();
                }
            }
        }
    }
}



pub fn render_transform_properties(
    ui: &mut egui::Ui,
    label: &str,
    translate: &mut Pos2,
    rotate: &mut f32,
    scale: &mut f32,
) -> bool {
    ui.separator();
    ui.label(label);

    let mut changed = false;
    let mut tx = translate.x;
    let mut ty = translate.y;
    ui.horizontal(|ui| {
        ui.label("X:");
        changed |= ui.add(egui::DragValue::new(&mut tx).speed(1.0)).changed();
        ui.label("Y:");
        changed |= ui.add(egui::DragValue::new(&mut ty).speed(1.0)).changed();
    });
    let mut deg = rotate.to_degrees();
    ui.horizontal(|ui| {
        ui.label("Rotation:");
        changed |= ui.add(egui::DragValue::new(&mut deg).speed(1.0).suffix("°")).changed();
    });
    ui.horizontal(|ui| {
        ui.label("Scale:");
        changed |= ui.add(egui::DragValue::new(scale).speed(0.1).range(0.01..=10.0)).changed();
    });

    if changed {
        translate.x = tx;
        translate.y = ty;
        *rotate = deg.to_radians();
    }
    changed
}
