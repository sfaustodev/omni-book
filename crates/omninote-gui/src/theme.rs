//! egui theme — the "Terminal / Mechanical" palette as a `Theme` token set with
//! Dark / Light / HighContrast presets (plus a custom-accent variant), and the
//! bridge from the portable `omninote_core::types::FontFamily` enum to
//! `egui::FontFamily`. Lives in the gui crate so `omninote-core` stays UI-free.
//!
//! Design intent: every egui style parameter is a deliberate choice — flat fills,
//! 1px hairline borders, zero rounding, zero shadow, no growth-on-hover. The real
//! selection signal in list rows is a `>` marker + an accent left-bar (later
//! phases), so the row washes here are intentionally faint. `Theme::apply` sets
//! the FULL `Visuals`/`Spacing`/`Interaction`/`Style` surface — no egui default is
//! left in place (asserted by `no_egui_defaults_remain`).
//!
//! Every `ui_*.rs` pulls colors from a `Theme`, so switching preset is a single
//! re-bind via `Theme::from_preset(..).apply(ctx)` (or the app's
//! `theme_for_config`, which also honours the legacy `dark_mode` flag).

use eframe::egui::{self, Color32};
use omninote_core::types::{FontFamily, NoteType, ThemePreset};

/// Identifier the OpenDyslexic face is registered under in `FontDefinitions`.
pub const OPEN_DYSLEXIC_NAME: &str = "OpenDyslexic";
/// Family name the Space Grotesk display face (headings) is registered under.
pub const SPACE_GROTESK_NAME: &str = "SpaceGrotesk";

/// Full token set for one theme variant. `dark` is carried explicitly rather
/// than inferred from a colour channel — a single-channel heuristic misclassifies
/// non-greyscale backgrounds (bright green r=0 reads "dark", dark red r>128 reads
/// "light"), which would pick the wrong egui base visuals.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Theme {
    pub bg: Color32,
    pub panel: Color32,
    /// Slightly raised panel surface (alt rows, nested cards, scrollbar handle).
    pub panel_alt: Color32,
    pub border: Color32,
    /// Heavier hairline for separators that need to read against `border`.
    pub border_strong: Color32,
    pub text: Color32,
    pub dim: Color32,
    /// The faintest legible ink — disabled affordances, placeholder text.
    pub faint: Color32,
    pub accent: Color32,
    /// Text/ink drawn on top of an accent fill.
    pub accent_ink: Color32,
    /// Whether this variant uses egui's dark base visuals.
    pub dark: bool,
}

impl Theme {
    /// Matrix terminal — the primary variant. A black void with phosphor-green
    /// text and dim-green hairline borders; bright-green for highlights/cursor.
    /// Green-on-black is the whole identity — grey chrome read as Windows 98.
    pub const fn obsidian_dark() -> Self {
        Self {
            bg: Color32::from_rgb(0x01, 0x06, 0x04),
            panel: Color32::from_rgb(0x04, 0x12, 0x0b),
            panel_alt: Color32::from_rgb(0x08, 0x20, 0x0f),
            border: Color32::from_rgb(0x0e, 0x3a, 0x22),
            border_strong: Color32::from_rgb(0x1d, 0x7a, 0x45),
            text: Color32::from_rgb(0x3c, 0xe0, 0x6f),
            dim: Color32::from_rgb(0x1f, 0x9b, 0x4d),
            faint: Color32::from_rgb(0x13, 0x5c, 0x2e),
            accent: Color32::from_rgb(0x6b, 0xff, 0x9a),
            accent_ink: Color32::from_rgb(0x00, 0x16, 0x09),
            dark: true,
        }
    }

    /// Terminal light — warm paper canvas keeping a darker green accent for
    /// contrast on a bright background.
    pub const fn obsidian_light() -> Self {
        Self {
            bg: Color32::from_rgb(0xec, 0xf1, 0xed),
            panel: Color32::from_rgb(0xdd, 0xe6, 0xe0),
            panel_alt: Color32::from_rgb(0xd0, 0xdb, 0xd4),
            border: Color32::from_rgb(0xb6, 0xc6, 0xbb),
            border_strong: Color32::from_rgb(0x8d, 0xa1, 0x95),
            text: Color32::from_rgb(0x0e, 0x17, 0x12),
            dim: Color32::from_rgb(0x4e, 0x60, 0x57),
            faint: Color32::from_rgb(0x86, 0x99, 0x8e),
            accent: Color32::from_rgb(0x0e, 0x8c, 0x42),
            accent_ink: Color32::from_rgb(0xff, 0xff, 0xff),
            dark: false,
        }
    }

