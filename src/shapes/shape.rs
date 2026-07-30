use eframe::egui::{pos2, Pos2, Vec2};
use crate::types::Line;

pub trait Shape {
    fn points(&self) -> &[Pos2];
    fn points_mut(&mut self) -> &mut Vec<Pos2>;
    fn lines(&self) -> &[Line];
    fn add_point(&mut self, p: Pos2);
    fn remove_point(&mut self, idx: usize);

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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn translation_only() {
        let result = apply_transform(pos2(0.0, 0.0), pos2(5.0, 3.0), 0.0, Vec2::new(1.0, 1.0));
        assert_eq!(result, pos2(5.0, 3.0));
    }

    #[test]
    fn scale_only() {
        let result = apply_transform(pos2(2.0, 3.0), pos2(0.0, 0.0), 0.0, Vec2::new(3.0, 3.0));
        assert_eq!(result, pos2(6.0, 9.0));
    }

    #[test]
    fn rotation_90_degrees() {
        let result = apply_transform(pos2(1.0, 0.0), pos2(0.0, 0.0), std::f32::consts::FRAC_PI_2, Vec2::new(1.0, 1.0));
        assert!((result.x - 0.0).abs() < 0.001);
        assert!((result.y - 1.0).abs() < 0.001);
    }

    #[test]
    fn translation_rotation_scale() {
        let result = apply_transform(pos2(1.0, 0.0), pos2(10.0, 20.0), std::f32::consts::FRAC_PI_2, Vec2::new(2.0, 2.0));
        assert!((result.x - 10.0).abs() < 0.001);
        assert!((result.y - 22.0).abs() < 0.001);
    }
}
