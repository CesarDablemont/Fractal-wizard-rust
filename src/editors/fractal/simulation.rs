use eframe::egui::Pos2;
use crate::fractal::random_walk::{self, RandomWalkStats, SimulationRunner};
use crate::heatmap;
use crate::types::{Line, RandomWalkInfo};

/// Temps de calcul accordé aux simulations à chaque frame : assez pour avancer
/// vite, assez court pour que l'interface reste fluide (~60 fps).
const SIMULATION_FRAME_BUDGET: std::time::Duration = std::time::Duration::from_millis(10);

/// Réglages du menu `Simulation`.
pub(super) struct SimulationParams {
    pub(super) count: u32,
    pub(super) min_steps: u64,
    pub(super) max_steps: u64,
    pub(super) allow_min: bool,
    pub(super) allow_max: bool,
    pub(super) beta: f32,
}

impl Default for SimulationParams {
    fn default() -> Self {
        Self {
            count: 10,
            min_steps: 100,
            max_steps: 5000,
            allow_min: true,
            allow_max: true,
            beta: 0.0,
        }
    }
}

/// Simulations lancées sur la fractale courante et leurs résultats
/// (statistiques, heatmaps, simulation sélectionnée).
#[derive(Default)]
pub(super) struct SimulationState {
    pub(super) params: SimulationParams,
    runner: Option<SimulationRunner>,
    simulations: Vec<RandomWalkInfo>,
    stats: Option<RandomWalkStats>,
    /// `beta` avec lequel les simulations affichées ont été lancées.
    beta: Option<f32>,
    global_heatmap: Vec<f32>,
    individual_heatmap: Vec<f32>,
    selected: Option<usize>,
}

impl SimulationState {
    /// Arrête la simulation en cours et efface tous les résultats.
    pub(super) fn reset(&mut self) {
        self.runner = None;
        self.simulations.clear();
        self.stats = None;
        self.beta = None;
        self.global_heatmap.clear();
        self.individual_heatmap.clear();
        self.selected = None;
    }

    /// Lance `params.count` simulations depuis le point `start_idx`.
    pub(super) fn start(&mut self, points: &[Pos2], lines: &[Line], start_idx: usize) {
        self.reset();
        let params = &self.params;
        self.beta = Some(params.beta);
        self.runner = Some(SimulationRunner::new(
            points,
            lines,
            start_idx,
            params.count,
            if params.allow_min { params.min_steps } else { 0 },
            if params.allow_max { params.max_steps } else { u64::MAX },
            params.beta,
        ));
    }

    pub(super) fn is_pending(&self) -> bool {
        self.runner.is_some()
    }

    /// `(faites, total)` de la simulation en cours.
    pub(super) fn progress(&self) -> Option<(u32, u32)> {
        self.runner.as_ref().map(SimulationRunner::progress)
    }

    /// Exécute autant de simulations que possible dans le budget d'une frame,
    /// puis met à jour statistiques et heatmap une seule fois pour tout le lot.
    /// Renvoie un message d'erreur si une simulation a dépassé sa limite de temps.
    pub(super) fn advance(&mut self, points_count: usize) -> Option<String> {
        let runner = self.runner.as_mut()?;
        if runner.is_done() {
            self.runner = None;
            return None;
        }

        let mut error = None;
        let frame_start = std::time::Instant::now();
        while !runner.is_done() && frame_start.elapsed() < SIMULATION_FRAME_BUDGET {
            let sim = runner.run_next();
            if sim.timed_out {
                let n = self.simulations.len() + 1;
                let steps = sim.steps();
                error = Some(format!(
                    "Simulation lente : la simulation {n} a dépassé la limite d'1s ({steps} étapes, marquée Pas fini)"
                ));
            }
            self.simulations.push(sim);
        }

        self.stats = Some(random_walk::calculate_stats(&self.simulations, self.params.count));
        self.global_heatmap = heatmap::normalize(runner.visits());
        if self.selected.is_none() && !self.simulations.is_empty() {
            self.select(0, points_count);
        }

        if let Some(runner) = self.runner.take_if(|runner| runner.is_done()) {
            if let Some(stats) = &mut self.stats {
                stats.max_simulation_time = runner.max_simulation_time();
                stats.total_simulation_time = runner.total_simulation_time();
            }
        }
        error
    }

    pub(super) fn simulations(&self) -> &[RandomWalkInfo] {
        &self.simulations
    }

    pub(super) fn stats(&self) -> Option<&RandomWalkStats> {
        self.stats.as_ref()
    }

    pub(super) fn selected_index(&self) -> Option<usize> {
        self.selected
    }

    pub(super) fn selected(&self) -> Option<&RandomWalkInfo> {
        self.selected.map(|i| &self.simulations[i])
    }

    pub(super) fn select(&mut self, idx: usize, points_count: usize) {
        self.selected = Some(idx);
        self.individual_heatmap = heatmap::calculate_individual_heatmap(points_count, &self.simulations[idx]);
    }

    pub(super) fn global_heatmap(&self) -> &[f32] {
        &self.global_heatmap
    }

    pub(super) fn individual_heatmap(&self) -> &[f32] {
        &self.individual_heatmap
    }

    /// `beta` des simulations affichées, `None` s'il n'y en a pas.
    pub(super) fn displayed_beta(&self) -> Option<f32> {
        self.beta
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use eframe::egui::pos2;

    fn run_triangle() -> SimulationState {
        let points = [pos2(0.0, 0.0), pos2(1.0, 0.0), pos2(0.5, 0.866)];
        let lines = [[0, 1], [1, 2], [2, 0]];
        let mut sim = SimulationState::default();
        sim.params.count = 3;
        sim.params.max_steps = 20;
        sim.start(&points, &lines, 0);
        while sim.is_pending() {
            sim.advance(points.len());
        }
        sim
    }

    #[test]
    fn advance_runs_all_simulations() {
        let sim = run_triangle();
        assert_eq!(sim.simulations().len(), 3);
        assert_eq!(sim.selected_index(), Some(0));
        assert!(sim.stats().is_some());
        assert_eq!(sim.global_heatmap().len(), 3);
        assert_eq!(sim.displayed_beta(), Some(0.0));
    }

    #[test]
    fn reset_clears_results_and_selection() {
        let mut sim = run_triangle();
        sim.reset();
        assert!(sim.simulations().is_empty());
        assert!(sim.selected().is_none());
        assert!(sim.stats().is_none());
        assert!(sim.global_heatmap().is_empty());
        assert!(sim.individual_heatmap().is_empty());
        assert!(sim.displayed_beta().is_none());
        assert!(!sim.is_pending());
    }
}
