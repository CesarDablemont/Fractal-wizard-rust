use eframe::egui::{self, Align2, Color32, FontId, pos2, Pos2, Shape, Vec2};
use serde::{Deserialize, Serialize};
use crate::fractal::generator::{self, FractalResult};
use crate::fractal::random_walk::{self, RandomWalkStats};
use crate::heatmap::{self, heatmap_color};
use crate::scene::camera::Camera;
use crate::scene::canvas::CanvasRenderer;
use crate::scene::chunk_grid::ChunkGrid;
use crate::shapes::polygon::Polygon;
use crate::shapes::free_linear::FreeLinearShape;
use crate::shapes::shape::Shape as ShapeTrait;
use crate::types::{EditorState, Line, RandomWalkInfo, RenderMode, ShapePatternData, LoopMode};
use crate::file_io;
use super::shared;

#[derive(Serialize, Deserialize)]
struct FractalFile {
    shape: Option<shared::ModelData>,
    pattern: PatternEditorData,
    initial: Vec<ShapePatternData>,
    iterations: usize,
    regroup: bool,
    add_delta: bool,
    delta: [f32; 2],
    delta_intervals: u32,
    display_parent: bool,
}

#[derive(Serialize, Deserialize)]
struct PatternEditorData {
    display_parent: bool,
    patterns: Vec<ShapePatternData>,
}

#[derive(Serialize, Deserialize)]
struct InitialFileWrap {
    shapes: Vec<ShapePatternData>,
}

pub struct FractalEditor {
    pub file_path: Option<String>,

    pub shape: Option<ShapeWrapper>,
    pub pattern_data: Vec<ShapePatternData>,
    pub initial_data: Vec<ShapePatternData>,
    pub display_parent: bool,

    pub fractal: Option<FractalResult>,

    pub simulations: Vec<RandomWalkInfo>,
    pub stats: Option<RandomWalkStats>,
    pub selected_simulation: Option<usize>,
    pub simulation_count: u32,
    pub min_steps: u64,
    pub max_steps: u64,
    pub allow_min: bool,
    pub allow_max: bool,

    pub global_heatmap: Vec<f32>,
    pub individual_heatmap: Vec<f32>,
    pub render_mode: RenderMode,
    pub show_heat_score: bool,

    pub current_step: usize,
    pub is_playing: bool,
    pub fps: f32,
    pub loop_mode: LoopMode,
    pub animation_time: std::time::Instant,
    pub ascending: bool,

    camera: Camera,
    canvas_renderer: CanvasRenderer,
    state: EditorState,
    selected_points: Vec<usize>,

    pub iterations: usize,
    pub regroup: bool,
    pub add_delta: bool,
    pub delta: Vec2,
    pub delta_intervals: u32,

    message: Option<String>,
}

pub enum ShapeWrapper {
    Polygon(Polygon),
    FreeLinear(FreeLinearShape),
}

impl ShapeWrapper {
    pub fn get_transformed_points(&self, t: Pos2, r: f32, s: f32) -> Vec<Pos2> {
        match self {
            ShapeWrapper::Polygon(p) => p.get_transformed_points(t, r, s),
            ShapeWrapper::FreeLinear(p) => p.get_transformed_points(t, r, s),
        }
    }

    pub fn get_lines(&self, t: Pos2, r: f32, s: f32) -> Vec<[Pos2; 2]> {
        match self {
            ShapeWrapper::Polygon(p) => p.get_lines(t, r, s),
            ShapeWrapper::FreeLinear(p) => p.get_lines(t, r, s),
        }
    }

    pub fn name(&self) -> &str {
        match self {
            ShapeWrapper::Polygon(_) => "Polygone",
            ShapeWrapper::FreeLinear(_) => "Forme libre",
        }
    }

    pub fn point_count(&self) -> usize {
        match self {
            ShapeWrapper::Polygon(p) => p.points().len(),
            ShapeWrapper::FreeLinear(p) => p.points().len(),
        }
    }
}

