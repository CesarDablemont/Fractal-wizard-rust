use eframe::egui::{self, Color32, Pos2, Shape, Stroke, Vec2};
use serde::{Deserialize, Serialize};
use crate::scene::camera::Camera;
use crate::scene::canvas::CanvasRenderer;
use crate::shapes::polygon::Polygon;
use crate::shapes::free_linear::FreeLinearShape;
use crate::shapes::shape::Shape as ShapeTrait;
use crate::types::{EditorState, FigureType, Line};
use crate::file_io;

#[derive(Serialize, Deserialize)]
struct FigureData {
    r#type: String,
    points: Vec<[f32; 2]>,
    #[serde(skip_serializing_if = "Vec::is_empty", default)]
    lines: Vec<Line>,
}

pub struct FigureEditor {
    pub is_open: bool,
    pub file_path: Option<String>,
    pub transfer_shape: Option<super::fractal::ShapeWrapper>,
    pub transfer_to_pattern: Option<(Vec<Pos2>, Vec<Line>)>,
    pub transfer_to_initial: Option<(Vec<Pos2>, Vec<Line>)>,

    camera: Camera,
    canvas_renderer: CanvasRenderer,
    shape: Option<FigureShape>,
    state: EditorState,
    figure_type: FigureType,
    selected_point: Option<usize>,
    message: Option<String>,
}

#[derive(Clone)]
enum FigureShape {
    Polygon(Polygon),
    FreeLinear(FreeLinearShape),
}

impl FigureShape {
    fn to_shape_wrapper(&self) -> super::fractal::ShapeWrapper {
        match self {
            FigureShape::Polygon(p) => super::fractal::ShapeWrapper::Polygon(p.clone()),
            FigureShape::FreeLinear(s) => super::fractal::ShapeWrapper::FreeLinear(s.clone()),
        }
    }

    fn points(&self) -> &[Pos2] {
        match self {
            FigureShape::Polygon(p) => p.points(),
            FigureShape::FreeLinear(p) => p.points(),
        }
    }

    fn points_mut(&mut self) -> &mut Vec<Pos2> {
        match self {
            FigureShape::Polygon(p) => p.points_mut(),
            FigureShape::FreeLinear(p) => p.points_mut(),
        }
    }

    fn add_point(&mut self, p: Pos2) {
        match self {
            FigureShape::Polygon(s) => s.add_point(p),
            FigureShape::FreeLinear(s) => s.add_point(p),
        }
    }

    fn remove_point(&mut self, idx: usize) {
        match self {
            FigureShape::Polygon(s) => s.remove_point(idx),
            FigureShape::FreeLinear(s) => s.remove_point(idx),
        }
    }

    #[allow(dead_code)]
    fn type_name(&self) -> &'static str {
        match self {
            FigureShape::Polygon(_) => "cPolygon",
            FigureShape::FreeLinear(_) => "cFreeLinearShape",
        }
    }

    fn hit_test(&self, world_pos: Pos2, point_size: f32) -> Option<usize> {
        let half = point_size / 2.0;
        self.points().iter().position(|&p| {
            let dx = (p.x - world_pos.x).abs();
            let dy = (p.y - world_pos.y).abs();
            dx <= half && dy <= half
        })
    }
}

impl Default for FigureEditor {
    fn default() -> Self {
        Self {
            is_open: true,
            file_path: None,
            transfer_shape: None,
            transfer_to_pattern: None,
            transfer_to_initial: None,
            camera: Camera::default(),
            canvas_renderer: CanvasRenderer::new(),
            shape: None,
            state: EditorState::Mouse,
            figure_type: FigureType::Polygon,
            selected_point: None,
            message: None,
        }
    }
}

impl FigureEditor {
    pub fn render(&mut self, ctx: &egui::Context) {
        egui::TopBottomPanel::top("figure_editor_menu").show(ctx, |ui| {
            self.render_menu(ui);
        });

        egui::CentralPanel::default().show(ctx, |ui| {
            self.render_canvas(ui);
        });
    }

