use eframe::egui;
use crate::shapes::shape::Shape as ShapeTrait;
use crate::types::RenderMode;
use super::FractalEditor;

impl FractalEditor {
    pub(super) fn render_outliner(&mut self, ui: &mut egui::Ui) {
        ui.heading("Simulations");

        if let Some((done, total)) = self.simulation.progress() {
            ui.label(format!("Simulation en cours : {done}/{total}"));
        }

        if self.simulation.simulations().is_empty() {
            if !self.simulation.is_pending() {
                ui.label("Aucune simulation");
            }
            return;
        }

        if self.simulation.selected_index().is_some() && !self.simulation.global_heatmap().is_empty() {
            ui.horizontal(|ui| {
                ui.label("Rendu:");
                egui::ComboBox::from_id_salt("render_mode")
                    .selected_text(match self.render_mode {
                        RenderMode::Normal => "Normal",
                        RenderMode::GlobalHeatMap => "Heatmap globale",
                        RenderMode::IndividualHeatMap => "Heatmap individuelle",
                    })
                    .show_ui(ui, |ui| {
                        ui.selectable_value(&mut self.render_mode, RenderMode::Normal, "Normal");
                        ui.selectable_value(&mut self.render_mode, RenderMode::GlobalHeatMap, "Heatmap globale");
                        ui.selectable_value(&mut self.render_mode, RenderMode::IndividualHeatMap, "Heatmap individuelle");
                    });
            });
            if self.render_mode != RenderMode::Normal {
                ui.checkbox(&mut self.show_heat_score, "Afficher score");
            }
            ui.separator();
        }

        let mut clicked: Option<usize> = None;
        egui::ScrollArea::vertical().show(ui, |ui| {
            for (i, sim) in self.simulation.simulations().iter().enumerate() {
                let label = format!(
                    "Sim {} : {} étapes, {}",
                    i + 1,
                    sim.steps(),
                    if sim.is_random_walk_done { "Fini" } else { "Pas fini" }
                );
                let selected = self.simulation.selected_index() == Some(i);
                if ui.selectable_label(selected, &label).clicked() {
                    clicked = Some(i);
                }
            }
        });

        if let Some(idx) = clicked {
            self.select_simulation(idx);
        }
    }

    pub(super) fn render_properties(&mut self, ui: &mut egui::Ui) {
        ui.heading("Propriétés");

        if let Some(ref fractal) = self.fractal {
            ui.label(format!("Dimension théorique: {:.4}", fractal.dimension));
            ui.label(format!("Lignes: {}", fractal.lines.len()));
            ui.label(format!("Points: {}", fractal.points.len()));

            if let Some(ref bc) = fractal.box_counting {
                ui.separator();
                ui.label(format!("Dimension boîtes: {:.4}", bc.dimension));
                ui.label(format!("Dimension d'information: {:.4}", bc.information_dimension));
                ui.label(format!("Dimension de corrélation: {:.4}", bc.correlation_dimension));
                ui.label(format!("Moyenne: {:.3e}", bc.proportion_mean).replace("e+", " e+").replace("e-", " e-"));
                ui.label(format!("Variance: {:.3e}", bc.proportion_variance).replace("e+", " e+").replace("e-", " e-"));

                // Diagnostic mono/multifractale basé sur la variance des proportions.
                // Pour un monofractale, p_i ≈ 1/N partout → η ≈ 0.
                // Pour un multifractale, p_i varie → η > 0.
                let eta = if bc.proportion_mean > 0.0 {
                    bc.proportion_variance / (bc.proportion_mean * bc.proportion_mean)
                } else {
                    0.0
                };
                ui.label(format!("  η (var/μ²): {:.4}", eta));
                if eta < 1.0 {
                    ui.label("-> Monofractal (mesure uniforme)");
                } else {
                    ui.label("-> Multifractal (mesure hétérogène)");
                }

                ui.separator();
                ui.label("Spectre D_q:");
                let q_labels = ["D₀", "D₁", "D₂", "D₃", "D₄"];
                let dq_range = bc.d_q_spectrum.iter().cloned().fold(f32::MIN, f32::max)
                    - bc.d_q_spectrum.iter().cloned().fold(f32::MAX, f32::min);
                for (i, label) in q_labels.iter().enumerate() {
                    ui.label(format!("  {} = {:.4}", label, bc.d_q_spectrum[i]));
                }
                ui.label(format!("  Étalement D_q: {:.4}", dq_range));
            }

            ui.separator();
            ui.label("Données d'entrée:");
            if let Some(ref s) = self.inputs.shape {
                ui.label(format!("Shape: {} ({} pts)", s.name(), s.points().len()));
            } else {
                ui.label("Shape: (aucune)");
            }
            ui.label(format!("Patterns: {}", self.inputs.pattern_data.len()));
            ui.label(format!("Figures initiales: {}", self.inputs.initial_data.len()));
        }

        if let Some(stats) = self.simulation.stats() {
            ui.separator();
            ui.heading("Statistiques");
            ui.label(format!("Nombre de Polya: {:.2}%", stats.polya_number));
            ui.label(format!("Réussites: {}/{}", stats.success_count, self.simulation.params.count));
            ui.label(format!("Nombre d'étapes moy: {:.2}", stats.average_steps));
            ui.label(format!("Variance: {:.2}", stats.variance_steps));
            ui.label(format!("Écart-type: {:.2}", stats.std_dev_steps));
            ui.label(format!("Distance moyenne: {:.2}", stats.average_length));
            if stats.max_simulation_time > 0.0 {
                ui.label(format!("Temps max simulation: {:.2}s", stats.max_simulation_time));
            }
            if stats.total_simulation_time > 0.0 {
                ui.label(format!("Temps total simulations: {:.2}s", stats.total_simulation_time));
            }
        }
    }
}
