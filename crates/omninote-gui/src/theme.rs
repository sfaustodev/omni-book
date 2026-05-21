//! egui-specific theme helpers — bridges `omninote_core::types::AppConfig`
//! to `egui::Style`. Lives here (not in core) to keep core UI-free.

use omninote_core::types::FontFamily;

/// Map a portable `FontFamily` enum (core, no egui dep) to its `egui::FontFamily`.
/// CAD-21: extracted from the old `FontFamily::as_egui_family()` method.
pub fn font_family_to_egui(f: FontFamily) -> egui::FontFamily {
    match f {
        FontFamily::System | FontFamily::Serif => egui::FontFamily::Proportional,
        FontFamily::Monospace => egui::FontFamily::Monospace,
    }
}