    fn render_menu(&mut self, ui: &mut egui::Ui) {
        ui.horizontal(|ui| {
            ui.menu_button("Fichier", |ui| {
                if ui.button("Nouveau Polygone").clicked() {
                    self.shape = Some(FigureShape::Polygon(Polygon::new()));
                    self.figure_type = FigureType::Polygon;
                    self.state = EditorState::Add;
                    ui.close_menu();
                }
                if ui.button("Nouveau Libre").clicked() {
                    self.shape = Some(FigureShape::FreeLinear(FreeLinearShape::new()));
                    self.figure_type = FigureType::FreeLinear;
                    self.state = EditorState::Add;
                    ui.close_menu();
                }
                ui.separator();
                if ui.button("Ouvrir").clicked() {
                    if let Some((path, content)) = file_io::open_json("Ouvrir une figure", "firfw") {
                        match serde_json::from_str::<FigureData>(&content) {
                            Ok(data) => {
                                self.file_path = Some(path.display().to_string());
                                self.shape = Some(match data.r#type.as_str() {
                                    "Polygon" | "cPolygon" => {
                                        let mut p = Polygon::new();
                                        for pt in &data.points {
                                            p.add_point(Pos2::new(pt[0], pt[1]));
                                        }
                                        FigureShape::Polygon(p)
                                    }
                                    _ => {
                                        let mut s = FreeLinearShape::new();
                                        for pt in &data.points {
                                            s.add_point(Pos2::new(pt[0], pt[1]));
                                        }
                                        for l in &data.lines {
                                            s.add_line_segment(l[0], l[1]);
                                        }
                                        FigureShape::FreeLinear(s)
                                    }
                                });
                                self.message = Some("Figure chargée".into());
                            }
                            Err(e) => self.message = Some(format!("Erreur: {}", e)),
                        }
                    }
                    ui.close_menu();
                }
                if ui.button("Enregistrer").clicked() {
                    if let Some(ref shape) = self.shape {
                        let data = match shape {
                            FigureShape::Polygon(p) => FigureData {
                                r#type: "Polygon".into(),
                                points: p.points().iter().map(|pt| [pt.x, pt.y]).collect(),
                                lines: Vec::new(),
                            },
                            FigureShape::FreeLinear(s) => FigureData {
                                r#type: "FreeLinear".into(),
                                points: s.points().iter().map(|pt| [pt.x, pt.y]).collect(),
                                lines: s.lines().to_vec(),
                            },
                        };
                        let json = serde_json::to_string_pretty(&data).unwrap();
                        let name = self.file_path.as_deref().and_then(|p| {
                            std::path::Path::new(p).file_stem().and_then(|s| s.to_str())
                        }).unwrap_or("figure");
                        if file_io::save_json_path("Enregistrer la figure", "firfw", &format!("{}.firfw", name), &json) {
                            self.message = Some("Figure enregistrée".into());
                        }
                    }
                    ui.close_menu();
                }
            });

            ui.separator();

            ui.menu_button("Options", |ui| {
                ui.checkbox(&mut self.camera.display_grid, "Grille");
                if self.camera.display_grid {
                    ui.add(egui::Slider::new(&mut self.camera.grid_spacing, 10.0..=200.0).text("Espacement"));
                    ui.checkbox(&mut self.camera.magnetism, "Magnétisme");
                }
                ui.checkbox(&mut self.camera.display_points, "Points");
                ui.add(egui::Slider::new(&mut self.camera.point_size, 2.0..=25.0).text("Taille"));
                ui.checkbox(&mut self.camera.display_origin, "Origine");
            });

            if let Some(ref shape) = self.shape {
                if ui.button("Mode souris").clicked() {
                    self.state = EditorState::Mouse;
                }
                if ui.button("Mode point").clicked() {
                    self.state = EditorState::Point;
                }
                ui.separator();
                            if ui.button("➡ Envoyer à Fractale").clicked() {
                        self.transfer_shape = Some(shape.to_shape_wrapper());
                    }
                    let pts = shape.points().to_vec();
                    let lns = match shape {
                        FigureShape::Polygon(_) => {
                            if pts.len() >= 2 {
                                let mut lines: Vec<Line> = (0..pts.len() - 1).map(|i| [i, i + 1]).collect();
                                if pts.len() > 2 {
                                    lines.push([pts.len() - 1, 0]);
                                }
                                lines
                            } else { Vec::new() }
                        }
                        FigureShape::FreeLinear(s) => s.lines().to_vec(),
                    };
                    if ui.button("➡ Envoyer à Pattern").clicked() {
                        self.transfer_to_pattern = Some((pts.clone(), lns.clone()));
                    }
                    if ui.button("➡ Envoyer à Initial").clicked() {
                        self.transfer_to_initial = Some((pts.clone(), lns.clone()));
                    }
            }

            if let Some(ref msg) = self.message {
                ui.separator();
                ui.label(msg);
            }
        });
    }

    fn render_canvas(&mut self, ui: &mut egui::Ui) {
        let (response, painter) = ui.allocate_painter(
            ui.available_size(),
            egui::Sense::click_and_drag(),
        );

        let canvas_rect = response.rect;
        let canvas_center = canvas_rect.center();
        let mut shapes: Vec<Shape> = Vec::new();

        if response.hovered() {
            let scroll = ui.input(|i| i.raw_scroll_delta);
            if scroll.y != 0.0 {
                let factor = 1.15f32.powf(scroll.y / 10.0);
                self.camera.zoom_at(factor, ui.input(|i| i.pointer.hover_pos().unwrap_or(canvas_center)), canvas_center);
            }
        }

        if response.dragged_by(egui::PointerButton::Middle)
            || (response.dragged_by(egui::PointerButton::Primary) && self.state == EditorState::Mouse && self.selected_point.is_none())
        {
            self.camera.pan(ui.input(|i| i.pointer.delta()));
        }

        self.canvas_renderer.draw_grid(&self.camera, canvas_rect, &mut shapes);
        self.canvas_renderer.draw_origin(&self.camera, canvas_rect, &mut shapes);

        if let Some(ref shape) = self.shape {
            let points = shape.points();
            if !points.is_empty() {
                let stroke = Stroke::new(2.0, Color32::YELLOW);
                let mut prev_screen = None;
                for &p in points {
                    let screen = self.camera.world_to_screen(p, canvas_center);
                    if let Some(prev) = prev_screen {
                        shapes.push(Shape::line_segment([prev, screen], stroke));
                    }
                    prev_screen = Some(screen);
                }
                if matches!(shape, FigureShape::Polygon(_)) && points.len() > 2 {
                    if let (Some(&first), Some(&last)) = (points.first(), points.last()) {
                        let s1 = self.camera.world_to_screen(first, canvas_center);
                        let s2 = self.camera.world_to_screen(last, canvas_center);
                        shapes.push(Shape::line_segment([s1, s2], stroke));
                    }
                }
            }
        }

        if let Some(ref shape) = self.shape {
            if self.camera.display_points {
                for (i, &p) in shape.points().iter().enumerate() {
                    let screen = self.camera.world_to_screen(p, canvas_center);
                    let color = if self.selected_point == Some(i) {
                        Color32::WHITE
                    } else {
                        Color32::RED
                    };
                    let half = self.camera.point_size / 2.0;
                    shapes.push(Shape::rect_filled(
                        egui::Rect::from_min_max(screen - Vec2::splat(half), screen + Vec2::splat(half)),
                        0.0,
                        color,
                    ));
                }
            }
        }

        if response.clicked_by(egui::PointerButton::Primary) {
            if let Some(mouse_pos) = ui.input(|i| i.pointer.interact_pos()) {
                let world_pos = self.camera.screen_to_world(mouse_pos, canvas_center);

                match self.state {
                    EditorState::Add | EditorState::Point => {
                        if let Some(ref mut shape) = self.shape {
                            let snapped = self.camera.snap(world_pos);
                            shape.add_point(snapped);
                            self.selected_point = Some(shape.points().len() - 1);
                        }
                    }
                    EditorState::Mouse => {
                        if let Some(ref shape) = self.shape {
                            self.selected_point = shape.hit_test(world_pos, self.camera.point_size);
                        }
                    }
                    _ => {}
                }
            }
        }

        if response.clicked_by(egui::PointerButton::Secondary) {
            if let Some(idx) = self.selected_point {
                if let Some(ref mut shape) = self.shape {
                    shape.remove_point(idx);
                    self.selected_point = None;
                }
            }
        }

        painter.extend(shapes);
    }
}