    /// High-contrast accessibility preset (§1.8) — pure black/white + pure green.
    /// `panel == bg` so there is no low-contrast layering; separation comes from
    /// the white border. Row/selection helpers must fall back to STROKE-only
    /// feedback here (a translucent fill is invisible on black).
    pub const fn high_contrast() -> Self {
        Self {
            bg: Color32::from_rgb(0x00, 0x00, 0x00),
            panel: Color32::from_rgb(0x00, 0x00, 0x00),
            panel_alt: Color32::from_rgb(0x0a, 0x0a, 0x0a),
            border: Color32::from_rgb(0xff, 0xff, 0xff),
            border_strong: Color32::from_rgb(0xff, 0xff, 0xff),
            text: Color32::from_rgb(0xff, 0xff, 0xff),
            dim: Color32::from_rgb(0xcc, 0xcc, 0xcc),
            faint: Color32::from_rgb(0xcc, 0xcc, 0xcc),
            accent: Color32::from_rgb(0x00, 0xff, 0x00),
            accent_ink: Color32::from_rgb(0x00, 0x00, 0x00),
            dark: true,
        }
    }

    /// Terminal dark with a user-chosen accent (settings color picker).
    pub const fn custom(accent: [u8; 3]) -> Self {
        let mut t = Self::obsidian_dark();
        t.accent = Color32::from_rgb(accent[0], accent[1], accent[2]);
        t
    }

    /// "Almanac" — warm parchment + terracotta editorial, recovered from the
    /// `cortex/off-1` design experiment's `Palette::light` (independently
    /// reconfirmed by `cortex/off-3`'s blind rebuild converging on the same
    /// identity). The primary/signature Almanac variant.
    pub const fn almanac_light() -> Self {
        Self {
            bg: Color32::from_rgb(0xEF, 0xE7, 0xD3),
            panel: Color32::from_rgb(0xE7, 0xDC, 0xC2),
            panel_alt: Color32::from_rgb(0xF6, 0xF0, 0xE0),
            border: Color32::from_rgb(0xCD, 0xBE, 0x9E),
            border_strong: Color32::from_rgb(0x9C, 0x8B, 0x6A),
            text: Color32::from_rgb(0x2A, 0x26, 0x20),
            dim: Color32::from_rgb(0x6B, 0x62, 0x53),
            faint: Color32::from_rgb(0x9A, 0x8F, 0x79),
            accent: Color32::from_rgb(0xBF, 0x4D, 0x26),
            accent_ink: Color32::from_rgb(0xFA, 0xF3, 0xE6),
            dark: false,
        }
    }

    /// "Almanac Noite" — the night variant from `cortex/off-1`'s `Palette::dark`.
    /// Same terracotta accent as [`Self::almanac_light`] for brand continuity.
    pub const fn almanac_dark() -> Self {
        Self {
            bg: Color32::from_rgb(0x1B, 0x18, 0x13),
            panel: Color32::from_rgb(0x22, 0x1E, 0x18),
            panel_alt: Color32::from_rgb(0x29, 0x24, 0x1C),
            border: Color32::from_rgb(0x3A, 0x33, 0x28),
            border_strong: Color32::from_rgb(0x7A, 0x70, 0x5C),
            text: Color32::from_rgb(0xEC, 0xE2, 0xCD),
            dim: Color32::from_rgb(0xA8, 0x9B, 0x82),
            faint: Color32::from_rgb(0x5C, 0x53, 0x43),
            accent: Color32::from_rgb(0xBF, 0x4D, 0x26),
            accent_ink: Color32::from_rgb(0xFA, 0xF3, 0xE6),
            dark: true,
        }
    }

