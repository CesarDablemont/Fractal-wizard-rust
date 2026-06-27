use eframe::egui::{pos2, Pos2, Rect, Vec2};
use std::collections::HashMap;

pub struct ChunkGrid {
    cell_size: f32,
    cells: HashMap<(i32, i32), Vec<usize>>,
}

impl ChunkGrid {
    pub fn new(points: &[Pos2], cell_size: f32) -> Self {
        let mut cells: HashMap<(i32, i32), Vec<usize>> = HashMap::new();
        for (i, &p) in points.iter().enumerate() {
            let key = Self::pos_to_chunk(p, cell_size);
            cells.entry(key).or_default().push(i);
        }
        Self { cell_size, cells }
    }

    fn pos_to_chunk(pos: Pos2, cell_size: f32) -> (i32, i32) {
        (
            (pos.x / cell_size).floor() as i32,
            (pos.y / cell_size).floor() as i32,
        )
    }

    pub fn visible_indices(&self, viewport: Rect) -> Vec<usize> {
        let min_key = Self::pos_to_chunk(
            pos2(viewport.min.x, viewport.min.y),
            self.cell_size,
        );
        let max_key = Self::pos_to_chunk(
            pos2(viewport.max.x, viewport.max.y),
            self.cell_size,
        );

        let mut result = Vec::new();
        for cx in min_key.0..=max_key.0 {
            for cy in min_key.1..=max_key.1 {
                if let Some(indices) = self.cells.get(&(cx, cy)) {
                    result.extend(indices);
                }
            }
        }
        result
    }

    pub fn visible_lines(
        &self,
        lines: &[[usize; 2]],
        points: &[Pos2],
        viewport: Rect,
    ) -> Vec<usize> {
        let margin = viewport.size().length(); // generous margin for lines crossing viewport
        let expanded = Rect::from_min_max(
            viewport.min - Vec2::splat(margin),
            viewport.max + Vec2::splat(margin),
        );
        lines
            .iter()
            .enumerate()
            .filter(|(_, &[a, b])| {
                if a >= points.len() || b >= points.len() {
                    return false;
                }
                expanded.contains(points[a]) || expanded.contains(points[b])
            })
            .map(|(i, _)| i)
            .collect()
    }
}
