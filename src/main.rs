mod app;
mod editors;
mod file_io;
mod fractal;
mod gizmo;
mod heatmap;
mod scene;
mod shapes;
mod types;

use eframe::egui;

fn main() -> eframe::Result<()> {
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size([1400.0, 900.0])
            .with_min_inner_size([900.0, 600.0])
            .with_title("FractalWizard - Éditeur de Figures Fractales"),
        ..Default::default()
    };

    eframe::run_native(
        "FractalWizard",
        options,
        Box::new(|_cc| Ok(Box::new(app::FractalWizardApp::default()))),
    )
}
