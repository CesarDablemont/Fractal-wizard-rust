use eframe::egui::Pos2;
use std::collections::HashMap;

pub struct BoxCountingResult {
    pub dimension: f32,
    pub information_dimension: f32,
    pub correlation_dimension: f32,
    pub proportion_mean: f32,
    pub proportion_variance: f32,
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

    let mut log_eps = Vec::with_capacity(max_level);
    let mut log_n = Vec::with_capacity(max_level);
    let mut entropy_vals = Vec::with_capacity(max_level);
    let mut log_sum_p2 = Vec::with_capacity(max_level);
    let total_mass = points.len() as f32;
    let mut last_p_mean = 0.0;
    let mut last_p_var = 0.0;

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

        for &count in cells.values() {
            let p_i = count as f32 / total_mass;
            sum_p2 += p_i * p_i;
            if p_i > 0.0 {
                entropy -= p_i * p_i.ln();
            }
            let diff = p_i - p_mean;
            var_sum += diff * diff;
        }

        if non_empty < total_mass * 0.9 || l <= 3 {
            last_p_mean = p_mean;
            last_p_var = var_sum / non_empty;
            log_eps.push((1.0 / epsilon).ln());
            log_n.push(non_empty.ln());
            entropy_vals.push(entropy);
            log_sum_p2.push(sum_p2.ln());
        }
    }

    if log_eps.len() < 2 {
        return None;
    }

    let dim = linear_regression_slope(&log_eps, &log_n);
    let info_dim = linear_regression_slope(&log_eps, &entropy_vals);
    let corr_slope = linear_regression_slope(&log_eps, &log_sum_p2);
    let corr_dim = -corr_slope;

    Some(BoxCountingResult {
        dimension: dim,
        information_dimension: info_dim,
        correlation_dimension: corr_dim,
        proportion_mean: last_p_mean,
        proportion_variance: last_p_var,
    })
}

fn linear_regression_slope(x: &[f32], y: &[f32]) -> f32 {
    let n = x.len() as f32;
    let sum_x: f32 = x.iter().sum();
    let sum_y: f32 = y.iter().sum();
    let sum_xy: f32 = x.iter().zip(y.iter()).map(|(&a, &b)| a * b).sum();
    let sum_xx: f32 = x.iter().map(|&a| a * a).sum();

    let denom = n * sum_xx - sum_x * sum_x;
    if denom == 0.0 {
        return 0.0;
    }
    (n * sum_xy - sum_x * sum_y) / denom
}
