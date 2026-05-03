use crate::app::OmniNoteApp;
use crate::types::{ConfirmAction, NoteType};
use egui::RichText;
#[allow(unused_imports)]
use std::path::Path;

/// v0.8 — items shown in the slash menu when user types `/` at start of a line.
/// Returns (label, snippet). Snippet replaces the `/` character.
fn slash_menu_items() -> &'static [(&'static str, &'static str)] {
    &[
        ("# H1", "# "),
        ("## H2", "## "),
        ("### H3", "### "),
        ("**negrito**", "****"),
        ("_itálico_", "__"),
        ("`código inline`", "``"),
        ("```bloco de código```", "```\n\n```"),
        ("> citação", "> "),
        ("- lista", "- "),
        ("1. lista numerada", "1. "),
        ("- [ ] todo", "- [ ] "),
        ("[link](url)", "[](url)"),
        ("[[wikilink]]", "[[]]"),
        ("--- divisor", "---\n"),
    ]
}

impl OmniNoteApp {
    pub fn show_editor(&mut self, ctx: &egui::Context) {
        use crate::theme;
        egui::CentralPanel::default()
            .frame(egui::Frame::none().fill(theme::BG))
            .show(ctx, |ui| {
                if self.active_note.is_none() {
                    self.swiss_empty_state(ui);
                    return;
                }

                // Top rule (48px) — three columns: breadcrumb | TITLE | edited time
                self.swiss_top_rule(ui);

                // Main content area — left rail (120px) + content
                egui::ScrollArea::vertical()
                    .id_salt("editor_scroll")
                    .show(ui, |ui| {
                        ui.add_space(48.0);
                        ui.horizontal_top(|ui| {
                            ui.add_space(48.0);
                            ui.allocate_ui_with_layout(
                                egui::vec2(120.0, ui.available_height()),
                                egui::Layout::top_down(egui::Align::Min),
                                |ui| self.swiss_left_rail(ui),
                            );
                            ui.add_space(32.0);
                            ui.allocate_ui_with_layout(
                                egui::vec2(
                                    ui.available_width() - 48.0,
                                    ui.available_height(),
                                ),
                                egui::Layout::top_down(egui::Align::Min),
                                |ui| {
                                    self.swiss_content(ui);
                                },
                            );
                        });
                    });
            });
    }

    fn swiss_empty_state(&mut self, ui: &mut egui::Ui) {
        use crate::theme;
        ui.centered_and_justified(|ui| {
            ui.vertical_centered(|ui| {
                ui.add_space(120.0);
                let (rect, _) = ui.allocate_exact_size(
                    egui::vec2(48.0, 48.0),
                    egui::Sense::hover(),
                );
                ui.painter().rect_filled(rect, 0.0, theme::ACCENT);
                ui.add_space(20.0);
                ui.label(
                    RichText::new("OmniNote")
                        .strong()
                        .size(32.0)
                        .color(theme::TEXT),
                );
                ui.add_space(28.0);
                for (kc, label) in [
                    ("⌘ N", "Nova nota"),
                    ("⌘ K", "Buscar"),
                    ("⌘ ,", "Configurações"),
                ] {
                    ui.horizontal(|ui| {
                        ui.add_space(ui.available_width() / 2.0 - 80.0);
                        ui.label(
                            RichText::new(kc)
                                .monospace()
                                .size(11.0)
                                .color(theme::ACCENT),
                        );
                        ui.add_space(12.0);
                        ui.label(
                            RichText::new(label)
                                .size(13.0)
                                .color(theme::DIM),
                        );
                    });
                }
            });
        });
    }

