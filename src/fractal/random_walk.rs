use eframe::egui::Pos2;
use rand::Rng;
use rand::rngs::ThreadRng;
use crate::fractal::generator::merge_vertices;
use crate::types::{Line, RandomWalkInfo};

const MAX_SIMULATION_TIME: f64 = 5.0;

pub struct RandomWalkStats {
    pub success_count: u64,
    pub polya_number: f32,
    pub average_steps: f32,
    pub variance_steps: f32,
    pub std_dev_steps: f32,
    pub average_length: f32,
    pub max_simulation_time: f64,
}

pub struct SimulationRunner {
    points: Vec<Pos2>,
    lines: Vec<Line>,
    start_index: usize,
    min_steps: u64,
    max_steps: u64,
    beta: f32,
    total_count: u32,
    done_count: u32,
    max_simulation_time: f64,
    rng: ThreadRng,
}

impl SimulationRunner {
    pub fn new(
        points: &[Pos2],
        lines: &[Line],
        start_index: usize,
        count: u32,
        min_steps: u64,
        max_steps: u64,
        beta: f32,
    ) -> Self {
        let (canonical, lines) = merge_vertices(points, lines);
        let start_index = canonical[start_index];
        Self {
            points: points.to_vec(),
            lines,
            start_index,
            min_steps,
            max_steps,
            beta,
            total_count: count,
            done_count: 0,
            max_simulation_time: 0.0,
            rng: rand::rng(),
        }
    }

    pub fn is_done(&self) -> bool {
        self.done_count >= self.total_count
    }

    pub fn progress(&self) -> (u32, u32) {
        (self.done_count, self.total_count)
    }

    pub fn max_simulation_time(&self) -> f64 {
        self.max_simulation_time
    }

    pub fn run_next(&mut self) -> RandomWalkInfo {
        let start = std::time::Instant::now();
        let sim = run_with_min_steps(
            &self.points,
            &self.lines,
            self.start_index,
            self.min_steps,
            self.max_steps,
            self.beta,
            &mut self.rng,
        );
        self.max_simulation_time = self.max_simulation_time.max(start.elapsed().as_secs_f64());
        self.done_count += 1;
        sim
    }
}

fn run_with_min_steps(
    points: &[Pos2],
    lines: &[Line],
    start: usize,
    min_steps: u64,
    max_steps: u64,
    beta: f32,
    rng: &mut impl Rng,
) -> RandomWalkInfo {
    let start_time = std::time::Instant::now();
    let mut sim = run_single(points, lines, start, max_steps, beta, rng);
    while min_steps > 0
        && sim.steps() < min_steps as usize
        && !sim.timed_out
        && start_time.elapsed().as_secs_f64() <= MAX_SIMULATION_TIME
    {
        sim = run_single(points, lines, start, max_steps, beta, rng);
    }
    sim
}

fn run_single(
    points: &[Pos2],
    lines: &[Line],
    start: usize,
    max_steps: u64,
    beta: f32,
    rng: &mut impl Rng,
) -> RandomWalkInfo {
    let mut info = RandomWalkInfo::default();
    info.walk_steps.push(start);

    let start_time = std::time::Instant::now();
    let mut current = start;
    while info.steps() < max_steps as usize
        && start_time.elapsed().as_secs_f64() <= MAX_SIMULATION_TIME
    {
        let connected: Vec<usize> = lines
            .iter()
            .filter(|l| l[0] == current || l[1] == current)
            .map(|l| if l[0] == current { l[1] } else { l[0] })
            .collect();

        if connected.is_empty() {
            break;
        }

        let next = if beta == 0.0 || connected.len() == 1 {
            connected[rng.random_range(0..connected.len())]
        } else {
            let cur = points[current];
            let weights: Vec<f32> = connected
                .iter()
                .map(|&idx| {
                    let dx = points[idx].x - cur.x;
                    let dy = points[idx].y - cur.y;
                    let dist = (dx * dx + dy * dy).sqrt().max(0.001);
                    (-beta * dist).exp()
                })
                .collect();
            let total: f32 = weights.iter().sum();
            let mut r = rng.random::<f32>() * total;
            let mut chosen = connected[0];
            for (i, &w) in weights.iter().enumerate() {
                r -= w;
                if r <= 0.0 {
                    chosen = connected[i];
                    break;
                }
            }
            chosen
        };

        let dx = points[next].x - points[current].x;
        let dy = points[next].y - points[current].y;
        info.length_walk += (dx * dx + dy * dy).sqrt();

        info.walk_steps.push(next);
        current = next;

        if next == start {
            info.is_random_walk_done = true;
            break;
        }
    }

    if !info.is_random_walk_done
        && info.steps() < max_steps as usize
        && start_time.elapsed().as_secs_f64() > MAX_SIMULATION_TIME
    {
        info.timed_out = true;
    }

    info
}

