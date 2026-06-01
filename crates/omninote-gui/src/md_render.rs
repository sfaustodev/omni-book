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

/// Resolved preview of a wikilink target, shown on hover (Slice 3b). `None` from
/// the resolver means the link is broken (rendered italic/weak).
pub struct LinkPreview {
    pub title: String,
    /// First non-empty lines of the target's body (already trimmed/capped).
    pub excerpt: String,
}

/// Render note body with inline OmniNote tokens. `resolve` maps a wikilink target
/// to a [`LinkPreview`] (or `None` if broken) — the caller wires this to the core
/// `VaultIndex` + note bodies. Returns the first action the user triggered.
pub fn render_body(
    ui: &mut egui::Ui,
    md_cache: &mut egui_commonmark::CommonMarkCache,
    content: &str,
    resolve: &dyn Fn(&str) -> Option<LinkPreview>,
) -> Option<MdAction> {
    let mut action = None;
    for line in content.lines() {
        // Fast path: a line with no tokens and no '#' renders as real markdown.
        if extract_spans(line).is_empty() && !line.contains('#') {
            egui_commonmark::CommonMarkViewer::new().show(ui, md_cache, line);
            continue;
        }
        let a = render_inline_line(ui, line, resolve);
        action = action.or(a);
    }
    action
}

/// Build the excerpt shown in a hover popup: the first few non-empty body lines,
/// skipping a YAML frontmatter block, capped so the popup stays small.
pub fn preview_excerpt(content: &str) -> String {
    let mut lines = content.lines().peekable();
    // Skip a leading `---` frontmatter fence if present.
    if lines.peek().map(|l| l.trim_end()) == Some("---") {
        lines.next();
        for l in lines.by_ref() {
            if l.trim_end() == "---" {
                break;
            }
        }
    }
    let body: Vec<&str> = lines
        .map(|l| l.trim_end())
        .filter(|l| !l.is_empty())
        .take(4)
        .collect();
    let joined = body.join("\n");
    if joined.chars().count() > 240 {
        let head: String = joined.chars().take(240).collect();
        format!("{head}…")
    } else {
        joined
    }
}

/// Render one line containing tokens as a wrapped row of widgets. The action is
/// produced inside the layout closure and returned via its `inner` value (egui
/// moves the closure, so it can't borrow an outer `action`).
fn render_inline_line(
    ui: &mut egui::Ui,
    line: &str,
    resolve: &dyn Fn(&str) -> Option<LinkPreview>,
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
                    let preview = resolve(&r.target);
                    // Broken links (no resolution) render italic+weak.
                    let text = if preview.is_some() {
                        RichText::new(format!("🔗 {label}"))
                    } else {
                        RichText::new(format!("🔗 {label}")).italics().weak()
                    };
                    let mut resp = ui.link(text);
                    // Hover preview (Slice 3b) — egui's default tooltip delay (~400ms)
                    // matches Q-23, so no manual timer needed.
                    if let Some(p) = &preview {
                        resp = resp.on_hover_ui(|ui| {
                            ui.set_max_width(320.0);
                            ui.label(RichText::new(&p.title).strong());
                            if !p.excerpt.is_empty() {
                                ui.separator();
                                ui.label(RichText::new(&p.excerpt).weak());
                            }
                        });
                    }
                    if resp.clicked() {
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
    /// content (links with previews + tags). Resolution is stubbed.
    #[test]
    fn render_body_renders_tokens_without_panic() {
        let ctx = egui::Context::default();
        let mut cache = egui_commonmark::CommonMarkCache::default();
        let mut got = None;
        let _ = ctx.run(Default::default(), |ctx| {
            egui::CentralPanel::default().show(ctx, |ui| {
                got = render_body(ui, &mut cache, "veja [[Alvo]] e #tag aqui", &|t| {
                    (t == "Alvo").then(|| LinkPreview {
                        title: "Alvo".into(),
                        excerpt: "corpo".into(),
                    })
                });
            });
        });
        // No click in a headless run → no action — must render cleanly.
        assert!(got.is_none());
    }

    #[test]
    fn extract_spans_drives_segmentation() {
        // Sanity that the core span extraction the renderer relies on is stable.
        let spans = extract_spans("a [[X]] b ![[y.png]] c");
        assert_eq!(spans.len(), 2);
    }

    #[test]
    fn preview_excerpt_skips_frontmatter_and_caps() {
        let md = "---\ntype: daily\ntags: [x]\n---\n\nPrimeira linha real.\nSegunda linha.\n";
        let ex = preview_excerpt(md);
        assert!(ex.starts_with("Primeira linha real."));
        assert!(!ex.contains("type: daily"));
        assert!(ex.contains("Segunda linha."));
    }

    #[test]
    fn preview_excerpt_caps_long_body() {
        let long = "x".repeat(500);
        let ex = preview_excerpt(&long);
        assert!(ex.ends_with('…'));
        assert!(ex.chars().count() <= 241);
    }

    #[test]
    fn preview_excerpt_empty_for_blank_note() {
        assert_eq!(preview_excerpt("---\nonly: frontmatter\n---\n"), "");
    }
}
