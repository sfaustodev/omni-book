use crate::app::OmniNoteApp;
use crate::types::{ConfirmAction, NoteType};
use egui::RichText;
use std::path::Path;

impl OmniNoteApp {
    pub fn show_editor(&mut self, ctx: &egui::Context) {
        egui::CentralPanel::default().show(ctx, |ui| {
            if self.active_note.is_none() {
                ui.centered_and_justified(|ui| {
                    ui.vertical_centered(|ui| {
                        ui.add_space(100.0);
                        ui.label(RichText::new("📓 OmniNote").size(24.0).weak());
                        ui.add_space(16.0);
                        ui.label(RichText::new("Ctrl+N  Nova nota").size(12.0).weak());
                        ui.label(RichText::new("Ctrl+K  Buscar").size(12.0).weak());
                        ui.label(RichText::new("Ctrl+,  Configurações").size(12.0).weak());
                    });
                });
                return;
            }

            // Sticky header
            ui.horizontal(|ui| {
                if let Some(note) = &self.active_note {
                    if let Some(parent) = note.rel_path.parent() {
                        if parent != Path::new("") {
                            ui.label(RichText::new(parent.to_string_lossy().as_ref()).weak().size(11.0));
                            ui.label(RichText::new("·").weak());
                        }
                    }
                }
                if ui.selectable_label(self.editing, "✎ Editar")
                    .on_hover_text("Ctrl+E")
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

            egui::ScrollArea::vertical().id_salt("editor_scroll").show(ui, |ui| {
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
        if ui.add(
            egui::TextEdit::singleline(&mut note.title)
                .font(egui::TextStyle::Heading)
                .hint_text("Título da nota")
                .desired_width(f32::INFINITY),
        ).changed() {
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
                        if ui.selectable_label(note.frontmatter.note_type == t,
                            format!("{} {}", t.icon(), t.label())).clicked()
                        {
                            note.frontmatter.note_type = t;
                            self.dirty = true;
                        }
                    }
                });

            ui.label("Tags:");
            let tags_str = note.frontmatter.tags.join(", ");
            let mut tags_edit = tags_str.clone();
            if ui.add(
                egui::TextEdit::singleline(&mut tags_edit)
                    .hint_text("rust, prog, ...")
                    .desired_width(160.0),
            ).changed() && tags_edit != tags_str {
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
                if ui.add(egui::TextEdit::singleline(&mut note.frontmatter.source)
                    .desired_width(150.0)).changed() { self.dirty = true; }
                ui.label("URL:");
                if ui.add(egui::TextEdit::singleline(&mut note.frontmatter.source_link)
                    .desired_width(200.0)).changed() { self.dirty = true; }
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

        if has_focus && ui.input(|i| i.key_pressed(egui::Key::Equals) && i.modifiers.ctrl) {
            let pos = cursor_pos.unwrap_or(note.content.len());
            if let Some((new_line, start, end)) =
                crate::autoformat::try_math_substitute(&note.content, pos)
            {
                note.content.replace_range(start..end, &new_line);
                self.dirty = true;
            }
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

        egui_commonmark::CommonMarkViewer::new().show(
            ui,
            &mut self.md_cache,
            &note.content,
        );
        ui.separator();

        // Backlinks
        let backlinks: Vec<(String, String)> = if let Some(v) = &self.vault {
            v.notes
                .iter()
                .filter(|n| {
                    n.frontmatter.id != note.frontmatter.id
                        && (n.frontmatter.linked_note.as_deref()
                            == Some(&note.frontmatter.id)
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
}
