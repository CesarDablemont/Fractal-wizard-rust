use eframe::egui::{Pos2, Rect};
use std::collections::HashMap;

type ChunkKey = (i32, i32);

#[derive(Default)]
struct Cell {
    points: Vec<usize>,
    lines: Vec<usize>,
}

pub struct ChunkGrid {
    cell_size: f32,
    cells: HashMap<ChunkKey, Cell>,
    min_key: ChunkKey,
    max_key: ChunkKey,
}

impl ChunkGrid {
    pub fn new(points: &[Pos2], lines: &[[usize; 2]], cell_size: f32) -> Self {
        let mut cells: HashMap<ChunkKey, Cell> = HashMap::new();
        for (i, &p) in points.iter().enumerate() {
            let key = Self::pos_to_chunk(p, cell_size);
            cells.entry(key).or_default().points.push(i);
        }
        for (i, &[a, b]) in lines.iter().enumerate() {
            let (Some(&pa), Some(&pb)) = (points.get(a), points.get(b)) else {
                continue;
            };
            Self::for_each_cell_on_segment(pa, pb, cell_size, |key| {
                cells.entry(key).or_default().lines.push(i);
            });
        }

        let min_key = cells.keys().fold((i32::MAX, i32::MAX), |m, k| (m.0.min(k.0), m.1.min(k.1)));
        let max_key = cells.keys().fold((i32::MIN, i32::MIN), |m, k| (m.0.max(k.0), m.1.max(k.1)));
        Self { cell_size, cells, min_key, max_key }
    }

    fn pos_to_chunk(pos: Pos2, cell_size: f32) -> ChunkKey {
        (
            (pos.x / cell_size).floor() as i32,
            (pos.y / cell_size).floor() as i32,
        )
    }

    /// Visite chaque cellule traversée par le segment `[a, b]` (parcours DDA).
    fn for_each_cell_on_segment(a: Pos2, b: Pos2, cell_size: f32, mut visit: impl FnMut(ChunkKey)) {
        let (mut cx, mut cy) = Self::pos_to_chunk(a, cell_size);
        let end = Self::pos_to_chunk(b, cell_size);
        let d = b - a;

        let axis = |cell: i32, origin: f32, delta: f32| -> (i32, f32, f32) {
            if delta > 0.0 {
                (1, ((cell + 1) as f32 * cell_size - origin) / delta, cell_size / delta)
            } else if delta < 0.0 {
                (-1, (cell as f32 * cell_size - origin) / delta, -cell_size / delta)
            } else {
                (0, f32::INFINITY, f32::INFINITY)
            }
        };
        let (step_x, mut t_max_x, t_delta_x) = axis(cx, a.x, d.x);
        let (step_y, mut t_max_y, t_delta_y) = axis(cy, a.y, d.y);

        visit((cx, cy));
        // Le nombre de pas est borné par la distance de Manhattan entre les cellules
        // extrêmes : les imprécisions flottantes ne peuvent ni boucler ni dépasser `end`.
        while (cx, cy) != end {
            let step_along_x = cy == end.1 || (cx != end.0 && t_max_x < t_max_y);
            if step_along_x {
                cx += step_x;
                t_max_x += t_delta_x;
            } else {
                cy += step_y;
                t_max_y += t_delta_y;
            }
            visit((cx, cy));
        }
    }

    /// Cellules non vides recouvertes par `viewport`.
    fn visible_cells(&self, viewport: Rect) -> impl Iterator<Item = &Cell> {
        let min_key = Self::pos_to_chunk(viewport.min, self.cell_size);
        let max_key = Self::pos_to_chunk(viewport.max, self.cell_size);
        // Borné à l'emprise de la grille pour ne pas parcourir des cellules vides en dézoom.
        let (x0, x1) = (min_key.0.max(self.min_key.0), max_key.0.min(self.max_key.0));
        let (y0, y1) = (min_key.1.max(self.min_key.1), max_key.1.min(self.max_key.1));

        (x0..=x1)
            .flat_map(move |cx| (y0..=y1).map(move |cy| (cx, cy)))
            .filter_map(|key| self.cells.get(&key))
    }

    pub fn visible_indices(&self, viewport: Rect) -> Vec<usize> {
        self.visible_cells(viewport)
            .flat_map(|cell| cell.points.iter().copied())
            .collect()
    }

    /// Indices des lignes traversant au moins une cellule visible, sans doublon.
    pub fn visible_lines(&self, viewport: Rect) -> Vec<usize> {
        let mut result: Vec<usize> = self
            .visible_cells(viewport)
            .flat_map(|cell| cell.lines.iter().copied())
            .collect();
        result.sort_unstable();
        result.dedup();
        result
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use eframe::egui::pos2;

    fn cells_on(a: Pos2, b: Pos2, cell_size: f32) -> Vec<ChunkKey> {
        let mut v = Vec::new();
        ChunkGrid::for_each_cell_on_segment(a, b, cell_size, |k| v.push(k));
        v
    }

    #[test]
    fn segment_in_single_cell() {
        assert_eq!(cells_on(pos2(0.1, 0.1), pos2(0.9, 0.9), 1.0), vec![(0, 0)]);
    }

    #[test]
    fn segment_horizontal_and_reversed() {
        assert_eq!(cells_on(pos2(0.5, 0.5), pos2(3.5, 0.5), 1.0), vec![(0, 0), (1, 0), (2, 0), (3, 0)]);
        assert_eq!(cells_on(pos2(3.5, 0.5), pos2(-0.5, 0.5), 1.0), vec![(3, 0), (2, 0), (1, 0), (0, 0), (-1, 0)]);
    }

    #[test]
    fn segment_diagonal_is_connected() {
        let cells = cells_on(pos2(0.2, 0.7), pos2(4.6, 2.1), 1.0);
        assert_eq!(cells.first(), Some(&(0, 0)));
        assert_eq!(cells.last(), Some(&(4, 2)));
        for w in cells.windows(2) {
            assert_eq!((w[1].0 - w[0].0).abs() + (w[1].1 - w[0].1).abs(), 1);
        }
    }

    #[test]
    fn visible_lines_includes_line_crossing_viewport() {
        // Les deux extrémités sont hors du viewport, mais le segment le traverse.
        let points = [pos2(-10.0, 0.5), pos2(10.0, 0.5), pos2(20.0, 20.0), pos2(21.0, 21.0)];
        let lines = [[0, 1], [2, 3]];
        let grid = ChunkGrid::new(&points, &lines, 1.0);
        let viewport = Rect::from_min_max(pos2(-1.0, -1.0), pos2(1.0, 1.0));
        assert_eq!(grid.visible_lines(viewport), vec![0]);
    }

    #[test]
    fn visible_lines_has_no_duplicates() {
        let points = [pos2(0.5, 0.5), pos2(5.5, 0.5)];
        let lines = [[0, 1], [1, 0]];
        let grid = ChunkGrid::new(&points, &lines, 1.0);
        let viewport = Rect::from_min_max(pos2(-100.0, -100.0), pos2(100.0, 100.0));
        assert_eq!(grid.visible_lines(viewport), vec![0, 1]);
    }

    #[test]
    fn invalid_line_indices_are_ignored() {
        let points = [pos2(0.5, 0.5)];
        let lines = [[0, 3]];
        let grid = ChunkGrid::new(&points, &lines, 1.0);
        let viewport = Rect::from_min_max(pos2(0.0, 0.0), pos2(1.0, 1.0));
        assert!(grid.visible_lines(viewport).is_empty());
        assert_eq!(grid.visible_indices(viewport), vec![0]);
    }
}
