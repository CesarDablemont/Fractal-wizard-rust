use eframe::egui::{self, Align2, Color32, FontId, Shape, Vec2};
use crate::heatmap::heatmap_color;
use crate::scene::canvas;
use crate::types::{DensityMode, EditorState, RenderMode};
use super::{shared, FractalEditor};

impl FractalEditor {
    pub(super) fn render_canvas(&mut self, ui: &mut egui::Ui) {
        let (response, painter) = ui.allocate_painter(
            ui.available_size(),
            egui::Sense::click_and_drag(),
        );

        let canvas_rect = response.rect;
        let canvas_center = canvas_rect.center();
        let mut shapes: Vec<Shape> = Vec::new();

        shared::handle_zoom_scroll(&response, ui, &mut self.camera, canvas_center);
        shared::handle_middle_pan(&response, ui, &mut self.camera);

        self.drag_density_sources(ui, &response, canvas_center);

        if response.dragged_by(egui::PointerButton::Primary) && self.state == EditorState::Mouse && self.dragging_density_source.is_none() {
            self.camera.pan(ui.input(|i| i.pointer.delta()));
        }

        self.canvas_renderer.draw_grid(&self.camera, canvas_rect, &mut shapes);
        self.canvas_renderer.draw_origin(&self.camera, canvas_rect, &mut shapes);
        {
            let fractal = self.fractal.as_ref();
            let points = fractal.map(|f| f.points.as_slice()).unwrap_or(&[]);
            let lines = fractal.map(|f| f.lines.as_slice()).unwrap_or(&[]);
            let point_scale = fractal.map(|f| f.point_scale.as_slice()).unwrap_or(&[]);

            if self.camera.display_lines && !lines.is_empty() {
                self.canvas_renderer.draw_fractal_lines(
                    &self.camera, canvas_rect, points, lines, Color32::YELLOW, &mut shapes,
                );
            }

            if !points.is_empty() {
                let highlight = self
                    .simulation
                    .selected()
                    .and_then(|sim| sim.walk_steps.get(self.animation.current_step()).copied());

                match self.render_mode {
                    RenderMode::Normal => {
                        self.canvas_renderer.draw_fractal_points(
                            &self.camera, canvas_rect,
                            &canvas::FractalDrawData { points, point_scale, colors: &[], highlight },
                            &mut shapes,
                        );
                    }
                    RenderMode::GlobalHeatMap | RenderMode::IndividualHeatMap => {
                        let heatmap = match self.render_mode {
                            RenderMode::GlobalHeatMap => self.simulation.global_heatmap(),
                            _ => self.simulation.individual_heatmap(),
                        };
                        let colors: Vec<Option<Color32>> = heatmap.iter().map(|&s| Some(heatmap_color(s))).collect();
                        self.canvas_renderer.draw_fractal_points(
                            &self.camera, canvas_rect,
                            &canvas::FractalDrawData { points, point_scale, colors: &colors, highlight },
                            &mut shapes,
                        );

                        if self.show_heat_score && heatmap.len() == points.len() {
                            for (i, &score) in heatmap.iter().enumerate() {
                                let screen = self.camera.world_to_screen(points[i], canvas_center);
                                painter.text(
                                    screen + Vec2::new(0.0, self.camera.point_size / 2.0 + 2.0),
                                    Align2::CENTER_TOP,
                                    format!("{score:.2}"),
                                    FontId::proportional(10.0),
                                    Color32::WHITE,
                                );
                            }
                        }
                    }
                }
            }

            if response.clicked_by(egui::PointerButton::Primary) && self.state == EditorState::SelectPointSimulation {
                let picked = ui.input(|i| i.pointer.interact_pos()).and_then(|mouse| {
                    canvas::pick_fractal_point(&self.camera, points, point_scale, mouse, canvas_center)
                });
                if let Some(i) = picked {
                    self.start_simulation(i);
                    self.state = EditorState::Mouse;
                }
            }
        }

        self.draw_density_sources(&painter, canvas_center, &mut shapes);

        painter.extend(shapes);
    }

