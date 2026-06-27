use eframe::egui;
use crate::editors::figure::FigureEditor;
use crate::editors::pattern::PatternEditor;
use crate::editors::initial::InitialEditor;
use crate::editors::fractal::FractalEditor;

#[derive(PartialEq)]
enum ActiveEditor {
    Figure,
    Pattern,
    Initial,
    Fractal,
}

pub struct FractalWizardApp {
    active: ActiveEditor,
    figure_editor: FigureEditor,
    pattern_editor: PatternEditor,
    initial_editor: InitialEditor,
    fractal_editor: FractalEditor,
}

impl Default for FractalWizardApp {
    fn default() -> Self {
        Self {
            active: ActiveEditor::Figure,
            figure_editor: FigureEditor::default(),
            pattern_editor: PatternEditor::default(),
            initial_editor: InitialEditor::default(),
            fractal_editor: FractalEditor::default(),
        }
    }
}

impl eframe::App for FractalWizardApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        ctx.set_pixels_per_point(1.5);

        egui::TopBottomPanel::top("main_toolbar").show(ctx, |ui| {
            ui.horizontal(|ui| {
                ui.heading("FractalWizard");

                ui.separator();

                ui.selectable_value(&mut self.active, ActiveEditor::Figure, "📐 Figure");
                ui.selectable_value(&mut self.active, ActiveEditor::Pattern, "🔷 Pattern");
                ui.selectable_value(&mut self.active, ActiveEditor::Initial, "🔰 Initial");
                ui.selectable_value(&mut self.active, ActiveEditor::Fractal, "❄️ Fractale");

                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    let reps = ui.input(|i| i.predicted_dt);
                    ui.label(format!("{:.0} FPS", 1.0 / reps));
                });
            });
        });

        match self.active {
            ActiveEditor::Figure => self.figure_editor.render(ctx),
            ActiveEditor::Pattern => self.pattern_editor.render(ctx),
            ActiveEditor::Initial => self.initial_editor.render(ctx),
            ActiveEditor::Fractal => self.fractal_editor.render(ctx),
        }

        if let Some(shape) = self.figure_editor.transfer_shape.take() {
            self.fractal_editor.import_shape(shape);
            self.active = ActiveEditor::Fractal;
        }
        if let Some(data) = self.figure_editor.transfer_to_pattern.take() {
            self.pattern_editor.receive_figure = Some(data);
            self.active = ActiveEditor::Pattern;
        }
        if let Some(data) = self.figure_editor.transfer_to_initial.take() {
            self.initial_editor.receive_figure = Some(data);
            self.active = ActiveEditor::Initial;
        }
        if let Some(data) = self.pattern_editor.transfer_patterns.take() {
            self.fractal_editor.import_pattern_data(data);
            self.active = ActiveEditor::Fractal;
        }
        if let Some(data) = self.initial_editor.transfer_shapes.take() {
            self.fractal_editor.import_initial_data(data);
            self.active = ActiveEditor::Fractal;
        }

        if self.fractal_editor.is_playing {
            ctx.request_repaint();
        }
    }
}
