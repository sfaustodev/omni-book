mod app;
mod autoformat;
mod import;
mod pdf;
mod types;
mod vault;

fn main() -> eframe::Result<()> {
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size([1200.0, 800.0])
            .with_min_inner_size([600.0, 400.0])
            .with_title("Caderno"),
        ..Default::default()
    };
    eframe::run_native(
        "Caderno",
        options,
        Box::new(|cc| Ok(Box::new(app::CadernoApp::new(cc)))),
    )
}