impl Default for FractalEditor {
    fn default() -> Self {
        Self {
            file_path: None,
            shape: None,
            pattern_data: Vec::new(),
            initial_data: Vec::new(),
            display_parent: false,
            fractal: None,
            simulations: Vec::new(),
            stats: None,
            selected_simulation: None,
            simulation_count: 10,
            min_steps: 100,
            max_steps: 5000,
            allow_min: true,
            allow_max: true,
            global_heatmap: Vec::new(),
            individual_heatmap: Vec::new(),
            render_mode: RenderMode::Normal,
            show_heat_score: false,
            current_step: 0,
            is_playing: false,
            fps: 60.0,
            loop_mode: LoopMode::Repeat,
            animation_time: std::time::Instant::now(),
            ascending: true,
            camera: Camera::default(),
            canvas_renderer: CanvasRenderer::new(),
            state: EditorState::Mouse,
            selected_points: Vec::new(),
            iterations: 3,
            regroup: false,
            add_delta: false,
            delta: Vec2::ZERO,
            delta_intervals: 1000,
            message: None,
        }
    }
}

impl FractalEditor {
    fn fractal_points(&self) -> &[Pos2] {
        self.fractal.as_ref().map(|f| f.points.as_slice()).unwrap_or(&[])
    }

    fn fractal_lines(&self) -> &[Line] {
        self.fractal.as_ref().map(|f| f.lines.as_slice()).unwrap_or(&[])
    }

    pub fn generate(&mut self) {
        let Some(ref shape) = self.shape else { return };
        if self.pattern_data.is_empty() || self.initial_data.is_empty() {
            return;
        }

        let pre_scale = if !self.pattern_data.is_empty() {
            self.pattern_data[0].scale.powf(self.iterations as f32)
        } else {
            1.0
        };
        let mut initial_scaled = self.initial_data.clone();
        for s in &mut initial_scaled {
            s.translate.x *= pre_scale;
            s.translate.y *= pre_scale;
            s.scale *= pre_scale;
        }

        let get_points = |t: Pos2, r: f32, s: f32| -> Vec<Pos2> {
            shape.get_transformed_points(t, r, s)
        };
        let get_lines = |t: Pos2, r: f32, s: f32| -> Vec<[Pos2; 2]> {
            shape.get_lines(t, r, s)
        };

        let start = std::time::Instant::now();
        let result = generator::generate_fractal(
            &get_points,
            &get_lines,
            &self.pattern_data,
            &initial_scaled,
            self.iterations,
            self.regroup,
            self.display_parent,
            if self.add_delta { self.delta.x.abs() } else { 0.0 },
        );
        let elapsed = start.elapsed();

        self.message = Some(format!("Générée en {:.2?}", elapsed));

        self.fractal = Some(result);
        self.canvas_renderer.rebuild_chunks = true;
        self.canvas_renderer.chunk_grid = None;
        self.canvas_renderer.mesh_dirty = true;
        self.simulations.clear();
        self.selected_simulation = None;
        self.current_step = 0;
        self.is_playing = false;
        self.stats = None;
        self.global_heatmap.clear();
        self.individual_heatmap.clear();
    }

    pub fn import_shape(&mut self, wrapper: ShapeWrapper) {
        self.initial_data = vec![ShapePatternData {
            translate: pos2(0.0, 0.0),
            rotate: 0.0,
            scale: 1.0,
        }];
        self.shape = Some(wrapper);
        self.canvas_renderer.rebuild_chunks = true;
    }

    pub fn import_pattern_data(&mut self, data: Vec<ShapePatternData>) {
        self.pattern_data = data;
    }

    pub fn import_initial_data(&mut self, data: Vec<ShapePatternData>) {
        self.initial_data = data;
    }

    fn import_firfw(&mut self, content: &str) -> Result<(), String> {
        let data: shared::ModelData = serde_json::from_str(content).map_err(|e| e.to_string())?;
        self.load_shape_from_data(&data);
        self.canvas_renderer.rebuild_chunks = true;
        Ok(())
    }

    fn import_ptnfw(&mut self, content: &str) -> Result<(), String> {
        let data: PatternEditorData = serde_json::from_str(content).map_err(|e| e.to_string())?;
        self.pattern_data = data.patterns;
        self.display_parent = data.display_parent;
        Ok(())
    }

