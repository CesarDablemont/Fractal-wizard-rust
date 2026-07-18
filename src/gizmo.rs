use eframe::egui::{Pos2, Rect, Shape, Stroke, Vec2};
use crate::scene::camera::Camera;

const ARROW_LENGTH: f32 = 80.0;
const ARROW_HEAD: f32 = 10.0;
const HIT_WIDTH: f32 = 8.0;
const CENTER_RADIUS: f32 = 8.0;

const COLOR_X: eframe::egui::Color32 = eframe::egui::Color32::from_rgb(220, 50, 50);
const COLOR_Y: eframe::egui::Color32 = eframe::egui::Color32::from_rgb(50, 200, 50);
const COLOR_CENTER: eframe::egui::Color32 = eframe::egui::Color32::from_rgb(80, 80, 220);
const COLOR_HOVER: eframe::egui::Color32 = eframe::egui::Color32::WHITE;

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum GizmoHit {
    None,
    X,
    Y,
    Center,
}

pub struct Gizmo;

impl Gizmo {
    pub fn draw(
        world_pos: Pos2,
        camera: &Camera,
        canvas_center: Pos2,
        hovered: GizmoHit,
        shapes: &mut Vec<Shape>,
    ) {
        let origin = camera.world_to_screen(world_pos, canvas_center);
        let stroke_w = 2.0;

        let x_color = if hovered == GizmoHit::X { COLOR_HOVER } else { COLOR_X };
        let y_color = if hovered == GizmoHit::Y { COLOR_HOVER } else { COLOR_Y };
        let c_color = if hovered == GizmoHit::Center { COLOR_HOVER } else { COLOR_CENTER };

        let x_end = origin + Vec2::new(ARROW_LENGTH, 0.0);
        let y_end = origin + Vec2::new(0.0, -ARROW_LENGTH);

        shapes.push(Shape::line_segment(
            [origin, x_end],
            Stroke::new(stroke_w, x_color),
        ));
        shapes.push(Shape::line_segment(
            [x_end + Vec2::new(-ARROW_HEAD, -ARROW_HEAD * 0.5), x_end],
            Stroke::new(stroke_w, x_color),
        ));
        shapes.push(Shape::line_segment(
            [x_end, x_end + Vec2::new(-ARROW_HEAD, ARROW_HEAD * 0.5)],
            Stroke::new(stroke_w, x_color),
        ));

        shapes.push(Shape::line_segment(
            [origin, y_end],
            Stroke::new(stroke_w, y_color),
        ));
        shapes.push(Shape::line_segment(
            [y_end + Vec2::new(-ARROW_HEAD * 0.5, ARROW_HEAD), y_end],
            Stroke::new(stroke_w, y_color),
        ));
        shapes.push(Shape::line_segment(
            [y_end, y_end + Vec2::new(ARROW_HEAD * 0.5, ARROW_HEAD)],
            Stroke::new(stroke_w, y_color),
        ));

        shapes.push(Shape::circle_filled(origin, CENTER_RADIUS, c_color));
    }

    pub fn hit_test(mouse: Pos2, world_pos: Pos2, camera: &Camera, canvas_center: Pos2) -> GizmoHit {
        let origin = camera.world_to_screen(world_pos, canvas_center);

        let to_center = mouse - origin;
        if to_center.length() <= CENTER_RADIUS + HIT_WIDTH {
            return GizmoHit::Center;
        }

        let x_rect = Rect::from_min_size(
            origin - Vec2::new(0.0, HIT_WIDTH),
            Vec2::new(ARROW_LENGTH, HIT_WIDTH * 2.0),
        );
        if x_rect.contains(mouse) {
            return GizmoHit::X;
        }

        let y_rect = Rect::from_min_size(
            origin + Vec2::new(-HIT_WIDTH, -ARROW_LENGTH),
            Vec2::new(HIT_WIDTH * 2.0, ARROW_LENGTH),
        );
        if y_rect.contains(mouse) {
            return GizmoHit::Y;
        }

        GizmoHit::None
    }

    pub fn handle_drag(
        hit: GizmoHit,
        screen_delta: Vec2,
        camera: &Camera,
    ) -> Vec2 {
        let world = camera.screen_delta_to_world(screen_delta);
        match hit {
            GizmoHit::X => Vec2::new(world.x, 0.0),
            GizmoHit::Y => Vec2::new(0.0, world.y),
            GizmoHit::Center => world,
            GizmoHit::None => Vec2::ZERO,
        }
    }
}
