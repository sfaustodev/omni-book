// OmniNote — desktop GUI (egui). "Almanac" edition: a warm editorial vault.
//
// Immediate-mode app: `OmniNoteApp::update()` runs every frame and paints the
// whole UI from state held on the struct. No retained widgets. Engine logic
// (vault CRUD, search, wikilinks, import, math) lives in `omninote-core`; this
// crate is the presentation + wiring layer.

#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod app;
mod chrome;
mod editor;
mod modals;
mod sidebar;
mod theme;
mod watcher;

fn main() -> eframe::Result<()> {
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_title("OmniNote · Almanac")
            .with_inner_size([1240.0, 820.0])
            .with_min_inner_size([760.0, 520.0])
            .with_app_id("omninote-almanac"),
        ..Default::default()
    };
    eframe::run_native(
        "OmniNote",
        options,
        Box::new(|cc| Ok(Box::new(app::OmniNoteApp::new(cc)))),
    )
}
