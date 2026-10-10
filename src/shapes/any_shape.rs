use eframe::egui::{Pos2, Vec2};
use crate::shapes::free_linear::FreeLinearShape;
use crate::shapes::polygon::Polygon;
use crate::shapes::shape::Shape;
use crate::types::Line;

/// Forme éditée dans `FigureEditor` puis utilisée comme motif par `FractalEditor`.
#[derive(Clone, Debug)]
pub enum AnyShape {
    Polygon(Polygon),
    FreeLinear(FreeLinearShape),
}

impl AnyShape {
    pub fn name(&self) -> &str {
        match self {
            AnyShape::Polygon(_) => "Polygone",
            AnyShape::FreeLinear(_) => "Forme libre",
        }
    }

    /// Barycentre des points, `None` si la forme est vide.
    pub fn center(&self) -> Option<Pos2> {
        let pts = self.points();
        if pts.is_empty() {
            return None;
        }
        let sum = pts.iter().fold(Vec2::ZERO, |acc, p| acc + p.to_vec2());
        Some((sum / pts.len() as f32).to_pos2())
    }

    /// Indice du point dont le carré de côté `point_size` contient `world_pos`.
    pub fn hit_test(&self, world_pos: Pos2, point_size: f32) -> Option<usize> {
        let half = point_size / 2.0;
        self.points().iter().position(|&p| {
            let dx = (p.x - world_pos.x).abs();
            let dy = (p.y - world_pos.y).abs();
            dx <= half && dy <= half
        })
    }

    pub fn translate_all(&mut self, delta: Vec2) {
        for p in self.points_mut() {
            *p += delta;
        }
    }
}

impl Shape for AnyShape {
    fn points(&self) -> &[Pos2] {
        match self {
            AnyShape::Polygon(s) => s.points(),
            AnyShape::FreeLinear(s) => s.points(),
        }
    }

    fn points_mut(&mut self) -> &mut Vec<Pos2> {
        match self {
            AnyShape::Polygon(s) => s.points_mut(),
            AnyShape::FreeLinear(s) => s.points_mut(),
        }
    }

    fn lines(&self) -> &[Line] {
        match self {
            AnyShape::Polygon(s) => s.lines(),
            AnyShape::FreeLinear(s) => s.lines(),
        }
    }

    fn add_point(&mut self, p: Pos2) {
        match self {
            AnyShape::Polygon(s) => s.add_point(p),
            AnyShape::FreeLinear(s) => s.add_point(p),
        }
    }

    fn remove_point(&mut self, idx: usize) {
        match self {
            AnyShape::Polygon(s) => s.remove_point(idx),
            AnyShape::FreeLinear(s) => s.remove_point(idx),
        }
    }
}
