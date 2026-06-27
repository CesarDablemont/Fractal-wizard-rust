use std::sync::Arc;
use eframe::egui::{pos2, Color32, Pos2, Rect, Shape, Stroke, Vec2};
use eframe::egui::epaint::{Mesh, Vertex};
use crate::scene::camera::Camera;
use crate::scene::chunk_grid::ChunkGrid;
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
            || self.cached_lines.as_ref().map_or(true, |c| {
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
        points: &[Pos2],
        point_scale: &[f32],
        colors: &[Option<Color32>],
        highlight: Option<usize>,
        shapes: &mut Vec<Shape>,
    ) {
        if !camera.display_points || points.is_empty() {
            return;
        }

        let center = canvas_rect.center();
        let half_size = camera.point_size / 2.0;
        let pixel_size = half_size * camera.zoom;
        if pixel_size < 0.5 {
            return;
        }

        let min_scale = 0.5 / camera.zoom;

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
                Color32::WHITE
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
    }
}
