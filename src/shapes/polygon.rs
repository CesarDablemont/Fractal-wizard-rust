use eframe::egui::Pos2;
use crate::shapes::shape::Shape;
use crate::types::Line;

#[derive(Clone, Debug)]
pub struct Polygon {
    points: Vec<Pos2>,
    lines: Vec<Line>,
}

impl Polygon {
    pub fn new() -> Self {
        Self {
            points: Vec::new(),
            lines: Vec::new(),
        }
    }

    pub fn from_points(pts: Vec<Pos2>) -> Self {
        let n = pts.len();
        let lines = if n < 2 {
            Vec::new()
        } else {
            let mut l: Vec<Line> = (0..n - 1).map(|i| [i, i + 1]).collect();
            if n > 2 {
                l.push([n - 1, 0]);
            }
            l
        };
        Self { points: pts, lines }
    }

    fn rebuild_lines(&mut self) {
        if self.points.len() < 2 {
            self.lines.clear();
            return;
        }
        self.lines = (0..self.points.len() - 1)
            .map(|i| [i, i + 1])
            .collect();
        if self.points.len() > 2 {
            self.lines.push([self.points.len() - 1, 0]);
        }
    }
}

impl Shape for Polygon {
    fn points(&self) -> &[Pos2] { &self.points }
    fn points_mut(&mut self) -> &mut Vec<Pos2> { &mut self.points }
    fn lines(&self) -> &[Line] { &self.lines }
    fn add_point(&mut self, p: Pos2) {
        self.points.push(p);
        self.rebuild_lines();
    }

    fn remove_point(&mut self, idx: usize) {
        if idx < self.points.len() {
            self.points.remove(idx);
            self.rebuild_lines();
        }
    }
}
