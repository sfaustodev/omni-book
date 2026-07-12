//! Shared "Terminal / Mechanical" UI helpers — focusable list rows plus the
//! small terminal primitives (section dividers, key-hint chips, the `>` prompt
//! marker) every `ui_*.rs` reuses so the redesign stays one consistent language.
//!
//! The legacy sidebar (`ui_sidebar.rs`) learned the hard a11y lesson: a plain
//! `Label::new(..).sense(click())` is invisible to keyboard navigation — it
//! takes no Tab focus, paints no focus ring, and ignores Enter/Space. The typed
//! views (tickets, timeline, sprint task ids, the DISCIPLINES section) shipped
//! with the same regression. Rather than re-copy the manual-paint recipe at each
//! call site (and forget it again next time), the recipe lives here once.
//!
//! [`clickable_row`] wraps arbitrary row content in a focusable container that
//! signals focus, re-announces to AccessKit, and reports activation on a mouse
//! click *or* Enter/Space while focused. The selection signal is a terminal
//! PROMPT — a `>` marker in a fixed left gutter plus a 2px accent left-bar — not a
//! fill wash or an outline box, so the text never shifts between states and the
//! active row reads loud. Keyboard focus brightens the `>` marker (the row content
//! itself is not redrawn here, so the caller may also brighten its text on focus);
//! it does NOT draw a border rectangle — a framed row read as Windows-98 chrome.
//! Font sizes inside the content must stay relative — use [`scaled`] so they track
//! the user's accessibility font size instead of pinning an absolute pixel.

use crate::theme::Theme;
use egui::{Color32, Response, RichText, Ui};

/// Width reserved at the left of a [`clickable_row`] for the `>` prompt marker.
/// Fixed (scaled) so the row's text starts at the same x in every state — the
/// marker appears/brightens in place instead of pushing the label sideways.
const PROMPT_GUTTER_PX: f32 = 14.0;

/// Scale an intended pixel size by the user's accessibility font setting. The
/// base `Body` text style is already scaled by `apply_style()` from
/// `config.font_size` (14pt baseline), so dividing by 14 recovers the live
/// multiplier. Use for the small/large accent text in typed views so a row's
/// glyph and label grow with the rest of the UI under font zoom.
pub fn scaled(ui: &Ui, intended_px: f32) -> f32 {
    let base = egui::TextStyle::Body.resolve(ui.style()).size;
    (intended_px * (base / 14.0)).round().max(1.0)
}

/// A `RichText` whose size tracks the accessibility font scale (see [`scaled`]).
pub fn scaled_text(ui: &Ui, text: impl Into<String>, intended_px: f32) -> RichText {
    RichText::new(text).size(scaled(ui, intended_px))
}

/// A terminal section divider: an uppercase, letter-spaced `dim` mono label
/// (`NOTES`, `RESUMO`). No drawn rule — in the void look the section break is the
/// uppercase label and the surrounding spacing, not a hairline box edge.
/// Letter-spacing is faked by interspersing thin spaces, since egui `RichText`
/// has no tracking control. Size tracks the a11y font scale.
pub fn section_header(ui: &mut Ui, theme: &Theme, label: &str) {
    ui.horizontal(|ui| {
        ui.add_space(2.0);
        ui.label(
            RichText::new(spaced_caps(label))
                .monospace()
                .size(scaled(ui, 10.0))
                .color(theme.dim),
        );
    });
}

/// A key-hint: bare `dim` mono text wrapping the keys in literal bracket
/// characters (`[^N]`, `[esc]`, `[⏎]`) — no box, no stroke. The brackets ARE the
/// affordance in the void look; a drawn frame read as Windows-98 chrome. Sized via
/// the a11y scale.
pub fn kbd_hint(ui: &mut Ui, theme: &Theme, keys: &str) {
    ui.label(
        RichText::new(format!("[{keys}]"))
            .monospace()
            .size(scaled(ui, 10.0))
            .color(theme.dim),
    );
}

