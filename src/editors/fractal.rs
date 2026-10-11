mod animation;
mod canvas;
mod export;
mod inputs;
mod menu;
mod panels;
mod simulation;

use eframe::egui::{self, Id, Vec2};
use serde::{Deserialize, Serialize};
use crate::fractal::generator::FractalResult;
use crate::scene::camera::{Camera, CameraSettings};
use crate::scene::canvas::CanvasRenderer;
use crate::scene::chunk_grid::ChunkGrid;
use crate::shapes::any_shape::AnyShape;
use crate::types::{EditorState, RenderMode, ShapePatternData};
use super::shared;
use animation::AnimationState;
use export::{ExportDialog, ExportSettings};
use inputs::FractalInputs;
use simulation::SimulationState;

/// Réglages des menus `Simulation`, `Grille`, `Options` et de la fenêtre
/// d'export, mémorisés entre deux lancements.
#[derive(Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct FractalSettings {
    pub simulation_count: u32,
    pub min_steps: u64,
    pub max_steps: u64,
    pub allow_min: bool,
    pub allow_max: bool,
    pub beta: f32,
    pub iterations: usize,
    pub regroup: bool,
    pub display_parent: bool,
    pub add_delta: bool,
    pub delta: [f32; 2],
    pub camera: CameraSettings,
    export: ExportSettings,
}

impl Default for FractalSettings {
    fn default() -> Self {
        FractalEditor::default().settings()
    }
}

pub struct FractalEditor {
    file_path: Option<String>,

    inputs: FractalInputs,
    fractal: Option<FractalResult>,
    simulation: SimulationState,
    animation: AnimationState,

    render_mode: RenderMode,
    show_heat_score: bool,
    camera: Camera,
    canvas_renderer: CanvasRenderer,
    state: EditorState,

    selected_density_source: Option<usize>,
    dragging_density_source: Option<usize>,

    export: ExportDialog,

    message: Option<shared::StatusMessage>,

    left_panel_version: u32,
    right_panel_version: u32,
}

impl Default for FractalEditor {
    fn default() -> Self {
        Self {
            file_path: None,
            inputs: FractalInputs::default(),
            fractal: None,
            simulation: SimulationState::default(),
            animation: AnimationState::default(),
            render_mode: RenderMode::Normal,
            show_heat_score: false,
            camera: Camera::default(),
            canvas_renderer: CanvasRenderer::new(),
            state: EditorState::Mouse,
            selected_density_source: None,
            dragging_density_source: None,
            export: ExportDialog::default(),
            message: None,
            left_panel_version: 0,
            right_panel_version: 0,
        }
    }
}

impl FractalEditor {
    pub fn settings(&self) -> FractalSettings {
        let params = &self.simulation.params;
        let inputs = &self.inputs;
        FractalSettings {
            simulation_count: params.count,
            min_steps: params.min_steps,
            max_steps: params.max_steps,
            allow_min: params.allow_min,
            allow_max: params.allow_max,
            beta: params.beta,
            iterations: inputs.iterations,
            regroup: inputs.regroup,
            display_parent: inputs.display_parent,
            add_delta: inputs.add_delta,
            delta: [inputs.delta.x, inputs.delta.y],
            camera: self.camera.settings(),
            export: self.export.settings.clone(),
        }
    }

    pub fn apply_settings(&mut self, settings: &FractalSettings) {
        let params = &mut self.simulation.params;
        params.count = settings.simulation_count;
        params.min_steps = settings.min_steps;
        params.max_steps = settings.max_steps;
        params.allow_min = settings.allow_min;
        params.allow_max = settings.allow_max;
        params.beta = settings.beta;
        let inputs = &mut self.inputs;
        inputs.iterations = settings.iterations;
        inputs.regroup = settings.regroup;
        inputs.display_parent = settings.display_parent;
        inputs.add_delta = settings.add_delta;
        inputs.delta = Vec2::from(settings.delta);
        self.camera.apply_settings(&settings.camera);
        self.export.settings = settings.export.clone();
    }

    pub fn import_shape(&mut self, shape: AnyShape) {
        self.inputs.import_shape(shape);
        self.canvas_renderer.rebuild_chunks = true;
    }

    pub fn import_pattern_data(&mut self, data: Vec<ShapePatternData>) {
        self.inputs.pattern_data = data;
    }

    pub fn import_initial_data(&mut self, data: Vec<ShapePatternData>) {
        self.inputs.initial_data = data;
    }

    /// Vrai tant qu'une animation est jouée ou qu'une simulation tourne.
    pub fn needs_repaint(&self) -> bool {
        self.animation.is_playing() || self.simulation.is_pending()
    }

    fn info(&mut self, text: impl Into<String>) {
        shared::set_status_message(&mut self.message, shared::StatusMessage::info(text));
    }

    fn error(&mut self, text: impl Into<String>) {
        shared::set_status_message(&mut self.message, shared::StatusMessage::error(text));
    }

    fn fractal_points_count(&self) -> usize {
        self.fractal.as_ref().map_or(0, |f| f.points.len())
    }

