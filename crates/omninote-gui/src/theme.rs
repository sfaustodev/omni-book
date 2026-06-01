//! egui theme — the Obsidian-style palette as a `Theme` token set with presets
//! (`obsidian_dark` / `obsidian_light` / `high_contrast` / custom accent), plus
//! the bridge from the portable `omninote_core::types::FontFamily` enum to
//! `egui::FontFamily`. Lives in the gui crate so `omninote-core` stays UI-free.
//!
//! Every `ui_*.rs` pulls colors from a `Theme`, so switching preset is a single
//! re-bind via `Theme::from_preset(..).apply(ctx)` (or the app's
//! `theme_for_config`, which also honours the legacy `dark_mode` flag).

use eframe::egui::{self, Color32};
use omninote_core::types::{FontFamily, ThemePreset};

/// Identifier the OpenDyslexic face is registered under in `FontDefinitions`.
pub const OPEN_DYSLEXIC_NAME: &str = "OpenDyslexic";

/// Full token set for one theme variant. `dark` is carried explicitly rather
/// than inferred from a colour channel — a single-channel heuristic misclassifies
/// non-greyscale backgrounds (bright green r=0 reads "dark", dark red r>128 reads
/// "light"), which would pick the wrong egui base visuals.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Theme {
    pub bg: Color32,
    pub panel: Color32,
    pub border: Color32,
    pub text: Color32,
    pub dim: Color32,
    pub accent: Color32,
    /// Text/ink drawn on top of an accent fill.
    pub accent_ink: Color32,
    /// Whether this variant uses egui's dark base visuals.
    pub dark: bool,
}

impl Theme {
    /// Obsidian soft-violet dark — the primary variant.
    pub const fn obsidian_dark() -> Self {
        Self {
            bg: Color32::from_rgb(0x1b, 0x1d, 0x22),
            panel: Color32::from_rgb(0x21, 0x24, 0x29),
            border: Color32::from_rgb(0x2d, 0x30, 0x37),
            text: Color32::from_rgb(0xda, 0xdd, 0xe2),
            dim: Color32::from_rgb(0x8b, 0x8f, 0x98),
            accent: Color32::from_rgb(0x8b, 0x7c, 0xff),
            accent_ink: Color32::from_rgb(0x0a, 0x0a, 0x0a),
            dark: true,
        }
    }

    /// Paper-light variant keeping the same violet accent.
    pub const fn obsidian_light() -> Self {
        Self {
            bg: Color32::from_rgb(0xfa, 0xfa, 0xf7),
            panel: Color32::from_rgb(0xf0, 0xf0, 0xec),
            border: Color32::from_rgb(0xd6, 0xd6, 0xd0),
            text: Color32::from_rgb(0x21, 0x24, 0x29),
            dim: Color32::from_rgb(0x5e, 0x64, 0x70),
            accent: Color32::from_rgb(0x8b, 0x7c, 0xff),
            accent_ink: Color32::from_rgb(0x0a, 0x0a, 0x0a),
            dark: false,
        }
    }

    /// High-contrast accessibility preset (§1.8) — pure black/white + yellow.
    pub const fn high_contrast() -> Self {
        Self {
            bg: Color32::from_rgb(0x00, 0x00, 0x00),
            panel: Color32::from_rgb(0x0a, 0x0a, 0x0a),
            border: Color32::from_rgb(0x66, 0x66, 0x66),
            text: Color32::from_rgb(0xff, 0xff, 0xff),
            dim: Color32::from_rgb(0xcc, 0xcc, 0xcc),
            accent: Color32::from_rgb(0xff, 0xff, 0x00),
            accent_ink: Color32::from_rgb(0x00, 0x00, 0x00),
            dark: true,
        }
    }

    /// Obsidian dark with a user-chosen accent (settings color picker).
    pub const fn custom(accent: [u8; 3]) -> Self {
        let mut t = Self::obsidian_dark();
        t.accent = Color32::from_rgb(accent[0], accent[1], accent[2]);
        t
    }

    /// Resolve the preset selected in `AppConfig` to a concrete token set.
    pub fn from_preset(preset: ThemePreset, accent: [u8; 3]) -> Self {
        match preset {
            ThemePreset::ObsidianDark => Self::obsidian_dark(),
            ThemePreset::ObsidianLight => Self::obsidian_light(),
            ThemePreset::HighContrast => Self::high_contrast(),
            ThemePreset::Custom => Self::custom(accent),
        }
    }