    fn import_filfw(&mut self, content: &str) -> Result<(), String> {
        // Try full InitialFile first, then fall back to bare array
        if let Ok(data) = serde_json::from_str::<InitialFileWrap>(content) {
            self.initial_data = data.shapes;
        } else {
            let data: Vec<ShapePatternData> = serde_json::from_str(content).map_err(|e| e.to_string())?;
            self.initial_data = data;
        }
        Ok(())
    }

    fn shape_to_data(&self) -> Option<shared::ModelData> {
        self.shape.as_ref().map(|s| match s {
            ShapeWrapper::Polygon(p) => shared::ModelData {
                r#type: "Polygon".into(),
                points: p.points().iter().map(|pt| [pt.x, pt.y]).collect(),
                lines: Vec::new(),
            },
            ShapeWrapper::FreeLinear(s) => shared::ModelData {
                r#type: "FreeLinear".into(),
                points: s.points().iter().map(|pt| [pt.x, pt.y]).collect(),
                lines: s.lines().to_vec(),
            },
        })
    }

    fn load_shape_from_data(&mut self, data: &shared::ModelData) {
        self.shape = Some(match data.r#type.as_str() {
            "Polygon" | "cPolygon" => {
                let pts = data.points.iter().map(|pt| Pos2::new(pt[0], pt[1])).collect();
                ShapeWrapper::Polygon(Polygon::from_points(pts))
            }
            _ => {
                let mut s = FreeLinearShape::new();
                for pt in &data.points {
                    s.add_point(Pos2::new(pt[0], pt[1]));
                }
                for l in &data.lines {
                    s.add_line_segment(l[0], l[1]);
                }
                ShapeWrapper::FreeLinear(s)
            }
        });
        self.canvas_renderer.rebuild_chunks = true;
    }

    pub fn run_simulation(&mut self, start_idx: usize) {
        let points = self.fractal_points().to_vec();
        let lines = self.fractal_lines().to_vec();
        if points.is_empty() || lines.is_empty() {
            return;
        }

        let (sims, stats) = random_walk::run_simulations(
            &points,
            &lines,
            start_idx,
            self.simulation_count,
            if self.allow_min { self.min_steps } else { 0 },
            if self.allow_max { self.max_steps } else { u64::MAX },
        );

        self.simulations = sims;
        self.stats = Some(stats);
        self.selected_simulation = Some(0);

        self.global_heatmap = heatmap::calculate_global_heatmap(points.len(), &self.simulations);
        if !self.simulations.is_empty() {
            self.individual_heatmap = heatmap::calculate_individual_heatmap(points.len(), &self.simulations[0]);
        }
    }

    fn update_chunks(&mut self) {
        if self.canvas_renderer.rebuild_chunks {
            let points = self.fractal_points().to_vec();
            if !points.is_empty() {
                let min_x = points.iter().map(|p| p.x).fold(f32::MAX, f32::min);
                let max_x = points.iter().map(|p| p.x).fold(f32::MIN, f32::max);
                let min_y = points.iter().map(|p| p.y).fold(f32::MAX, f32::min);
                let max_y = points.iter().map(|p| p.y).fold(f32::MIN, f32::max);
                let size = (max_x - min_x).max(max_y - min_y);
                let cell_size = (size / 50.0).max(size / (points.len() as f32).sqrt()).max(0.01);
                self.canvas_renderer.chunk_grid = Some(ChunkGrid::new(&points, cell_size));
            }
            self.canvas_renderer.rebuild_chunks = false;
        }
    }

    pub fn render(&mut self, ctx: &egui::Context) {
        self.update_chunks();

        egui::TopBottomPanel::top("fractal_editor_menu").show(ctx, |ui| {
            self.render_menu(ui);
        });

        egui::SidePanel::left("fractal_outliner")
            .resizable(true)
            .default_width(220.0)
            .show(ctx, |ui| {
                self.render_outliner(ui);
            });

        egui::CentralPanel::default().show(ctx, |ui| {
            self.render_canvas(ui);
        });

        egui::SidePanel::right("fractal_properties")
            .resizable(true)
            .default_width(220.0)
            .show(ctx, |ui| {
                self.render_properties(ui);
            });

        egui::TopBottomPanel::bottom("fractal_player")
            .min_height(40.0)
            .show(ctx, |ui| {
                self.render_player(ui);
            });
    }

    fn render_menu(&mut self, ui: &mut egui::Ui) {
        ui.horizontal(|ui| {
            ui.menu_button("Fichier", |ui| {
                if ui.button("Générer la fractale").clicked() {
                    self.generate();
                    ui.close_menu();
                }
                ui.separator();
                if ui.button("Ouvrir (ftlfw)").clicked() {
                    if let Some((path, content)) = file_io::open_json("Ouvrir une fractale", "ftlfw") {
                        match serde_json::from_str::<FractalFile>(&content) {
                            Ok(data) => {
                                self.file_path = Some(path.display().to_string());
                                if let Some(sd) = data.shape {
                                    self.load_shape_from_data(&sd);
                                } else {
                                    self.shape = None;
                                }
                                self.pattern_data = data.pattern.patterns;
                                self.display_parent = data.display_parent;
                                self.initial_data = data.initial;
                                self.iterations = data.iterations;
                                self.regroup = data.regroup;
                                self.add_delta = data.add_delta;
                                self.delta = Vec2::new(data.delta[0], data.delta[1]);
                                self.delta_intervals = data.delta_intervals;
                                self.message = Some("Fractale chargée".into());
                                self.canvas_renderer.rebuild_chunks = true;
                            }
                            Err(e) => self.message = Some(format!("Erreur: {}", e)),
                        }
                    }
                    ui.close_menu();
                }
                if ui.button("Enregistrer (ftlfw)").clicked() {
                    let data = FractalFile {
                        shape: self.shape_to_data(),
                        pattern: PatternEditorData {
                            display_parent: self.display_parent,
                            patterns: self.pattern_data.clone(),
                        },
                        initial: self.initial_data.clone(),
                        iterations: self.iterations,
                        regroup: self.regroup,
                        add_delta: self.add_delta,
                        delta: [self.delta.x, self.delta.y],
                        delta_intervals: self.delta_intervals,
                        display_parent: self.display_parent,
                    };
                    let json = serde_json::to_string_pretty(&data).unwrap();
                    let name = self.file_path.as_deref().and_then(|p| {
                        std::path::Path::new(p).file_stem().and_then(|s| s.to_str())
                    }).unwrap_or("fractale");
                    if file_io::save_json_path("Enregistrer la fractale", "ftlfw", &format!("{}.ftlfw", name), &json) {
                        self.message = Some("Fractale enregistrée".into());
                    }
                    ui.close_menu();
                }
                ui.separator();
                if ui.button("Importer figure (firfw)").clicked() {
                    if let Some((_path, content)) = file_io::open_json("Importer une figure", "firfw") {
                        match self.import_firfw(&content) {
                            Ok(()) => self.message = Some("Figure importée".into()),
                            Err(e) => self.message = Some(format!("Erreur: {}", e)),
                        }
                    }
                    ui.close_menu();
                }
                if ui.button("Importer pattern (ptnfw)").clicked() {
                    if let Some((_path, content)) = file_io::open_json("Importer un pattern", "ptnfw") {
                        match self.import_ptnfw(&content) {
                            Ok(()) => self.message = Some("Pattern importé".into()),
                            Err(e) => self.message = Some(format!("Erreur: {}", e)),
                        }
                    }
                    ui.close_menu();
                }
                if ui.button("Importer initiale (filfw)").clicked() {
                    if let Some((_path, content)) = file_io::open_json("Importer une figure initiale", "filfw") {
                        match self.import_filfw(&content) {
                            Ok(()) => self.message = Some("Figure initiale importée".into()),
                            Err(e) => self.message = Some(format!("Erreur: {}", e)),
                        }
                    }
                    ui.close_menu();
                }
            });

            if ui.button("Générer").clicked() {
                self.generate();
            }

            ui.separator();

            ui.menu_button("Simulation", |ui| {
                ui.checkbox(&mut self.allow_min, "Min steps");
                if self.allow_min {
                    ui.add(egui::DragValue::new(&mut self.min_steps).range(1..=1000000));
                }
                ui.checkbox(&mut self.allow_max, "Max steps");
                if self.allow_max {
                    ui.add(egui::DragValue::new(&mut self.max_steps).range(1..=1000000));
                }
                ui.add(egui::DragValue::new(&mut self.simulation_count).range(1..=10000).prefix("Nb simulations: "));

                if ui.button("Sélectionner point de départ").clicked() {
                    self.state = EditorState::SelectPointSimulation;
                    ui.close_menu();
                }
            });

            ui.menu_button("Grille", |ui| {
                ui.checkbox(&mut self.camera.display_grid, "Afficher");
                if self.camera.display_grid {
                    ui.add(egui::Slider::new(&mut self.camera.grid_spacing, 10.0..=200.0).text("Espacement"));
                }
            });

            ui.menu_button("Options", |ui| {
                ui.checkbox(&mut self.camera.display_points, "Points");
                ui.add(egui::Slider::new(&mut self.camera.point_size, 2.0..=25.0).text("Taille points"));
                ui.checkbox(&mut self.camera.display_origin, "Origine");

                ui.separator();
                ui.label("Génération");
                ui.add(egui::Slider::new(&mut self.iterations, 1..=10).text("Itérations"));
                ui.checkbox(&mut self.regroup, "Regrouper points");
                ui.checkbox(&mut self.display_parent, "Afficher parents");
                ui.checkbox(&mut self.add_delta, "Incertitude");
                if self.add_delta {
                    ui.add(egui::DragValue::new(&mut self.delta.x).speed(0.1).prefix("Rayon:"));
                }

                if self.fractal.is_some() {
                    ui.separator();
                    if ui.button("Reset").clicked() {
                        self.fractal = None;
                        self.simulations.clear();
                        self.stats = None;
                        self.global_heatmap.clear();
                        self.individual_heatmap.clear();
                    }
                }
            });

            if let Some(ref msg) = self.message {
                ui.separator();
                ui.label(msg);
            }
        });
    }

    fn render_outliner(&mut self, ui: &mut egui::Ui) {
        ui.heading("Simulations");

        if self.simulations.is_empty() {
            ui.label("Aucune simulation");
            return;
        }

        let mut clicked: Option<usize> = None;
        for (i, sim) in self.simulations.iter().enumerate() {
            let label = format!(
                "Sim {} : {} étapes, {}",
                i + 1,
                sim.steps(),
                if sim.is_random_walk_done { "Fini" } else { "Pas fini" }
            );
            let selected = self.selected_simulation == Some(i);
            if ui.selectable_label(selected, &label).clicked() {
                clicked = Some(i);
            }
        }

        if let Some(idx) = clicked {
            self.selected_simulation = Some(idx);
            self.individual_heatmap = heatmap::calculate_individual_heatmap(
                self.fractal_points().len(),
                &self.simulations[idx],
            );
            self.current_step = 0;
            self.is_playing = false;
        }

        if self.selected_simulation.is_some() && !self.global_heatmap.is_empty() {
            ui.separator();
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
        }
    }

    fn render_canvas(&mut self, ui: &mut egui::Ui) {
        let (response, painter) = ui.allocate_painter(
            ui.available_size(),
            egui::Sense::click_and_drag(),
        );

        let canvas_rect = response.rect;
        let canvas_center = canvas_rect.center();
        let mut shapes: Vec<Shape> = Vec::new();

        shared::handle_zoom_scroll(&response, ui, &mut self.camera, canvas_center);
        shared::handle_middle_pan(&response, ui, &mut self.camera);

        if response.dragged_by(egui::PointerButton::Primary) && self.state == EditorState::Mouse && self.selected_points.is_empty() {
            self.camera.pan(ui.input(|i| i.pointer.delta()));
        }

        self.canvas_renderer.draw_grid(&self.camera, canvas_rect, &mut shapes);
        self.canvas_renderer.draw_origin(&self.camera, canvas_rect, &mut shapes);
        {
            let fractal = self.fractal.as_ref();
            let points = fractal.map(|f| f.points.as_slice()).unwrap_or(&[]);
            let lines = fractal.map(|f| f.lines.as_slice()).unwrap_or(&[]);
            let point_scale = fractal.map(|f| f.point_scale.as_slice()).unwrap_or(&[]);

            if !lines.is_empty() {
                self.canvas_renderer.draw_fractal_lines(
                    &self.camera, canvas_rect, points, lines, Color32::YELLOW, &mut shapes,
                );
            }

            if !points.is_empty() {
                let highlight = if let Some(sim_idx) = self.selected_simulation {
                    if self.current_step < self.simulations[sim_idx].walk_steps.len() {
                        Some(self.simulations[sim_idx].walk_steps[self.current_step])
                    } else {
                        None
                    }
                } else {
                    None
                };

                match self.render_mode {
                    RenderMode::Normal => {
                        self.canvas_renderer.draw_fractal_points(
                            &self.camera, canvas_rect, points, point_scale, &[], highlight, &mut shapes,
                        );
                    }
                    RenderMode::GlobalHeatMap | RenderMode::IndividualHeatMap => {
                        let heatmap = match self.render_mode {
                            RenderMode::GlobalHeatMap => &self.global_heatmap,
                            _ => &self.individual_heatmap,
                        };
                        let colors: Vec<Option<Color32>> = heatmap.iter().map(|&s| Some(heatmap_color(s))).collect();
                        self.canvas_renderer.draw_fractal_points(
                            &self.camera, canvas_rect, points, point_scale, &colors, highlight, &mut shapes,
                        );

                        if self.show_heat_score {
                            let center = canvas_rect.center();
                            if heatmap.len() == points.len() {
                                for (i, &score) in heatmap.iter().enumerate() {
                                    let screen = self.camera.world_to_screen(points[i], center);
                                    let text = format!("{:.2}", score);
                                    painter.text(
                                        screen + Vec2::new(0.0, self.camera.point_size / 2.0 + 2.0),
                                        Align2::CENTER_TOP,
                                        text,
                                        FontId::proportional(10.0),
                                        Color32::WHITE,
                                    );
                                }
                            }
                        }
                    }
                }
            }

            if response.clicked_by(egui::PointerButton::Primary) && self.state == EditorState::SelectPointSimulation {
                if let Some(mouse) = ui.input(|i| i.pointer.interact_pos()) {
                    let world = self.camera.screen_to_world(mouse, canvas_center);
                    let half = self.camera.point_size / 2.0;
                    for (i, &p) in points.iter().enumerate() {
                        if (p.x - world.x).abs() <= half && (p.y - world.y).abs() <= half {
                            self.run_simulation(i);
                            self.state = EditorState::Mouse;
                            break;
                        }
                    }
                }
            }
        }

        painter.extend(shapes);
    }

    fn render_properties(&mut self, ui: &mut egui::Ui) {
        ui.heading("Propriétés");

        if let Some(ref fractal) = self.fractal {
            ui.label(format!("Dimension théorique: {:.4}", fractal.dimension));
            ui.label(format!("Points: {}", fractal.points.len()));
            ui.label(format!("Lignes: {}", fractal.lines.len()));

            if let Some(ref bc) = fractal.box_counting {
                ui.separator();
                ui.label(format!("Dimension boîtes: {:.4}", bc.dimension));
                ui.label(format!("Dimension d'information: {:.4}", bc.information_dimension));
                ui.label(format!("Dimension de corrélation: {:.4}", bc.correlation_dimension));
                ui.label(format!("Moyenne: {:.6}", bc.proportion_mean));
                ui.label(format!("Variance: {:.2e}", bc.proportion_variance));
            }

            ui.separator();
            ui.label("Données d'entrée:");
            if let Some(ref s) = self.shape {
                ui.label(format!("Shape: {} ({} pts)", s.name(), s.point_count()));
            } else {
                ui.label("Shape: (aucune)");
            }
            ui.label(format!("Patterns: {}", self.pattern_data.len()));
            ui.label(format!("Figures initiales: {}", self.initial_data.len()));
        }

        if let Some(ref stats) = self.stats {
            ui.separator();
            ui.heading("Statistiques");
            ui.label(format!("Nombre de Polya: {:.2}%", stats.polya_number));
            ui.label(format!("Réussites: {}/{}", stats.success_count, self.simulation_count));
            ui.label(format!("Étapes moyennes: {:.2}", stats.average_steps));
            ui.label(format!("Variance: {:.2}", stats.variance_steps));
            ui.label(format!("Écart-type: {:.2}", stats.std_dev_steps));
            ui.label(format!("Distance moyenne: {:.2}", stats.average_length));
        }
    }

    fn render_player(&mut self, ui: &mut egui::Ui) {
        ui.horizontal(|ui| {
            let enabled = self.selected_simulation.is_some();
            if !enabled {
                ui.disable();
            }

            let play_label = if self.is_playing { "⏸ Pause" } else { "▶ Jouer" };
            if ui.button(play_label).clicked() {
                self.is_playing = !self.is_playing;
                self.animation_time = std::time::Instant::now();
            }

            if ui.button("⏹ Reset").clicked() {
                self.current_step = 0;
                self.is_playing = false;
                self.ascending = true;
            }

            ui.separator();

            if let Some(sim_idx) = self.selected_simulation {
                let max_step = self.simulations[sim_idx].steps();
                ui.add(egui::Slider::new(&mut self.current_step, 0..=max_step).text("Étape"));
            }

            ui.separator();

            egui::ComboBox::from_id_salt("loop_mode")
                .selected_text(LoopMode::variants()[self.loop_mode as usize])
                .show_ui(ui, |ui| {
                    for (i, name) in LoopMode::variants().iter().enumerate() {
                        let mode: LoopMode = match i {
                            1 => LoopMode::PlayOnceReset,
                            2 => LoopMode::Repeat,
                            3 => LoopMode::PlayOnceMirror,
                            4 => LoopMode::RepeatMirror,
                            _ => LoopMode::PlayOnce,
                        };
                        ui.selectable_value(&mut self.loop_mode, mode, *name);
                    }
                });

            ui.add(egui::DragValue::new(&mut self.fps).range(1.0..=240.0).prefix("FPS: "));
        });

        if self.is_playing {
            if let Some(sim_idx) = self.selected_simulation {
                let elapsed = self.animation_time.elapsed().as_secs_f32();
                if elapsed >= 1.0 / self.fps {
                    self.animation_time = std::time::Instant::now();
                    let max = self.simulations[sim_idx].steps();

                    match self.loop_mode {
                        LoopMode::PlayOnce => {
                            if self.current_step < max {
                                self.current_step += 1;
                            } else {
                                self.is_playing = false;
                            }
                        }
                        LoopMode::PlayOnceReset => {
                            if self.current_step < max {
                                self.current_step += 1;
                            } else {
                                self.is_playing = false;
                                self.current_step = 0;
                            }
                        }
                        LoopMode::Repeat => {
                            self.current_step += 1;
                            if self.current_step > max {
                                self.current_step = 0;
                            }
                        }
                        LoopMode::PlayOnceMirror => {
                            if self.ascending {
                                if self.current_step < max {
                                    self.current_step += 1;
                                } else {
                                    self.ascending = false;
                                }
                            } else {
                                if self.current_step > 0 {
                                    self.current_step -= 1;
                                } else {
                                    self.ascending = true;
                                    self.is_playing = false;
                                }
                            }
                        }
                        LoopMode::RepeatMirror => {
                            if self.ascending {
                                if self.current_step < max {
                                    self.current_step += 1;
                                } else {
                                    self.ascending = false;
                                }
                            } else {
                                if self.current_step > 0 {
                                    self.current_step -= 1;
                                } else {
                                    self.ascending = true;
                                }
                            }
                        }
                    }

                    self.current_step = self.current_step.clamp(0, max);
                }
            }
        }
    }
}
