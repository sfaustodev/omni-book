//! "Almanac" visual identity — a warm editorial palette (Parchment light / Ink dark)
//! with a single terracotta accent, hairline rules, and low rounding. Deliberately
//! distinct from generic gray+blue tool chrome.

use egui::{Color32, Rounding, Stroke, Visuals};
use omninote_core::types::NoteType;

#[derive(Clone, Copy)]
pub struct Theme {
    pub dark: bool,
    pub bg: Color32,
    pub panel: Color32,
    pub panel_alt: Color32,
    pub border: Color32,
    pub text: Color32,
    pub dim: Color32,
    pub faint: Color32,
    pub accent: Color32,
    pub accent_soft: Color32,
    pub on_accent: Color32,
}

impl Theme {
    pub fn parchment() -> Self {
        Self {
            dark: false,
            bg: Color32::from_rgb(0xf4, 0xec, 0xdd),
            panel: Color32::from_rgb(0xef, 0xe6, 0xd4),
            panel_alt: Color32::from_rgb(0xe7, 0xdc, 0xc6),
            border: Color32::from_rgb(0xd6, 0xc6, 0xa9),
            text: Color32::from_rgb(0x2b, 0x26, 0x20),
            dim: Color32::from_rgb(0x6b, 0x63, 0x53),
            faint: Color32::from_rgb(0x9a, 0x91, 0x82),
            accent: Color32::from_rgb(0xbf, 0x4d, 0x26),
            accent_soft: Color32::from_rgb(0xe8, 0xcf, 0xbf),
            on_accent: Color32::from_rgb(0xf7, 0xef, 0xe2),
        }
    }

    pub fn ink() -> Self {
        Self {
            dark: true,
            bg: Color32::from_rgb(0x1a, 0x17, 0x14),
            panel: Color32::from_rgb(0x22, 0x1d, 0x18),
            panel_alt: Color32::from_rgb(0x2c, 0x25, 0x1e),
            border: Color32::from_rgb(0x3b, 0x32, 0x2a),
            text: Color32::from_rgb(0xec, 0xe3, 0xd2),
            dim: Color32::from_rgb(0xb0, 0xa4, 0x8f),
            faint: Color32::from_rgb(0x7d, 0x73, 0x63),
            accent: Color32::from_rgb(0xe0, 0x71, 0x2f),
            accent_soft: Color32::from_rgb(0x3a, 0x2a, 0x1f),
            on_accent: Color32::from_rgb(0x1a, 0x17, 0x14),
        }
    }

    pub fn from_dark(dark: bool) -> Self {
        if dark {
            Self::ink()
        } else {
            Self::parchment()
        }
    }

    /// Per-category hue, kept legible on both backgrounds.
    pub fn note_type_color(&self, t: NoteType) -> Color32 {
        match t {
            NoteType::Resumo => Color32::from_rgb(0x4f, 0x8a, 0xc7),
            NoteType::Citacao => Color32::from_rgb(0x9a, 0x6c, 0xc4),
            NoteType::Codigo => Color32::from_rgb(0x4f, 0xa3, 0x6b),
            NoteType::Exercicio => Color32::from_rgb(0xd0, 0x96, 0x2f),
            NoteType::Duvida => Color32::from_rgb(0xcb, 0x5b, 0x4c),
            NoteType::Definicao => Color32::from_rgb(0x3f, 0x9d, 0x9a),
        }
    }

    /// Build the full egui visuals for this palette.
    pub fn visuals(&self) -> Visuals {
        let mut v = if self.dark {
            Visuals::dark()
        } else {
            Visuals::light()
        };
        let rounding = Rounding::same(2.0);

        v.override_text_color = Some(self.text);
        v.hyperlink_color = self.accent;
        v.panel_fill = self.panel;
        v.window_fill = self.panel;
        v.window_stroke = Stroke::new(1.0, self.border);
        v.window_rounding = Rounding::same(4.0);
        v.extreme_bg_color = self.bg;
        v.faint_bg_color = self.panel_alt;
        v.code_bg_color = self.panel_alt;
        v.warn_fg_color = self.accent;
        v.error_fg_color = Color32::from_rgb(0xcb, 0x5b, 0x4c);

        v.widgets.noninteractive.bg_fill = self.panel;
        v.widgets.noninteractive.weak_bg_fill = self.panel;
        v.widgets.noninteractive.bg_stroke = Stroke::new(1.0, self.border);
        v.widgets.noninteractive.fg_stroke = Stroke::new(1.0, self.dim);
        v.widgets.noninteractive.rounding = rounding;

        v.widgets.inactive.bg_fill = self.panel_alt;
        v.widgets.inactive.weak_bg_fill = self.panel_alt;
        v.widgets.inactive.bg_stroke = Stroke::new(1.0, self.border);
        v.widgets.inactive.fg_stroke = Stroke::new(1.0, self.text);
        v.widgets.inactive.rounding = rounding;

        v.widgets.hovered.bg_fill = self.accent_soft;
        v.widgets.hovered.weak_bg_fill = self.accent_soft;
        v.widgets.hovered.bg_stroke = Stroke::new(1.0, self.accent);
        v.widgets.hovered.fg_stroke = Stroke::new(1.0, self.text);
        v.widgets.hovered.rounding = rounding;

        v.widgets.active.bg_fill = self.accent;
        v.widgets.active.weak_bg_fill = self.accent;
        v.widgets.active.bg_stroke = Stroke::new(1.0, self.accent);
        v.widgets.active.fg_stroke = Stroke::new(1.0, self.on_accent);
        v.widgets.active.rounding = rounding;

        v.selection.bg_fill = self.accent_soft;
        v.selection.stroke = Stroke::new(1.0, self.accent);

        v
    }
}
