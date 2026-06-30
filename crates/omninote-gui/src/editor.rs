//! Central panel: the edit/read surface. Edit mode is a multiline `TextEdit` wired
//! to the slash menu and inline math evaluation; read mode delegates to `md_render`
//! and appends a backlinks section. Mutations are deferred until after the panel
//! closure so the `&mut active_note` borrow doesn't collide with `&mut self` calls.

use omninote_core::autoformat;

use crate::app::{char_to_byte, OmniNoteApp, SLASH_ITEMS};

impl OmniNoteApp {
    pub fn show_editor(&mut self, ctx: &egui::Context) {
        let theme = self.theme;

        // Editor-local shortcut: evaluate math on the current line (Cmd/Ctrl+=).
        let do_math = ctx.input_mut(|i| {
            i.consume_shortcut(&egui::KeyboardShortcut::new(
                egui::Modifiers::COMMAND,
                egui::Key::Equals,
            ))
        });

        let mut nav: Option<String> = None;
        let mut back_nav: Option<String> = None;
        let mut changed = false;
        let mut new_cursor = self.cursor_byte;

        egui::CentralPanel::default()
            .frame(egui::Frame::none().fill(theme.bg).inner_margin(16.0))
            .show(ctx, |ui| {
                if self.active_note.is_none() {
                    self.empty_state(ui);
                    return;
                }

                // Header: type badge + title + mode toggle.
                ui.horizontal(|ui| {
                    if let Some(note) = self.active_note.as_ref() {
                        let ty = note.frontmatter.note_type;
                        ui.label(
                            egui::RichText::new(format!("{} {}", ty.icon(), ty.label()))
                                .color(theme.note_type_color(ty)),
                        );
                        ui.label(egui::RichText::new(&note.title).heading().color(theme.text));
                    }
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        let label = if self.editing {
                            "👁 Ler"
                        } else {
                            "✎ Editar"
                        };
                        if ui
                            .button(label)
                            .on_hover_text("Alternar edição/leitura (Cmd/Ctrl+E)")
                            .clicked()
                        {
                            self.editing = !self.editing;
                        }
                        if ui.button("🗑").on_hover_text("Apagar nota").clicked() {
                            if let Some(id) = self.active_id.clone() {
                                self.pending = Some(crate::app::Pending::DeleteNote(id));
                            }
                        }
                    });
                });
                ui.separator();

                if self.editing {
                    egui::ScrollArea::vertical()
                        .auto_shrink([false, false])
                        .show(ui, |ui| {
                            if let Some(note) = self.active_note.as_mut() {
                                let out = egui::TextEdit::multiline(&mut note.content)
                                    .desired_width(f32::INFINITY)
                                    .desired_rows(22)
                                    .font(egui::TextStyle::Body)
                                    .show(ui);
                                if out.response.changed() {
                                    changed = true;
                                }
                                if let Some(r) = out.cursor_range {
                                    new_cursor =
                                        char_to_byte(&note.content, r.primary.ccursor.index);
                                }
                            }
                            ui.add_space(6.0);
                            ui.label(
                                egui::RichText::new(
                                    "/ menu · Cmd/Ctrl+= avalia matemática na linha",
                                )
                                .small()
                                .color(theme.faint),
                            );
                        });
                } else {
                    egui::ScrollArea::vertical()
                        .auto_shrink([false, false])
                        .show(ui, |ui| {
                            nav = self.render_view(ui);
                            back_nav = self.show_backlinks(ui);
                        });
                }
            });

        // Deferred mutations (borrows released).
        self.cursor_byte = new_cursor;
        if changed {
            self.mark_dirty();
        }
        if let Some(t) = nav {
            self.nav_target = Some(t);
        }
        if let Some(id) = back_nav {
            self.select_note(&id);
        }
        if do_math {
            self.eval_math();
        }

        self.update_slash_state();
        if let Some(snippet) = self.show_slash_menu(ctx) {
            self.apply_slash(snippet);
        }
    }

    fn empty_state(&mut self, ui: &mut egui::Ui) {
        let theme = self.theme;
        ui.add_space(40.0);
        ui.vertical_centered(|ui| {
            ui.label(
                egui::RichText::new("OmniNote")
                    .size(34.0)
                    .color(theme.accent)
                    .family(egui::FontFamily::Name("display".into())),
            );
            ui.label(
                egui::RichText::new("Selecione uma nota na barra lateral, ou crie a primeira.")
                    .color(theme.dim),
            );
            ui.add_space(12.0);
            if ui.button("+ Nova nota  (Cmd/Ctrl+N)").clicked() {
                self.new_in_folder = None;
                self.show_new = true;
            }
        });
    }

    /// Backlinks panel under the rendered note; returns an id to open if clicked.
    fn show_backlinks(&self, ui: &mut egui::Ui) -> Option<String> {
        let theme = self.theme;
        let rel = self.active_note.as_ref()?.rel_path.clone();
        let links: Vec<(String, String)> = self
            .vault
            .as_ref()
            .map(|v| {
                v.index
                    .backlinks_to(&rel, &v.notes)
                    .iter()
                    .filter_map(|b| {
                        v.notes
                            .iter()
                            .find(|n| n.rel_path == b.source)
                            .map(|n| (n.frontmatter.id.clone(), n.title.clone()))
                    })
                    .collect()
            })
            .unwrap_or_default();
        if links.is_empty() {
            return None;
        }

        let mut pick = None;
        ui.add_space(16.0);
        ui.separator();
        ui.label(
            egui::RichText::new("↩ BACKLINKS")
                .small()
                .strong()
                .color(theme.dim),
        );
        for (id, title) in links {
            if ui
                .link(egui::RichText::new(format!("• {title}")).color(theme.accent))
                .clicked()
            {
                pick = Some(id);
            }
        }
        pick
    }

    fn eval_math(&mut self) {
        if !self.editing {
            return;
        }
        let cb = self.cursor_byte;
        let res = self
            .active_note
            .as_ref()
            .and_then(|n| autoformat::try_math_substitute(&n.content, cb));
        if let Some((line, start, end)) = res {
            if let Some(note) = self.active_note.as_mut() {
                if end <= note.content.len() {
                    note.content.replace_range(start..end, &line);
                }
            }
            self.mark_dirty();
        }
    }

    /// Track whether a `/` trigger is active at the cursor.
    fn update_slash_state(&mut self) {
        if !self.editing {
            self.slash_at = None;
            return;
        }
        let Some(note) = self.active_note.as_ref() else {
            self.slash_at = None;
            return;
        };
        let cb = self.cursor_byte.min(note.content.len());
        let bytes = note.content.as_bytes();

        // Invalidate a stale trigger (cursor moved before it, or filter got non-alnum).
        if let Some(at) = self.slash_at {
            let valid = at < note.content.len()
                && bytes[at] == b'/'
                && cb > at
                && note.content[at + 1..cb]
                    .chars()
                    .all(|c| c.is_alphanumeric());
            if !valid {
                self.slash_at = None;
            }
        }

        // Detect a fresh trigger: `/` right before the cursor at a word boundary.
        if self.slash_at.is_none() && cb >= 1 && bytes[cb - 1] == b'/' {
            let prefix = &note.content[..cb - 1];
            if prefix.is_empty() || prefix.ends_with([' ', '\n', '\t']) {
                self.slash_at = Some(cb - 1);
                self.slash_sel = 0;
            }
        }
    }

    /// Render the slash popup; returns the chosen snippet, if any.
    fn show_slash_menu(&mut self, ctx: &egui::Context) -> Option<&'static str> {
        let at = self.slash_at?;
        let theme = self.theme;
        let query = self
            .active_note
            .as_ref()
            .map(|n| {
                let cb = self.cursor_byte.min(n.content.len()).max(at + 1);
                n.content[at + 1..cb].to_lowercase()
            })
            .unwrap_or_default();

        let items: Vec<&'static (&'static str, &'static str)> = SLASH_ITEMS
            .iter()
            .filter(|(label, _)| query.is_empty() || label.to_lowercase().contains(&query))
            .collect();
        if items.is_empty() {
            return None;
        }

        let mut pick = None;
        egui::Area::new(egui::Id::new("slash-menu"))
            .anchor(egui::Align2::CENTER_TOP, egui::vec2(0.0, 120.0))
            .show(ctx, |ui| {
                egui::Frame::popup(ui.style())
                    .fill(theme.panel)
                    .stroke(egui::Stroke::new(1.0, theme.accent))
                    .show(ui, |ui| {
                        ui.label(egui::RichText::new("MENU /").small().color(theme.faint));
                        for (label, snippet) in items {
                            if ui.selectable_label(false, *label).clicked() {
                                pick = Some(*snippet);
                            }
                        }
                    });
            });
        pick
    }

    fn apply_slash(&mut self, snippet: &str) {
        if let Some(at) = self.slash_at.take() {
            let cb = self.cursor_byte;
            if let Some(note) = self.active_note.as_mut() {
                let end = cb.min(note.content.len()).max(at);
                note.content.replace_range(at..end, snippet);
            }
            self.mark_dirty();
        }
    }
}
