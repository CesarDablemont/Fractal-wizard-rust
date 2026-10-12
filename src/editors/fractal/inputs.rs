use eframe::egui::{pos2, Pos2, Vec2};
use serde::{Deserialize, Serialize};
use crate::fractal::generator::{self, FractalResult};
use crate::shapes::any_shape::AnyShape;
use crate::shapes::shape::Shape as ShapeTrait;
use crate::types::{DensitySource, ShapePatternData};
use crate::editors::shared;

/// Format `.ftlfw`.
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
    #[serde(default)]
    density_sources: Vec<DensitySource>,
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

/// Données d'entrée de la génération : forme, pattern, figures initiales et
/// réglages du menu `Génération`.
pub(super) struct FractalInputs {
    pub(super) shape: Option<AnyShape>,
    pub(super) pattern_data: Vec<ShapePatternData>,
    pub(super) initial_data: Vec<ShapePatternData>,
    pub(super) display_parent: bool,
    pub(super) iterations: usize,
    pub(super) regroup: bool,
    pub(super) add_delta: bool,
    pub(super) delta: Vec2,
    delta_intervals: u32,
    pub(super) density_sources: Vec<DensitySource>,
}

impl Default for FractalInputs {
    fn default() -> Self {
        Self {
            shape: None,
            pattern_data: Vec::new(),
            initial_data: Vec::new(),
            display_parent: false,
            iterations: 3,
            regroup: true,
            add_delta: false,
            delta: Vec2::ZERO,
            delta_intervals: 1000,
            density_sources: Vec::new(),
        }
    }
}

impl FractalInputs {
    /// Génère la fractale, `None` s'il manque la forme, le pattern ou les figures initiales.
    pub(super) fn generate(&self) -> Option<FractalResult> {
        let shape = self.shape.as_ref()?;
        if self.pattern_data.is_empty() || self.initial_data.is_empty() {
            return None;
        }

        let pre_scale = self.pattern_data[0].scale.powf(self.iterations as f32);
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

        Some(generator::generate_fractal(&generator::FractalConfig {
            get_points: &get_points,
            get_lines: &get_lines,
            pattern: &self.pattern_data,
            initial: &initial_scaled,
            iterations: self.iterations,
            regroup: self.regroup,
            display_parent: self.display_parent,
            delta_radius: if self.add_delta { self.delta.x.abs() } else { 0.0 },
            density_sources: &self.density_sources,
        }))
    }

    /// Remplace la forme et repart d'une seule figure initiale à l'origine.
    pub(super) fn import_shape(&mut self, shape: AnyShape) {
        self.initial_data = vec![ShapePatternData {
            translate: pos2(0.0, 0.0),
            rotate: 0.0,
            scale: 1.0,
        }];
        self.shape = Some(shape);
    }

    pub(super) fn import_firfw(&mut self, content: &str) -> Result<(), String> {
        let data: shared::ModelData = serde_json::from_str(content).map_err(|e| e.to_string())?;
        self.shape = Some(data.to_shape());
        Ok(())
    }

    pub(super) fn import_ptnfw(&mut self, content: &str) -> Result<(), String> {
        let data: PatternEditorData = serde_json::from_str(content).map_err(|e| e.to_string())?;
        self.pattern_data = data.patterns;
        self.display_parent = data.display_parent;
        Ok(())
    }

    pub(super) fn import_filfw(&mut self, content: &str) -> Result<(), String> {
        // Try full InitialFile first, then fall back to bare array
        if let Ok(data) = serde_json::from_str::<InitialFileWrap>(content) {
            self.initial_data = data.shapes;
        } else {
            let data: Vec<ShapePatternData> = serde_json::from_str(content).map_err(|e| e.to_string())?;
            self.initial_data = data;
        }
        Ok(())
    }

    /// Charge un fichier `.ftlfw`.
    pub(super) fn load_ftlfw(&mut self, content: &str) -> Result<(), String> {
        let data: FractalFile = serde_json::from_str(content).map_err(|e| e.to_string())?;
        self.shape = data.shape.map(|sd| sd.to_shape());
        self.pattern_data = data.pattern.patterns;
        self.display_parent = data.pattern.display_parent;
        self.initial_data = data.initial;
        self.iterations = data.iterations;
        self.regroup = data.regroup;
        self.add_delta = data.add_delta;
        self.delta = Vec2::new(data.delta[0], data.delta[1]);
        self.delta_intervals = data.delta_intervals;
        self.density_sources = data.density_sources;
        Ok(())
    }

    /// Contenu d'un fichier `.ftlfw`.
    pub(super) fn to_ftlfw(&self) -> Result<String, String> {
        let data = FractalFile {
            shape: self.shape.as_ref().map(shared::ModelData::from_shape),
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
            density_sources: self.density_sources.clone(),
        };
        serde_json::to_string_pretty(&data).map_err(|e| e.to_string())
    }
}
