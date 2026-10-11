use eframe::egui;
use crate::file_io;
use crate::types::{DensityMode, DensitySource, EditorState};
use super::{shared, FractalEditor};

impl FractalEditor {
    pub(super) fn render_menu(&mut self, ui: &mut egui::Ui) {
        ui.horizontal(|ui| {
            ui.menu_button("Fichier", |ui| {
                self.render_file_menu(ui);
            });

            if ui.button("Générer").clicked() {
                self.generate();
            }

            ui.separator();

            ui.menu_button("Simulation", |ui| {
                let params = &mut self.simulation.params;
                ui.checkbox(&mut params.allow_min, "Min steps");
                if params.allow_min {
                    ui.add(egui::DragValue::new(&mut params.min_steps).range(1..=1_000_000));
                }
                ui.checkbox(&mut params.allow_max, "Max steps");
                if params.allow_max {
                    ui.add(egui::DragValue::new(&mut params.max_steps).range(1..=1_000_000));
                }
                ui.add(egui::DragValue::new(&mut params.count).range(1..=10000).prefix("Nb simulations: "));
                ui.horizontal(|ui| {
                    ui.label("Beta (distance):");
                    ui.add(egui::DragValue::new(&mut params.beta).speed(0.1).range(0.0..=10.0));
                });

                if ui.button("Sélectionner point de départ").clicked() {
                    self.state = EditorState::SelectPointSimulation;
                    ui.close_menu();
                }
            });

            ui.menu_button("Génération", |ui| {
                let inputs = &mut self.inputs;
                ui.add(egui::Slider::new(&mut inputs.iterations, 1..=10).text("Itérations"));
                ui.checkbox(&mut inputs.regroup, "Regrouper points");
                ui.checkbox(&mut inputs.display_parent, "Afficher parents");
                ui.checkbox(&mut inputs.add_delta, "Incertitude");
                if inputs.add_delta {
                    ui.add(egui::DragValue::new(&mut inputs.delta.x).speed(0.1).prefix("Rayon:"));
                }

                ui.separator();
                self.render_density_sources(ui);

                if self.fractal.is_some() {
                    ui.separator();
                    if ui.button("Reset").clicked() {
                        self.set_fractal(None);
                    }
                }
            });

            ui.menu_button("Options", |ui| {
                shared::view_options(ui, &mut self.camera);
                shared::points_option(ui, &mut self.camera);
                ui.checkbox(&mut self.camera.display_lines, "Lignes");
            });

            shared::render_status_message(ui, &mut self.message);
        });
    }

    fn render_file_menu(&mut self, ui: &mut egui::Ui) {
        if ui.button("Ouvrir (ftlfw)").clicked() {
            match file_io::open_json("Ouvrir une fractale", "ftlfw") {
                Ok(Some((path, content))) => match self.inputs.load_ftlfw(&content) {
                    Ok(()) => {
                        self.file_path = Some(path.display().to_string());
                        self.selected_density_source = None;
                        self.canvas_renderer.rebuild_chunks = true;
                        self.info("Fractale chargée");
                    }
                    Err(e) => self.error(e),
                }
                Ok(None) => {}
                Err(e) => self.error(e),
            }
            ui.close_menu();
        }
        if ui.button("Enregistrer (ftlfw)").clicked() {
            let json = self.inputs.to_ftlfw();
            let file_name = format!("{}.ftlfw", self.file_stem());
            match file_io::save_json_path("Enregistrer la fractale", "ftlfw", &file_name, &json) {
                Ok(Some(_)) => self.info("Fractale enregistrée"),
                Ok(None) => {}
                Err(e) => self.error(e),
            }
            ui.close_menu();
        }
        ui.separator();
        if ui.button("Importer figure (firfw)").clicked() {
            match file_io::open_json("Importer une figure", "firfw") {
                Ok(Some((_path, content))) => match self.inputs.import_firfw(&content) {
                    Ok(()) => {
                        self.canvas_renderer.rebuild_chunks = true;
                        self.info("Figure importée");
                    }
                    Err(e) => self.error(e),
                }
                Ok(None) => {}
                Err(e) => self.error(e),
            }
            ui.close_menu();
        }
        if ui.button("Importer pattern (ptnfw)").clicked() {
            match file_io::open_json("Importer un pattern", "ptnfw") {
                Ok(Some((_path, content))) => match self.inputs.import_ptnfw(&content) {
                    Ok(()) => self.info("Pattern importé"),
                    Err(e) => self.error(e),
                }
                Ok(None) => {}
                Err(e) => self.error(e),
            }
            ui.close_menu();
        }
        if ui.button("Importer initiale (filfw)").clicked() {
            match file_io::open_json("Importer une figure initiale", "filfw") {
                Ok(Some((_path, content))) => match self.inputs.import_filfw(&content) {
                    Ok(()) => self.info("Figure initiale importée"),
                    Err(e) => self.error(e),
                }
                Ok(None) => {}
                Err(e) => self.error(e),
            }
            ui.close_menu();
        }
        ui.separator();
        if ui.button("Exporter (csv)").clicked() {
            self.open_export_window();
            ui.close_menu();
        }
    }