pub const ICON_BUTTON_MIN_SIDE: f32 = 28.0;
pub const MODE_SEGMENT_MIN_WIDTH: f32 = 56.0;

pub fn command_shortcut(ctx: &egui::Context, key: egui::Key, shift: bool) -> String {
    let modifiers = if shift {
        egui::Modifiers::COMMAND.plus(egui::Modifiers::SHIFT)
    } else {
        egui::Modifiers::COMMAND
    };
    ctx.format_shortcut(&egui::KeyboardShortcut::new(modifiers, key))
}

#[derive(Clone, Copy)]
pub struct IconButtonSpec<'a> {
    glyph: &'a str,
    label: &'a str,
    shortcut: Option<&'a str>,
    selected: bool,
    enabled: bool,
    disabled_reason: Option<&'a str>,
}

impl<'a> IconButtonSpec<'a> {
    pub const fn new(glyph: &'a str, label: &'a str) -> Self {
        Self {
            glyph,
            label,
            shortcut: None,
            selected: false,
            enabled: true,
            disabled_reason: None,
        }
    }

    pub const fn shortcut(mut self, shortcut: &'a str) -> Self {
        self.shortcut = Some(shortcut);
        self
    }

    pub const fn selected(mut self, selected: bool) -> Self {
        self.selected = selected;
        self
    }

    pub const fn enabled(mut self, enabled: bool, disabled_reason: &'a str) -> Self {
        self.enabled = enabled;
        self.disabled_reason = if enabled { None } else { Some(disabled_reason) };
        self
    }

    fn tooltip_text(self) -> String {
        if let Some(reason) = self.disabled_reason {
            format!("{} — {reason}", self.label)
        } else if let Some(shortcut) = self.shortcut {
            format!("{} ({shortcut})", self.label)
        } else {
            self.label.to_owned()
        }
    }
}

fn configure_framed_control(ui: &mut Ui, selected: bool, text_control: bool) {
    let normal_ink = ui.visuals().widgets.inactive.fg_stroke.color;
    let visuals = &mut ui.style_mut().visuals;
    visuals.selection.stroke = egui::Stroke::new(1.0_f32, normal_ink);
    visuals.widgets.hovered.bg_stroke = egui::Stroke::new(1.0_f32, normal_ink);
    visuals.widgets.active.bg_stroke = egui::Stroke::new(2.0_f32, normal_ink);
    if selected {
        visuals.widgets.hovered.fg_stroke.color = normal_ink;
        visuals.widgets.active.fg_stroke.color = normal_ink;
    } else if text_control {
        visuals.widgets.hovered.fg_stroke.color = normal_ink;
    }
}

fn selected_emphasis_width(
    selected: bool,
    hovered: bool,
    focused: bool,
    pointer_down: bool,
) -> f32 {
    if selected && (hovered || focused || pointer_down) {
        2.0
    } else {
        0.0
    }
}

fn paint_selected_emphasis(ui: &Ui, response: &Response, selected: bool) {
    let width = selected_emphasis_width(
        selected,
        response.hovered(),
        response.has_focus(),
        response.is_pointer_button_down_on(),
    );
    if width > 0.0 {
        let ink = ui.visuals().widgets.inactive.fg_stroke.color;
        ui.painter().rect_stroke(
            response.rect,
            egui::Rounding::ZERO,
            egui::Stroke::new(width, ink),
        );
    }
}

pub fn icon_button(ui: &mut Ui, spec: IconButtonSpec<'_>) -> Response {
    let side = ICON_BUTTON_MIN_SIDE;
    let response = ui
        .scope(|ui| {
            configure_framed_control(ui, spec.selected, false);
            let response = ui.add_enabled(
                spec.enabled,
                egui::Button::new(spec.glyph)
                    .frame(true)
                    .selected(spec.selected)
                    .min_size(egui::Vec2::splat(side)),
            );
            response.widget_info(|| {
                egui::WidgetInfo::selected(
                    egui::WidgetType::Button,
                    spec.enabled,
                    spec.selected,
                    spec.label,
                )
            });
            response
        })
        .inner;
    paint_selected_emphasis(ui, &response, spec.selected);
    let tooltip = spec.tooltip_text();
    if spec.enabled {
        response.on_hover_text(tooltip)
    } else {
        response.on_disabled_hover_text(tooltip)
    }
}

