use std::sync::Arc;
use eframe::egui::{pos2, Color32, Pos2, Rect, Shape, Stroke, Vec2};
use eframe::egui::epaint::{Mesh, Vertex};
use crate::scene::camera::Camera;
use crate::scene::chunk_grid::ChunkGrid;

/// Échelle en dessous de laquelle un point de la fractale n'est pas dessiné.
fn min_visible_scale(camera: &Camera) -> f32 {
    0.5 / camera.zoom
}

/// Indice du point affiché le plus proche de `mouse` (coordonnées écran), si le
/// clic tombe dans son carré. Les points masqués au zoom courant sont ignorés.
pub fn pick_fractal_point(
    camera: &Camera,
    points: &[Pos2],
    point_scale: &[f32],
    mouse: Pos2,
    canvas_center: Pos2,
) -> Option<usize> {
    let half = camera.point_size / 2.0;
    let min_scale = min_visible_scale(camera);
    points
        .iter()
        .enumerate()
        .filter(|&(i, _)| point_scale.get(i).is_none_or(|&s| s >= min_scale))
        .map(|(i, &p)| (i, camera.world_to_screen(p, canvas_center) - mouse))
        .filter(|(_, d)| d.x.abs() <= half && d.y.abs() <= half)
        .min_by(|a, b| a.1.length_sq().total_cmp(&b.1.length_sq()))
        .map(|(i, _)| i)
}

pub struct FractalDrawData<'a> {
    pub points: &'a [Pos2],
    pub point_scale: &'a [f32],
    pub colors: &'a [Option<Color32>],
    pub highlight: Option<usize>,
}
use crate::types::Line;

#[derive(Default)]
pub struct CachedMesh {
    mesh: Arc<Mesh>,
    camera_pos: Vec2,
    camera_zoom: f32,
    canvas_center: Pos2,
    points_len: usize,
    lines_len: usize,
}

#[derive(Default)]
pub struct CanvasRenderer {
    pub chunk_grid: Option<ChunkGrid>,
    pub rebuild_chunks: bool,
    pub mesh_dirty: bool,
    cached_lines: Option<CachedMesh>,
}

impl CanvasRenderer {
    pub fn new() -> Self {
        Self {
            chunk_grid: None,
            rebuild_chunks: true,
            mesh_dirty: true,
            cached_lines: None,
        }
    }

    pub fn draw_grid(&self, camera: &Camera, canvas_rect: Rect, shapes: &mut Vec<Shape>) {
        if !camera.display_grid {
            return;
        }

        let center = canvas_rect.center();
        let viewport = camera.visible_world_rect(canvas_rect, center);
        let spacing = Camera::choose_grid_spacing(camera.zoom);

        let start_x = (viewport.min.x / spacing).floor() * spacing;
        let end_x = (viewport.max.x / spacing).ceil() * spacing;
        let start_y = (viewport.min.y / spacing).floor() * spacing;
        let end_y = (viewport.max.y / spacing).ceil() * spacing;

        let grid_color = Color32::from_rgba_premultiplied(120, 120, 120, 60);
        let origin_color = Color32::from_rgba_premultiplied(120, 120, 120, 140);
        let stroke = Stroke::new(1.0, grid_color);
        let origin_stroke = Stroke::new(1.5, origin_color);

        let mut x = start_x;
        while x <= end_x {
            let p1 = camera.world_to_screen(pos2(x, viewport.min.y), center);
            let p2 = camera.world_to_screen(pos2(x, viewport.max.y), center);
            let s = if x == 0.0 { origin_stroke } else { stroke };
            shapes.push(Shape::line_segment([p1, p2], s));
            x += spacing;
        }

        let mut y = start_y;
        while y <= end_y {
            let p1 = camera.world_to_screen(pos2(viewport.min.x, y), center);
            let p2 = camera.world_to_screen(pos2(viewport.max.x, y), center);
            let s = if y == 0.0 { origin_stroke } else { stroke };
            shapes.push(Shape::line_segment([p1, p2], s));
            y += spacing;
        }
    }

