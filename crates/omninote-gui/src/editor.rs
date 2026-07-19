// Central panel (note title, edit/read, editing surface) plus the right-side
// "Conexões" panel (outgoing wikilinks + backlinks). Editing wires four engine
// features: eval-math (autoformat), attach/embed (vault attachments + wikilinks),
// inline image embeds (egui_extras image loader), and a slash command menu.

use egui::{Context, RichText};
use omninote_core::autoformat;
use omninote_core::wikilinks::{self, Wikilink};
use std::path::PathBuf;

use crate::app::OmniNoteApp;

/// Map a char index (egui cursor) to a byte offset in `s`.
fn char_to_byte(s: &str, ci: usize) -> usize {
    s.char_indices().nth(ci).map(|(b, _)| b).unwrap_or(s.len())
}

const SLASH_COMMANDS: &[(&str, &str, &str)] = &[
    ("h1", "Título", "# "),
    ("h2", "Subtítulo", "## "),
    ("todo", "Tarefa", "- [ ] "),
    ("list", "Lista", "- "),
    ("quote", "Citação", "> "),
    ("code", "Código", "```\n\n```"),
    ("link", "Wikilink", "[[]]"),
    ("date", "Data de hoje", "@date"),
];

impl OmniNoteApp {
    pub fn show_editor(&mut self, ctx: &Context) {
        egui::CentralPanel::default().show(ctx, |ui| {
            if self.active_note.is_none() {
                ui.vertical_centered(|ui| {
                    ui.add_space(90.0);
                    ui.label(RichText::new("OmniNote · Almanac").heading());
                    ui.add_space(6.0);
                    ui.label(
                        RichText::new("Selecione uma nota na lateral, ou crie uma (Cmd/Ctrl+N).")
                            .weak(),
                    );
                });
                return;
            }

            // Title + mode toggle.
            let mut title_changed = false;
            ui.horizontal(|ui| {
                if let Some(n) = &mut self.active_note {
                    let resp = ui.add(
                        egui::TextEdit::singleline(&mut n.title)
                            .font(egui::TextStyle::Heading)
                            .frame(false)
                            .desired_width(f32::INFINITY),
                    );
                    title_changed = resp.changed();
                }
            });
            ui.horizontal(|ui| {
                if ui.selectable_label(self.editing, "✎ Editar").clicked() {
                    self.editing = true;
                }
                if ui.selectable_label(!self.editing, "👁 Ler").clicked() {
                    self.editing = false;
                }
            });
            if title_changed {
                self.dirty = true;
            }
            ui.separator();

            if self.editing {
                self.edit_area(ui);
            } else {
                let content = self
                    .active_note
                    .as_ref()
                    .map(|n| self.preprocess_embeds(&n.content))
                    .unwrap_or_default();
                egui::ScrollArea::vertical().show(ui, |ui| {
                    egui_commonmark::CommonMarkViewer::new().show(ui, &mut self.md_cache, &content);
                });
            }
        });
    }

    fn edit_area(&mut self, ui: &mut egui::Ui) {
        let mut do_eval =
            ui.input_mut(|i| i.consume_key(egui::Modifiers::COMMAND, egui::Key::Equals));
        let mut do_attach = false;
        ui.horizontal(|ui| {
            if ui
                .button("∑ Avaliar")
                .on_hover_text("Resolver 'expr =' na linha · Cmd/Ctrl+=")
                .clicked()
            {
                do_eval = true;
            }
            if ui
                .button("📎 Anexar")
                .on_hover_text("Inserir imagem/arquivo como embed")
                .clicked()
            {
                do_attach = true;
            }
            ui.label(RichText::new("Digite \"/\" para comandos").small().weak());
        });

        // Render the text area, capturing cursor position + change.
        let cfg = self.cfg.clone();
        let mut cursor_byte = 0usize;
        let mut changed = false;
        if let Some(n) = self.active_note.as_mut() {
            let mut layouter = |ui: &egui::Ui, text: &str, wrap: f32| {
                crate::theme::body_layout(ui.ctx(), &cfg, text, wrap)
            };
            let output = egui::TextEdit::multiline(&mut n.content)
                .layouter(&mut layouter)
                .frame(false)
                .desired_width(f32::INFINITY)
                .desired_rows(18)
                .show(ui);
            changed = output.response.changed();
            if let Some(range) = output.cursor_range {
                cursor_byte = char_to_byte(&n.content, range.primary.ccursor.index);
            }
        }
        if changed {
            self.dirty = true;
            self.last_edit = ui.input(|i| i.time);
        }
        if do_eval {
            self.eval_at(cursor_byte);
        }
        if do_attach {
            self.attach_embed(cursor_byte);
        }

        self.slash_menu(ui, cursor_byte);
    }