pub fn mode_segment_button(ui: &mut Ui, label: &str, selected: bool, tooltip: &str) -> Response {
    let response = ui
        .scope(|ui| {
            configure_framed_control(ui, selected, true);
            let response = ui.add(
                egui::Button::new(label)
                    .frame(true)
                    .selected(selected)
                    .min_size(egui::vec2(MODE_SEGMENT_MIN_WIDTH, ICON_BUTTON_MIN_SIDE)),
            );
            response.widget_info(|| {
                egui::WidgetInfo::selected(egui::WidgetType::Button, true, selected, label)
            });
            response
        })
        .inner;
    paint_selected_emphasis(ui, &response, selected);
    response.on_hover_text(tooltip)
}

/// Paint the `>` prompt marker centred in a row's left gutter. Color encodes the
/// row state: `accent` when selected or keyboard-focused (the loud signal),
/// `border_strong`/`accent` on hover, otherwise hidden (the default row has no
/// marker). Kept separate so [`clickable_row`] can paint it AFTER interaction,
/// when hover/focus are known, without shifting the already-laid-out content.
fn prompt_marker(ui: &Ui, row: egui::Rect, color: Color32) {
    let size = scaled(ui, 12.0);
    let font = egui::FontId::monospace(size);
    ui.painter().text(
        egui::pos2(row.left() + PROMPT_GUTTER_PX * 0.5, row.center().y),
        egui::Align2::CENTER_CENTER,
        ">",
        font,
        color,
    );
}

/// Render `content` as a single focusable, keyboard-activatable row in the
/// Terminal language.
///
/// Returns `(response, activated)` where `activated` is true on a primary click
/// or on Enter/Space while the row holds keyboard focus. The visual selection
/// signal is a terminal prompt rather than a fill or an outline box:
/// - a fixed left gutter holds a `>` marker so text never jumps between states;
/// - resting rows have no background and no marker;
/// - hover brightens the `>` marker (`border_strong`/`accent`);
/// - keyboard focus brightens the `>` marker to `accent` (no border rectangle);
/// - the selected/active row gets a 2px `accent` left-bar + `>` marker.
///
/// Under high-contrast the row is stroke-only (a translucent wash is invisible on
/// pure black): a 1px outline on hover/focus, 2px-bar + outline on selection, still
/// with the marker — the WCAG preset keeps its visible focus rectangle.
/// The row announces itself to screen readers as a button labelled `label`,
/// reporting `selected` so the active row is announced as selected.
pub fn clickable_row(
    ui: &mut Ui,
    theme: &Theme,
    label: &str,
    selected: bool,
    content: impl FnOnce(&mut Ui),
) -> (Response, bool) {
    // Lay the content out first to learn its rect, then claim that rect as a
    // single focusable click widget. `Sense::click()` is `focusable: true`, so
    // `ui.interact` (allow_focus) makes the row a Tab stop — the property a bare
    // `Label::new(..).sense(click())` lacks. The leading `add_space` reserves the
    // prompt gutter so every caller's content starts past the `>` marker without
    // each call site knowing about it.
    let inner = ui.horizontal(|ui| {
        ui.style_mut().interaction.selectable_labels = false;
        ui.add_space(PROMPT_GUTTER_PX);
        content(ui);
    });
    let id = inner.response.id;

    // Expand the rect to cover the full available width of the panel.
    let mut rect = inner.response.rect;
    let available_w = ui.available_width();
    rect.max.x = (rect.min.x + available_w).max(rect.max.x);

    let resp = ui
        .interact(rect, id, egui::Sense::click())
        .on_hover_cursor(egui::CursorIcon::PointingHand);

    let hc = theme.is_high_contrast();
    let focused = resp.has_focus();
    let hovered = resp.hovered();

    // Decide the marker color from state (none when resting). Selection and focus
    // win over hover; both speak in the accent.
    let marker = if selected || focused {
        Some(theme.accent)
    } else if hovered {
        Some(if hc {
            theme.accent
        } else {
            theme.border_strong
        })
    } else {
        None
    };

    if selected {
        // The loud signal: a 2px accent left-bar (NOT an outline box). Under HC,
        // add a full 1px outline since there is no fill/bar contrast to read the
        // selection from on pure black.
        if hc {
            ui.painter().rect_stroke(
                resp.rect,
                egui::Rounding::ZERO,
                egui::Stroke::new(1.0_f32, theme.accent),
            );
        }
        let bar = egui::Rect::from_min_max(
            resp.rect.min,
            egui::pos2(resp.rect.min.x + 2.0, resp.rect.max.y),
        );
        ui.painter()
            .rect_filled(bar, egui::Rounding::ZERO, theme.accent);
    } else if hc && (hovered || focused) {
        // High-contrast has no phosphor wash to lean on, and a brightened glyph
        // alone is too faint for the WCAG preset — keep a visible focus/hover
        // outline here ONLY. Every other theme signals focus via the accent `>`.
        ui.painter().rect_stroke(
            resp.rect,
            egui::Rounding::ZERO,
            egui::Stroke::new(1.0_f32, theme.accent),
        );
    }

    // Keyboard focus in the void themes is the `>` marker brightened to accent
    // (set above) — no border rectangle is drawn, a framed row read as Windows 98.
    if let Some(color) = marker {
        prompt_marker(ui, resp.rect, color);
    }

    // Re-announce to AccessKit — the manual interact replaced the implicit
    // semantics a SelectableLabel/Button would have carried.
    resp.widget_info(|| {
        egui::WidgetInfo::selected(egui::WidgetType::Button, true, selected, label)
    });
    let kbd =
        focused && ui.input(|i| i.key_pressed(egui::Key::Enter) || i.key_pressed(egui::Key::Space));
    let activated = resp.clicked() || kbd;
    (resp, activated)
}

