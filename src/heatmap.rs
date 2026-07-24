use std::collections::HashMap;
use eframe::egui::Color32;
use crate::types::RandomWalkInfo;

pub fn calculate_global_heatmap(
    points_count: usize,
    simulations: &[RandomWalkInfo],
) -> Vec<f32> {
    let mut counts: HashMap<usize, u32> = HashMap::new();
    for i in 0..points_count {
        counts.insert(i, 0);
    }
    for sim in simulations {
        for &step in &sim.walk_steps {
            *counts.entry(step).or_insert(0) += 1;
        }
    }
    let max = counts.values().copied().max().unwrap_or(1).max(1);
    (0..points_count)
        .map(|i| counts.get(&i).copied().unwrap_or(0) as f32 / max as f32)
        .collect()
}

pub fn calculate_individual_heatmap(
    points_count: usize,
    simulation: &RandomWalkInfo,
) -> Vec<f32> {
    let mut counts: HashMap<usize, u32> = HashMap::new();
    for i in 0..points_count {
        counts.insert(i, 0);
    }
    for &step in &simulation.walk_steps {
        *counts.entry(step).or_insert(0) += 1;
    }
    let max = counts.values().copied().max().unwrap_or(1).max(1);
    (0..points_count)
        .map(|i| counts.get(&i).copied().unwrap_or(0) as f32 / max as f32)
        .collect()
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
