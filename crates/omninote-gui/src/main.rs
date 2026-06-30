//! OmniNote desktop GUI (egui/eframe). Binary entry point.

mod app;
mod editor;
mod md_render;
mod modals;
mod palette;
mod sidebar;
mod theme;
mod watcher;

use app::OmniNoteApp;

fn main() -> eframe::Result<()> {
    // Surface panics with a clear message even in release builds.
    let prev = std::panic::take_hook();
    std::panic::set_hook(Box::new(move |info| {
        eprintln!("omninote panicked: {info}");
        prev(info);
    }));

    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size([1200.0, 800.0])
            .with_min_inner_size([640.0, 420.0])
            .with_title("OmniNote"),
        ..Default::default()
    };

    eframe::run_native(
        "OmniNote",
        options,
        Box::new(|cc| Ok(Box::new(OmniNoteApp::new(cc)))),
    )
}
