use eframe::egui::Pos2;
use crate::shapes::shape::Shape;
use crate::types::Line;

#[derive(Clone, Debug)]
pub struct FreeLinearShape {
    points: Vec<Pos2>,
    lines: Vec<Line>,
}

impl FreeLinearShape {
    pub fn new() -> Self {
        Self {
            points: Vec::new(),
            lines: Vec::new(),
        }
    }

    pub fn add_line_segment(&mut self, a: usize, b: usize) {
        if a < self.points.len() && b < self.points.len() {
            self.lines.push([a, b]);
        }
    }
}

impl Shape for FreeLinearShape {
    fn points(&self) -> &[Pos2] { &self.points }
    fn points_mut(&mut self) -> &mut Vec<Pos2> { &mut self.points }
    fn lines(&self) -> &[Line] { &self.lines }

    fn add_point(&mut self, p: Pos2) {
        if !self.points.is_empty() {
            self.lines.push([self.points.len() - 1, self.points.len()]);
        }
        self.points.push(p);
    }

    fn remove_point(&mut self, idx: usize) {
        if idx >= self.points.len() {
            return;
        }
        self.points.remove(idx);
        self.lines.retain_mut(|l| {
            if l[0] == idx || l[1] == idx {
                return false;
            }
            if l[0] > idx { l[0] -= 1; }
            if l[1] > idx { l[1] -= 1; }
            true
        });
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use eframe::egui::pos2;

    #[test]
    fn add_point_first_adds_no_line() {
        let mut s = FreeLinearShape::new();
        s.add_point(pos2(0.0, 0.0));
        assert_eq!(s.points().len(), 1);
        assert!(s.lines().is_empty());
    }

    #[test]
    fn add_point_second_creates_line() {
        let mut s = FreeLinearShape::new();
        s.add_point(pos2(0.0, 0.0));
        s.add_point(pos2(1.0, 0.0));
        assert_eq!(s.points().len(), 2);
        assert_eq!(s.lines(), &[[0, 1]]);
    }

    #[test]
    fn remove_point_middle_removes_incident_lines() {
        let mut s = FreeLinearShape::new();
        s.add_point(pos2(0.0, 0.0));
        s.add_point(pos2(1.0, 0.0));
        s.add_point(pos2(2.0, 0.0));
        // lines: [[0,1], [1,2]]
        s.remove_point(1);
        assert_eq!(s.points().len(), 2);
        assert_eq!(s.points()[0], pos2(0.0, 0.0));
        assert_eq!(s.points()[1], pos2(2.0, 0.0));
        // lines touching removed index are removed
        assert!(s.lines().is_empty());
    }

    #[test]
    fn remove_point_last() {
        let mut s = FreeLinearShape::new();
        s.add_point(pos2(0.0, 0.0));
        s.add_point(pos2(1.0, 0.0));
        s.remove_point(1);
        assert_eq!(s.points().len(), 1);
        assert!(s.lines().is_empty());
    }
}
