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
    #[allow(dead_code)]
    pub max_steps: u64,
    pub length_walk: f32,
    pub is_random_walk_done: bool,
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
            max_steps: 5000,
            length_walk: 0.0,
            is_random_walk_done: false,
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