    /// "Blueprint" — cool navy/cyan drafting-table palette, recovered from the
    /// `cortex/off-2` design experiment's `Palette::blueprint`. The cyan accent
    /// is lifted from off-2's own high-contrast variant, the only place it
    /// hardcoded a concrete "cyan" (the normal variant parameterized accent via
    /// `AppConfig`). The primary/signature Blueprint variant.
    pub const fn blueprint() -> Self {
        Self {
            bg: Color32::from_rgb(0x0E, 0x1A, 0x2B),
            panel: Color32::from_rgb(0x0A, 0x14, 0x22),
            panel_alt: Color32::from_rgb(0x14, 0x25, 0x3B),
            border: Color32::from_rgb(0x21, 0x3D, 0x5E),
            border_strong: Color32::from_rgb(0x4A, 0x6E, 0x90),
            text: Color32::from_rgb(0xDC, 0xE8, 0xF2),
            dim: Color32::from_rgb(0x7E, 0x9C, 0xB6),
            faint: Color32::from_rgb(0x4E, 0x6B, 0x85),
            accent: Color32::from_rgb(0x4F, 0xC3, 0xF7),
            accent_ink: Color32::from_rgb(0x0A, 0x14, 0x22),
            dark: true,
        }
    }

    /// "Blueprint Rascunho" — the light "draft on white" variant from
    /// `cortex/off-2`'s `Palette::draft`. Uses a deeper technical blue accent
    /// than [`Self::blueprint`]'s cyan, which reads washed-out on a white ground.
    pub const fn blueprint_light() -> Self {
        Self {
            bg: Color32::from_rgb(0xF2, 0xF6, 0xFB),
            panel: Color32::from_rgb(0xE6, 0xEE, 0xF6),
            panel_alt: Color32::from_rgb(0xFF, 0xFF, 0xFF),
            border: Color32::from_rgb(0xB8, 0xCC, 0xDD),
            border_strong: Color32::from_rgb(0x74, 0x88, 0x98),
            text: Color32::from_rgb(0x0E, 0x22, 0x30),
            dim: Color32::from_rgb(0x4A, 0x60, 0x75),
            faint: Color32::from_rgb(0x8C, 0xA0, 0xB0),
            accent: Color32::from_rgb(0x00, 0x6D, 0xA6),
            accent_ink: Color32::from_rgb(0xFF, 0xFF, 0xFF),
            dark: false,
        }
    }

    /// "Swiss" — Bauhaus/International-Typographic-Style black + warm orange,
    /// recovered from the `omninote-swiss-theme` stash (a Claude-Design HTML
    /// handoff, never committed). Dark-only — no light variant was ever built.
    pub const fn swiss() -> Self {
        Self {
            bg: Color32::from_rgb(0x0E, 0x0E, 0x0E),
            panel: Color32::from_rgb(0x14, 0x14, 0x14),
            panel_alt: Color32::from_rgb(0x1C, 0x1C, 0x1C),
            border: Color32::from_rgb(0x26, 0x26, 0x26),
            border_strong: Color32::from_rgb(0x3A, 0x3A, 0x3A),
            text: Color32::from_rgb(0xFA, 0xFA, 0xFA),
            dim: Color32::from_rgb(0x8A, 0x8A, 0x8A),
            faint: Color32::from_rgb(0x5A, 0x5A, 0x5A),
            accent: Color32::from_rgb(0xFF, 0x5A, 0x1F),
            accent_ink: Color32::from_rgb(0x00, 0x00, 0x00),
            dark: true,
        }
    }

    /// The accent at an explicit alpha. Single helper so every translucent accent
    /// wash (row hover/selection, text selection) derives from one place instead
    /// of scattered `from_rgba_unmultiplied` literals.
    pub fn accent_alpha(&self, a: u8) -> Color32 {
        let c = self.accent;
        Color32::from_rgba_unmultiplied(c.r(), c.g(), c.b(), a)
    }

    /// Translucent accent wash for a hovered list row (paints over the panel).
    /// Subtle on purpose. The Terminal `clickable_row` no longer fills on hover —
    /// it brightens the `>` prompt marker instead — so this token is currently
    /// unused; kept as part of the palette for any non-Terminal hover surface.
    #[allow(dead_code)]
    pub fn row_hover(&self) -> Color32 {
        self.accent_alpha(26)
    }

