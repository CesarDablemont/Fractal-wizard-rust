use eframe::egui::{Pos2, Rect, Vec2};
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug)]
pub struct Camera {
    pub position: Vec2,
    pub zoom: f32,

    pub display_grid: bool,
    pub magnetism: bool,

    pub display_points: bool,
    pub display_lines: bool,
    pub point_size: f32,
    pub display_origin: bool,
}

impl Default for Camera {
    fn default() -> Self {
        Self {
            position: Vec2::ZERO,
            zoom: 10.0,
            display_grid: true,
            magnetism: true,
            display_points: true,
            display_lines: true,
            point_size: 6.0,
            display_origin: true,
        }
    }
}

/// Réglages d'affichage de la caméra, mémorisés entre deux lancements.
#[derive(Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct CameraSettings {
    pub display_grid: bool,
    pub magnetism: bool,
    pub display_points: bool,
    pub display_lines: bool,
    pub point_size: f32,
    pub display_origin: bool,
}

impl Default for CameraSettings {
    fn default() -> Self {
        Camera::default().settings()
    }
}

impl Camera {
    pub fn settings(&self) -> CameraSettings {
        CameraSettings {
            display_grid: self.display_grid,
            magnetism: self.magnetism,
            display_points: self.display_points,
            display_lines: self.display_lines,
            point_size: self.point_size,
            display_origin: self.display_origin,
        }
    }

    pub fn apply_settings(&mut self, settings: &CameraSettings) {
        self.display_grid = settings.display_grid;
        self.magnetism = settings.magnetism;
        self.display_points = settings.display_points;
        self.display_lines = settings.display_lines;
        self.point_size = settings.point_size;
        self.display_origin = settings.display_origin;
    }

    pub fn screen_to_world(&self, screen: Pos2, canvas_center: Pos2) -> Pos2 {
        let delta = screen - canvas_center;
        let world_delta = delta / self.zoom;
        Pos2::new(
            self.position.x + world_delta.x,
            self.position.y - world_delta.y, // Y-up
        )
    }

    pub fn world_to_screen(&self, world: Pos2, canvas_center: Pos2) -> Pos2 {
        let delta = Vec2::new(
            world.x - self.position.x,
            -(world.y - self.position.y), // Y-up
        );
        canvas_center + delta * self.zoom
    }

    pub fn visible_world_rect(&self, canvas_rect: Rect, canvas_center: Pos2) -> Rect {
        let top_left = self.screen_to_world(canvas_rect.min, canvas_center);
        let bot_right = self.screen_to_world(canvas_rect.max, canvas_center);
        let min_x = top_left.x.min(bot_right.x);
        let max_x = top_left.x.max(bot_right.x);
        let min_y = top_left.y.min(bot_right.y);
        let max_y = top_left.y.max(bot_right.y);
        Rect::from_min_max(Pos2::new(min_x, min_y), Pos2::new(max_x, max_y))
    }

    pub fn zoom_at(&mut self, factor: f32, screen_pos: Pos2, canvas_center: Pos2) {
        let world_before = self.screen_to_world(screen_pos, canvas_center);
        self.zoom *= factor;
        self.zoom = self.zoom.clamp(0.01, 100.0);
        let world_after = self.screen_to_world(screen_pos, canvas_center);
        self.position += world_before.to_vec2() - world_after.to_vec2();
    }

    pub fn pan(&mut self, delta: Vec2) {
        self.position -= Vec2::new(delta.x, -delta.y) / self.zoom;
    }

    pub fn screen_delta_to_world(&self, screen_delta: Vec2) -> Vec2 {
        Vec2::new(screen_delta.x / self.zoom, -screen_delta.y / self.zoom)
    }

    pub fn choose_grid_spacing(zoom: f32) -> f32 {
        let target = 50.0 / zoom;
        let candidates = [5.0, 25.0, 100.0, 500.0, 2500.0, 12500.0];
        candidates.into_iter().min_by(|a, b| {
            let da = (a - target).abs();
            let db = (b - target).abs();
            da.partial_cmp(&db).unwrap()
        }).unwrap_or(25.0)
    }

    pub fn snap(&self, world_pos: Pos2) -> Pos2 {
        if self.magnetism {
            let spacing = Self::choose_grid_spacing(self.zoom);
            Pos2::new(
                (world_pos.x / spacing).round() * spacing,
                (world_pos.y / spacing).round() * spacing,
            )
        } else {
            world_pos
        }
    }
}
