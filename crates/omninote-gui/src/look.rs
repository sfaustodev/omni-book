// "Blueprint" look — a cool drafting-table palette (navy ground, cyan grid rules,
// mono headings) with a light "draft on white" variant and a high-contrast variant.
// Everything derives from `AppConfig`, so the settings dialog drives the whole look.

use egui::{Color32, FontFamily as EguiFamily, FontId, Rounding, Stroke, TextStyle, Visuals};
use omninote_core::types::{AppConfig, FontFamily, ThemePreset};

#[derive(Clone, Copy)]
pub struct Palette {
    pub dark: bool,
    pub bg: Color32,
    pub panel: Color32,
    pub surface: Color32,
    pub ink: Color32,
    pub ink_soft: Color32,
    pub accent: Color32,
    pub grid: Color32, // cyan-navy hairline rules
}

fn rgb(r: u8, g: u8, b: u8) -> Color32 {
    Color32::from_rgb(r, g, b)
}

impl Palette {
    pub fn blueprint(accent: Color32) -> Self {
        Self {
            dark: true,
            bg: rgb(0x0E, 0x1A, 0x2B),
            panel: rgb(0x0A, 0x14, 0x22),
            surface: rgb(0x14, 0x25, 0x3B),
            ink: rgb(0xDC, 0xE8, 0xF2),
            ink_soft: rgb(0x7E, 0x9C, 0xB6),
            accent,
            grid: rgb(0x21, 0x3D, 0x5E),
        }
    }
    pub fn draft(accent: Color32) -> Self {
        Self {
            dark: false,
            bg: rgb(0xF2, 0xF6, 0xFB),
            panel: rgb(0xE6, 0xEE, 0xF6),
            surface: rgb(0xFF, 0xFF, 0xFF),
            ink: rgb(0x0E, 0x22, 0x30),
            ink_soft: rgb(0x4A, 0x60, 0x75),
            accent,
            grid: rgb(0xB8, 0xCC, 0xDD),
        }
    }
    pub fn high_contrast(dark: bool) -> Self {
        if dark {
            Self {
                dark: true,
                bg: Color32::BLACK,
                panel: rgb(0x0A, 0x0A, 0x0A),
                surface: rgb(0x14, 0x14, 0x14),
                ink: Color32::WHITE,
                ink_soft: rgb(0xD0, 0xD0, 0xD0),
                accent: rgb(0x4F, 0xC3, 0xF7),
                grid: rgb(0x80, 0x80, 0x80),
            }
        } else {
            Self {
                dark: false,
                bg: Color32::WHITE,
                panel: rgb(0xF0, 0xF0, 0xF0),
                surface: Color32::WHITE,
                ink: Color32::BLACK,
                ink_soft: rgb(0x30, 0x30, 0x30),
                accent: rgb(0x00, 0x44, 0xCC),
                grid: rgb(0x40, 0x40, 0x40),
            }
        }
    }
}

pub struct Look {
    pub pal: Palette,
}

impl Look {
    pub fn from_config(cfg: &AppConfig) -> Self {
        let accent = Color32::from_rgb(
            cfg.accent_color[0],
            cfg.accent_color[1],
            cfg.accent_color[2],
        );
        let pal = match cfg.theme_preset {
            ThemePreset::HighContrast => Palette::high_contrast(cfg.dark_mode),
            ThemePreset::ObsidianLight => Palette::draft(accent),
            ThemePreset::ObsidianDark => Palette::blueprint(accent),
            ThemePreset::Custom => {
                if cfg.dark_mode {
                    Palette::blueprint(accent)
                } else {
                    Palette::draft(accent)
                }
            }
        };
        Self { pal }
    }