/// Uppercase a label and intersperse thin spaces to fake letter-spacing for the
/// terminal section dividers (egui `RichText` has no tracking control).
fn spaced_caps(label: &str) -> String {
    let upper = label.to_uppercase();
    let mut out = String::with_capacity(upper.len() * 2);
    let mut first = true;
    for ch in upper.chars() {
        if !first {
            out.push('\u{2009}'); // thin space
        }
        out.push(ch);
        first = false;
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn spaced_caps_uppercases_and_separates() {
        let s = spaced_caps("notes");
        assert!(s.starts_with('N'));
        assert!(s.contains('\u{2009}'), "thin space between glyphs");
        // 5 letters → 4 separators interleaved.
        assert_eq!(s.matches('\u{2009}').count(), 4);
        assert_eq!(s.chars().filter(|c| c.is_alphabetic()).count(), 5);
    }

    #[test]
    fn spaced_caps_handles_empty() {
        assert_eq!(spaced_caps(""), "");
    }

    #[test]
    fn icon_button_target_never_smaller_than_28() {
        let size = std::cell::Cell::new(egui::Vec2::ZERO);
        egui::__run_test_ui(|ui| {
            size.set(
                icon_button(ui, IconButtonSpec::new("⚙", "Configurações"))
                    .rect
                    .size(),
            );
        });
        let size = size.get();
        assert!(size.x >= ICON_BUTTON_MIN_SIDE);
        assert!(size.y >= ICON_BUTTON_MIN_SIDE);
    }

    #[test]
    fn icon_button_fits_fixed_chrome_at_maximum_font_size() {
        let size = std::cell::Cell::new(egui::Vec2::ZERO);
        egui::__run_test_ui(|ui| {
            ui.style_mut().text_styles.insert(
                egui::TextStyle::Body,
                egui::FontId::new(23.0, egui::FontFamily::Monospace),
            );
            ui.style_mut().text_styles.insert(
                egui::TextStyle::Button,
                egui::FontId::new(21.0, egui::FontFamily::Monospace),
            );
            size.set(
                icon_button(ui, IconButtonSpec::new("⚙", "Configurações"))
                    .rect
                    .size(),
            );
        });
        assert!(size.get().y <= 34.0, "maximum font must fit the titlebar");
    }

    #[test]
    fn selected_control_keeps_readable_ink_in_hover_and_focus_states() {
        for preset in omninote_core::types::ThemePreset::all() {
            let theme = Theme::from_preset(preset, [0x6b, 0xff, 0x9a]);
            let ctx = egui::Context::default();
            theme.apply(&ctx);
            let _ = ctx.run(egui::RawInput::default(), |ctx| {
                egui::CentralPanel::default().show(ctx, |ui| {
                    ui.scope(|ui| {
                        configure_framed_control(ui, true, true);
                        assert_eq!(
                            ui.visuals().widgets.hovered.fg_stroke.color,
                            theme.text,
                            "selected hover ink in {preset:?}"
                        );
                        assert_eq!(
                            ui.visuals().widgets.active.fg_stroke.color,
                            theme.text,
                            "selected focus ink in {preset:?}"
                        );
                    });
                });
            });
        }
    }

    #[test]
    fn selected_control_emphasizes_hover_focus_and_press() {
        assert_eq!(selected_emphasis_width(true, false, false, false), 0.0);
        assert_eq!(selected_emphasis_width(true, true, false, false), 2.0);
        assert_eq!(selected_emphasis_width(true, false, true, false), 2.0);
        assert_eq!(selected_emphasis_width(true, false, false, true), 2.0);
        assert_eq!(selected_emphasis_width(false, true, true, true), 0.0);
    }

    #[test]
    fn icon_button_exposes_selected_human_accessibility_name() {
        let ctx = egui::Context::default();
        ctx.enable_accesskit();
        let output = ctx.run(egui::RawInput::default(), |ctx| {
            egui::CentralPanel::default().show(ctx, |ui| {
                icon_button(
                    ui,
                    IconButtonSpec::new("⊞", "Painel direito")
                        .shortcut("Ctrl+\\")
                        .selected(true),
                );
            });
        });
        let update = output
            .platform_output
            .accesskit_update
            .expect("accessibility tree");
        let node = update
            .nodes
            .iter()
            .find_map(|(_, node)| (node.name() == Some("Painel direito")).then_some(node))
            .expect("human-labelled button");
        assert_eq!(node.toggled().map(|state| state as u8), Some(1));

        let spec = IconButtonSpec::new("⊞", "Painel direito").shortcut("Ctrl+\\");
        assert_eq!(spec.tooltip_text(), "Painel direito (Ctrl+\\)");
    }

    #[test]
    fn disabled_icon_button_retains_name_and_reason() {
        let spec = IconButtonSpec::new("🎙", "Ditado").enabled(false, "Em breve");
        assert_eq!(spec.tooltip_text(), "Ditado — Em breve");

        let ctx = egui::Context::default();
        ctx.enable_accesskit();
        let output = ctx.run(egui::RawInput::default(), |ctx| {
            egui::CentralPanel::default().show(ctx, |ui| {
                icon_button(ui, spec);
            });
        });
        let update = output
            .platform_output
            .accesskit_update
            .expect("accessibility tree");
        let node = update
            .nodes
            .iter()
            .find_map(|(_, node)| (node.name() == Some("Ditado")).then_some(node))
            .expect("disabled human-labelled button");
        assert!(node.is_disabled());
    }

    #[test]
    fn mode_segment_target_and_selected_semantics_are_accessible() {
        let ctx = egui::Context::default();
        ctx.enable_accesskit();
        let mut size = egui::Vec2::ZERO;
        let output = ctx.run(egui::RawInput::default(), |ctx| {
            egui::CentralPanel::default().show(ctx, |ui| {
                size = mode_segment_button(ui, "Editar", true, "Alternar modo (Ctrl+E)")
                    .rect
                    .size();
            });
        });
        assert!(size.x >= MODE_SEGMENT_MIN_WIDTH);
        assert!(size.y >= ICON_BUTTON_MIN_SIDE);
        let update = output
            .platform_output
            .accesskit_update
            .expect("accessibility tree");
        let node = update
            .nodes
            .iter()
            .find_map(|(_, node)| (node.name() == Some("Editar")).then_some(node))
            .expect("mode segment");
        assert_eq!(node.toggled().map(|state| state as u8), Some(1));
    }

    #[test]
    fn clickable_row_activates_when_pointer_clicks_its_text() {
        let ctx = egui::Context::default();
        let theme = Theme::almanac_light();
        theme.apply(&ctx);
        let label_rect = std::cell::Cell::new(egui::Rect::NOTHING);
        let render = |input: egui::RawInput| {
            let mut activated = false;
            let _ = ctx.run(input, |ctx| {
                egui::CentralPanel::default().show(ctx, |ui| {
                    let (_, hit) = clickable_row(ui, &theme, "Nota clicável", false, |ui| {
                        label_rect.set(ui.label("Nota clicável").rect);
                    });
                    activated = hit;
                });
            });
            activated
        };
        let input = |time, events| egui::RawInput {
            time: Some(time),
            screen_rect: Some(egui::Rect::from_min_size(
                egui::Pos2::ZERO,
                egui::vec2(400.0, 200.0),
            )),
            events,
            ..Default::default()
        };

        assert!(!render(input(0.0, vec![])));
        let pos = label_rect.get().center();
        assert!(!render(input(
            0.1,
            vec![
                egui::Event::PointerMoved(pos),
                egui::Event::PointerButton {
                    pos,
                    button: egui::PointerButton::Primary,
                    pressed: true,
                    modifiers: egui::Modifiers::default(),
                },
            ],
        )));
        assert!(render(input(
            0.2,
            vec![
                egui::Event::PointerMoved(pos),
                egui::Event::PointerButton {
                    pos,
                    button: egui::PointerButton::Primary,
                    pressed: false,
                    modifiers: egui::Modifiers::default(),
                },
            ],
        )));
    }

    #[test]
    fn audited_icon_only_callsites_use_the_shared_helper() {
        let sources = [
            include_str!("ui_titlebar.rs"),
            include_str!("ui_sidebar.rs"),
            include_str!("ui_tabs.rs"),
            include_str!("ui_breadcrumb.rs"),
            include_str!("ui_calendar.rs"),
            include_str!("ui_discipline.rs"),
        ]
        .join("\n");
        for forbidden in [
            ".small_button(\"⚙\")",
            ".small_button(\"☀/🌙\")",
            ".small_button(\"📂\")",
            "Button::new(\"⊟\").small()",
            ".button(\"⚙\")",
            ".button(\"◐\")",
            ".button(\"📅\")",
            ".button(\"⌘P\")",
            "Button::new(\"🎙\")",
            ".small_button(\"×\")",
            "SelectableLabel::new(open, \"⊞\")",
            "Button::new(\"↷\").small()",
            "Button::new(\"↶\").small()",
            ".button(\"🗑\")",
            ".small_button(\"‹\")",
            ".small_button(\"›\")",
            "Button::new(\"⤴\")",
            "Button::new(\"⟲\")",
        ] {
            assert!(
                !sources.contains(forbidden),
                "icon-only control bypasses shared helper: {forbidden}"
            );
        }
    }

    #[test]
    fn visible_shortcut_hints_are_platform_aware() {
        let sources = [include_str!("ui_calendar.rs"), include_str!("ui_editor.rs")].join("\n");
        for forbidden in ["RichText::new(\"• Cmd+", "RichText::new(\"Cmd+"] {
            assert!(
                !sources.contains(forbidden),
                "visible shortcut bypasses Context::format_shortcut: {forbidden}"
            );
        }
    }
}
