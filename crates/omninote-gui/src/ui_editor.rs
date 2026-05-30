use crate::app::OmniNoteApp;
use egui::RichText;
use omninote_core::types::{ConfirmAction, NoteType};
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
        egui::CentralPanel::default().show(ctx, |ui| {
            if self.active_note.is_none() {
                ui.centered_and_justified(|ui| {
                    ui.vertical_centered(|ui| {
                        ui.add_space(100.0);
                        ui.label(RichText::new("📓 OmniNote").size(24.0).weak());
                        ui.add_space(16.0);
                        ui.label(RichText::new("Cmd+N  Nova nota").size(12.0).weak());
                        ui.label(RichText::new("Cmd+K  Buscar").size(12.0).weak());
                        ui.label(RichText::new("Cmd+,  Configurações").size(12.0).weak());
                    });
                });
                return;
            }

            // Sticky header
            ui.horizontal(|ui| {
                if let Some(note) = &self.active_note {
                    if let Some(parent) = note.rel_path.parent() {
                        if parent != Path::new("") {
                            ui.label(
                                RichText::new(parent.to_string_lossy().as_ref())
                                    .weak()
                                    .size(11.0),
                            );
                            ui.label(RichText::new("·").weak());
                        }
                    }
                }
                if ui
                    .selectable_label(self.editing, "✎ Editar")
                    .on_hover_text("Cmd+E")
                    .clicked()
                {
                    self.editing = !self.editing;
                }
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    if let Some(id) = self.active_note.as_ref().map(|n| n.frontmatter.id.clone()) {
                        if ui.button("🗑").on_hover_text("Deletar nota").clicked() {
                            self.confirm_action = Some(ConfirmAction::DeleteNote(id));
                        }
                    }
                });
            });
            ui.separator();

            egui::ScrollArea::vertical()
                .id_salt("editor_scroll")
                .show(ui, |ui| {
                    if self.editing {
                        self.show_edit_panel(ui);
                    } else {
                        self.show_view_panel(ui);
                    }
                });
        });
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

        // Cmd+= math substitution on current line
        // Extract cursor pos before output is dropped (output holds &mut note.content)
        let cursor_pos = output.cursor_range.map(|r| r.primary.ccursor.index);
        let has_focus = output.response.has_focus();
        drop(output);

        let math_sc = egui::KeyboardShortcut::new(egui::Modifiers::COMMAND, egui::Key::Equals);
        if has_focus && ui.input_mut(|i| i.consume_shortcut(&math_sc)) {
            let pos = cursor_pos.unwrap_or(note.content.len());
            if let Some((new_line, start, end)) =
                omninote_core::autoformat::try_math_substitute(&note.content, pos)
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
        let note = match self.active_note.clone() {
            Some(n) => n,
            None => return,
        };

        ui.heading(&note.title);

        if !note.frontmatter.tags.is_empty() {
            ui.horizontal_wrapped(|ui| {
                for tag in &note.frontmatter.tags {
                    if ui.link(format!("#{}", tag)).clicked() {
                        self.query = tag.clone();
                    }
                }
            });
        }

        if note.frontmatter.note_type == NoteType::Citacao && !note.frontmatter.source.is_empty() {
            ui.horizontal(|ui| {
                ui.label("📖");
                ui.label(&note.frontmatter.source);
                if !note.frontmatter.source_link.is_empty() {
                    ui.hyperlink_to("🔗 link", &note.frontmatter.source_link);
                }
            });
        }
        ui.separator();

        egui_commonmark::CommonMarkViewer::new().show(ui, &mut self.md_cache, &note.content);
        ui.separator();

        // Wikilinks + embeds (v0.4 — CAD-10)
        let wikis = omninote_core::wikilinks::extract(&note.content);
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
    ///
    /// CAD-20: handles new grammar with aliases (`[[A|label]]`), anchors
    /// (`[[A#H]]` / `[[A#^id]]`), path-based targets (`[[folder/A]]`), and
    /// note-embed (`![[A]]`). Display uses alias if provided, otherwise target.
    fn render_wikilinks(
        &mut self,
        ui: &mut egui::Ui,
        wikis: &[omninote_core::wikilinks::Wikilink],
    ) {
        use omninote_core::wikilinks::Wikilink;
        use std::collections::HashSet;

        // Dedupe by display key while preserving order. Notes and note-embeds
        // are surfaced together — both navigate to the same target.
        let mut seen: HashSet<String> = HashSet::new();
        // (display_label, target_to_resolve)
        let mut notes: Vec<(String, String)> = Vec::new();
        let mut images: Vec<String> = Vec::new();
        let mut files: Vec<String> = Vec::new();
        for w in wikis {
            let key = match w {
                Wikilink::Note(r) | Wikilink::NoteEmbed(r) => format!("n:{}", r.target),
                Wikilink::Image(e) => format!("i:{}", e.path),
                Wikilink::File(e) => format!("f:{}", e.path),
            };
            if !seen.insert(key) {
                continue;
            }
            match w {
                Wikilink::Note(r) | Wikilink::NoteEmbed(r) => {
                    let label = r.alias.clone().unwrap_or_else(|| r.target.clone());
                    notes.push((label, r.target.clone()));
                }
                Wikilink::Image(e) => images.push(e.path.clone()),
                Wikilink::File(e) => files.push(e.path.clone()),
            }
        }

        // Notes referenciadas (CAD-20: resolves via VaultIndex)
        if !notes.is_empty() {
            ui.collapsing(format!("🔗 Notas referenciadas ({})", notes.len()), |ui| {
                let mut pending_select: Option<String> = None;
                let mut pending_create: Option<String> = None;
                for (label, target) in &notes {
                    let resolved = self
                        .vault
                        .as_ref()
                        .and_then(|v| v.index.resolve(target).cloned());

                    ui.horizontal(|ui| {
                        if resolved.is_some() {
                            if ui.link(format!("→ {}", label)).clicked() {
                                pending_select = Some(target.clone());
                            }
                        } else {
                            ui.label(egui::RichText::new(format!("⚠ {}", label)).weak().italics())
                                .on_hover_text(format!("Link não resolve: {target}"));
                            if ui.small_button("➕ criar").clicked() {
                                pending_create = Some(target.clone());
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
            match v.create_note(None, title, omninote_core::types::NoteType::default()) {
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
