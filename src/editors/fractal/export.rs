use eframe::egui::{self, Align2, Vec2};
use serde::{Deserialize, Serialize};
use crate::fractal::generator::{self, FractalResult};
use crate::fractal::hopping;
use crate::file_io;
use crate::editors::shared::StatusMessage;

/// Réglages de la fenêtre `Exporter (csv)`, mémorisés entre deux lancements.
#[derive(Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct ExportSettings {
    pub points: bool,
    pub lines: bool,
    pub weights: bool,
}

impl Default for ExportSettings {
    fn default() -> Self {
        Self {
            points: true,
            lines: true,
            weights: false,
        }
    }
}

/// Fenêtre `Exporter (csv)`.
#[derive(Default)]
pub(super) struct ExportDialog {
    pub(super) settings: ExportSettings,
    open: bool,
    /// `beta` des poids exportés, pré-rempli à l'ouverture de la fenêtre.
    beta: f32,
}

impl ExportDialog {
    pub(super) fn open(&mut self, default_beta: f32) {
        self.beta = default_beta;
        self.open = true;
    }

    /// Affiche la fenêtre si elle est ouverte ; renvoie `true` quand l'utilisateur
    /// clique sur `Exporter`. `beta_source` décrit d'où vient `default_beta`.
    pub(super) fn show(&mut self, ctx: &egui::Context, default_beta: f32, beta_source: &str) -> bool {
        if !self.open {
            return false;
        }
        let mut export = false;
        egui::Window::new("Exporter (csv)")
            .collapsible(false)
            .resizable(false)
            .anchor(Align2::CENTER_CENTER, Vec2::ZERO)
            .show(ctx, |ui| {
                ui.label("Éléments à exporter :");
                ui.checkbox(&mut self.settings.points, "Points (x, y)");
                ui.checkbox(&mut self.settings.lines, "Liaisons (i, j)");
                ui.add_enabled_ui(self.settings.lines, |ui| {
                    ui.indent("export_weights", |ui| {
                        ui.checkbox(&mut self.settings.weights, "avec poids t (i, j, t)").on_hover_text(
                            "Hopping relatif t = exp(-beta * (d / d0 - 1)) par liaison, d0 = longueur médiane des liaisons.",
                        );
                    });
                });
                if self.settings.lines && self.settings.weights {
                    self.render_beta(ui, default_beta, beta_source);
                }

                ui.separator();
                ui.horizontal(|ui| {
                    if ui.button("Exporter").clicked() {
                        self.open = false;
                        export = true;
                    }
                    if ui.button("Annuler").clicked() {
                        self.open = false;
                    }
                });
            });
        export
    }

    fn render_beta(&mut self, ui: &mut egui::Ui, default_beta: f32, beta_source: &str) {
        ui.indent("export_beta", |ui| {
            ui.horizontal(|ui| {
                ui.label("Beta (distance):");
                ui.add(egui::DragValue::new(&mut self.beta).speed(0.1).range(0.0..=10.0));
                if ui
                    .add_enabled(self.beta != default_beta, egui::Button::new("↺"))
                    .on_hover_text(format!("Revenir à {default_beta:.2} ({beta_source})"))
                    .clicked()
                {
                    self.beta = default_beta;
                }
            });
            if self.beta == 0.0 {
                ui.colored_label(
                    ui.visuals().warn_fg_color,
                    "beta = 0 : tous les t valent 1 (liaisons uniformes)",
                );
            }
        });
    }

    /// Demande où enregistrer puis écrit les CSV de `fractal`, nommés d'après `stem`.
    /// Renvoie le message à afficher, `None` si l'utilisateur a annulé.
    pub(super) fn export_csv(&self, fractal: Option<&FractalResult>, stem: &str) -> Option<StatusMessage> {
        let Some(fractal) = fractal else {
            return Some(StatusMessage::info("Aucune fractale à exporter"));
        };
        let settings = &self.settings;
        if !settings.points && !settings.lines {
            return Some(StatusMessage::error("Sélectionnez au moins un élément à exporter"));
        }

        let base = file_io::pick_csv_base("Exporter (csv)", stem)?;

        let beta = self.beta;
        let use_weights = settings.lines && settings.weights;

        // Même graphe que celui parcouru par le marcheur ivre.
        let graph = generator::merge_graph(&fractal.points, &fractal.lines);
        let mut files: Vec<(std::path::PathBuf, String, String)> = Vec::new();
        if settings.points {
            files.push((
                file_io::csv_path(&base, "_points"),
                file_io::points_to_csv(&graph.points),
                format!("{} points", graph.points.len()),
            ));
        }
        if settings.lines {
            let weights = use_weights.then(|| hopping::edge_weights(&graph.points, &graph.lines, beta));
            let label = if use_weights {
                format!("{} liaisons (t, beta = {beta:.2})", graph.lines.len())
            } else {
                format!("{} liaisons", graph.lines.len())
            };
            let csv = file_io::edges_to_csv(&graph.lines, weights.as_deref());
            files.push((file_io::csv_path(&base, "_edges"), csv, label));
        }

        let mut exported: Vec<String> = Vec::new();
        for (path, csv, label) in files {
            if let Err(err) = std::fs::write(&path, csv) {
                return Some(StatusMessage::error(format!("Échec de l'écriture de {} : {err}", path.display())));
            }
            exported.push(label);
        }
        Some(StatusMessage::info(format!("Exporté : {}", exported.join(", "))))
    }
}
