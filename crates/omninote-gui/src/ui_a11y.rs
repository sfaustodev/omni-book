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
}
