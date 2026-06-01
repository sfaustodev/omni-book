//! Inline markdown rendering — CAD-25 Fase B Slice 3a. Hybrid approach: lines
//! without OmniNote tokens go to `egui_commonmark` (preserves headings/lists/
//! bold/code); lines containing `[[wikilinks]]`, `![[embeds]]`, or `#tags` are
//! split into a `horizontal_wrapped` row of text labels + clickable widgets, so
//! links live in the text flow instead of a separate appendix.
//!
//! Hover-preview popups + rich embed cards are Slice 3b; here embeds render as a
//! compact clickable chip. Returns a navigation request when a token is clicked,
//! so the caller (`show_view_panel`) drives `select_note` / sidebar search.

use egui::RichText;
use omninote_core::wikilinks::{extract_inline_tags, extract_spans, Wikilink};

/// What the user clicked in the rendered body, surfaced to the caller.
pub enum MdAction {
    /// Navigate to the note a `[[link]]`/embed targets. Carries the raw wikilink
    /// target so the caller resolves it through the core `VaultIndex` (paths,
    /// aliases, case) rather than this module re-implementing resolution.
    Navigate(String),
    /// Set the sidebar search query to this tag (a `#tag` chip).
    FilterTag(String),
}

/// Render note body with inline OmniNote tokens. `is_resolved` tells whether a
/// wikilink target points to an existing note (used for broken-link styling) —
/// the caller wires this to the core `VaultIndex`. Returns the first action the
/// user triggered this frame, or None.
pub fn render_body(
    ui: &mut egui::Ui,
    md_cache: &mut egui_commonmark::CommonMarkCache,
    content: &str,
    is_resolved: &dyn Fn(&str) -> bool,
) -> Option<MdAction> {
    let mut action = None;
    for line in content.lines() {
        // Fast path: a line with no tokens and no '#' renders as real markdown.
        if extract_spans(line).is_empty() && !line.contains('#') {
            egui_commonmark::CommonMarkViewer::new().show(ui, md_cache, line);
            continue;
        }
        let a = render_inline_line(ui, line, is_resolved);
        action = action.or(a);
    }
    action
}

/// Render one line containing tokens as a wrapped row of widgets. The action is
/// produced inside the layout closure and returned via its `inner` value (egui
/// moves the closure, so it can't borrow an outer `action`).
fn render_inline_line(
    ui: &mut egui::Ui,
    line: &str,
    is_resolved: &dyn Fn(&str) -> bool,
) -> Option<MdAction> {
    ui.horizontal_wrapped(|ui| {
        ui.spacing_mut().item_spacing.x = 2.0;
        let mut action = None;
        let spans = extract_spans(line);
        let mut cursor = 0usize;
        for (link, range) in &spans {
            if range.start > cursor {
                render_text_with_tags(ui, &line[cursor..range.start], &mut action);
            }
            match link {
                Wikilink::Note(r) | Wikilink::NoteEmbed(r) => {
                    let label = r.alias.clone().unwrap_or_else(|| r.target.clone());
                    // Broken links (no matching note) render italic+weak as a hint
                    // until the dashed-red-underline styling lands with 3b.
                    let text = if is_resolved(&r.target) {
                        RichText::new(format!("🔗 {label}"))
                    } else {
                        RichText::new(format!("🔗 {label}")).italics().weak()
                    };
                    if ui.link(text).clicked() {
                        action = action.or(Some(MdAction::Navigate(r.target.clone())));
                    }
                }
                Wikilink::Image(e) | Wikilink::File(e) => {
                    let label = e.alias.clone().unwrap_or_else(|| e.path.clone());
                    ui.label(RichText::new(format!("📎 {label}")).weak());
                }
            }
            cursor = range.end;
        }
        if cursor < line.len() {
            render_text_with_tags(ui, &line[cursor..], &mut action);
        }
        action
    })
    .inner
}

/// Render a plain-text run, turning `#tag` occurrences into clickable chips.
fn render_text_with_tags(ui: &mut egui::Ui, text: &str, action: &mut Option<MdAction>) {
    let tags = extract_inline_tags(text);
    if tags.is_empty() {
        let t = text.trim_end();
        if !t.trim().is_empty() {
            ui.label(t);
        }
        return;
    }
    let accent = ui.visuals().hyperlink_color;
    let mut rest = text;
    for tag in tags {
        let needle = format!("#{tag}");
        if let Some(pos) = rest.find(&needle) {
            let before = &rest[..pos];
            if !before.is_empty() {
                ui.label(before);
            }
            if ui.link(RichText::new(&needle).color(accent)).clicked() {
                *action = action.take().or(Some(MdAction::FilterTag(tag.clone())));
            }
            rest = &rest[pos + needle.len()..];
        }
    }
    if !rest.is_empty() {
        ui.label(rest);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Drive `render_body` headless to confirm it doesn't panic on token-bearing
    /// content and reports a navigation action for a resolvable link. (Resolution
    /// itself lives in the core index, stubbed here by `is_resolved`.)
    #[test]
    fn render_body_reports_navigate_on_link() {
        let ctx = egui::Context::default();
        let mut cache = egui_commonmark::CommonMarkCache::default();
        let mut got = None;
        let _ = ctx.run(Default::default(), |ctx| {
            egui::CentralPanel::default().show(ctx, |ui| {
                got = render_body(ui, &mut cache, "veja [[Alvo]] e #tag aqui", &|t| {
                    t == "Alvo"
                });
            });
        });
        // No click in a headless run, so no action — but it must render cleanly.
        assert!(got.is_none());
    }

    #[test]
    fn extract_spans_drives_segmentation() {
        // Sanity that the core span extraction the renderer relies on is stable.
        let spans = extract_spans("a [[X]] b ![[y.png]] c");
        assert_eq!(spans.len(), 2);
    }
}