    pub fn draw_origin(&self, camera: &Camera, canvas_rect: Rect, shapes: &mut Vec<Shape>) {
        if !camera.display_origin {
            return;
        }

        let center = canvas_rect.center();
        let viewport = camera.visible_world_rect(canvas_rect, center);

        let ox = camera.world_to_screen(pos2(0.0, viewport.min.y), center);
        let oy = camera.world_to_screen(pos2(viewport.min.x, 0.0), center);
        let ex = camera.world_to_screen(pos2(0.0, viewport.max.y), center);
        let ey = camera.world_to_screen(pos2(viewport.max.x, 0.0), center);

        shapes.push(Shape::line_segment([ox, ex], Stroke::new(2.0, Color32::RED)));
        shapes.push(Shape::line_segment([oy, ey], Stroke::new(2.0, Color32::GREEN)));
    }

    pub fn draw_fractal_lines(
        &mut self,
        camera: &Camera,
        canvas_rect: Rect,
        points: &[Pos2],
        lines: &[Line],
        color: Color32,
        shapes: &mut Vec<Shape>,
    ) {
        if points.is_empty() || lines.is_empty() {
            return;
        }

        let center = canvas_rect.center();
        let viewport = camera.visible_world_rect(canvas_rect, center);

        let needs_rebuild = self.mesh_dirty
            || self.cached_lines.as_ref().is_none_or(|c| {
                c.camera_pos != camera.position
                    || c.camera_zoom != camera.zoom
                    || c.canvas_center != center
                    || c.points_len != points.len()
                    || c.lines_len != lines.len()
            });

        if let Some(cached) = &self.cached_lines {
            if !needs_rebuild {
                shapes.push(Shape::Mesh(cached.mesh.clone()));
                return;
            }
        }

        let half_width = 0.75;
        let mut mesh = Mesh::default();

        if let Some(grid) = &self.chunk_grid {
            let visible = grid.visible_lines(lines, points, viewport);
            for &li in &visible {
                let [a, b] = lines[li];
                if a >= points.len() || b >= points.len() {
                    continue;
                }
                let p1 = camera.world_to_screen(points[a], center);
                let p2 = camera.world_to_screen(points[b], center);
                let dir = (p2 - p1).normalized();
                let perp = Vec2::new(-dir.y, dir.x) * half_width;

                let idx = mesh.vertices.len() as u32;
                mesh.vertices.push(Vertex { pos: p1 + perp, uv: Pos2::ZERO, color });
                mesh.vertices.push(Vertex { pos: p1 - perp, uv: Pos2::ZERO, color });
                mesh.vertices.push(Vertex { pos: p2 - perp, uv: Pos2::ZERO, color });
                mesh.vertices.push(Vertex { pos: p2 + perp, uv: Pos2::ZERO, color });
                mesh.indices.extend_from_slice(&[idx, idx + 1, idx + 2, idx + 2, idx + 3, idx]);
            }
        } else {
            for l in lines {
                let [a, b] = *l;
                if a >= points.len() || b >= points.len() {
                    continue;
                }
                let p1 = camera.world_to_screen(points[a], center);
                let p2 = camera.world_to_screen(points[b], center);
                let dir = (p2 - p1).normalized();
                let perp = Vec2::new(-dir.y, dir.x) * half_width;

                let idx = mesh.vertices.len() as u32;
                mesh.vertices.push(Vertex { pos: p1 + perp, uv: Pos2::ZERO, color });
                mesh.vertices.push(Vertex { pos: p1 - perp, uv: Pos2::ZERO, color });
                mesh.vertices.push(Vertex { pos: p2 - perp, uv: Pos2::ZERO, color });
                mesh.vertices.push(Vertex { pos: p2 + perp, uv: Pos2::ZERO, color });
                mesh.indices.extend_from_slice(&[idx, idx + 1, idx + 2, idx + 2, idx + 3, idx]);
            }
        }

        if !mesh.vertices.is_empty() {
            let arc = Arc::new(mesh);
            self.cached_lines = Some(CachedMesh {
                mesh: arc.clone(),
                camera_pos: camera.position,
                camera_zoom: camera.zoom,
                canvas_center: center,
                points_len: points.len(),
                lines_len: lines.len(),
            });
            self.mesh_dirty = false;
            shapes.push(Shape::Mesh(arc));
        }
    }