    /// Stronger accent wash for the selected/active list row. Still restrained —
    /// the loud selection signal (a `>` marker + accent left-bar) lands in later
    /// phases; this only tints the background.
    pub fn row_selected(&self) -> Color32 {
        self.accent_alpha(56)
    }

    /// True for the pure-black high-contrast preset, where a translucent row wash
    /// is invisible — callers draw a solid accent outline instead. WCAG (§1.8).
    pub fn is_high_contrast(&self) -> bool {
        self.bg == Color32::BLACK
    }

    /// Semantic status semaphores (sprint/ticket lifecycle). These are fixed
    /// content hues — green=done, amber=doing, red=blocked, slate=todo — not
    /// themeable chrome, so they live as named accessors here rather than
    /// scattered literals (mirrors the success/error toast colors). Under
    /// high-contrast they fold to the accent so the WCAG preset stays legible.
    pub fn status_done(&self) -> Color32 {
        if self.is_high_contrast() {
            self.accent
        } else {
            Color32::from_rgb(0x4a, 0xde, 0x80)
        }
    }
    pub fn status_doing(&self) -> Color32 {
        if self.is_high_contrast() {
            self.accent
        } else {
            Color32::from_rgb(0xfb, 0xbf, 0x24)
        }
    }
    pub fn status_blocked(&self) -> Color32 {
        if self.is_high_contrast() {
            self.accent
        } else {
            Color32::from_rgb(0xf8, 0x71, 0x71)
        }
    }
    /// Resting / not-started status. Folds to `dim` under high-contrast (a muted
    /// grey on pure black would be a weak "neutral" — the theme's `dim` is the
    /// legible one there).
    pub fn status_todo(&self) -> Color32 {
        if self.is_high_contrast() {
            self.dim
        } else {
            Color32::from_rgb(0x6b, 0x75, 0x85)
        }
    }
    /// Tag color for a Jira-origin ticket row (Notion uses `accent`).
    pub fn provider_jira(&self) -> Color32 {
        if self.is_high_contrast() {
            self.accent
        } else {
            Color32::from_rgb(0x38, 0xbd, 0xf8)
        }
    }

    /// Per-`NoteType` content hue (badge/chip ink). Fixed semantic colors, folded
    /// to the accent under high-contrast so the WCAG preset stays single-hue.
    pub fn note_type_color(&self, ty: NoteType) -> Color32 {
        if self.is_high_contrast() {
            return self.accent;
        }
        match ty {
            NoteType::Resumo => Color32::from_rgb(0x38, 0xbd, 0xf8),
            NoteType::Citacao => Color32::from_rgb(0xe8, 0x79, 0xf9),
            NoteType::Codigo => Color32::from_rgb(0x4a, 0xde, 0x80),
            NoteType::Exercicio => Color32::from_rgb(0xfb, 0xbf, 0x24),
            NoteType::Duvida => Color32::from_rgb(0xf8, 0x71, 0x71),
            NoteType::Definicao => Color32::from_rgb(0xa7, 0x8b, 0xfa),
        }
    }

    /// Resolve the preset selected in `AppConfig` to a concrete token set.
    pub fn from_preset(preset: ThemePreset, accent: [u8; 3]) -> Self {
        match preset {
            ThemePreset::ObsidianDark => Self::obsidian_dark(),
            ThemePreset::ObsidianLight => Self::obsidian_light(),
            ThemePreset::HighContrast => Self::high_contrast(),
            ThemePreset::Custom => Self::custom(accent),
            ThemePreset::AlmanacLight => Self::almanac_light(),
            ThemePreset::AlmanacDark => Self::almanac_dark(),
            ThemePreset::Blueprint => Self::blueprint(),
            ThemePreset::BlueprintLight => Self::blueprint_light(),
            ThemePreset::Swiss => Self::swiss(),
        }
    }

