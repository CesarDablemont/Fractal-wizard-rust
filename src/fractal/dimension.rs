use eframe::egui::Pos2;
use std::collections::HashMap;

pub struct BoxCountingResult {
    pub dimension: f32,
    pub information_dimension: f32,
    pub correlation_dimension: f32,
    pub proportion_mean: f32,
    pub proportion_variance: f32,
    /// Spectre D_q pour q = 0, 1, 2, 3, 4.
    pub d_q_spectrum: [f32; 5],
}

/// Valeurs de q pour le spectre D_q (q ≥ 0 uniquement).
/// D₀ = dimension de capacité (boîtes)
/// D₁ = dimension d'information (Shannon)
/// D₂ = dimension de corrélation
/// D₃, D₄ = dimensions d'ordre supérieur
const Q_VALUES: [f32; 5] = [0.0, 1.0, 2.0, 3.0, 4.0];

/// Données brutes d'un niveau de box-counting.
struct LevelData {
    log_eps: f32,
    p_mean: f32,
    p_var: f32,
    /// Probabilités p_i = count_i / total_mass pour chaque boîte non vide.
    /// Utilisé pour calculer Σ p_i^q pour n'importe quel q.
    probs: Vec<f32>,
    /// Valeurs pré-calculées pour D₀, D₁, D₂.
    log_n: f32,
    entropy: f32,
    log_sum_p2: f32,
}

pub fn box_counting(points: &[Pos2], iterations: usize) -> Option<BoxCountingResult> {
    if points.is_empty() {
        return None;
    }

    let min_x = points.iter().map(|p| p.x).fold(f32::MAX, f32::min);
    let max_x = points.iter().map(|p| p.x).fold(f32::MIN, f32::max);
    let min_y = points.iter().map(|p| p.y).fold(f32::MAX, f32::min);
    let max_y = points.iter().map(|p| p.y).fold(f32::MIN, f32::max);

    let size = (max_x - min_x).max(max_y - min_y);
    if size <= 0.0 {
        return None;
    }

    let max_level = (iterations + 3).min((points.len().ilog2() + 1) as usize);

    let total_mass = points.len() as f32;
    let mut all_levels: Vec<LevelData> = Vec::with_capacity(max_level);

    for l in 1..=max_level {
        let epsilon = size / (1u64 << l) as f32;
        if epsilon <= 0.0 {
            continue;
        }

        let mut cells: HashMap<(i32, i32), usize> = HashMap::new();
        for p in points {
            let cx = ((p.x - min_x) / epsilon).floor() as i32;
            let cy = ((p.y - min_y) / epsilon).floor() as i32;
            *cells.entry((cx, cy)).or_default() += 1;
        }

        let non_empty = cells.len() as f32;
        if non_empty == 0.0 {
            continue;
        }

        let p_mean = 1.0 / non_empty;
        let mut sum_p2 = 0.0;
        let mut entropy = 0.0;
        let mut var_sum = 0.0;
        let mut probs: Vec<f32> = Vec::with_capacity(cells.len());

        for &count in cells.values() {
            let p_i = count as f32 / total_mass;
            probs.push(p_i);
            sum_p2 += p_i * p_i;
            if p_i > 0.0 {
                entropy -= p_i * p_i.ln();
            }
            let diff = p_i - p_mean;
            var_sum += diff * diff;
        }

        all_levels.push(LevelData {
            log_eps: (1.0 / epsilon).ln(),
            p_mean,
            p_var: var_sum / non_empty,
            probs,
            log_n: non_empty.ln(),
            entropy,
            log_sum_p2: sum_p2.ln(),
        });
    }

    if all_levels.len() < 2 {
        return None;
    }

    let log_eps: Vec<f32> = all_levels.iter().map(|d| d.log_eps).collect();

    // Zone de scaling unique basée sur D₀ (la plus robuste).
    let log_n_all: Vec<f32> = all_levels.iter().map(|d| d.log_n).collect();
    let (start, end) = find_scaling_region(&log_eps, &log_n_all);

    let dim = linear_regression_slope(&log_eps[start..end], &log_n_all[start..end]);

    let entropy_all: Vec<f32> = all_levels.iter().map(|d| d.entropy).collect();
    let info_dim = linear_regression_slope(&log_eps[start..end], &entropy_all[start..end]);

    let log_sp2_all: Vec<f32> = all_levels.iter().map(|d| d.log_sum_p2).collect();
    let corr_slope = linear_regression_slope(&log_eps[start..end], &log_sp2_all[start..end]);
    let corr_dim = -corr_slope;

    // Spectre D_q : même zone de scaling pour tous les q.
    let mut d_q_spectrum = [0.0_f32; 5];
    for (qi, &q) in Q_VALUES.iter().enumerate() {
        if q == 0.0 {
            d_q_spectrum[qi] = dim;
        } else if (q - 1.0).abs() < 1e-10 {
            d_q_spectrum[qi] = info_dim;
        } else {
            let log_sum_pq: Vec<f32> = all_levels
                .iter()
                .map(|d| {
                    let s: f32 = d.probs.iter().filter(|&&p| p > 0.0).map(|&p| p.powf(q)).sum();
                    if s > 0.0 { s.ln() } else { f32::MIN }
                })
                .collect();

            let slope = linear_regression_slope(&log_eps[start..end], &log_sum_pq[start..end]);
            d_q_spectrum[qi] = slope / (1.0 - q);
        }
    }

    let last = &all_levels[end - 1];

    Some(BoxCountingResult {
        dimension: dim,
        information_dimension: info_dim,
        correlation_dimension: corr_dim,
        proportion_mean: last.p_mean,
        proportion_variance: last.p_var,
        d_q_spectrum,
    })
}

