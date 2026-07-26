use eframe::egui::Pos2;
use std::collections::HashMap;

pub struct BoxCountingResult {
    pub dimension: f32,
    pub information_dimension: f32,
    pub correlation_dimension: f32,
    pub proportion_mean: f32,
    pub proportion_variance: f32,
    /// Spectre de dimensions généralisées D_q pour q = -4, -2, 0, 1, 2, 4.
    /// Permet de distinguer monofractal (D_q constant) de multifractal (D_q varie).
    pub d_q_spectrum: [f32; 6],
}

/// Structure interne pour stocker les données d'un niveau de box-counting.
struct LevelData {
    log_eps: f32,
    log_n: f32,
    entropy: f32,
    log_sum_p2: f32,
    p_mean: f32,
    p_var: f32,
    /// Pour chaque valeur de q, la somme des p_i^q.
    /// Stocké comme Vec<(q, sum_pq, log_sum_pq)>.
    sum_pq: Vec<(f32, f32, f32)>,
}

/// Valeurs de q pour le spectre D_q.
const Q_VALUES: [f32; 6] = [-4.0, -2.0, 0.0, 1.0, 2.0, 4.0];

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

        // Calculer sum(p_i^q) pour chaque q
        let mut sum_pq: Vec<(f32, f32, f32)> = Vec::with_capacity(Q_VALUES.len());

        for &count in cells.values() {
            let p_i = count as f32 / total_mass;
            sum_p2 += p_i * p_i;
            if p_i > 0.0 {
                entropy -= p_i * p_i.ln();
            }
            let diff = p_i - p_mean;
            var_sum += diff * diff;
        }

        // Calculer sum(p_i^q) pour chaque q
        for &q in &Q_VALUES {
            let mut s = 0.0_f32;
            for &count in cells.values() {
                let p_i = count as f32 / total_mass;
                if p_i > 0.0 {
                    s += p_i.powf(q);
                }
            }
            let log_s = if s > 0.0 { s.ln() } else { f32::MIN };
            sum_pq.push((q, s, log_s));
        }

        all_levels.push(LevelData {
            log_eps: (1.0 / epsilon).ln(),
            log_n: non_empty.ln(),
            entropy,
            log_sum_p2: sum_p2.ln(),
            p_mean,
            p_var: var_sum / non_empty,
            sum_pq,
        });
    }

    if all_levels.len() < 2 {
        return None;
    }

    // Calculer les pentes locales entre niveaux consécutifs.
    let mut local_slopes: Vec<f32> = Vec::with_capacity(all_levels.len() - 1);
    for i in 1..all_levels.len() {
        let d_log_n = all_levels[i].log_n - all_levels[i - 1].log_n;
        let d_log_eps = all_levels[i].log_eps - all_levels[i - 1].log_eps;
        if d_log_eps.abs() > 1e-10 {
            local_slopes.push(d_log_n / d_log_eps);
        } else {
            local_slopes.push(0.0);
        }
    }

    // Trouver la pente maximale (zone de scaling fractal pur).
    let max_slope = local_slopes.iter().cloned().fold(0.0f32, f32::max);
    if max_slope <= 0.0 {
        return None;
    }

    // Seuil: garder les niveaux où la pente est >= 80% de la pente max.
    let threshold = max_slope * 0.8;

    // Trouver la plus longue séquence contiguë de pentes >= threshold.
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

    let mut start_level = best_start;
    let mut end_level = (best_start + best_len + 1).min(all_levels.len());

    if end_level - start_level < 2 {
        start_level = 0;
        end_level = all_levels.len().min(3);
    }

    let log_eps: Vec<f32> = all_levels[start_level..end_level]
        .iter()
        .map(|d| d.log_eps)
        .collect();
    let log_n: Vec<f32> = all_levels[start_level..end_level]
        .iter()
        .map(|d| d.log_n)
        .collect();
    let entropy_vals: Vec<f32> = all_levels[start_level..end_level]
        .iter()
        .map(|d| d.entropy)
        .collect();
    let log_sum_p2: Vec<f32> = all_levels[start_level..end_level]
        .iter()
        .map(|d| d.log_sum_p2)
        .collect();

    let last = &all_levels[end_level - 1];

    let dim = linear_regression_slope(&log_eps, &log_n);
    let info_dim = linear_regression_slope(&log_eps, &entropy_vals);
    let corr_slope = linear_regression_slope(&log_eps, &log_sum_p2);
    let corr_dim = -corr_slope;

    // Calculer le spectre D_q pour chaque valeur de q.
    // D_q = 1/(q-1) * lim(log(Z(q,eps)) / log(eps))
    // Comme log(eps) = -log(1/eps), et qu'on régresse vs log(1/eps):
    // D_q = -slope / (q-1) = slope / (1-q)
    let mut d_q_spectrum = [0.0_f32; 6];
    for (qi, &q) in Q_VALUES.iter().enumerate() {
        let log_sum_pq: Vec<f32> = all_levels[start_level..end_level]
            .iter()
            .map(|d| d.sum_pq[qi].2)
            .collect();
        let slope = linear_regression_slope(&log_eps, &log_sum_pq);
        if (q - 1.0).abs() > 1e-10 {
            d_q_spectrum[qi] = slope / (1.0 - q);
        } else {
            // q=1: utiliser la dimension d'information
            d_q_spectrum[qi] = info_dim;
        }
    }

    Some(BoxCountingResult {
        dimension: dim,
        information_dimension: info_dim,
        correlation_dimension: corr_dim,
        proportion_mean: last.p_mean,
        proportion_variance: last.p_var,
        d_q_spectrum,
    })
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