    fn render_density_sources(&mut self, ui: &mut egui::Ui) {
        ui.label("Champ de densité");
        let sources = &mut self.inputs.density_sources;
        if ui.button("+ Ajouter source").clicked() {
            sources.push(DensitySource::default());
            self.selected_density_source = Some(sources.len() - 1);
        }
        let mut remove_idx = None;
        for (i, src) in sources.iter_mut().enumerate() {
            ui.horizontal(|ui| {
                let mode_label = match src.mode {
                    DensityMode::Displacement => "",
                    DensityMode::Contraction => " [C]",
                    DensityMode::Iteration => " [I]",
                };
                let label = if src.force >= 0.0 {
                    format!("Attraction{}{}", mode_label, i + 1)
                } else {
                    format!("Répulsion{}{}", mode_label, i + 1)
                };
                if ui.selectable_label(self.selected_density_source == Some(i), &label).clicked() {
                    self.selected_density_source = Some(i);
                }
                if ui.small_button("x").clicked() {
                    remove_idx = Some(i);
                }
            });
            if self.selected_density_source == Some(i) {
                ui.indent("density_src_indent", |ui| {
                    ui.horizontal(|ui| {
                        ui.label("Mode:");
                        egui::ComboBox::from_id_salt(format!("density_mode_{i}"))
                            .selected_text(src.mode.label())
                            .show_ui(ui, |ui| {
                                ui.selectable_value(&mut src.mode, DensityMode::Displacement, DensityMode::Displacement.label());
                                ui.selectable_value(&mut src.mode, DensityMode::Contraction, DensityMode::Contraction.label());
                                ui.selectable_value(&mut src.mode, DensityMode::Iteration, DensityMode::Iteration.label());
                            });
                    });
                    ui.horizontal(|ui| {
                        ui.label("X:");
                        ui.add(egui::DragValue::new(&mut src.position.x).speed(1.0));
                        ui.label("Y:");
                        ui.add(egui::DragValue::new(&mut src.position.y).speed(1.0));
                    });
                    ui.horizontal(|ui| {
                        ui.label("Rayon:");
                        ui.add(egui::DragValue::new(&mut src.radius).speed(1.0).range(1.0..=10000.0));
                    });
                    ui.horizontal(|ui| {
                        ui.label("Force:");
                        ui.add(egui::DragValue::new(&mut src.force).speed(1.0).range(-100.0..=100.0));
                    });
                    ui.horizontal(|ui| {
                        ui.label("Exposant:");
                        ui.add(egui::DragValue::new(&mut src.exponent).speed(0.1).range(0.1..=5.0));
                    });
                });
            }
        }
        if let Some(idx) = remove_idx {
            sources.remove(idx);
            self.selected_density_source = None;
        }
    }
}
