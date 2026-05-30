//! egui-specific theme — Swiss / Bauhaus palette + the bridge from the
//! portable `omninote_core::types::FontFamily` enum to `egui::FontFamily`.
//! Lives in the gui crate so `omninote-core` stays free of any UI dependency.
//!
//! Design language: rigorous grid, grotesk sans, a single warm accent,
//! generous whitespace. Dark is the primary variant; the light variant keeps
//! the same accent over a paper background so the `dark_mode` toggle is useful.

use eframe::egui::{self, Color32};
use omninote_core::types::FontFamily;

/// Identifier the OpenDyslexic face is registered under in `FontDefinitions`.
pub const OPEN_DYSLEXIC_NAME: &str = "OpenDyslexic";

// Dark palette.
pub const BG: Color32 = Color32::from_rgb(0x0e, 0x0e, 0x0e);
pub const PANEL: Color32 = Color32::from_rgb(0x14, 0x14, 0x14);
pub const BORDER: Color32 = Color32::from_rgb(0x26, 0x26, 0x26);
pub const TEXT: Color32 = Color32::from_rgb(0xfa, 0xfa, 0xfa);
pub const DIM: Color32 = Color32::from_rgb(0x8a, 0x8a, 0x8a);

// Light palette — paper background, ink text, lighter hairlines.
pub const BG_LIGHT: Color32 = Color32::from_rgb(0xfa, 0xfa, 0xf7);
pub const PANEL_LIGHT: Color32 = Color32::from_rgb(0xf0, 0xf0, 0xec);
pub const BORDER_LIGHT: Color32 = Color32::from_rgb(0xd6, 0xd6, 0xd0);
pub const TEXT_LIGHT: Color32 = Color32::from_rgb(0x14, 0x14, 0x14);
pub const DIM_LIGHT: Color32 = Color32::from_rgb(0x60, 0x60, 0x60);

// Shared accent — the single warm signal color in both variants.
pub const ACCENT: Color32 = Color32::from_rgb(0xff, 0x5a, 0x1f);
pub const ACCENT_INK: Color32 = Color32::from_rgb(0x00, 0x00, 0x00);

/// Apply the Swiss theme to the egui context. `dark` selects the variant; the
/// accent and tight stroke baseline are shared. Re-applying is cheap and idempotent.
pub fn apply_theme(ctx: &egui::Context, dark: bool) {
    let (bg, panel, border, text, dim) = if dark {
        (BG, PANEL, BORDER, TEXT, DIM)
    } else {
        (BG_LIGHT, PANEL_LIGHT, BORDER_LIGHT, TEXT_LIGHT, DIM_LIGHT)
    };

    let mut visuals = if dark {
        egui::Visuals::dark()
    } else {
        egui::Visuals::light()
    };
    visuals.override_text_color = Some(text);
    visuals.window_fill = bg;
    visuals.panel_fill = bg;
    visuals.extreme_bg_color = panel;
    visuals.faint_bg_color = panel;
    visuals.code_bg_color = panel;
    visuals.window_stroke = egui::Stroke::new(1.0, border);
    visuals.widgets.noninteractive.bg_stroke = egui::Stroke::new(1.0, border);
    visuals.widgets.noninteractive.fg_stroke = egui::Stroke::new(1.0, dim);
    visuals.widgets.inactive.bg_fill = panel;
    visuals.widgets.inactive.weak_bg_fill = panel;
    visuals.widgets.inactive.bg_stroke = egui::Stroke::new(1.0, border);
    visuals.widgets.inactive.fg_stroke = egui::Stroke::new(1.0, dim);
    visuals.widgets.hovered.bg_fill = panel;
    visuals.widgets.hovered.weak_bg_fill = panel;
    visuals.widgets.hovered.bg_stroke = egui::Stroke::new(1.0, ACCENT);
    visuals.widgets.hovered.fg_stroke = egui::Stroke::new(1.0, text);
    visuals.widgets.active.bg_fill = ACCENT;
    visuals.widgets.active.weak_bg_fill = ACCENT;
    visuals.widgets.active.bg_stroke = egui::Stroke::new(1.0, ACCENT);
    visuals.widgets.active.fg_stroke = egui::Stroke::new(1.0, ACCENT_INK);
    visuals.selection.bg_fill = ACCENT.linear_multiply(0.35);
    visuals.selection.stroke = egui::Stroke::new(1.0, ACCENT);
    visuals.hyperlink_color = ACCENT;
    ctx.set_visuals(visuals);
}

/// Map a portable `FontFamily` (core, no egui dep) to its `egui::FontFamily`.
/// `Dyslexic` resolves to the OpenDyslexic face registered under
/// [`OPEN_DYSLEXIC_NAME`]; callers must register it before use.
pub fn font_family_to_egui(f: FontFamily) -> egui::FontFamily {
    match f {
        FontFamily::System | FontFamily::Serif => egui::FontFamily::Proportional,
        FontFamily::Monospace => egui::FontFamily::Monospace,
        FontFamily::Dyslexic => egui::FontFamily::Name(OPEN_DYSLEXIC_NAME.into()),
    }
}