    /// Bind this token set onto the egui context. Cheap and idempotent.
    ///
    /// Every field of `Visuals`, `Spacing`, and `Interaction` is set from the
    /// token set or a deliberate Terminal/Mechanical constant — egui's defaults
    /// are not relied on for any visible parameter. Typography (the `text_styles`
    /// map and the accessibility `item_spacing.y`) is owned by
    /// `OmniNoteApp::apply_style`, which runs after this.
    pub fn apply(&self, ctx: &egui::Context) {
        let hairline = egui::Stroke::new(1.0, self.border);
        let none = egui::Rounding::ZERO;

        let widget = |fill: Color32, weak: Color32, stroke: egui::Stroke, fg: Color32| {
            egui::style::WidgetVisuals {
                bg_fill: fill,
                weak_bg_fill: weak,
                bg_stroke: stroke,
                rounding: none,
                fg_stroke: egui::Stroke::new(1.0, fg),
                expansion: 0.0,
            }
        };

        // Borderless by default — buttons/inputs/labels are green text on the void,
        // not framed boxes (boxy 1px-outline chrome read as Windows 98). Feedback is
        // text brightening + a faint phosphor wash on touch, never an outline rect.
        let no_stroke = egui::Stroke::NONE;
        let widgets = egui::style::Widgets {
            // Labels / window backgrounds: no fill, no outline, dim ink.
            noninteractive: widget(
                Color32::TRANSPARENT,
                Color32::TRANSPARENT,
                no_stroke,
                self.dim,
            ),
            // Resting buttons/inputs: invisible until touched — no fill, no border.
            inactive: widget(
                Color32::TRANSPARENT,
                Color32::TRANSPARENT,
                no_stroke,
                self.text,
            ),
            // Hover: a faint phosphor wash + brightened accent text, no outline.
            hovered: widget(
                self.accent_alpha(20),
                self.accent_alpha(20),
                no_stroke,
                self.accent,
            ),
            // Active/pressed: a brief solid phosphor block.
            active: widget(self.accent, self.accent, no_stroke, self.accent_ink),
            // Open combo/menu owner: faint wash, accent text, no outline.
            open: widget(
                self.accent_alpha(28),
                self.accent_alpha(28),
                no_stroke,
                self.text,
            ),
        };

        let visuals = egui::Visuals {
            dark_mode: self.dark,
            override_text_color: Some(self.text),
            widgets,
            selection: egui::style::Selection {
                bg_fill: self.accent_alpha(56),
                stroke: egui::Stroke::new(1.0, self.accent),
            },
            hyperlink_color: self.accent,
            faint_bg_color: self.panel_alt,
            extreme_bg_color: self.panel,
            code_bg_color: self.panel_alt,
            warn_fg_color: self.status_doing(),
            error_fg_color: self.status_blocked(),
            window_rounding: none,
            window_shadow: egui::epaint::Shadow::NONE,
            window_fill: self.panel,
            window_stroke: hairline,
            window_highlight_topmost: false,
            menu_rounding: none,
            panel_fill: self.bg,
            popup_shadow: egui::epaint::Shadow::NONE,
            resize_corner_size: 10.0,
            text_cursor: egui::style::TextCursorStyle {
                stroke: egui::Stroke::new(1.5, self.accent),
                preview: false,
                blink: true,
                on_duration: 0.5,
                off_duration: 0.5,
            },
            clip_rect_margin: 3.0,
            // No frames anywhere — buttons are bare green text (a frame box reads as
            // Windows 98); collapsing headers and indents stay flat and lineless.
            button_frame: false,
            collapsing_header_frame: false,
            indent_has_left_vline: false,
            striped: false,
            slider_trailing_fill: true,
            handle_shape: egui::style::HandleShape::Rect { aspect_ratio: 0.5 },
            interact_cursor: Some(egui::CursorIcon::PointingHand),
            image_loading_spinners: true,
            numeric_color_space: egui::style::NumericColorSpace::GammaByte,
        };

        // ScrollStyle: floating bars that overlay the content (no reserved chrome
        // gutter — a permanent bar would box the scroll region against the void),
        // deliberately narrow, fading in on hover/drag.
        let scroll = egui::style::ScrollStyle {
            floating: true,
            bar_width: 10.0,
            handle_min_length: 16.0,
            bar_inner_margin: 2.0,
            bar_outer_margin: 0.0,
            floating_width: 2.0,
            floating_allocated_width: 0.0,
            foreground_color: false,
            dormant_background_opacity: 0.0,
            active_background_opacity: 0.4,
            interact_background_opacity: 0.7,
            dormant_handle_opacity: 0.4,
            active_handle_opacity: 0.7,
            interact_handle_opacity: 1.0,
        };

        let spacing = egui::style::Spacing {
            item_spacing: egui::vec2(6.0, 4.0),
            window_margin: egui::Margin::same(8.0),
            button_padding: egui::vec2(8.0, 4.0),
            menu_margin: egui::Margin::same(4.0),
            indent: 14.0,
            interact_size: egui::vec2(40.0, 20.0),
            slider_width: 140.0,
            slider_rail_height: 6.0,
            combo_width: 100.0,
            text_edit_width: 280.0,
            icon_width: 14.0,
            icon_width_inner: 8.0,
            icon_spacing: 6.0,
            default_area_size: egui::vec2(600.0, 400.0),
            tooltip_width: 480.0,
            menu_width: 400.0,
            menu_spacing: 2.0,
            indent_ends_with_horizontal_line: false,
            combo_height: 240.0,
            scroll,
        };

        let interaction = egui::style::Interaction {
            interact_radius: 5.0,
            resize_grab_radius_side: 4.0,
            resize_grab_radius_corner: 8.0,
            show_tooltips_only_when_still: true,
            tooltip_delay: 0.3,
            tooltip_grace_time: 0.2,
            selectable_labels: true,
            multi_widget_text_select: true,
        };

        ctx.style_mut(|style| {
            style.visuals = visuals.clone();
            style.spacing = spacing.clone();
            style.interaction = interaction.clone();
            // Snappy, near-instant transitions — mechanical, not animated.
            style.animation_time = 0.05;
            // The accessibility line-height (`item_spacing.y`) is overwritten by
            // `apply_style()` immediately after; the value set above is the
            // resting default for the no-vault welcome screen.
        });
    }
}

