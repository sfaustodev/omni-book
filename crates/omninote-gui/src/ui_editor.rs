use crate::app::OmniNoteApp;
use egui::RichText;
use omninote_core::types::NoteType;

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

            // Tab strip + breadcrumb (CAD-25 Slice 2) replace the old sticky header.
            self.show_tab_strip(ui);
            self.show_breadcrumb(ui);

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
        // Extract cursor pos before output is dropped (output holds &mut note.content).
        // `ccursor.index` is a CHAR index; autoformat and the slash menu below index
        // `content` by BYTE, so convert once here — a char index used as a byte offset
        // corrupts/erases text (and can panic in replace_range) on non-ASCII lines.
        let cursor_pos = output.cursor_range.map(|r| {
            let char_idx = r.primary.ccursor.index;
            note.content
                .char_indices()
                .nth(char_idx)
                .map(|(byte_idx, _)| byte_idx)
                .unwrap_or(note.content.len())
        });
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

        // Inline renderer (CAD-25 Slice 3a): wikilinks/embeds/#tags rendered in
        // the text flow, replacing the old "CommonMarkViewer + appendix" split.
        // Resolution is delegated to the core VaultIndex (closure + the existing
        // select_note_by_target), so md_render doesn't reimplement it.
        let action = {
            let is_resolved = |target: &str| {
                self.vault
                    .as_ref()
                    .map(|v| v.index.resolve(target).is_some())
                    .unwrap_or(false)
            };
            crate::md_render::render_body(ui, &mut self.md_cache, &note.content, &is_resolved)
        };
        match action {
            Some(crate::md_render::MdAction::Navigate(target)) => {
                self.select_note_by_target(&target);
            }
            Some(crate::md_render::MdAction::FilterTag(tag)) => self.query = tag,
            None => {}
        }

        // Backlinks moved to the right rail (ui_right_rail.rs), which resolves
        // them target-side via the core index instead of a substring match.
    }
}
