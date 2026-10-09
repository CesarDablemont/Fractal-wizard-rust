use eframe::egui::Color32;
use crate::types::RandomWalkInfo;

/// Ajoute à `visits` les passages de `simulation`, indexés par point.
pub fn add_visits(visits: &mut [u32], simulation: &RandomWalkInfo) {
    for &step in &simulation.walk_steps {
        visits[step] += 1;
    }
}

/// Ramène les passages entre 0 et 1, relativement au point le plus visité.
pub fn normalize(visits: &[u32]) -> Vec<f32> {
    let max = visits.iter().copied().max().unwrap_or(0).max(1) as f32;
    visits.iter().map(|&v| v as f32 / max).collect()
}

pub fn calculate_individual_heatmap(
    points_count: usize,
    simulation: &RandomWalkInfo,
) -> Vec<f32> {
    let mut visits = vec![0; points_count];
    add_visits(&mut visits, simulation);
    normalize(&visits)
}

pub fn heatmap_color(score: f32) -> Color32 {
    let min = Color32::from_rgb(0, 0, 255);
    let mid = Color32::from_rgb(255, 255, 0);
    let max = Color32::from_rgb(255, 0, 0);

    let t = score.clamp(0.0, 1.0);
    if t < 0.5 {
        let u = t / 0.5;
        Color32::from_rgb(
            (f32::from(min[0]) + (f32::from(mid[0]) - f32::from(min[0])) * u) as u8,
            (f32::from(min[1]) + (f32::from(mid[1]) - f32::from(min[1])) * u) as u8,
            (f32::from(min[2]) + (f32::from(mid[2]) - f32::from(min[2])) * u) as u8,
        )
    } else {
        let u = (t - 0.5) / 0.5;
        Color32::from_rgb(
            (f32::from(mid[0]) + (f32::from(max[0]) - f32::from(mid[0])) * u) as u8,
            (f32::from(mid[1]) + (f32::from(max[1]) - f32::from(mid[1])) * u) as u8,
            (f32::from(mid[2]) + (f32::from(max[2]) - f32::from(mid[2])) * u) as u8,
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn walk(steps: &[usize]) -> RandomWalkInfo {
        RandomWalkInfo { walk_steps: steps.to_vec(), ..RandomWalkInfo::default() }
    }

    #[test]
    fn visits_accumulate_across_simulations() {
        let mut visits = vec![0; 3];
        add_visits(&mut visits, &walk(&[0, 1, 0]));
        add_visits(&mut visits, &walk(&[0, 2]));
        assert_eq!(visits, vec![3, 1, 1]);
        assert_eq!(normalize(&visits), vec![1.0, 1.0 / 3.0, 1.0 / 3.0]);
    }

    #[test]
    fn normalize_without_visits_is_zero() {
        assert_eq!(normalize(&[0, 0]), vec![0.0, 0.0]);
    }
}
