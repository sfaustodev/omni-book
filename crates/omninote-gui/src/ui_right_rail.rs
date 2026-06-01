//! Right rail (320px) — CAD-25 Fase B §2.5. Tabs: Backlinks (who links here,
//! via the core resolver) and Outline (this note's heading tree). The AI Chat
//! tab is wired in a later slice once it has a backing view; here the rail shows
//! only the two tabs that have real data.
//!
//! A `SidePanel::right`, shown after the left sidebar and before the central
//! editor in `update()`. Toggled by `config.right_rail_open`.

use crate::app::OmniNoteApp;
use egui::RichText;
use omninote_core::resolver::VaultIndex;
use omninote_core::types::RightRailTab;

/// One outline entry: heading depth (1–6) and its text.
struct Heading {
    depth: usize,
    text: String,
}

/// Parse ATX headings (`#`..`######`) from note body, skipping fenced code so a
/// `# comment` inside a ``` block isn't mistaken for a heading.
fn outline(content: &str) -> Vec<Heading> {
    let mut out = Vec::new();
    let mut in_fence = false;
    for line in content.lines() {
        let trimmed = line.trim_start();
        if trimmed.starts_with("```") || trimmed.starts_with("~~~") {
            in_fence = !in_fence;
            continue;
        }
        if in_fence {
            continue;
        }
        let hashes = trimmed.chars().take_while(|&c| c == '#').count();
        if (1..=6).contains(&hashes) && trimmed[hashes..].starts_with(' ') {
            out.push(Heading {
                depth: hashes,
                text: trimmed[hashes..].trim().to_string(),
            });
        }
    }
    out
}

impl OmniNoteApp {
    pub fn show_right_rail(&mut self, ctx: &egui::Context) {
        let Some(v) = &self.vault else { return };
        if !v.config.right_rail_open {
            return;
        }

        egui::SidePanel::right("right_rail")
            .exact_width(320.0)
            .resizable(false)
            .show(ctx, |ui| {
                let mut tab = self
                    .vault
                    .as_ref()
                    .map(|v| v.config.right_rail_tab)
                    .unwrap();
                ui.horizontal(|ui| {
                    ui.selectable_value(&mut tab, RightRailTab::Backlinks, "🔗 Backlinks");
                    ui.selectable_value(&mut tab, RightRailTab::Outline, "≡ Outline");
                });
                if let Some(v) = &mut self.vault {
                    v.config.right_rail_tab = tab;
                }
                ui.separator();

                match tab {
                    RightRailTab::Backlinks => self.show_backlinks_tab(ui),
                    RightRailTab::Outline => self.show_outline_tab(ui),
                    // AI Chat lands in a later slice; fall back to backlinks for now.
                    RightRailTab::AiChat => self.show_backlinks_tab(ui),
                }
            });
    }

    fn show_backlinks_tab(&mut self, ui: &mut egui::Ui) {
        let Some(note) = &self.active_note else {
            ui.label(RichText::new("Nenhuma nota aberta.").weak());
            return;
        };
        let Some(v) = &self.vault else { return };

        // Target-side resolution (handles aliases / path / case) via the core
        // index, replacing the old substring match. Rebuilt per render — cheap
        // for vault sizes here; cache later if it shows up in a profile.
        let index = VaultIndex::build(&v.notes);
        let backlinks = index.backlinks_to(&note.rel_path, &v.notes);

        if backlinks.is_empty() {
            ui.label(RichText::new("Nenhum backlink ainda.").weak());
            return;
        }

        ui.label(
            RichText::new(format!("{} nota(s) apontam aqui", backlinks.len()))
                .size(11.0)
                .weak(),
        );
        ui.add_space(4.0);

        let mut pending: Option<String> = None;
        egui::ScrollArea::vertical()
            .id_salt("backlinks_scroll")
            .show(ui, |ui| {
                for bl in &backlinks {
                    let title = v
                        .notes
                        .iter()
                        .find(|n| n.rel_path == bl.source)
                        .map(|n| n.title.clone())
                        .unwrap_or_else(|| bl.source.to_string_lossy().into_owned());
                    let label = if bl.is_embed {
                        format!("⧉ {title}")
                    } else {
                        format!("← {title}")
                    };
                    if ui.link(label).clicked() {
                        if let Some(n) = v.notes.iter().find(|n| n.rel_path == bl.source) {
                            pending = Some(n.frontmatter.id.clone());
                        }
                    }
                    if let Some(anchor) = &bl.anchor {
                        use omninote_core::wikilinks::Anchor;
                        let a = match anchor {
                            Anchor::Heading(h) => format!("   #{h}"),
                            Anchor::Block(b) => format!("   #^{b}"),
                        };
                        ui.label(RichText::new(a).size(10.0).weak());
                    }
                }
            });
        if let Some(id) = pending {
            self.select_note(&id);
        }
    }

    fn show_outline_tab(&mut self, ui: &mut egui::Ui) {
        let Some(note) = &self.active_note else {
            ui.label(RichText::new("Nenhuma nota aberta.").weak());
            return;
        };
        let headings = outline(&note.content);
        if headings.is_empty() {
            ui.label(RichText::new("Sem cabeçalhos.").weak());
            return;
        }
        egui::ScrollArea::vertical()
            .id_salt("outline_scroll")
            .show(ui, |ui| {
                for h in &headings {
                    // Indent by depth; H1 flush-left, deeper headings step in.
                    ui.horizontal(|ui| {
                        ui.add_space((h.depth as f32 - 1.0) * 12.0);
                        let size = 14.0 - (h.depth as f32 - 1.0);
                        ui.label(RichText::new(&h.text).size(size.max(10.0)));
                    });
                }
            });
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn outline_extracts_atx_headings_with_depth() {
        let md = "# Title\nbody\n## Section\n### Sub\nnot # a heading\n";
        let hs = outline(md);
        assert_eq!(hs.len(), 3);
        assert_eq!(hs[0].depth, 1);
        assert_eq!(hs[0].text, "Title");
        assert_eq!(hs[2].depth, 3);
        assert_eq!(hs[2].text, "Sub");
    }

    #[test]
    fn outline_skips_fenced_code() {
        let md = "# Real\n```\n# fake heading in code\n```\n## AlsoReal\n";
        let hs = outline(md);
        assert_eq!(hs.len(), 2);
        assert_eq!(hs[0].text, "Real");
        assert_eq!(hs[1].text, "AlsoReal");
    }

    #[test]
    fn outline_requires_space_after_hashes() {
        // `#tag` (no space) is not a heading.
        let hs = outline("#tag\n#### deep\n");
        assert_eq!(hs.len(), 1);
        assert_eq!(hs[0].depth, 4);
        assert_eq!(hs[0].text, "deep");
    }
}
