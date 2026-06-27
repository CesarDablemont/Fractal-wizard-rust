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

    pub fn connect_start_end(&mut self) {
        if self.points.len() >= 2 {
            self.lines.push([self.points.len() - 1, self.points.len() - 2]);
        }
    }
}

impl Shape for FreeLinearShape {
    fn name(&self) -> &'static str { "cFreeLinearShape" }
    fn points(&self) -> &[Pos2] { &self.points }
    fn points_mut(&mut self) -> &mut Vec<Pos2> { &mut self.points }
    fn lines(&self) -> &[Line] { &self.lines }
    fn set_lines(&mut self, lines: Vec<Line>) { self.lines = lines; }
    fn is_closed(&self) -> bool { false }

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