    fn eval_at(&mut self, pos: usize) {
        let sub = self
            .active_note
            .as_ref()
            .and_then(|n| autoformat::try_math_substitute(&n.content, pos));
        if let Some((new_line, start, end)) = sub {
            if let Some(n) = self.active_note.as_mut() {
                n.content.replace_range(start..end, &new_line);
            }
            self.dirty = true;
            self.status = "= avaliado".to_string();
        }
    }

    fn attach_embed(&mut self, pos: usize) {
        let src = match rfd::FileDialog::new().pick_file() {
            Some(p) => p,
            None => return,
        };
        let stored = match self.vault.as_ref() {
            Some(v) => v.import_attachment(&src),
            None => return,
        };
        match stored {
            Ok(filename) => {
                let embed = format!("![[{}]]", filename);
                if let Some(n) = self.active_note.as_mut() {
                    let at = pos.min(n.content.len());
                    n.content.insert_str(at, &embed);
                }
                self.dirty = true;
            }
            Err(e) => self.status = format!("Erro ao anexar: {e}"),
        }
    }

    /// Detect a `/command` token ending at the cursor and offer a command popup.
    fn slash_menu(&mut self, ui: &mut egui::Ui, cursor_byte: usize) {
        let content = match &self.active_note {
            Some(n) => n.content.clone(),
            None => return,
        };
        let (line, line_start, _) = autoformat::current_line(&content, cursor_byte);
        let col = cursor_byte.saturating_sub(line_start);
        let prefix = &line[..col.min(line.len())];
        let slash_rel = match prefix.rfind('/') {
            Some(p) => p,
            None => return,
        };
        // Only trigger when "/" starts the line or follows whitespace.
        if slash_rel > 0 && !prefix[..slash_rel].ends_with(char::is_whitespace) {
            return;
        }
        let query = &prefix[slash_rel + 1..];
        if !query.chars().all(|c| c.is_alphanumeric()) {
            return;
        }
        let token_start = line_start + slash_rel;
        let token_end = cursor_byte;

        let mut apply: Option<String> = None;
        egui::Area::new(egui::Id::new("almanac_slash"))
            .order(egui::Order::Foreground)
            .fixed_pos(ui.next_widget_position())
            .show(ui.ctx(), |ui| {
                egui::Frame::popup(ui.style()).show(ui, |ui| {
                    ui.set_max_width(240.0);
                    let q = query.to_lowercase();
                    for (key, label, snippet) in SLASH_COMMANDS {
                        if !q.is_empty() && !key.starts_with(&q) {
                            continue;
                        }
                        if ui
                            .add(
                                egui::Label::new(format!("/{key}  —  {label}"))
                                    .sense(egui::Sense::click()),
                            )
                            .clicked()
                        {
                            apply = Some((*snippet).to_string());
                        }
                    }
                });
            });

        if let Some(snippet) = apply {
            let snippet = if snippet == "@date" {
                date_today()
            } else {
                snippet
            };
            if let Some(n) = self.active_note.as_mut() {
                if token_end <= n.content.len() {
                    n.content.replace_range(token_start..token_end, &snippet);
                }
            }
            self.dirty = true;
        }
    }