    pub fn apply(&self, ctx: &egui::Context, cfg: &AppConfig) {
        let p = self.pal;
        let mut v = if p.dark {
            Visuals::dark()
        } else {
            Visuals::light()
        };
        v.dark_mode = p.dark;
        v.override_text_color = Some(p.ink);
        v.hyperlink_color = p.accent;
        v.window_fill = p.bg;
        v.panel_fill = p.bg;
        v.faint_bg_color = p.panel;
        v.extreme_bg_color = p.surface;
        v.window_stroke = Stroke::new(1.0, p.grid);
        // Blueprint reads "technical": near-square corners.
        let rounding = Rounding::same(2.0);
        v.window_rounding = rounding;
        v.menu_rounding = rounding;
        v.selection.bg_fill =
            Color32::from_rgba_unmultiplied(p.accent.r(), p.accent.g(), p.accent.b(), 60);
        v.selection.stroke = Stroke::new(1.0, p.accent);

        let set = |w: &mut egui::style::WidgetVisuals, fill: Color32, fg: Color32| {
            w.bg_fill = fill;
            w.weak_bg_fill = fill;
            w.bg_stroke = Stroke::new(1.0, p.grid);
            w.fg_stroke = Stroke::new(1.0, fg);
            w.rounding = rounding;
        };
        set(&mut v.widgets.noninteractive, p.panel, p.ink_soft);
        set(&mut v.widgets.inactive, p.surface, p.ink);
        set(&mut v.widgets.hovered, p.surface, p.ink);
        set(
            &mut v.widgets.active,
            p.accent.gamma_multiply(if p.dark { 0.5 } else { 0.85 }),
            p.ink,
        );
        set(&mut v.widgets.open, p.surface, p.ink);
        v.widgets.hovered.bg_stroke = Stroke::new(1.0, p.accent);

        ctx.set_visuals(v);

        // Headings + metadata in monospace for the drafting feel; body honors the
        // accessibility font family.
        let body_fam = font_family_to_egui(cfg.font_family);
        let sz = cfg.font_size.clamp(9.0, 30.0);
        ctx.style_mut(|s| {
            s.text_styles.insert(
                TextStyle::Heading,
                FontId::new((sz * 1.55).round(), EguiFamily::Monospace),
            );
            s.text_styles
                .insert(TextStyle::Body, FontId::new(sz, body_fam.clone()));
            s.text_styles.insert(
                TextStyle::Button,
                FontId::new((sz * 0.95).round(), EguiFamily::Monospace),
            );
            s.text_styles.insert(
                TextStyle::Small,
                FontId::new((sz * 0.82).round(), EguiFamily::Monospace),
            );
            s.text_styles
                .insert(TextStyle::Monospace, FontId::new(sz, EguiFamily::Monospace));
            s.spacing.item_spacing = egui::vec2(8.0, (cfg.line_height * 6.0).clamp(4.0, 16.0));
            s.spacing.button_padding = egui::vec2(8.0, 5.0);
        });
    }
}

/// Map the core `FontFamily` to an egui family. Serif/Dyslexic fall back to
/// Proportional (typeface files not bundled); size + spacing + the mono switch
/// still give a real a11y effect.
pub fn font_family_to_egui(f: FontFamily) -> EguiFamily {
    match f {
        FontFamily::Monospace => EguiFamily::Monospace,
        _ => EguiFamily::Proportional,
    }
}

/// Editor text layouter honoring accessibility line-height + letter-spacing, so
/// those settings visibly affect the main writing surface.
pub fn body_layout(
    ctx: &egui::Context,
    cfg: &AppConfig,
    text: &str,
    wrap_width: f32,
) -> std::sync::Arc<egui::Galley> {
    use egui::text::{LayoutJob, TextFormat};
    let fam = font_family_to_egui(cfg.font_family);
    let mut job = LayoutJob::default();
    job.wrap.max_width = wrap_width;
    let mut fmt = TextFormat {
        font_id: FontId::new(cfg.font_size.clamp(9.0, 30.0), fam),
        color: ctx.style().visuals.text_color(),
        ..Default::default()
    };
    fmt.extra_letter_spacing = cfg.letter_spacing.clamp(0.0, 6.0);
    fmt.line_height = Some((cfg.font_size * cfg.line_height).clamp(10.0, 60.0));
    job.append(text, 0.0, fmt);
    ctx.fonts(|f| f.layout_job(job))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn preset_drives_darkness() {
        let mut c = AppConfig {
            theme_preset: ThemePreset::ObsidianDark,
            ..Default::default()
        };
        assert!(Look::from_config(&c).pal.dark);
        c.theme_preset = ThemePreset::ObsidianLight;
        assert!(!Look::from_config(&c).pal.dark);
    }

    #[test]
    fn mono_family_maps_through() {
        assert_eq!(
            font_family_to_egui(FontFamily::Monospace),
            EguiFamily::Monospace
        );
        assert_eq!(
            font_family_to_egui(FontFamily::System),
            EguiFamily::Proportional
        );
    }
}
