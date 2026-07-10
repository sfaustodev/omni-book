// OmniNote GUI — egui app. Vault ops live in `omninote-core`.
mod app;
mod md_render;
mod native_menu;
mod theme;
mod ui_a11y;
mod ui_breadcrumb;
mod ui_calendar;
mod ui_discipline;
mod ui_editor;
mod ui_modals;
mod ui_palette;
mod ui_right_rail;
mod ui_sidebar;
mod ui_statusbar;
mod ui_tabs;
mod ui_timeline;
mod ui_titlebar;
mod ui_toasts;
mod watcher;

fn main() -> eframe::Result<()> {
    // Surface the real panic message, location, and a backtrace even in release
    // builds (eframe otherwise swallows them behind a generic abort).
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
        eprintln!("{}", std::backtrace::Backtrace::capture());
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
