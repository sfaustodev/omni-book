//! Read-mode renderer. egui_commonmark gives us no link-click events, so view mode
//! is a small line walker: plain lines get light markdown styling, while
//! `wikilinks::extract_spans` lets us interleave real clickable `[[links]]` and
//! inline image/PDF embeds. Returns the wikilink target the user clicked, if any.

use omninote_core::wikilinks::{self, EmbedRef, NoteRef, Wikilink};

use crate::app::OmniNoteApp;

impl OmniNoteApp {
    /// Resolve a clicked `[[wikilink]]` target to a note and open it.
    pub fn navigate_wikilink(&mut self, target: &str) {
        let rel = self
            .vault
            .as_ref()
            .and_then(|v| v.index.resolve(target).cloned());
        let id = rel.and_then(|rel| {
            self.vault
                .as_ref()
                .and_then(|v| v.notes.iter().find(|n| n.rel_path == rel))
                .map(|n| n.frontmatter.id.clone())
        });
        match id {
            Some(id) => self.select_note(&id),
            None => self.toast_error(format!("Link não resolvido: [[{target}]]")),
        }
    }

    /// Render the active note's body in read mode; returns a clicked link target.
    pub fn render_view(&self, ui: &mut egui::Ui) -> Option<String> {
        let content = match &self.active_note {
            Some(n) => n.content.clone(),
            None => return None,
        };
        let mut nav: Option<String> = None;
        let mut fenced = false;
        let mut code = String::new();

        for line in content.lines() {
            if line.trim_start().starts_with("```") {
                if fenced {
                    self.render_code(ui, &code);
                    code.clear();
                }
                fenced = !fenced;
                continue;
            }
            if fenced {
                code.push_str(line);
                code.push('\n');
                continue;
            }
            self.render_line(ui, line, &mut nav);
        }
        if fenced && !code.is_empty() {
            self.render_code(ui, &code);
        }
        nav
    }

    fn render_code(&self, ui: &mut egui::Ui, code: &str) {
        egui::Frame::none()
            .fill(self.theme.panel_alt)
            .stroke(egui::Stroke::new(1.0, self.theme.border))
            .inner_margin(8.0)
            .show(ui, |ui| {
                ui.label(
                    egui::RichText::new(code.trim_end())
                        .monospace()
                        .color(self.theme.text),
                );
            });
    }

    fn render_line(&self, ui: &mut egui::Ui, line: &str, nav: &mut Option<String>) {
        let spans = wikilinks::extract_spans(line);
        if spans.is_empty() {
            self.render_plain(ui, line);
            return;
        }
        ui.horizontal_wrapped(|ui| {
            ui.spacing_mut().item_spacing.x = 2.0;
            let mut cursor = 0usize;
            for (link, range) in spans {
                if range.start > cursor {
                    let pre = &line[cursor..range.start];
                    if !pre.is_empty() {
                        ui.label(egui::RichText::new(pre).color(self.theme.text));
                    }
                }
                self.render_link(ui, &link, nav);
                cursor = range.end;
            }
            if cursor < line.len() {
                let rest = &line[cursor..];
                if !rest.is_empty() {
                    ui.label(egui::RichText::new(rest).color(self.theme.text));
                }
            }
        });
    }

    fn render_link(&self, ui: &mut egui::Ui, link: &Wikilink, nav: &mut Option<String>) {
        match link {
            Wikilink::Note(nr) | Wikilink::NoteEmbed(nr) => self.render_note_link(ui, nr, nav),
            Wikilink::Image(er) => self.render_image(ui, er),
            Wikilink::File(er) => self.render_file(ui, er),
        }
    }

    fn render_note_link(&self, ui: &mut egui::Ui, nr: &NoteRef, nav: &mut Option<String>) {
        let label = nr.alias.clone().unwrap_or_else(|| nr.target.clone());
        let resolved = self
            .vault
            .as_ref()
            .is_some_and(|v| v.index.contains(&nr.target));
        let color = if resolved {
            self.theme.accent
        } else {
            self.theme.faint
        };
        let resp = ui.link(egui::RichText::new(format!("[[{label}]]")).color(color));
        if resp.clicked() {
            *nav = Some(nr.target.clone());
        }
    }

    fn render_image(&self, ui: &mut egui::Ui, er: &EmbedRef) {
        let abs = self
            .vault
            .as_ref()
            .and_then(|v| v.attachment_path(&er.path));
        match abs {
            Some(path) => {
                ui.add(
                    egui::Image::new(format!("file://{}", path.display()))
                        .max_width(360.0)
                        .max_height(300.0)
                        .fit_to_original_size(1.0),
                );
            }
            None => {
                ui.colored_label(self.theme.faint, format!("🖼 {} (não encontrado)", er.path));
            }
        }
    }

    fn render_file(&self, ui: &mut egui::Ui, er: &EmbedRef) {
        let abs = self
            .vault
            .as_ref()
            .and_then(|v| v.attachment_path(&er.path));
        let resp = ui.button(format!("📄 abrir {}", er.path));
        if resp.clicked() {
            if let Some(path) = abs {
                let _ = open::that(path);
            }
        }
    }

    fn render_plain(&self, ui: &mut egui::Ui, line: &str) {
        let t = line.trim_end();
        if let Some(h) = t.strip_prefix("### ") {
            ui.label(
                egui::RichText::new(h)
                    .size(16.0)
                    .strong()
                    .color(self.theme.text),
            );
        } else if let Some(h) = t.strip_prefix("## ") {
            ui.label(
                egui::RichText::new(h)
                    .size(20.0)
                    .strong()
                    .color(self.theme.text),
            );
        } else if let Some(h) = t.strip_prefix("# ") {
            ui.label(egui::RichText::new(h).heading().color(self.theme.accent));
        } else if let Some(q) = t.strip_prefix("> ") {
            ui.label(egui::RichText::new(q).italics().color(self.theme.dim));
        } else if let Some(b) = t.strip_prefix("- ").or_else(|| t.strip_prefix("* ")) {
            ui.label(egui::RichText::new(format!("•  {b}")).color(self.theme.text));
        } else if t.is_empty() {
            ui.add_space(4.0);
        } else {
            ui.label(egui::RichText::new(t).color(self.theme.text));
        }
    }
}
