use eframe::egui::Pos2;
use crate::types::Line;

/// En dessous de cette longueur, une liaison est considérée comme dégénérée
/// (sommets confondus) et ignorée pour le calcul de la distance de référence.
const MIN_LENGTH: f32 = 1e-3;

fn edge_length(points: &[Pos2], &[a, b]: &Line) -> f32 {
    points[a].distance(points[b])
}

/// Distance de référence `d0` : longueur médiane des liaisons.
///
/// La médiane correspond à la liaison la plus fréquente (les plus petites
/// dans une fractale) tout en restant robuste au bruit de `delta_radius`.
/// Renvoie 1 s'il n'y a aucune liaison exploitable.
pub fn reference_distance(points: &[Pos2], lines: &[Line]) -> f32 {
    let mut lengths: Vec<f32> = lines
        .iter()
        .map(|l| edge_length(points, l))
        .filter(|&d| d > MIN_LENGTH)
        .collect();
    if lengths.is_empty() {
        return 1.0;
    }
    let mid = lengths.len() / 2;
    let (_, median, _) = lengths.select_nth_unstable_by(mid, f32::total_cmp);
    *median
}

/// Poids relatif d'une liaison : `exp(-beta * (d / d0 - 1))`.
///
/// Vaut 1 pour une liaison de longueur `d0`, plus pour une liaison plus courte,
/// moins pour une plus longue. `beta` est sans dimension : le résultat ne
/// dépend pas de l'échelle du dessin.
pub fn hopping_weight(dist: f32, d0: f32, beta: f32) -> f32 {
    (-beta * (dist / d0 - 1.0)).exp()
}

/// Poids relatif de chaque liaison, dans l'ordre de `lines`.
pub fn edge_weights(points: &[Pos2], lines: &[Line], beta: f32) -> Vec<f32> {
    let d0 = reference_distance(points, lines);
    lines
        .iter()
        .map(|l| hopping_weight(edge_length(points, l), d0, beta))
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use eframe::egui::pos2;

    #[test]
    fn reference_distance_is_median() {
        let points = [pos2(0.0, 0.0), pos2(1.0, 0.0), pos2(1.0, 1.0), pos2(1.0, 4.0)];
        let lines = [[0, 1], [1, 2], [2, 3]];
        assert!((reference_distance(&points, &lines) - 1.0).abs() < 1e-6);
    }

    #[test]
    fn reference_distance_ignores_degenerate_edges() {
        let points = [pos2(0.0, 0.0), pos2(0.0, 0.0), pos2(2.0, 0.0)];
        let lines = [[0, 1], [1, 2]];
        assert!((reference_distance(&points, &lines) - 2.0).abs() < 1e-6);
    }

    #[test]
    fn weight_is_one_at_reference_distance() {
        assert!((hopping_weight(3.0, 3.0, 5.0) - 1.0).abs() < 1e-6);
    }

    #[test]
    fn weights_are_scale_invariant() {
        let small = [pos2(0.0, 0.0), pos2(1.0, 0.0), pos2(1.0, 2.0)];
        let big: Vec<Pos2> = small.iter().map(|p| pos2(p.x * 40.0, p.y * 40.0)).collect();
        let lines = [[0, 1], [1, 2], [2, 0]];
        let w_small = edge_weights(&small, &lines, 2.0);
        let w_big = edge_weights(&big, &lines, 2.0);
        for (a, b) in w_small.iter().zip(&w_big) {
            assert!((a - b).abs() < 1e-5);
        }
    }

    #[test]
    fn zero_beta_gives_uniform_weights() {
        let points = [pos2(0.0, 0.0), pos2(3.0, 4.0), pos2(3.0, 5.0)];
        let lines = [[0, 1], [1, 2]];
        for w in edge_weights(&points, &lines, 0.0) {
            assert!((w - 1.0).abs() < 1e-6);
        }
    }
}
