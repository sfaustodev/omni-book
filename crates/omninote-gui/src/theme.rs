// "Almanac" theme — a warm editorial palette (parchment + oxblood) with a night
// variant and a high-contrast variant. All colors derive from `AppConfig`, so the
// settings panel (accent picker, dark toggle, preset) drives the whole look.

use egui::{Color32, FontFamily as EguiFamily, FontId, Rounding, Stroke, TextStyle, Visuals};
use omninote_core::types::{AppConfig, FontFamily, ThemePreset};

#[derive(Clone, Copy)]
pub struct Palette {
    pub dark: bool,
    pub bg: Color32,       // app background (parchment / night)
    pub panel: Color32,    // sidebar / rails
    pub surface: Color32,  // cards, text fields
    pub ink: Color32,      // primary text
    pub ink_soft: Color32, // secondary text
    pub accent: Color32,   // oxblood / amber
    pub rule: Color32,     // hairline rules
}

fn rgb(r: u8, g: u8, b: u8) -> Color32 {
    Color32::from_rgb(r, g, b)
}

impl Palette {
    pub fn light(accent: Color32) -> Self {
        Self {
            dark: false,
            bg: rgb(0xEF, 0xE7, 0xD3),
            panel: rgb(0xE7, 0xDC, 0xC2),
            surface: rgb(0xF6, 0xF0, 0xE0),
            ink: rgb(0x2A, 0x26, 0x20),
            ink_soft: rgb(0x6B, 0x62, 0x53),
            accent,
            rule: rgb(0xCD, 0xBE, 0x9E),
        }
    }
    pub fn dark(accent: Color32) -> Self {
        Self {
            dark: true,
            bg: rgb(0x1B, 0x18, 0x13),
            panel: rgb(0x22, 0x1E, 0x18),
            surface: rgb(0x29, 0x24, 0x1C),
            ink: rgb(0xEC, 0xE2, 0xCD),
            ink_soft: rgb(0xA8, 0x9B, 0x82),
            accent,
            rule: rgb(0x3A, 0x33, 0x28),
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
                accent: rgb(0xFF, 0xD7, 0x00),
                rule: rgb(0x80, 0x80, 0x80),
            }
        } else {
            Self {
                dark: false,
                bg: Color32::WHITE,
                panel: rgb(0xF0, 0xF0, 0xF0),
                surface: Color32::WHITE,
                ink: Color32::BLACK,
                ink_soft: rgb(0x30, 0x30, 0x30),
                accent: rgb(0x00, 0x33, 0xCC),
                rule: rgb(0x40, 0x40, 0x40),
            }
        }
    }
}

pub struct Theme {
    pub pal: Palette,
}

impl Theme {
    pub fn from_config(cfg: &AppConfig) -> Self {
        let accent = Color32::from_rgb(
            cfg.accent_color[0],
            cfg.accent_color[1],
            cfg.accent_color[2],
        );
        let pal = match cfg.theme_preset {
            ThemePreset::HighContrast => Palette::high_contrast(cfg.dark_mode),
            ThemePreset::ObsidianLight => Palette::light(accent),
            ThemePreset::ObsidianDark => Palette::dark(accent),
            // Presets herdados da galeria de temas da main sem paleta própria
            // neste rebuild caem no par light/dark padrão.
            _ => {
                if cfg.dark_mode {
                    Palette::dark(accent)
                } else {
                    Palette::light(accent)
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
        v.window_stroke = Stroke::new(1.0_f32, p.rule);
        let rounding = Rounding::same(7.0);
        v.window_rounding = rounding;
        v.menu_rounding = rounding;
        v.selection.bg_fill =
            Color32::from_rgba_unmultiplied(p.accent.r(), p.accent.g(), p.accent.b(), 64);
        v.selection.stroke = Stroke::new(1.0_f32, p.accent);

        let set = |w: &mut egui::style::WidgetVisuals, fill: Color32, fg: Color32| {
            w.bg_fill = fill;
            w.weak_bg_fill = fill;
            w.bg_stroke = Stroke::new(1.0_f32, p.rule);
            w.fg_stroke = Stroke::new(1.0_f32, fg);
            w.rounding = rounding;
        };
        set(&mut v.widgets.noninteractive, p.panel, p.ink_soft);
        set(&mut v.widgets.inactive, p.surface, p.ink);
        set(&mut v.widgets.hovered, p.panel, p.ink);
        set(
            &mut v.widgets.active,
            p.accent.gamma_multiply(if p.dark { 0.55 } else { 0.85 }),
            p.ink,
        );
        set(&mut v.widgets.open, p.surface, p.ink);
        v.widgets.hovered.bg_stroke = Stroke::new(1.0_f32, p.accent);

        ctx.set_visuals(v);

        let fam = font_family_to_egui(cfg.font_family);
        let sz = cfg.font_size.clamp(9.0, 30.0);
        ctx.style_mut(|s| {
            s.text_styles.insert(
                TextStyle::Heading,
                FontId::new((sz * 1.7).round(), fam.clone()),
            );
            s.text_styles
                .insert(TextStyle::Body, FontId::new(sz, fam.clone()));
            s.text_styles
                .insert(TextStyle::Button, FontId::new(sz, fam.clone()));
            s.text_styles.insert(
                TextStyle::Small,
                FontId::new((sz * 0.82).round(), fam.clone()),
            );
            s.text_styles.insert(
                TextStyle::Monospace,
                FontId::new((sz * 0.94).round(), EguiFamily::Monospace),
            );
            s.spacing.item_spacing = egui::vec2(8.0, (cfg.line_height * 6.0).clamp(4.0, 16.0));
            s.spacing.button_padding = egui::vec2(9.0, 5.0);
        });
    }
}

/// Map the core `FontFamily` enum to an egui family. Serif/Dyslexic fall back to
/// Proportional — bundling those typeface files is out of scope for this build;
/// size + spacing + the mono/proportional switch still give a real a11y effect.
pub fn font_family_to_egui(f: FontFamily) -> EguiFamily {
    match f {
        FontFamily::Monospace => EguiFamily::Monospace,
        _ => EguiFamily::Proportional,
    }
}

/// Lay out editor text honoring the accessibility line-height + letter-spacing.
/// Used as the `TextEdit::layouter` so those settings have a real visible effect
/// on the main writing surface, not just in theory.
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
    fn preset_drives_palette_darkness() {
        let mut c = AppConfig {
            theme_preset: ThemePreset::ObsidianDark,
            ..Default::default()
        };
        assert!(Theme::from_config(&c).pal.dark);
        c.theme_preset = ThemePreset::ObsidianLight;
        assert!(!Theme::from_config(&c).pal.dark);
    }

    #[test]
    fn custom_preset_follows_dark_mode_flag() {
        let c = AppConfig {
            theme_preset: ThemePreset::Custom,
            dark_mode: true,
            ..Default::default()
        };
        assert!(Theme::from_config(&c).pal.dark);
    }

    #[test]
    fn mono_family_maps_to_monospace() {
        assert_eq!(
            font_family_to_egui(FontFamily::Monospace),
            EguiFamily::Monospace
        );
        assert_eq!(
            font_family_to_egui(FontFamily::Serif),
            EguiFamily::Proportional
        );
    }
}
