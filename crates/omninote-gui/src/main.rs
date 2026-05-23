// OmniNote GUI — egui app. Vault ops live in `omninote-core`.
mod app;
mod theme;
mod ui_editor;
mod ui_modals;
mod ui_sidebar;
mod watcher;

fn main() -> eframe::Result<()> {
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size([1200.0, 800.0])
            .with_min_inner_size([600.0, 400.0])
            .with_title("OmniNote"),
        ..Default::default()
    };
    eframe::run_native(
        "OmniNote",
        options,
        Box::new(|cc| Ok(Box::new(app::OmniNoteApp::new(cc)))),
    )
}
