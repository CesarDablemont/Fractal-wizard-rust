use eframe::egui::{Color32, Pos2, Shape, Stroke, Vec2};
use serde::{Deserialize, Serialize};
use crate::scene::camera::Camera;
use crate::shapes::shape::apply_transform;
use crate::types::Line;

#[derive(Serialize, Deserialize)]
pub struct ModelData {
    pub r#type: String,
    pub points: Vec<[f32; 2]>,
    #[serde(skip_serializing_if = "Vec::is_empty", default)]
    pub lines: Vec<Line>,
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
    translate: Pos2,
    rotate: f32,
    scale: f32,
    color: Color32,
    shapes: &mut Vec<Shape>,
) {
    if model_points.is_empty() {
        return;
    }
    let stroke = Stroke::new(1.5, color);
    let transformed: Vec<Pos2> = model_points
        .iter()
        .map(|&p| apply_transform(p, translate, rotate, Vec2::new(scale, scale)))
        .collect();
    for &[a, b] in model_lines {
        if a < transformed.len() && b < transformed.len() {
            let p1 = camera.world_to_screen(transformed[a], canvas_center);
            let p2 = camera.world_to_screen(transformed[b], canvas_center);
            shapes.push(Shape::line_segment([p1, p2], stroke));
        }
    }
}