/// Map a portable `FontFamily` (core, no egui dep) to its `egui::FontFamily`.
/// `System`/`Monospace` both resolve to the mono body face (Terminal = mono).
/// `Serif` keeps egui's proportional chain. `Dyslexic` resolves to the
/// OpenDyslexic face registered under [`OPEN_DYSLEXIC_NAME`]; callers must
/// register it before use.
pub fn font_family_to_egui(f: FontFamily) -> egui::FontFamily {
    match f {
        // System defaults to the mono body — the primary Proportional family is
        // registered as JetBrains Mono in `register_custom_fonts`.
        FontFamily::System => egui::FontFamily::Proportional,
        FontFamily::Monospace => egui::FontFamily::Monospace,
        FontFamily::Serif => egui::FontFamily::Proportional,
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
            Color32::from_rgb(0x01, 0x06, 0x04)
        );
        assert_eq!(Theme::high_contrast().bg, Color32::BLACK);
        assert_ne!(Theme::obsidian_dark().bg, Theme::obsidian_light().bg);
    }

    #[test]
    fn new_presets_have_pairwise_distinct_backgrounds() {
        let bgs = [
            Theme::obsidian_dark().bg,
            Theme::obsidian_light().bg,
            Theme::high_contrast().bg,
            Theme::almanac_light().bg,
            Theme::almanac_dark().bg,
            Theme::blueprint().bg,
            Theme::blueprint_light().bg,
            Theme::swiss().bg,
        ];
        for (i, a) in bgs.iter().enumerate() {
            for (j, b) in bgs.iter().enumerate() {
                if i != j {
                    assert_ne!(a, b, "preset {i} and {j} share a background");
                }
            }
        }
    }

    #[test]
    fn new_presets_carry_the_right_darkness_flag() {
        assert!(!Theme::almanac_light().dark);
        assert!(Theme::almanac_dark().dark);
        assert!(Theme::blueprint().dark);
        assert!(!Theme::blueprint_light().dark);
        assert!(Theme::swiss().dark);
    }

    #[test]
    fn almanac_variants_share_the_terracotta_accent() {
        // Brand continuity: both Almanac variants keep the same accent so the
        // "Almanac" identity reads consistently regardless of dark/light pick.
        assert_eq!(Theme::almanac_light().accent, Theme::almanac_dark().accent);
        assert_eq!(
            Theme::almanac_light().accent,
            Color32::from_rgb(0xBF, 0x4D, 0x26)
        );
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
        assert_eq!(
            Theme::from_preset(ThemePreset::AlmanacLight, accent),
            Theme::almanac_light()
        );
        assert_eq!(
            Theme::from_preset(ThemePreset::AlmanacDark, accent),
            Theme::almanac_dark()
        );
        assert_eq!(
            Theme::from_preset(ThemePreset::Blueprint, accent),
            Theme::blueprint()
        );
        assert_eq!(
            Theme::from_preset(ThemePreset::BlueprintLight, accent),
            Theme::blueprint_light()
        );
        assert_eq!(
            Theme::from_preset(ThemePreset::Swiss, accent),
            Theme::swiss()
        );
    }

    #[test]
    fn accent_alpha_keeps_rgb_sets_alpha() {
        let t = Theme::obsidian_dark();
        let c = t.accent_alpha(56);
        // `Color32` stores premultiplied, so the unmultiplied accent RGB at alpha
        // 56 is the canonical value — assert the constructor identity + alpha. (A
        // raw r()/g()/b() comparison would fail: egui premultiplies on build.)
        assert_eq!(
            c,
            Color32::from_rgba_unmultiplied(t.accent.r(), t.accent.g(), t.accent.b(), 56)
        );
        assert_eq!(c.a(), 56);
        // Full opacity round-trips the accent exactly (premultiply is identity).
        assert_eq!(t.accent_alpha(255), t.accent);
        // The row washes derive from accent_alpha, so they keep the accent hue.
        assert_eq!(t.row_selected(), t.accent_alpha(56));
        assert_eq!(t.row_hover(), t.accent_alpha(26));
    }

    #[test]
    fn high_contrast_folds_semantics_to_accent_or_dim() {
        let hc = Theme::high_contrast();
        assert_eq!(hc.status_done(), hc.accent);
        assert_eq!(hc.status_doing(), hc.accent);
        assert_eq!(hc.status_blocked(), hc.accent);
        assert_eq!(hc.provider_jira(), hc.accent);
        assert_eq!(hc.note_type_color(NoteType::Citacao), hc.accent);
        // todo folds to dim (a legible neutral on pure black), not accent.
        assert_eq!(hc.status_todo(), hc.dim);
    }

    #[test]
    fn note_type_colors_are_distinct_in_dark() {
        let t = Theme::obsidian_dark();
        let mut seen = std::collections::HashSet::new();
        for ty in NoteType::all() {
            assert!(
                seen.insert(t.note_type_color(ty).to_array()),
                "note-type colors must be distinct"
            );
        }
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

    /// Mechanical "no egui default" gate. Applies each preset onto a fresh
    /// `egui::Context` and asserts the Terminal/Mechanical surface differs from
    /// `Style::default()` / `Visuals::default()` across the parameters the audit
    /// flagged as previously-default. If a future edit drops one of these
    /// overrides, this fails.
    #[test]
    fn no_egui_defaults_remain() {
        let def_style = egui::Style::default();
        let def_vis = egui::Visuals::default();
        let def_dark = egui::Visuals::dark();
        let def_light = egui::Visuals::light();

        for theme in [
            Theme::obsidian_dark(),
            Theme::obsidian_light(),
            Theme::high_contrast(),
            Theme::custom([0x12, 0x34, 0x56]),
            Theme::almanac_light(),
            Theme::almanac_dark(),
            Theme::blueprint(),
            Theme::blueprint_light(),
            Theme::swiss(),
        ] {
            let ctx = egui::Context::default();
            theme.apply(&ctx);
            let style = ctx.style();
            let v = &style.visuals;

            // --- Rounding: every widget state + window + menu must be ZERO ---
            for w in [
                &v.widgets.noninteractive,
                &v.widgets.inactive,
                &v.widgets.hovered,
                &v.widgets.active,
                &v.widgets.open,
            ] {
                assert_eq!(
                    w.rounding,
                    egui::Rounding::ZERO,
                    "widget rounding flattened"
                );
                assert_eq!(w.expansion, 0.0, "no growth-on-hover expansion");
            }
            assert_eq!(v.window_rounding, egui::Rounding::ZERO);
            assert_eq!(v.menu_rounding, egui::Rounding::ZERO);

            // --- Shadows removed (default is a soft drop shadow) ---
            assert_eq!(v.window_shadow, egui::epaint::Shadow::NONE);
            assert_eq!(v.popup_shadow, egui::epaint::Shadow::NONE);
            assert_ne!(
                v.window_shadow, def_vis.window_shadow,
                "default window shadow must be overridden"
            );

            // --- Fills/strokes are token-driven, not egui base ---
            assert_eq!(v.override_text_color, Some(theme.text));
            assert_eq!(v.panel_fill, theme.bg);
            assert_eq!(v.window_fill, theme.panel);
            assert_eq!(v.extreme_bg_color, theme.panel);
            assert_eq!(v.faint_bg_color, theme.panel_alt);
            assert_eq!(v.code_bg_color, theme.panel_alt);
            assert_eq!(v.window_stroke, egui::Stroke::new(1.0, theme.border));
            assert_ne!(
                v.panel_fill, def_dark.panel_fill,
                "panel fill differs from egui dark base"
            );
            assert_ne!(
                v.panel_fill, def_light.panel_fill,
                "panel fill differs from egui light base"
            );

            // --- Semantic + link colors overridden ---
            assert_eq!(v.hyperlink_color, theme.accent);
            assert_eq!(v.error_fg_color, theme.status_blocked());
            assert_eq!(v.warn_fg_color, theme.status_doing());
            assert_ne!(
                v.hyperlink_color, def_dark.hyperlink_color,
                "hyperlink color is the accent, not egui's blue"
            );

            // --- Selection wash != default (default is ~opaque accent) ---
            assert_eq!(v.selection.bg_fill, theme.accent_alpha(56));
            assert_eq!(v.selection.stroke, egui::Stroke::new(1.0, theme.accent));
            assert_ne!(
                v.selection.bg_fill, def_vis.selection.bg_fill,
                "selection fill is a deliberate translucent wash"
            );

            // --- Misc Visuals flags moved off their defaults where chosen ---
            assert!(!v.window_highlight_topmost);
            assert!(!v.collapsing_header_frame);
            assert!(v.slider_trailing_fill, "default is OFF");
            assert!(v.interact_cursor.is_some(), "default is None");

            // --- Spacing: all the previously-default knobs are set ---
            let sp = &style.spacing;
            assert_eq!(sp.item_spacing, egui::vec2(6.0, 4.0));
            assert_eq!(sp.button_padding, egui::vec2(8.0, 4.0));
            assert_eq!(sp.menu_margin, egui::Margin::same(4.0));
            assert_eq!(sp.indent, 14.0);
            assert_eq!(sp.interact_size, egui::vec2(40.0, 20.0));
            assert_eq!(sp.slider_width, 140.0);
            assert_eq!(sp.combo_height, 240.0);
            assert!(
                sp.scroll.floating,
                "scrollbars float over the void (no reserved gutter)"
            );
            assert_eq!(sp.scroll.bar_width, 10.0);
            assert_ne!(
                sp.indent, def_style.spacing.indent,
                "indent differs from egui's 18.0 default"
            );

            // --- Interaction: deliberate, not default ---
            let it = &style.interaction;
            assert_eq!(it.tooltip_delay, 0.3);
            assert_eq!(it.resize_grab_radius_side, 4.0);
            assert_eq!(it.resize_grab_radius_corner, 8.0);
            assert_ne!(
                it.tooltip_delay, def_style.interaction.tooltip_delay,
                "tooltip delay differs from egui's 0.5 default"
            );

            // --- animation_time is the snappy mechanical value ---
            assert_eq!(style.animation_time, 0.05);
            assert_ne!(
                style.animation_time, def_style.animation_time,
                "animation differs from egui's 0.2 default"
            );
        }
    }
}