pub fn calculate_stats(
    simulations: &[RandomWalkInfo],
    total_count: u32,
) -> RandomWalkStats {
    let successful: Vec<&RandomWalkInfo> =
        simulations.iter().filter(|s| s.is_random_walk_done).collect();
    let success_count = successful.len() as u64;

    if success_count == 0 {
        return RandomWalkStats {
            success_count: 0,
            polya_number: 0.0,
            average_steps: 0.0,
            variance_steps: 0.0,
            std_dev_steps: 0.0,
            average_length: 0.0,
            max_simulation_time: 0.0,
        };
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

    RandomWalkStats {
        success_count,
        polya_number: polya,
        average_steps: avg_steps,
        variance_steps: variance,
        std_dev_steps: std_dev,
        average_length: avg_length,
        max_simulation_time: 0.0,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use eframe::egui::pos2;

    fn run_all(
        points: &[Pos2],
        lines: &[Line],
        start: usize,
        count: u32,
        min_steps: u64,
        max_steps: u64,
        beta: f32,
    ) -> (Vec<RandomWalkInfo>, RandomWalkStats) {
        let mut runner = SimulationRunner::new(points, lines, start, count, min_steps, max_steps, beta);
        let mut sims = Vec::with_capacity(count as usize);
        while !runner.is_done() {
            sims.push(runner.run_next());
        }
        let mut stats = calculate_stats(&sims, count);
        stats.max_simulation_time = runner.max_simulation_time();
        (sims, stats)
    }

    #[test]
    fn trivial_triangle() {
        let points = vec![pos2(0.0, 0.0), pos2(1.0, 0.0), pos2(0.5, 0.866)];
        let lines = vec![[0, 1], [1, 2], [2, 0]];
        let (_, stats) = run_all(&points, &lines, 0, 10, 0, 100, 0.0);
        assert!(stats.success_count > 0);
    }

    #[test]
    fn compute_stats_valid() {
        let points = vec![pos2(0.0, 0.0), pos2(1.0, 0.0), pos2(0.5, 0.866)];
        let lines = vec![[0, 1], [1, 2], [2, 0]];
        let (_, stats) = run_all(&points, &lines, 0, 20, 0, 100, 0.0);
        assert!(stats.polya_number >= 0.0 && stats.polya_number <= 100.0);
        assert!(stats.average_steps > 0.0);
        assert!(stats.variance_steps >= 0.0);
    }

    #[test]
    fn max_steps_respected() {
        let points = vec![pos2(0.0, 0.0), pos2(1.0, 0.0), pos2(0.5, 0.866)];
        let lines = vec![[0, 1], [1, 2]];
        let max_steps = 5;
        let (sims, _) = run_all(&points, &lines, 0, 5, 0, max_steps, 0.0);
        for sim in &sims {
            assert!(sim.steps() <= max_steps as usize);
        }
    }

    #[test]
    fn min_steps_respected() {
        use crate::fractal::generator;
        use crate::shapes::shape::apply_transform;
        use crate::types::ShapePatternData;
        use eframe::egui::vec2;

        let base = vec![pos2(-5.0, 0.0), pos2(5.0, 0.0), pos2(0.0, 8.660254)];
        let pattern = vec![
            ShapePatternData { translate: pos2(-2.5, 0.0), rotate: 0.0, scale: 2.0 },
            ShapePatternData { translate: pos2(2.5, 0.0), rotate: 0.0, scale: 2.0 },
            ShapePatternData { translate: pos2(-6.7055225e-8, 4.330127), rotate: 0.0, scale: 2.0 },
        ];
        let initial = vec![ShapePatternData::default()];
        let config = generator::FractalConfig {
            get_points: &|t, r, s| {
                base.iter().map(|&p| apply_transform(p, t, r, vec2(s, s))).collect()
            },
            get_lines: &|t, r, s| {
                let pts: Vec<Pos2> = base.iter().map(|&p| apply_transform(p, t, r, vec2(s, s))).collect();
                vec![[pts[0], pts[1]], [pts[1], pts[2]], [pts[2], pts[0]]]
            },
            pattern: &pattern,
            initial: &initial,
            iterations: 5,
            regroup: false,
            display_parent: false,
            delta_radius: 0.0,
            density_sources: &[],
        };
        let result = generator::generate_fractal(&config);
        let apex_idx = result.points.iter().enumerate()
            .max_by(|a, b| a.1.y.total_cmp(&b.1.y))
            .unwrap().0;
        let min_steps = 100;
        let (sims, _) = run_all(&result.points, &result.lines, apex_idx, 10, min_steps, 10_000, 0.0);
        for sim in &sims {
            assert!(sim.steps() >= min_steps as usize);
        }
    }

    #[test]
    fn min_steps_greater_than_max_does_not_hang() {
        let points = vec![pos2(0.0, 0.0), pos2(1.0, 0.0), pos2(0.5, 0.866)];
        let lines = vec![[0, 1], [1, 2], [2, 0]];
        let (sims, _) = run_all(&points, &lines, 0, 1, 10_000, 50, 0.0);
        for sim in &sims {
            assert!(sim.steps() <= 50);
        }
    }
}
