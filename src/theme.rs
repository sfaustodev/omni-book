//! Swiss / Bauhaus dark theme — palette, fonts, styles.
//!
//! Reference: design handoff `OmniNote Swiss.html` (chats/chat1.md).
//! Idea: rigorous grid, grotesk sans, single accent, generous whitespace.

use eframe::egui::{self, Color32};

pub const BG: Color32 = Color32::from_rgb(0x0e, 0x0e, 0x0e);
pub const PANEL: Color32 = Color32::from_rgb(0x14, 0x14, 0x14);
pub const BORDER: Color32 = Color32::from_rgb(0x26, 0x26, 0x26);
pub const TEXT: Color32 = Color32::from_rgb(0xfa, 0xfa, 0xfa);
pub const DIM: Color32 = Color32::from_rgb(0x8a, 0x8a, 0x8a);
pub const DIMMER: Color32 = Color32::from_rgb(0x5a, 0x5a, 0x5a);
pub const ACCENT: Color32 = Color32::from_rgb(0xff, 0x4d, 0x2e);
pub const ACCENT_INK: Color32 = Color32::from_rgb(0x00, 0x00, 0x00);

/// Apply Swiss theme to the egui context. Overrides `Visuals` colors and
/// sets a tighter spacing baseline. Call after `Visuals::dark()` if dark
/// mode is on.
pub fn apply_swiss(ctx: &egui::Context) {
    let mut visuals = egui::Visuals::dark();
    visuals.override_text_color = Some(TEXT);
    visuals.window_fill = BG;
    visuals.panel_fill = BG;
    visuals.extreme_bg_color = PANEL;
    visuals.faint_bg_color = PANEL;
    visuals.code_bg_color = PANEL;
    visuals.window_stroke = egui::Stroke::new(1.0, BORDER);
    visuals.widgets.noninteractive.bg_stroke = egui::Stroke::new(1.0, BORDER);
    visuals.widgets.inactive.bg_fill = PANEL;
    visuals.widgets.inactive.weak_bg_fill = PANEL;
    visuals.widgets.inactive.bg_stroke = egui::Stroke::new(1.0, BORDER);
    visuals.widgets.inactive.fg_stroke = egui::Stroke::new(1.0, DIM);
    visuals.widgets.hovered.bg_fill = PANEL;
    visuals.widgets.hovered.weak_bg_fill = PANEL;
    visuals.widgets.hovered.bg_stroke = egui::Stroke::new(1.0, ACCENT);
    visuals.widgets.hovered.fg_stroke = egui::Stroke::new(1.0, TEXT);
    visuals.widgets.active.bg_fill = ACCENT;
    visuals.widgets.active.weak_bg_fill = ACCENT;
    visuals.widgets.active.bg_stroke = egui::Stroke::new(1.0, ACCENT);
    visuals.widgets.active.fg_stroke = egui::Stroke::new(1.0, ACCENT_INK);
    visuals.selection.bg_fill = ACCENT.linear_multiply(0.35);
    visuals.selection.stroke = egui::Stroke::new(1.0, ACCENT);
    visuals.hyperlink_color = ACCENT;
    visuals.widgets.noninteractive.fg_stroke = egui::Stroke::new(1.0, DIM);
    ctx.set_visuals(visuals);
}

/// Render a small uppercase monospace section label like `VIEWS —`.
pub fn section_label(ui: &mut egui::Ui, text: &str) {
    ui.add_space(4.0);
    ui.label(
        egui::RichText::new(format!("{} —", text))
            .monospace()
            .size(9.0)
            .color(DIMMER),
    );
    ui.add_space(2.0);
}

/// Render a horizontal 1px hairline separator in the border color.
pub fn hairline(ui: &mut egui::Ui) {
    let (rect, _) = ui.allocate_exact_size(
        egui::vec2(ui.available_width(), 1.0),
        egui::Sense::hover(),
    );
    ui.painter().rect_filled(rect, 0.0, BORDER);
}

/// Format a 1-based index as a 2-digit zero-padded mono prefix, e.g. `01`, `12`.
pub fn mono_index(n: usize) -> String {
    format!("{:02}", n)
}