    /// Sélectionne et déplace une source de densité au clic-glisser.
    fn drag_density_sources(&mut self, ui: &egui::Ui, response: &egui::Response, canvas_center: egui::Pos2) {
        let pointer_pressed = ui.input(|i| i.pointer.any_pressed());
        let pointer_released = ui.input(|i| i.pointer.any_released());
        let sources = &mut self.inputs.density_sources;

        if pointer_pressed && self.dragging_density_source.is_none() {
            if let Some(mouse) = ui.input(|i| i.pointer.interact_pos()) {
                let world = self.camera.screen_to_world(mouse, canvas_center);
                let hit_radius = 15.0 / self.camera.zoom;
                for (i, src) in sources.iter().enumerate() {
                    let dx = src.position.x - world.x;
                    let dy = src.position.y - world.y;
                    if (dx * dx + dy * dy) <= hit_radius * hit_radius {
                        self.selected_density_source = Some(i);
                        self.dragging_density_source = Some(i);
                        break;
                    }
                }
            }
        }

        if let Some(idx) = self.dragging_density_source {
            if pointer_released {
                self.dragging_density_source = None;
            } else if response.dragged_by(egui::PointerButton::Primary) {
                let delta = ui.input(|i| i.pointer.delta());
                if delta != Vec2::ZERO {
                    sources[idx].position += self.camera.screen_delta_to_world(delta);
                }
            }
        }
    }

    fn draw_density_sources(&self, painter: &egui::Painter, canvas_center: egui::Pos2, shapes: &mut Vec<Shape>) {
        for (i, src) in self.inputs.density_sources.iter().enumerate() {
            let center = self.camera.world_to_screen(src.position, canvas_center);
            let screen_radius = src.radius * self.camera.zoom;
            let (fill, stroke_color, label) = match (src.mode, src.force >= 0.0) {
                (DensityMode::Displacement, true) => (
                    Color32::from_rgba_premultiplied(0, 180, 0, 20),
                    Color32::from_rgba_premultiplied(0, 180, 0, 60),
                    format!("A{}", i + 1),
                ),
                (DensityMode::Displacement, false) => (
                    Color32::from_rgba_premultiplied(180, 0, 0, 20),
                    Color32::from_rgba_premultiplied(180, 0, 0, 60),
                    format!("R{}", i + 1),
                ),
                (DensityMode::Contraction, true) => (
                    Color32::from_rgba_premultiplied(0, 120, 255, 20),
                    Color32::from_rgba_premultiplied(0, 120, 255, 60),
                    format!("C{}", i + 1),
                ),
                (DensityMode::Contraction, false) => (
                    Color32::from_rgba_premultiplied(255, 120, 0, 20),
                    Color32::from_rgba_premultiplied(255, 120, 0, 60),
                    format!("D{}", i + 1),
                ),
                (DensityMode::Iteration, true) => (
                    Color32::from_rgba_premultiplied(180, 0, 255, 20),
                    Color32::from_rgba_premultiplied(180, 0, 255, 60),
                    format!("I+{}", i + 1),
                ),
                (DensityMode::Iteration, false) => (
                    Color32::from_rgba_premultiplied(255, 255, 0, 20),
                    Color32::from_rgba_premultiplied(255, 255, 0, 60),
                    format!("I-{}", i + 1),
                ),
            };
            let stroke = egui::Stroke::new(1.5_f32, stroke_color);
            shapes.push(Shape::Circle(egui::epaint::CircleShape {
                center,
                radius: screen_radius,
                fill,
                stroke,
            }));
            painter.text(
                center,
                Align2::CENTER_CENTER,
                &label,
                FontId::proportional(12.0),
                Color32::WHITE,
            );
        }
    }
}
