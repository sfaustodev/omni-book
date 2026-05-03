mod app;
mod autoformat;
mod import;
mod pdf;
mod theme;
mod types;
mod ui_editor;
mod ui_modals;
mod ui_sidebar;
mod vault;
mod watcher;
mod wikilinks;

fn main() -> eframe::Result<()> {
    // Panic hook so we see the actual panic message even in release builds.
    std::panic::set_hook(Box::new(|info| {
        eprintln!("\n[omninote] PANIC: {}", info);
        if let Some(loc) = info.location() {
            eprintln!(
                "[omninote] at {}:{}:{}",
                loc.file(),
                loc.line(),
                loc.column()
            );
        }
        eprintln!("[omninote] backtrace (set RUST_BACKTRACE=1 for full):");
        let bt = std::backtrace::Backtrace::capture();
        eprintln!("{}", bt);
    }));

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