    pub fn draw_fractal_points(
        &self,
        camera: &Camera,
        canvas_rect: Rect,
        data: &FractalDrawData<'_>,
        shapes: &mut Vec<Shape>,
    ) {
        let FractalDrawData { points, point_scale, colors, highlight } = *data;

        if !camera.display_points || points.is_empty() {
            return;
        }

        let center = canvas_rect.center();
        let half_size = camera.point_size / 2.0;
        let pixel_size = half_size * camera.zoom;
        if pixel_size < 0.5 {
            return;
        }

        let min_scale = min_visible_scale(camera);

        let mut mesh = Mesh::default();

        let iter: Box<dyn Iterator<Item = usize>> = if let Some(grid) = &self.chunk_grid {
            let viewport = camera.visible_world_rect(canvas_rect, center);
            Box::new(grid.visible_indices(viewport).into_iter())
        } else {
            Box::new(0..points.len())
        };

        for i in iter {
            if i < point_scale.len() && point_scale[i] < min_scale {
                continue;
            }
            let p = camera.world_to_screen(points[i], center);
            let color = if highlight == Some(i) {
                Color32::from_rgb(0, 180, 0)
            } else {
                colors.get(i).copied().flatten().unwrap_or(Color32::RED)
            };
            let rect = Rect::from_min_max(p - Vec2::splat(half_size), p + Vec2::splat(half_size));
            let idx = mesh.vertices.len() as u32;
            mesh.vertices.push(Vertex { pos: rect.left_top(), uv: Pos2::ZERO, color });
            mesh.vertices.push(Vertex { pos: rect.right_top(), uv: Pos2::ZERO, color });
            mesh.vertices.push(Vertex { pos: rect.right_bottom(), uv: Pos2::ZERO, color });
            mesh.vertices.push(Vertex { pos: rect.left_bottom(), uv: Pos2::ZERO, color });
            mesh.indices.extend_from_slice(&[idx, idx + 1, idx + 2, idx + 2, idx + 3, idx]);
        }

        if !mesh.vertices.is_empty() {
            shapes.push(Shape::Mesh(Arc::new(mesh)));
        }

        if let Some(idx) = highlight {
            if idx < points.len() {
                let p = camera.world_to_screen(points[idx], center);
                let stroke_width = (half_size * 0.6).clamp(1.0, 4.0);
                let rect = Rect::from_center_size(p, Vec2::splat(half_size * 2.0 - stroke_width));
                shapes.push(Shape::rect_stroke(rect, 0.0, Stroke::new(stroke_width, Color32::BLACK), eframe::egui::StrokeKind::Outside));
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn pick_returns_nearest_point_not_first() {
        // zoom 10 : les points 0 et 1 sont à 2 px l'un de l'autre, le clic est sur le 1
        let camera = Camera::default();
        let center = pos2(0.0, 0.0);
        let points = [pos2(0.0, 0.0), pos2(0.2, 0.0)];
        let mouse = camera.world_to_screen(points[1], center);
        assert_eq!(pick_fractal_point(&camera, &points, &[], mouse, center), Some(1));
    }

    #[test]
    fn pick_uses_screen_size() {
        // point à 1 unité monde = 10 px à zoom 10, hors du carré de 6 px
        let camera = Camera::default();
        let center = pos2(0.0, 0.0);
        let points = [pos2(1.0, 0.0)];
        assert_eq!(pick_fractal_point(&camera, &points, &[], center, center), None);
    }

    #[test]
    fn pick_ignores_hidden_points() {
        let camera = Camera::default();
        let center = pos2(0.0, 0.0);
        let points = [pos2(0.0, 0.0)];
        let scales = [min_visible_scale(&camera) / 2.0];
        assert_eq!(pick_fractal_point(&camera, &points, &scales, center, center), None);
    }
}
