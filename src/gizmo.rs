#![allow(dead_code)]
use eframe::egui::{Pos2, Rect, Vec2};
use crate::scene::camera::Camera;

pub struct ArrowGizmo {
    pub position: Pos2,
    pub size: f32,
    pub dragging_all: bool,
    pub dragging_x: bool,
    pub dragging_y: bool,
}

impl Default for ArrowGizmo {
    fn default() -> Self {
        Self {
            position: Pos2::ZERO,
            size: 80.0,
            dragging_all: false,
            dragging_x: false,
            dragging_y: false,
        }
    }
}

impl ArrowGizmo {
    pub fn set_position(&mut self, pos: Pos2) {
        self.position = pos;
    }

    pub fn x_arrow_rect(&self, camera: &Camera, canvas_center: Pos2) -> Rect {
        let origin = camera.world_to_screen(self.position, canvas_center);
        Rect::from_min_max(
            origin - Vec2::new(0.0, 4.0),
            origin + Vec2::new(self.size, 4.0),
        )
    }

    pub fn y_arrow_rect(&self, camera: &Camera, canvas_center: Pos2) -> Rect {
        let origin = camera.world_to_screen(self.position, canvas_center);
        Rect::from_min_max(
            origin - Vec2::new(4.0, 0.0),
            origin + Vec2::new(4.0, self.size),
        )
    }

    pub fn all_rect(&self, camera: &Camera, canvas_center: Pos2) -> Rect {
        let origin = camera.world_to_screen(self.position, canvas_center);
        Rect::from_min_max(
            origin - Vec2::splat(8.0),
            origin + Vec2::splat(8.0),
        )
    }

    pub fn is_hovered(&self, mouse: Pos2, camera: &Camera, canvas_center: Pos2) -> GizmoHit {
        if self.all_rect(camera, canvas_center).contains(mouse) {
            return GizmoHit::All;
        }
        if self.x_arrow_rect(camera, canvas_center).contains(mouse) {
            return GizmoHit::X;
        }
        if self.y_arrow_rect(camera, canvas_center).contains(mouse) {
            return GizmoHit::Y;
        }
        GizmoHit::None
    }
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum GizmoHit {
    None,
    All,
    X,
    Y,
}