    /// Rewrite Obsidian-style `![[asset]]` embeds so the markdown viewer renders
    /// images inline (via egui_extras' file loader) and turns file embeds into
    /// openable links.
    fn preprocess_embeds(&self, content: &str) -> String {
        let v = match &self.vault {
            Some(v) => v,
            None => return content.to_string(),
        };
        let mut out = content.to_string();
        // Replace back-to-front so earlier byte offsets stay valid.
        for (w, range) in wikilinks::extract_spans(content).into_iter().rev() {
            let replacement = match w {
                Wikilink::Image(e) => v
                    .attachment_path(&e.path)
                    .map(|p| format!("![{}]({})", e.alias.unwrap_or_default(), file_url(&p))),
                Wikilink::File(e) => v
                    .attachment_path(&e.path)
                    .map(|p| format!("[📄 {}]({})", e.path, file_url(&p))),
                _ => None,
            };
            if let Some(rep) = replacement {
                out.replace_range(range, &rep);
            }
        }
        out
    }

    pub fn show_connections(&mut self, ctx: &Context) {
        if self.active_note.is_none() {
            return;
        }
        let (out_links, backlinks) = self.compute_connections();
        let mut nav: Option<PathBuf> = None;

        egui::SidePanel::right("almanac_connections")
            .resizable(true)
            .default_width(214.0)
            .width_range(160.0..=320.0)
            .show(ctx, |ui| {
                ui.add_space(6.0);
                ui.label(RichText::new("Conexões").strong());
                ui.separator();

                ui.label(RichText::new("Links desta nota").small().weak());
                if out_links.is_empty() {
                    ui.label(RichText::new("—").weak().small());
                }
                for (label, rel) in &out_links {
                    let text = match rel {
                        Some(_) => RichText::new(label),
                        None => RichText::new(format!("{label} (sem destino)")).weak(),
                    };
                    if ui.link(text).clicked() {
                        if let Some(r) = rel {
                            nav = Some(r.clone());
                        }
                    }
                }

                ui.add_space(10.0);
                ui.label(RichText::new("Backlinks").small().weak());
                if backlinks.is_empty() {
                    ui.label(RichText::new("—").weak().small());
                }
                for (label, rel) in &backlinks {
                    if ui.link(label).clicked() {
                        nav = Some(rel.clone());
                    }
                }
            });

        if let Some(rel) = nav {
            self.select_note_by_rel(&rel);
        }
    }

    #[allow(clippy::type_complexity)]
    fn compute_connections(&self) -> (Vec<(String, Option<PathBuf>)>, Vec<(String, PathBuf)>) {
        let mut out = Vec::new();
        let mut back = Vec::new();
        if let (Some(n), Some(v)) = (&self.active_note, &self.vault) {
            for w in wikilinks::extract(&n.content) {
                if let Wikilink::Note(r) | Wikilink::NoteEmbed(r) = w {
                    let resolved = v.index.resolve(&r.target).cloned();
                    let label = r.alias.unwrap_or(r.target);
                    out.push((label, resolved));
                }
            }
            for bl in v.index.backlinks_to(&n.rel_path, &v.notes) {
                let label = bl
                    .source
                    .file_stem()
                    .map(|s| s.to_string_lossy().to_string())
                    .unwrap_or_default();
                back.push((label, bl.source.clone()));
            }
        }
        (out, back)
    }
}

fn file_url(p: &std::path::Path) -> String {
    format!("file://{}", p.display())
}

fn date_today() -> String {
    chrono::Local::now().format("%Y-%m-%d").to_string()
}

#[cfg(test)]
mod tests {
    use super::char_to_byte;

    #[test]
    fn char_to_byte_ascii() {
        assert_eq!(char_to_byte("hello", 3), 3);
    }

    #[test]
    fn char_to_byte_multibyte() {
        // "á" is 2 bytes in UTF-8, so char index 1 → byte offset 2.
        assert_eq!(char_to_byte("áé", 1), 2);
    }

    #[test]
    fn char_to_byte_past_end_clamps() {
        assert_eq!(char_to_byte("hi", 99), 2);
    }
}
