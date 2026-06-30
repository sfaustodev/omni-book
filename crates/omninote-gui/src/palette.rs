//! Search palette (Cmd/Ctrl+K). Ranks note titles with a fuzzy matcher and falls
//! back to full-text hits from `omninote_core::search` so a query matches body text
//! too. Enter / click opens the selected note.

use fuzzy_matcher::skim::SkimMatcherV2;
use fuzzy_matcher::FuzzyMatcher;
use omninote_core::search::{self, SearchOpts};

use crate::app::OmniNoteApp;

struct Cand {
    id: String,
    title: String,
    sub: String,
    score: i64,
}

impl OmniNoteApp {
    pub fn show_palette(&mut self, ctx: &egui::Context) {
        if !self.palette_open {
            return;
        }
        let theme = self.theme;
        let query = self.palette_query.clone();
        let cands = self.palette_candidates(&query);

        // Keyboard navigation.
        if ctx.input(|i| i.key_pressed(egui::Key::ArrowDown)) {
            self.palette_sel = (self.palette_sel + 1).min(cands.len().saturating_sub(1));
        }
        if ctx.input(|i| i.key_pressed(egui::Key::ArrowUp)) {
            self.palette_sel = self.palette_sel.saturating_sub(1);
        }
        if self.palette_sel >= cands.len() {
            self.palette_sel = cands.len().saturating_sub(1);
        }

        let mut chosen: Option<String> = None;
        if ctx.input(|i| i.key_pressed(egui::Key::Enter)) {
            if let Some(c) = cands.get(self.palette_sel) {
                chosen = Some(c.id.clone());
            }
        }

        egui::Window::new("palette")
            .title_bar(false)
            .resizable(false)
            .anchor(egui::Align2::CENTER_TOP, egui::vec2(0.0, 90.0))
            .fixed_size(egui::vec2(540.0, 0.0))
            .frame(
                egui::Frame::popup(&ctx.style())
                    .fill(theme.panel)
                    .stroke(egui::Stroke::new(1.0, theme.accent)),
            )
            .show(ctx, |ui| {
                let resp = ui.add(
                    egui::TextEdit::singleline(&mut self.palette_query)
                        .hint_text("buscar notas… (título e conteúdo)")
                        .desired_width(f32::INFINITY),
                );
                if std::mem::take(&mut self.focus_palette) {
                    resp.request_focus();
                }
                ui.separator();
                egui::ScrollArea::vertical()
                    .max_height(340.0)
                    .auto_shrink([false, false])
                    .show(ui, |ui| {
                        if cands.is_empty() {
                            ui.label(egui::RichText::new("nenhum resultado").color(theme.faint));
                        }
                        for (i, c) in cands.iter().enumerate() {
                            let sel = i == self.palette_sel;
                            let resp = ui.selectable_label(
                                sel,
                                egui::RichText::new(&c.title).color(theme.text).strong(),
                            );
                            if !c.sub.is_empty() {
                                ui.label(egui::RichText::new(&c.sub).small().color(theme.faint));
                            }
                            if resp.clicked() {
                                chosen = Some(c.id.clone());
                            }
                        }
                    });
            });

        if let Some(id) = chosen {
            self.select_note(&id);
            self.palette_open = false;
            self.palette_query.clear();
            self.palette_sel = 0;
        }
    }

    fn palette_candidates(&self, query: &str) -> Vec<Cand> {
        let Some(vault) = self.vault.as_ref() else {
            return Vec::new();
        };
        let q = query.trim();

        if q.is_empty() {
            return vault
                .notes
                .iter()
                .take(50)
                .map(|n| Cand {
                    id: n.frontmatter.id.clone(),
                    title: n.title.clone(),
                    sub: String::new(),
                    score: 0,
                })
                .collect();
        }

        let matcher = SkimMatcherV2::default();
        let mut out: Vec<Cand> = Vec::new();
        for n in &vault.notes {
            if let Some(score) = matcher.fuzzy_match(&n.title, q) {
                out.push(Cand {
                    id: n.frontmatter.id.clone(),
                    title: n.title.clone(),
                    sub: String::new(),
                    score: score + 1000, // title matches rank above body matches
                });
            }
        }

        // Body hits for notes whose titles didn't already match.
        let hits = search::search(
            &vault.notes,
            q,
            SearchOpts {
                case_sensitive: false,
                limit: Some(30),
            },
        );
        for hit in hits {
            let already = out.iter().any(|c| c.title == hit.title);
            if already {
                continue;
            }
            if let Some(n) = vault.notes.iter().find(|n| n.rel_path == hit.rel_path) {
                out.push(Cand {
                    id: n.frontmatter.id.clone(),
                    title: hit.title.clone(),
                    sub: hit.snippet.clone(),
                    score: 0,
                });
            }
        }

        out.sort_by_key(|c| std::cmp::Reverse(c.score));
        out.truncate(50);
        out
    }
}
