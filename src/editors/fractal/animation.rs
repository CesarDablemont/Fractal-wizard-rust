use eframe::egui;
use crate::types::LoopMode;

/// Lecteur d'animation d'une simulation : étape affichée et lecture.
pub(super) struct AnimationState {
    current_step: usize,
    is_playing: bool,
    pub(super) fps: f32,
    pub(super) loop_mode: LoopMode,
    last_frame: std::time::Instant,
    ascending: bool,
}

impl Default for AnimationState {
    fn default() -> Self {
        Self {
            current_step: 0,
            is_playing: false,
            fps: 60.0,
            loop_mode: LoopMode::PlayOnceReset,
            last_frame: std::time::Instant::now(),
            ascending: true,
        }
    }
}

impl AnimationState {
    pub(super) fn current_step(&self) -> usize {
        self.current_step
    }

    pub(super) fn is_playing(&self) -> bool {
        self.is_playing
    }

    /// Revient à l'étape 0 et arrête la lecture.
    pub(super) fn reset(&mut self) {
        self.current_step = 0;
        self.is_playing = false;
        self.ascending = true;
    }

    /// Barre du lecteur, puis avance d'une étape si c'est l'heure.
    /// `max_step` est le nombre d'étapes de la simulation sélectionnée, `None` sans sélection.
    pub(super) fn show(&mut self, ui: &mut egui::Ui, max_step: Option<usize>) {
        ui.horizontal(|ui| {
            if max_step.is_none() {
                ui.disable();
            }

            let play_label = if self.is_playing { "⏸ Pause" } else { "▶ Jouer" };
            if ui.button(play_label).clicked() {
                self.is_playing = !self.is_playing;
                self.last_frame = std::time::Instant::now();
            }

            if ui.button("⏹ Reset").clicked() {
                self.reset();
            }

            ui.separator();

            if let Some(max_step) = max_step {
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

        if let Some(max) = max_step {
            self.tick(max);
        }
    }

    fn tick(&mut self, max: usize) {
        if !self.is_playing || self.last_frame.elapsed().as_secs_f32() < 1.0 / self.fps {
            return;
        }
        self.last_frame = std::time::Instant::now();
        self.step(max);
    }

    /// Avance d'une étape selon `loop_mode`, `max` étant la dernière étape.
    fn step(&mut self, max: usize) {
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
                } else if self.current_step > 0 {
                    self.current_step -= 1;
                } else {
                    self.ascending = true;
                    self.is_playing = false;
                }
            }
            LoopMode::RepeatMirror => {
                if self.ascending {
                    if self.current_step < max {
                        self.current_step += 1;
                    } else {
                        self.ascending = false;
                    }
                } else if self.current_step > 0 {
                    self.current_step -= 1;
                } else {
                    self.ascending = true;
                }
            }
        }

        self.current_step = self.current_step.min(max);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn playing(loop_mode: LoopMode) -> AnimationState {
        AnimationState { is_playing: true, loop_mode, ..Default::default() }
    }

    #[test]
    fn play_once_reset_stops_at_zero() {
        let mut anim = playing(LoopMode::PlayOnceReset);
        for _ in 0..3 {
            anim.step(2);
        }
        assert_eq!(anim.current_step(), 0);
        assert!(!anim.is_playing());
    }

    #[test]
    fn repeat_wraps_around() {
        let mut anim = playing(LoopMode::Repeat);
        for _ in 0..3 {
            anim.step(2);
        }
        assert_eq!(anim.current_step(), 0);
        assert!(anim.is_playing());
    }

    #[test]
    fn play_once_mirror_goes_back_then_stops() {
        let mut anim = playing(LoopMode::PlayOnceMirror);
        let steps: Vec<usize> = (0..6).map(|_| { anim.step(2); anim.current_step() }).collect();
        assert_eq!(steps, [1, 2, 2, 1, 0, 0]);
        assert!(!anim.is_playing());
    }

    #[test]
    fn reset_rewinds_and_pauses() {
        let mut anim = playing(LoopMode::RepeatMirror);
        anim.step(5);
        anim.reset();
        assert_eq!(anim.current_step(), 0);
        assert!(!anim.is_playing());
    }
}
