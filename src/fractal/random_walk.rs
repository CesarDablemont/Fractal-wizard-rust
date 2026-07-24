use eframe::egui::Pos2;
use rand::Rng;
use crate::types::{Line, RandomWalkInfo};

pub struct RandomWalkStats {
    pub success_count: u64,
    pub polya_number: f32,
    pub average_steps: f32,
    pub variance_steps: f32,
    pub std_dev_steps: f32,
    pub average_length: f32,
}

pub fn run_simulations(
    points: &[Pos2],
    lines: &[Line],
    start_index: usize,
    count: u32,
    min_steps: u64,
    max_steps: u64,
) -> (Vec<RandomWalkInfo>, RandomWalkStats) {
    let mut simulations = Vec::with_capacity(count as usize);
    let mut rng = rand::rng();

    for _ in 0..count {
        let sim = run_single(points, lines, start_index, max_steps, &mut rng);
        simulations.push(sim);
    }

    if min_steps > 0 {
        for sim in &mut simulations {
            while sim.steps() < min_steps as usize {
                *sim = run_single(points, lines, start_index, max_steps, &mut rng);
            }
        }
    }

    compute_stats(&simulations, count)
}

fn run_single(
    points: &[Pos2],
    lines: &[Line],
    start: usize,
    max_steps: u64,
    rng: &mut impl Rng,
) -> RandomWalkInfo {
    let mut info = RandomWalkInfo {
        max_steps,
        ..Default::default()
    };
    info.walk_steps.push(start);

    let mut current = start;
    while info.steps() < max_steps as usize {
        let connected: Vec<usize> = lines
            .iter()
            .filter(|l| l[0] == current || l[1] == current)
            .map(|l| if l[0] == current { l[1] } else { l[0] })
            .collect();

        if connected.is_empty() {
            break;
        }

        let next_idx = rng.random_range(0..connected.len());
        let next = connected[next_idx];

        let dx = points[next].x - points[info.walk_steps[info.walk_steps.len() - 1]].x;
        let dy = points[next].y - points[info.walk_steps[info.walk_steps.len() - 1]].y;
        info.length_walk += (dx * dx + dy * dy).sqrt();

        info.walk_steps.push(next);
        current = next;

        if next == start {
            info.is_random_walk_done = true;
            break;
        }
    }

    info
}

fn compute_stats(
    simulations: &[RandomWalkInfo],
    total_count: u32,
) -> (Vec<RandomWalkInfo>, RandomWalkStats) {
    let successful: Vec<&RandomWalkInfo> =
        simulations.iter().filter(|s| s.is_random_walk_done).collect();
    let success_count = successful.len() as u64;

    if success_count == 0 {
        return (
            simulations.to_vec(),
            RandomWalkStats {
                success_count: 0,
                polya_number: 0.0,
                average_steps: 0.0,
                variance_steps: 0.0,
                std_dev_steps: 0.0,
                average_length: 0.0,
            },
        );
    }

    let avg_steps: f32 =
        successful.iter().map(|s| s.steps() as f32).sum::<f32>() / success_count as f32;
    let avg_length: f32 =
        successful.iter().map(|s| s.length_walk).sum::<f32>() / success_count as f32;
    let polya = (success_count as f32 / total_count as f32) * 100.0;

    let squared_mean: f32 = successful
        .iter()
        .map(|s| (s.steps() as f32) * (s.steps() as f32))
        .sum::<f32>()
        / success_count as f32;
    let variance = squared_mean - avg_steps * avg_steps;
    let std_dev = variance.sqrt();

    (
        simulations.to_vec(),
        RandomWalkStats {
            success_count,
            polya_number: polya,
            average_steps: avg_steps,
            variance_steps: variance,
            std_dev_steps: std_dev,
            average_length: avg_length,
        },
    )
}