    /// Nom du fichier ouvert sans extension, `fractale` à défaut.
    fn file_stem(&self) -> &str {
        self.file_path
            .as_deref()
            .and_then(|p| std::path::Path::new(p).file_stem().and_then(|s| s.to_str()))
            .unwrap_or("fractale")
    }

    /// Remplace la fractale : les simulations faites sur l'ancienne n'ont plus de sens.
    fn set_fractal(&mut self, fractal: Option<FractalResult>) {
        self.fractal = fractal;
        self.canvas_renderer.rebuild_chunks = true;
        self.canvas_renderer.chunk_grid = None;
        self.canvas_renderer.mesh_dirty = true;
        self.simulation.reset();
        self.animation.reset();
    }

    fn generate(&mut self) {
        let start = std::time::Instant::now();
        let Some(result) = self.inputs.generate() else { return };
        let elapsed = start.elapsed();
        self.info(format!("Générée en {elapsed:.2?}"));
        self.set_fractal(Some(result));
    }

    fn start_simulation(&mut self, start_idx: usize) {
        let Some(fractal) = &self.fractal else { return };
        if fractal.points.is_empty() || fractal.lines.is_empty() {
            return;
        }
        self.simulation.start(&fractal.points, &fractal.lines, start_idx);
        self.animation.reset();
    }

    fn advance_simulation(&mut self) {
        if let Some(error) = self.simulation.advance(self.fractal_points_count()) {
            self.error(error);
        }
    }

    fn select_simulation(&mut self, idx: usize) {
        self.simulation.select(idx, self.fractal_points_count());
        self.animation.reset();
    }

    fn update_chunks(&mut self) {
        if self.canvas_renderer.rebuild_chunks {
            let points = self.fractal.as_ref().map(|f| f.points.as_slice()).unwrap_or(&[]);
            let lines = self.fractal.as_ref().map(|f| f.lines.as_slice()).unwrap_or(&[]);
            if !points.is_empty() {
                let min_x = points.iter().map(|p| p.x).fold(f32::MAX, f32::min);
                let max_x = points.iter().map(|p| p.x).fold(f32::MIN, f32::max);
                let min_y = points.iter().map(|p| p.y).fold(f32::MAX, f32::min);
                let max_y = points.iter().map(|p| p.y).fold(f32::MIN, f32::max);
                let size = (max_x - min_x).max(max_y - min_y);
                let cell_size = (size / 50.0).max(size / (points.len() as f32).sqrt()).max(0.01);
                self.canvas_renderer.chunk_grid = Some(ChunkGrid::new(points, lines, cell_size));
            }
            self.canvas_renderer.rebuild_chunks = false;
        }
    }

    pub fn render(&mut self, ctx: &egui::Context) {
        self.update_chunks();
        self.advance_simulation();

        egui::TopBottomPanel::top("fractal_editor_menu").show(ctx, |ui| {
            self.render_menu(ui);
        });

        let outliner_id = Id::new("fractal_outliner").with(self.left_panel_version);
        egui::SidePanel::left(outliner_id)
            .resizable(true)
            .default_width(220.0)
            .show(ctx, |ui| {
                self.render_outliner(ui);
            });

        let left_resize_id = outliner_id.with("__resize");
        if let Some(resp) = ctx.read_response(left_resize_id) {
            if resp.hovered() && ctx.input(|i| i.pointer.button_double_clicked(egui::PointerButton::Primary)) {
                self.left_panel_version = self.left_panel_version.wrapping_add(1);
            }
        }

        egui::CentralPanel::default().show(ctx, |ui| {
            self.render_canvas(ui);
        });

        let properties_id = Id::new("fractal_properties").with(self.right_panel_version);
        egui::SidePanel::right(properties_id)
            .resizable(true)
            .default_width(220.0)
            .show(ctx, |ui| {
                self.render_properties(ui);
            });

        let right_resize_id = properties_id.with("__resize");
        if let Some(resp) = ctx.read_response(right_resize_id) {
            if resp.hovered() && ctx.input(|i| i.pointer.button_double_clicked(egui::PointerButton::Primary)) {
                self.right_panel_version = self.right_panel_version.wrapping_add(1);
            }
        }

        egui::TopBottomPanel::bottom("fractal_player")
            .min_height(40.0)
            .show(ctx, |ui| {
                let max_step = self.simulation.selected().map(|sim| sim.steps());
                self.animation.show(ui, max_step);
            });

        self.render_export_window(ctx);
    }

    /// `beta` par défaut des poids exportés et sa provenance : celui des
    /// simulations affichées s'il y en a, sinon le réglage courant.
    fn default_export_beta(&self) -> (f32, &'static str) {
        match self.simulation.displayed_beta() {
            Some(beta) => (beta, "simulations affichées"),
            None => (self.simulation.params.beta, "réglage Simulation"),
        }
    }

    fn open_export_window(&mut self) {
        let (beta, _) = self.default_export_beta();
        self.export.open(beta);
    }

    fn render_export_window(&mut self, ctx: &egui::Context) {
        let (default_beta, source) = self.default_export_beta();
        if self.export.show(ctx, default_beta, source) {
            if let Some(msg) = self.export.export_csv(self.fractal.as_ref(), self.file_stem()) {
                shared::set_status_message(&mut self.message, msg);
            }
        }
    }
}
