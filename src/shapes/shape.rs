use eframe::egui::{pos2, Pos2, Vec2};
use crate::types::Line;

pub trait Shape {
    fn name(&self) -> &'static str;
    fn points(&self) -> &[Pos2];
    fn points_mut(&mut self) -> &mut Vec<Pos2>;
    fn lines(&self) -> &[Line];
    fn set_lines(&mut self, lines: Vec<Line>);
    fn add_point(&mut self, p: Pos2);
    fn remove_point(&mut self, idx: usize);
    fn is_closed(&self) -> bool;

    fn get_transformed_points(&self, translate: Pos2, rotate: f32, scale: f32) -> Vec<Pos2> {
        self.points()
            .iter()
            .map(|&p| apply_transform(p, translate, rotate, Vec2::new(scale, scale)))
            .collect()
    }

    fn get_lines(&self, translate: Pos2, rotate: f32, scale: f32) -> Vec<[Pos2; 2]> {
        self.lines()
            .iter()
            .map(|&[a, b]| {
                [
                    apply_transform(self.points()[a], translate, rotate, Vec2::new(scale, scale)),
                    apply_transform(self.points()[b], translate, rotate, Vec2::new(scale, scale)),
                ]
            })
            .collect()
    }

    fn hit_test(&self, world_pos: Pos2, point_size: f32) -> Option<usize> {
        let half = point_size / 2.0;
        self.points().iter().position(|&p| {
            let dx = (p.x - world_pos.x).abs();
            let dy = (p.y - world_pos.y).abs();
            dx <= half && dy <= half
        })
    }
}

pub fn apply_transform(
    point: Pos2,
    translate: Pos2,
    rotate: f32,
    scale: Vec2,
) -> Pos2 {
    let cos = rotate.cos();
    let sin = rotate.sin();
    let x = point.x * cos - point.y * sin;
    let y = point.x * sin + point.y * cos;
    pos2(
        x * scale.x + translate.x,
        y * scale.y + translate.y,
    )
}