    fn swiss_top_rule(&mut self, ui: &mut egui::Ui) {
        use crate::theme;
        let id_text = self
            .active_note
            .as_ref()
            .map(|n| {
                let id = &n.frontmatter.id;
                // last 3 hex chars from uuid simple
                let tail: String = id.chars().rev().take(3).collect::<String>().chars().rev().collect();
                let folder = n
                    .rel_path
                    .parent()
                    .and_then(|p| p.file_name())
                    .and_then(|s| s.to_str())
                    .unwrap_or("ROOT")
                    .to_uppercase();
                format!("{} / {}", folder, tail)
            })
            .unwrap_or_else(|| "—".into());

        let title_text = self
            .active_note
            .as_ref()
            .map(|n| n.title.to_uppercase())
            .unwrap_or_default();

        let edited_text = self
            .active_note
            .as_ref()
            .map(|_| "EDITED NOW".to_string())
            .unwrap_or_default();

        let bar = egui::Frame::none()
            .fill(theme::BG)
            .stroke(egui::Stroke::new(1.0, theme::BORDER))
            .inner_margin(egui::Margin::symmetric(32.0, 14.0));
        bar.show(ui, |ui| {
            ui.set_height(20.0);
            ui.columns(3, |cols| {
                cols[0].with_layout(egui::Layout::left_to_right(egui::Align::Center), |ui| {
                    ui.label(
                        RichText::new(id_text)
                            .monospace()
                            .size(10.0)
                            .color(theme::DIMMER),
                    );
                });
                cols[1].with_layout(egui::Layout::top_down(egui::Align::Center), |ui| {
                    let id = self
                        .active_note
                        .as_ref()
                        .map(|n| n.frontmatter.id.clone())
                        .unwrap_or_default();
                    ui.horizontal(|ui| {
                        let editing_label = if self.editing { "EDIT" } else { "READ" };
                        if ui
                            .selectable_label(
                                self.editing,
                                RichText::new(editing_label)
                                    .monospace()
                                    .size(10.0)
                                    .color(if self.editing { theme::ACCENT } else { theme::DIMMER }),
                            )
                            .on_hover_text("⌘E")
                            .clicked()
                        {
                            self.editing = !self.editing;
                        }
                        ui.add_space(12.0);
                        ui.label(
                            RichText::new(title_text.clone())
                                .monospace()
                                .size(10.0)
                                .color(theme::TEXT),
                        );
                        ui.add_space(12.0);
                        if !id.is_empty()
                            && ui
                                .small_button(
                                    RichText::new("DELETE")
                                        .monospace()
                                        .size(10.0)
                                        .color(theme::DIMMER),
                                )
                                .clicked()
                        {
                            self.confirm_action = Some(ConfirmAction::DeleteNote(id));
                        }
                    });
                });
                cols[2].with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    ui.label(
                        RichText::new(edited_text)
                            .monospace()
                            .size(10.0)
                            .color(theme::DIMMER),
                    );
                });
            });
        });
    }

    fn swiss_left_rail(&self, ui: &mut egui::Ui) {
        use crate::theme;
        let (number, type_label, version, date) = match &self.active_note {
            Some(note) => {
                // Use last 3 hex chars of id as the "№"
                let id = &note.frontmatter.id;
                let tail: String = id.chars().rev().take(3).collect::<String>().chars().rev().collect();
                let date = note
                    .frontmatter
                    .created
                    .split('T')
                    .next()
                    .unwrap_or("")
                    .replace('-', "·");
                (
                    format!("№ {}", tail.to_uppercase()),
                    note.frontmatter.note_type.label().to_uppercase(),
                    "v1.0".to_string(),
                    if date.is_empty() {
                        "—".to_string()
                    } else {
                        date
                    },
                )
            }
            None => ("№ —".into(), "".into(), "v1.0".into(), "—".into()),
        };

        ui.label(
            RichText::new(number)
                .monospace()
                .size(10.0)
                .color(theme::ACCENT),
        );
        ui.add_space(2.0);
        ui.label(
            RichText::new(type_label)
                .monospace()
                .size(10.0)
                .color(theme::DIMMER),
        );
        ui.add_space(14.0);
        let (rule_rect, _) =
            ui.allocate_exact_size(egui::vec2(40.0, 1.0), egui::Sense::hover());
        ui.painter().rect_filled(rule_rect, 0.0, theme::DIMMER);
        ui.add_space(14.0);
        ui.label(
            RichText::new(version)
                .monospace()
                .size(10.0)
                .color(theme::DIMMER),
        );
        ui.label(
            RichText::new(date)
                .monospace()
                .size(10.0)
                .color(theme::DIMMER),
        );
        ui.label(
            RichText::new("J. FAUSTA")
                .monospace()
                .size(10.0)
                .color(theme::DIMMER),
        );
    }

    fn swiss_content(&mut self, ui: &mut egui::Ui) {
        if self.editing {
            self.show_edit_panel(ui);
        } else {
            self.show_view_panel(ui);
        }
    }

    fn show_edit_panel(&mut self, ui: &mut egui::Ui) {
        let note = match self.active_note.as_mut() {
            Some(n) => n,
            None => return,
        };

        // Title
        if ui
            .add(
                egui::TextEdit::singleline(&mut note.title)
                    .font(egui::TextStyle::Heading)
                    .hint_text("Título da nota")
                    .desired_width(f32::INFINITY),
            )
            .changed()
        {
            self.dirty = true;
        }

        // Metadata row
        ui.horizontal(|ui| {
            ui.label("Tipo:");
            let current = note.frontmatter.note_type;
            egui::ComboBox::from_id_salt("note_type_combo")
                .selected_text(format!("{} {}", current.icon(), current.label()))
                .show_ui(ui, |ui| {
                    for t in NoteType::all() {
                        if ui
                            .selectable_label(
                                note.frontmatter.note_type == t,
                                format!("{} {}", t.icon(), t.label()),
                            )
                            .clicked()
                        {
                            note.frontmatter.note_type = t;
                            self.dirty = true;
                        }
                    }
                });

            ui.label("Tags:");
            let tags_str = note.frontmatter.tags.join(", ");
            let mut tags_edit = tags_str.clone();
            if ui
                .add(
                    egui::TextEdit::singleline(&mut tags_edit)
                        .hint_text("rust, prog, ...")
                        .desired_width(160.0),
                )
                .changed()
                && tags_edit != tags_str
            {
                note.frontmatter.tags = tags_edit
                    .split(',')
                    .map(|s| s.trim().to_string())
                    .filter(|s| !s.is_empty())
                    .collect();
                self.dirty = true;
            }
        });

        // Citation-only fields
        if note.frontmatter.note_type == NoteType::Citacao {
            ui.horizontal(|ui| {
                ui.label("Fonte:");
                if ui
                    .add(
                        egui::TextEdit::singleline(&mut note.frontmatter.source)
                            .desired_width(150.0),
                    )
                    .changed()
                {
                    self.dirty = true;
                }
                ui.label("URL:");
                if ui
                    .add(
                        egui::TextEdit::singleline(&mut note.frontmatter.source_link)
                            .desired_width(200.0),
                    )
                    .changed()
                {
                    self.dirty = true;
                }
            });
        }
        ui.separator();

        // Content editor — use show() to access cursor_range for math substitution
        let output = egui::TextEdit::multiline(&mut note.content)
            .code_editor()
            .desired_rows(30)
            .desired_width(ui.available_width())
            .hint_text("Escreva em markdown...")
            .show(ui);

        if output.response.changed() {
            self.dirty = true;
        }

        // Ctrl+= math substitution on current line
        // Extract cursor pos before output is dropped (output holds &mut note.content)
        let cursor_pos = output.cursor_range.map(|r| r.primary.ccursor.index);
        let has_focus = output.response.has_focus();
        drop(output);

        let math_sc =
            egui::KeyboardShortcut::new(egui::Modifiers::COMMAND, egui::Key::Equals);
        if has_focus && ui.ctx().input_mut(|i| i.consume_shortcut(&math_sc)) {
            let pos = cursor_pos.unwrap_or(note.content.len());
            if let Some((new_line, start, end)) =
                crate::autoformat::try_math_substitute(&note.content, pos)
            {
                note.content.replace_range(start..end, &new_line);
                self.dirty = true;
            }
        }

        // v0.8 — Slash menu: detect "/" at start of a line
        if has_focus {
            if let Some(pos) = cursor_pos {
                let bytes = note.content.as_bytes();
                if pos > 0 && bytes[pos - 1] == b'/' {
                    let at_line_start = pos == 1 || bytes[pos - 2] == b'\n';
                    if at_line_start && self.slash_menu_pos != Some(pos - 1) {
                        self.slash_menu_pos = Some(pos - 1);
                    }
                } else if let Some(slash_at) = self.slash_menu_pos {
                    // Close if cursor moved away from the slash position area
                    let still_valid = pos >= slash_at
                        && pos <= note.content.len()
                        && bytes.get(slash_at).copied() == Some(b'/');
                    if !still_valid {
                        self.slash_menu_pos = None;
                    }
                }
            }
        }

        // Esc closes the slash menu
        if self.slash_menu_pos.is_some() && ui.input(|i| i.key_pressed(egui::Key::Escape)) {
            self.slash_menu_pos = None;
        }

        // Render slash menu popup
        let mut pending_replacement: Option<(usize, &'static str)> = None;
        if let Some(slash_at) = self.slash_menu_pos {
            egui::Window::new("Inserir bloco")
                .id(egui::Id::new("slash_menu"))
                .collapsible(false)
                .resizable(false)
                .anchor(egui::Align2::CENTER_TOP, [0.0, 80.0])
                .show(ui.ctx(), |ui| {
                    ui.label(
                        egui::RichText::new("/ no início da linha — Esc fecha")
                            .size(10.0)
                            .weak(),
                    );
                    ui.separator();
                    for (label, snippet) in slash_menu_items() {
                        if ui.button(*label).clicked() {
                            pending_replacement = Some((slash_at, *snippet));
                        }
                    }
                });
        }
        if let Some((slash_at, snippet)) = pending_replacement {
            // Replace the "/" with the snippet
            note.content.replace_range(slash_at..slash_at + 1, snippet);
            self.dirty = true;
            self.slash_menu_pos = None;
        }

        // Attach file
        ui.horizontal(|ui| {
            if ui.button("📎 Anexar arquivo").clicked() {
                if let Some(path) = rfd::FileDialog::new().pick_file() {
                    if let Some(v) = &self.vault {
                        match v.import_attachment(&path) {
                            Ok(name) => {
                                let note = self.active_note.as_mut().unwrap();
                                note.content.push_str(&format!("\n![[{}]]", name));
                                note.frontmatter.attachments.push(name);
                                self.dirty = true;
                            }
                            Err(e) => self.error_msg = Some(e),
                        }
                    }
                }
            }
        });
    }

    fn show_view_panel(&mut self, ui: &mut egui::Ui) {
        use crate::theme;
        let note = match self.active_note.clone() {
            Some(n) => n,
            None => return,
        };

        // Swiss big title — title text in white, type label in accent below
        ui.label(
            RichText::new(&note.title)
                .strong()
                .size(56.0)
                .color(theme::TEXT),
        );
        ui.label(
            RichText::new(format!("{}.", note.frontmatter.note_type.label().to_lowercase()))
                .strong()
                .size(56.0)
                .color(theme::ACCENT),
        );
        ui.add_space(8.0);

        if !note.frontmatter.tags.is_empty() {
            ui.horizontal_wrapped(|ui| {
                for tag in &note.frontmatter.tags {
                    if ui
                        .link(
                            RichText::new(format!("#{}", tag))
                                .size(13.0)
                                .color(theme::DIM),
                        )
                        .clicked()
                    {
                        self.query = tag.clone();
                    }
                }
            });
            ui.add_space(8.0);
        }

        if note.frontmatter.note_type == NoteType::Citacao && !note.frontmatter.source.is_empty() {
            ui.horizontal(|ui| {
                ui.label(
                    RichText::new("SOURCE —")
                        .monospace()
                        .size(10.0)
                        .color(theme::ACCENT),
                );
                ui.label(
                    RichText::new(&note.frontmatter.source)
                        .size(13.0)
                        .color(theme::TEXT),
                );
                if !note.frontmatter.source_link.is_empty() {
                    ui.hyperlink_to(
                        RichText::new("→ link")
                            .size(13.0)
                            .color(theme::ACCENT),
                        &note.frontmatter.source_link,
                    );
                }
            });
        }
        ui.add_space(28.0);
        theme::hairline(ui);
        ui.add_space(20.0);

        // Body section label
        ui.label(
            RichText::new("01 — CONTENT")
                .monospace()
                .size(10.0)
                .color(theme::ACCENT),
        );
        ui.add_space(8.0);

        egui_commonmark::CommonMarkViewer::new().show(ui, &mut self.md_cache, &note.content);
        ui.separator();

        // Wikilinks + embeds (v0.4 — CAD-10)
        let wikis = crate::wikilinks::extract(&note.content);
        if !wikis.is_empty() {
            self.render_wikilinks(ui, &wikis);
            ui.separator();
        }

        // Backlinks
        let backlinks: Vec<(String, String)> = if let Some(v) = &self.vault {
            v.notes
                .iter()
                .filter(|n| {
                    n.frontmatter.id != note.frontmatter.id
                        && (n.frontmatter.linked_note.as_deref() == Some(&note.frontmatter.id)
                            || n.content.contains(&format!("[[{}]]", note.title)))
                })
                .map(|n| (n.frontmatter.id.clone(), n.title.clone()))
                .collect()
        } else {
            vec![]
        };

        if !backlinks.is_empty() {
            ui.collapsing(format!("🔗 Backlinks ({})", backlinks.len()), |ui| {
                let mut pending: Option<String> = None;
                for (id, title) in &backlinks {
                    if ui.link(format!("← {}", title)).clicked() {
                        pending = Some(id.clone());
                    }
                }
                if let Some(id) = pending {
                    self.select_note(&id);
                }
            });
        }
    }

    /// Render wikilinks (`[[Title]]`) as clickable links and embeds (`![[file]]`)
    /// as inline images (for image extensions) or open buttons (for other files).
    fn render_wikilinks(&mut self, ui: &mut egui::Ui, wikis: &[crate::wikilinks::Wikilink]) {
        use crate::wikilinks::Wikilink;
        use std::collections::HashSet;

        // Dedupe by display key while preserving order
        let mut seen: HashSet<String> = HashSet::new();
        let mut notes: Vec<String> = Vec::new();
        let mut images: Vec<String> = Vec::new();
        let mut files: Vec<String> = Vec::new();
        for w in wikis {
            let key = match w {
                Wikilink::Note(t) => format!("n:{}", t),
                Wikilink::Image(f) => format!("i:{}", f),
                Wikilink::File(f) => format!("f:{}", f),
            };
            if !seen.insert(key) {
                continue;
            }
            match w {
                Wikilink::Note(t) => notes.push(t.clone()),
                Wikilink::Image(f) => images.push(f.clone()),
                Wikilink::File(f) => files.push(f.clone()),
            }
        }

        // Notes referenciadas
        if !notes.is_empty() {
            ui.collapsing(format!("🔗 Notas referenciadas ({})", notes.len()), |ui| {
                let mut pending_select: Option<String> = None;
                let mut pending_create: Option<String> = None;
                for title in &notes {
                    let exists = self
                        .vault
                        .as_ref()
                        .map(|v| v.notes.iter().any(|n| n.title.eq_ignore_ascii_case(title)))
                        .unwrap_or(false);

                    ui.horizontal(|ui| {
                        if exists {
                            if ui.link(format!("→ {}", title)).clicked() {
                                pending_select = Some(title.clone());
                            }
                        } else {
                            ui.label(egui::RichText::new(format!("⚠ {}", title)).weak().italics())
                                .on_hover_text("Nota não existe ainda");
                            if ui.small_button("➕ criar").clicked() {
                                pending_create = Some(title.clone());
                            }
                        }
                    });
                }
                if let Some(t) = pending_select {
                    self.select_note_by_title(&t);
                }
                if let Some(t) = pending_create {
                    self.create_note_from_wikilink(&t);
                }
            });
        }

        // Embeds: imagens
        if !images.is_empty() {
            ui.collapsing(format!("🖼 Imagens ({})", images.len()), |ui| {
                if let Some(v) = &self.vault {
                    for filename in &images {
                        let path = v.root.join("_attachments").join(filename);
                        if path.exists() {
                            let uri = format!("file://{}", path.to_string_lossy());
                            ui.label(egui::RichText::new(filename).size(11.0).weak());
                            ui.add(
                                egui::Image::new(uri)
                                    .max_width(ui.available_width().min(600.0))
                                    .maintain_aspect_ratio(true),
                            );
                        } else {
                            ui.label(
                                egui::RichText::new(format!("⚠ {} (não encontrado)", filename))
                                    .weak(),
                            );
                        }
                    }
                }
            });
        }

        // Embeds: arquivos (PDFs, etc)
        if !files.is_empty() {
            ui.collapsing(format!("📎 Arquivos ({})", files.len()), |ui| {
                if let Some(v) = &self.vault {
                    for filename in &files {
                        let path = v.root.join("_attachments").join(filename);
                        ui.horizontal(|ui| {
                            if path.exists() {
                                if ui.button(format!("📄 Abrir {}", filename)).clicked() {
                                    let _ = open::that(&path);
                                }
                            } else {
                                ui.label(
                                    egui::RichText::new(format!("⚠ {} (não encontrado)", filename))
                                        .weak(),
                                );
                            }
                        });
                    }
                }
            });
        }
    }

    /// Create a new note matching a wikilink target title that doesn't exist yet.
    fn create_note_from_wikilink(&mut self, title: &str) {
        self.flush_active();
        if let Some(v) = &mut self.vault {
            match v.create_note(None, title, crate::types::NoteType::default()) {
                Ok(note) => {
                    self.active_note = Some(note);
                    self.editing = true;
                    self.dirty = false;
                }
                Err(e) => self.error_msg = Some(e),
            }
        }
    }
}