    /// Bind this token set onto the egui context. Cheap and idempotent.
    pub fn apply(&self, ctx: &egui::Context) {
        let mut visuals = if self.dark {
            egui::Visuals::dark()
        } else {
            egui::Visuals::light()
        };
        visuals.override_text_color = Some(self.text);
        visuals.window_fill = self.bg;
        visuals.panel_fill = self.bg;
        visuals.extreme_bg_color = self.panel;
        visuals.faint_bg_color = self.panel;
        visuals.code_bg_color = self.panel;
        visuals.window_stroke = egui::Stroke::new(1.0, self.border);
        visuals.widgets.noninteractive.bg_stroke = egui::Stroke::new(1.0, self.border);
        visuals.widgets.noninteractive.fg_stroke = egui::Stroke::new(1.0, self.dim);
        visuals.widgets.inactive.bg_fill = self.panel;
        visuals.widgets.inactive.weak_bg_fill = self.panel;
        visuals.widgets.inactive.bg_stroke = egui::Stroke::new(1.0, self.border);
        visuals.widgets.inactive.fg_stroke = egui::Stroke::new(1.0, self.dim);
        visuals.widgets.hovered.bg_fill = self.panel;
        visuals.widgets.hovered.weak_bg_fill = self.panel;
        visuals.widgets.hovered.bg_stroke = egui::Stroke::new(1.0, self.accent);
        visuals.widgets.hovered.fg_stroke = egui::Stroke::new(1.0, self.text);
        visuals.widgets.active.bg_fill = self.accent;
        visuals.widgets.active.weak_bg_fill = self.accent;
        visuals.widgets.active.bg_stroke = egui::Stroke::new(1.0, self.accent);
        visuals.widgets.active.fg_stroke = egui::Stroke::new(1.0, self.accent_ink);
        visuals.selection.bg_fill = self.accent.linear_multiply(0.35);
        visuals.selection.stroke = egui::Stroke::new(1.0, self.accent);
        visuals.hyperlink_color = self.accent;
        ctx.set_visuals(visuals);
    }
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn presets_resolve_distinct_backgrounds() {
        assert_eq!(
            Theme::obsidian_dark().bg,
            Color32::from_rgb(0x1b, 0x1d, 0x22)
        );
        assert_eq!(Theme::high_contrast().bg, Color32::BLACK);
        assert_ne!(Theme::obsidian_dark().bg, Theme::obsidian_light().bg);
    }

    #[test]
    fn custom_overrides_only_accent() {
        let t = Theme::custom([0x5f, 0xb2, 0xf0]);
        assert_eq!(t.accent, Color32::from_rgb(0x5f, 0xb2, 0xf0));
        assert_eq!(t.bg, Theme::obsidian_dark().bg);
    }

    #[test]
    fn from_preset_dispatches() {
        let accent = [1, 2, 3];
        assert_eq!(
            Theme::from_preset(ThemePreset::HighContrast, accent),
            Theme::high_contrast()
        );
        assert_eq!(
            Theme::from_preset(ThemePreset::Custom, accent).accent,
            Color32::from_rgb(1, 2, 3)
        );
    }

    #[test]
    fn legacy_light_user_not_flipped_to_dark() {
        // A v1.0 config (dark_mode=false, no theme_preset) must resolve to the
        // light theme, not the ObsidianDark default. Regression for the
        // disconnected-source-of-truth bug between dark_mode and theme_preset.
        use omninote_core::types::AppConfig;
        let cfg = AppConfig {
            dark_mode: false,
            ..Default::default()
        };
        assert_eq!(crate::app::theme_for_config(&cfg), Theme::obsidian_light());

        let dark_cfg = AppConfig {
            dark_mode: true,
            ..Default::default()
        };
        assert_eq!(
            crate::app::theme_for_config(&dark_cfg),
            Theme::obsidian_dark()
        );

        // An explicit non-default preset wins regardless of dark_mode.
        let hc = AppConfig {
            dark_mode: false,
            theme_preset: ThemePreset::HighContrast,
            ..Default::default()
        };
        assert_eq!(crate::app::theme_for_config(&hc), Theme::high_contrast());
    }

    #[test]
    fn toggle_preserves_high_contrast_and_flips_obsidian() {
        use omninote_core::types::{AppConfig, ThemePreset};

        // Plain obsidian flips dark↔light, keeping both sources of truth in sync.
        let mut cfg = AppConfig {
            dark_mode: true,
            theme_preset: ThemePreset::ObsidianDark,
            ..Default::default()
        };
        crate::app::toggle_light_dark(&mut cfg);
        assert!(!cfg.dark_mode);
        assert_eq!(cfg.theme_preset, ThemePreset::ObsidianLight);

        // High-contrast is an accessibility choice — a dark toggle must not drop it.
        let mut hc = AppConfig {
            theme_preset: ThemePreset::HighContrast,
            ..Default::default()
        };
        crate::app::toggle_light_dark(&mut hc);
        assert_eq!(hc.theme_preset, ThemePreset::HighContrast);
    }
}
