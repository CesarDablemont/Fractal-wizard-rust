use eframe::egui::Pos2;
use serde::{Deserialize, Serialize};

pub mod pos2_serde {
use eframe::egui::Pos2;
    use serde::{Deserialize, Deserializer, Serialize, Serializer};

    pub fn serialize<S>(p: &Pos2, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        [p.x, p.y].serialize(serializer)
    }

    pub fn deserialize<'de, D>(deserializer: D) -> Result<Pos2, D::Error>
    where
        D: Deserializer<'de>,
    {
        let [x, y] = <[f32; 2]>::deserialize(deserializer)?;
        Ok(Pos2::new(x, y))
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ShapePatternData {
    #[serde(with = "pos2_serde")]
    pub translate: Pos2,
    pub rotate: f32,
    pub scale: f32,
}

impl Default for ShapePatternData {
    fn default() -> Self {
        Self {
            translate: Pos2::ZERO,
            rotate: 0.0,
            scale: 1.0,
        }
    }
}

#[derive(Clone, Debug)]
pub struct RandomWalkInfo {
    pub walk_steps: Vec<usize>,
    pub length_walk: f32,
    pub is_random_walk_done: bool,
    pub timed_out: bool,
}

impl RandomWalkInfo {
    pub fn steps(&self) -> usize {
        self.walk_steps.len()
    }
}

impl Default for RandomWalkInfo {
    fn default() -> Self {
        Self {
            walk_steps: Vec::new(),
            length_walk: 0.0,
            is_random_walk_done: false,
            timed_out: false,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum EditorState {
    Mouse,
    Point,
    SelectPointSimulation,
    Add,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RenderMode {
    Normal,
    GlobalHeatMap,
    IndividualHeatMap,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum FigureType {
    Polygon,
    FreeLinear,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum LoopMode {
    PlayOnce,
    PlayOnceReset,
    Repeat,
    PlayOnceMirror,
    RepeatMirror,
}

impl LoopMode {
    pub fn variants() -> &'static [&'static str] {
        &[
            "Jouer une fois",
            "Jouer une fois et réinitialiser",
            "Répéter",
            "Jouer une fois en miroir",
            "Répéter en miroir",
        ]
    }
}

pub type Line = [usize; 2];

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum DensityMode {
    /// Déplace les points après génération (comportement original)
    Displacement,
    /// Module le facteur de contraction pendant les itérations (multifractal)
    Contraction,
    /// Module le nombre d'itérations par branche selon la position
    Iteration,
}

impl Default for DensityMode {
    fn default() -> Self {
        DensityMode::Displacement
    }
}

impl DensityMode {
    pub fn label(self) -> &'static str {
        match self {
            DensityMode::Displacement => "Déplacement",
            DensityMode::Contraction => "Contraction",
            DensityMode::Iteration => "Itération",
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct DensitySource {
    #[serde(with = "pos2_serde")]
    pub position: Pos2,
    pub radius: f32,
    pub force: f32,
    pub exponent: f32,
    #[serde(default)]
    pub mode: DensityMode,
}

impl Default for DensitySource {
    fn default() -> Self {
        Self {
            position: Pos2::ZERO,
            radius: 100.0,
            force: 1.0,
            exponent: 1.0,
            mode: DensityMode::Displacement,
        }
    }
}
