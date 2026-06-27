//! Shared accessibility helpers for clickable list rows — CAD-25b Slice 5.
//!
//! The legacy sidebar (`ui_sidebar.rs`) learned the hard a11y lesson: a plain
//! `Label::new(..).sense(click())` is invisible to keyboard navigation — it
//! takes no Tab focus, paints no focus ring, and ignores Enter/Space. The typed
//! views (tickets, timeline, sprint task ids, the DISCIPLINES section) shipped
//! with the same regression. Rather than re-copy the manual-paint recipe at each
//! call site (and forget it again next time), the recipe lives here once.
//!
//! [`clickable_row`] wraps arbitrary row content in a focusable container that
//! draws the focus ring, re-announces to AccessKit, and reports activation on a
//! mouse click *or* Enter/Space while focused. Font sizes inside the content
//! must stay relative — use [`scaled`] so they track the user's accessibility
//! font size instead of pinning an absolute pixel size.

use crate::theme::Theme;
use egui::{Response, RichText, Ui};

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

/// Render `content` as a single focusable, keyboard-activatable row.
///
/// Returns `(response, activated)` where `activated` is true on a primary click
/// or on Enter/Space while the row holds keyboard focus. The row paints an
/// accent focus ring when focused and announces itself to screen readers as a
/// button labelled `label`, reporting `selected` as its selected state so the
/// currently-active row is announced as selected rather than always unselected.
/// Mirrors the hand-painted sidebar row but keeps arbitrary inline content
/// (glyphs, ids, titles) inside the closure.
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
    // `Label::new(..).sense(click())` lacks.
    let inner = ui.horizontal(|ui| content(ui));
    let id = inner.response.id;

    // Expand the rect to cover the full available width of the panel.
    let mut rect = inner.response.rect;
    let available_w = ui.available_width();
    rect.max.x = (rect.min.x + available_w).max(rect.max.x);

    let resp = ui
        .interact(rect, id, egui::Sense::click())
        .on_hover_cursor(egui::CursorIcon::PointingHand);

    let hc = theme.is_high_contrast();
    if selected {
        if hc {
            ui.painter().rect_stroke(
                resp.rect,
                egui::Rounding::same(4.0),
                egui::Stroke::new(2.0, theme.accent),
            );
        } else {
            ui.painter()
                .rect_filled(resp.rect, egui::Rounding::same(4.0), theme.row_selected());
        }
        let bar = egui::Rect::from_min_max(
            resp.rect.min,
            egui::pos2(resp.rect.min.x + 3.0, resp.rect.max.y),
        );
        ui.painter()
            .rect_filled(bar, egui::Rounding::same(1.5), theme.accent);
    } else if resp.hovered() {
        if hc {
            ui.painter().rect_stroke(
                resp.rect,
                egui::Rounding::same(4.0),
                egui::Stroke::new(1.0, theme.accent),
            );
        } else {
            ui.painter()
                .rect_filled(resp.rect, egui::Rounding::same(4.0), theme.row_hover());
        }
    }

    if resp.has_focus() {
        ui.painter().rect_stroke(
            resp.rect.expand(1.0),
            egui::Rounding::same(4.0),
            egui::Stroke::new(1.5, theme.accent),
        );
    }
    // Re-announce to AccessKit — the manual interact replaced the implicit
    // semantics a SelectableLabel/Button would have carried.
    resp.widget_info(|| {
        egui::WidgetInfo::selected(egui::WidgetType::Button, true, selected, label)
    });
    let kbd = resp.has_focus()
        && ui.input(|i| i.key_pressed(egui::Key::Enter) || i.key_pressed(egui::Key::Space));
    let activated = resp.clicked() || kbd;
    (resp, activated)
}
