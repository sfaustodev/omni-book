// OmniNote — desktop GUI (egui). "Blueprint" edition: a cool drafting-table vault.
//
// Immediate-mode: `OmniNoteApp::update()` paints the whole UI from struct state
// every frame. Engine logic (vault, search, wikilinks, import, math) lives in
// `omninote-core`; this crate is presentation + wiring. Layout: a command bar on
// top, an Index rail on the left, the Sheet (editor) in the center, and a
// References dock along the bottom.

#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod dialogs;
mod fswatch;
mod header;
mod index;
mod look;
mod sheet;
mod state;

fn main() -> eframe::Result<()> {
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_title("OmniNote · Blueprint")
            .with_inner_size([1280.0, 840.0])
            .with_min_inner_size([780.0, 520.0])
            .with_app_id("omninote-blueprint"),
        ..Default::default()
    };
    eframe::run_native(
        "OmniNote",
        options,
        Box::new(|cc| Ok(Box::new(state::OmniNoteApp::new(cc)))),
    )
}