/// Trouve la zone de scaling fractal dans une courbe log-log.
/// Sélectionne la plus longue séquence de pentes locales >= 80% de la pente max.
fn find_scaling_region(log_eps: &[f32], log_y: &[f32]) -> (usize, usize) {
    if log_eps.len() < 2 {
        return (0, log_eps.len());
    }

    let mut local_slopes: Vec<f32> = Vec::with_capacity(log_eps.len() - 1);
    for i in 1..log_eps.len() {
        let dy = log_y[i] - log_y[i - 1];
        let dx = log_eps[i] - log_eps[i - 1];
        if dx.abs() > 1e-10 {
            local_slopes.push(dy / dx);
        } else {
            local_slopes.push(0.0);
        }
    }

    let max_slope = local_slopes.iter().cloned().fold(0.0f32, f32::max);
    if max_slope <= 0.0 {
        return (0, log_eps.len().min(3));
    }

    let threshold = max_slope * 0.8;

    let mut best_start = 0usize;
    let mut best_len = 0usize;
    let mut cur_start = 0usize;
    let mut cur_len = 0usize;

    for (i, &slope) in local_slopes.iter().enumerate() {
        if slope >= threshold {
            if cur_len == 0 {
                cur_start = i;
            }
            cur_len += 1;
        } else {
            if cur_len > best_len {
                best_start = cur_start;
                best_len = cur_len;
            }
            cur_len = 0;
        }
    }
    if cur_len > best_len {
        best_start = cur_start;
        best_len = cur_len;
    }

    let start = best_start;
    let end = (best_start + best_len + 1).min(log_eps.len());

    if end - start < 2 {
        (0, log_eps.len().min(3))
    } else {
        (start, end)
    }
}

fn linear_regression_slope(x: &[f32], y: &[f32]) -> f32 {
    let n = x.len() as f32;
    let sum_x: f32 = x.iter().sum();
    let sum_y: f32 = y.iter().sum();
    let sum_xy_products: f32 = x.iter().zip(y.iter()).map(|(&a, &b)| a * b).sum();
    let sum_xx: f32 = x.iter().map(|&a| a * a).sum();

    let denom = n * sum_xx - sum_x * sum_x;
    if denom == 0.0 {
        return 0.0;
    }
    (n * sum_xy_products - sum_x * sum_y) / denom
}

#[cfg(test)]
mod tests {
    use super::*;
    use eframe::egui::pos2;

    #[test]
    fn box_counting_square() {
        let mut points = Vec::new();
        for x in 0..10 {
            for y in 0..10 {
                points.push(pos2(x as f32, y as f32));
            }
        }
        let result = box_counting(&points, 5);
        assert!(result.is_some());
        let r = result.unwrap();
        assert!(r.dimension > 1.5 && r.dimension < 2.5);
    }

    #[test]
    fn linear_regression_slope_known() {
        let x = vec![1.0, 2.0, 3.0, 4.0, 5.0];
        let y = vec![3.0, 5.0, 7.0, 9.0, 11.0];
        let slope = linear_regression_slope(&x, &y);
        assert!((slope - 2.0).abs() < 0.001);
    }

    #[test]
    fn dimensions_in_range() {
        let mut points = Vec::new();
        for x in 0..10 {
            for y in 0..10 {
                points.push(pos2(x as f32, y as f32));
            }
        }
        let result = box_counting(&points, 5);
        assert!(result.is_some());
        let r = result.unwrap();
        assert!(r.dimension >= 0.0 && r.dimension <= 2.0);
        assert!(r.information_dimension >= 0.0 && r.information_dimension <= 2.0);
        assert!(r.correlation_dimension >= 0.0 && r.correlation_dimension <= 2.0);
    }
}
