use crate::app::OmniNoteApp;
use egui::RichText;
use omninote_core::types::{ConfirmAction, NoteType};
use std::path::{Path, PathBuf};

impl OmniNoteApp {
    pub fn show_sidebar(&mut self, ctx: &egui::Context) {
        egui::SidePanel::left("sidebar")
            .exact_width(280.0)
            .show(ctx, |ui| {
                ui.spacing_mut().item_spacing.y = 4.0;

                // Header
                ui.horizontal(|ui| {
                    ui.label(RichText::new("📓 OmniNote").strong().size(16.0));
                    if let Some(v) = &self.vault {
                        let name = v
                            .root
                            .file_name()
                            .and_then(|s| s.to_str())
                            .unwrap_or("vault");
                        ui.label(RichText::new(name).size(10.0).weak());
                    }
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        if ui
                            .small_button("⚙")
                            .on_hover_text("Configurações (Cmd+,)")
                            .clicked()
                        {
                            self.show_settings = true;
                        }
                        if ui
                            .small_button("☀/🌙")
                            .on_hover_text("Tema (Cmd+Shift+D)")
                            .clicked()
                        {
                            if let Some(v) = &mut self.vault {
                                crate::app::toggle_light_dark(&mut v.config);
                                crate::app::theme_for_config(&v.config).apply(ctx);
                            }
                        }
                        if ui
                            .small_button("📂")
                            .on_hover_text("Trocar vault")
                            .clicked()
                        {
                            self.pick_vault_with_ctx(ctx);
                        }
                        if ui
                            .small_button("⊟")
                            .on_hover_text("Painel direito (backlinks/outline)")
                            .clicked()
                        {
                            if let Some(v) = &mut self.vault {
                                v.config.right_rail_open = !v.config.right_rail_open;
                            }
                        }
                    });
                });
                ui.separator();

                // Search
                let search = ui.add(
                    egui::TextEdit::singleline(&mut self.query)
                        .hint_text("🔍 Buscar... (Cmd+K)")
                        .desired_width(f32::INFINITY),
                );
                if ctx.input(|i| i.key_pressed(egui::Key::K) && i.modifiers.command) {
                    search.request_focus();
                }

                // Type filter chips
                ui.horizontal_wrapped(|ui| {
                    ui.spacing_mut().item_spacing.x = 4.0;
                    if ui
                        .selectable_label(self.type_filter.is_none(), "todos")
                        .clicked()
                    {
                        self.type_filter = None;
                    }
                    for t in NoteType::all() {
                        let selected = self.type_filter == Some(t);
                        if ui
                            .selectable_label(selected, format!("{} {}", t.icon(), t.label()))
                            .clicked()
                        {
                            self.type_filter = if selected { None } else { Some(t) };
                        }
                    }
                });
                ui.separator();

                // Note/folder tree
                egui::ScrollArea::vertical()
                    .id_salt("sidebar_scroll")
                    .show(ui, |ui| {
                        self.show_folder_tree(ui, PathBuf::new());
                        self.show_notes_in_folder(ui, &PathBuf::new());
                    });

                // Footer
                ui.separator();
                ui.horizontal(|ui| {
                    if ui.button("➕ Nota").on_hover_text("Cmd+N").clicked() {
                        self.show_new = true;
                    }
                    if ui.button("📁 Pasta").clicked() {
                        if let Some(v) = &mut self.vault {
                            let _ = v.create_folder(None, "Nova pasta");
                        }
                    }
                    if ui.button("📥 Importar").clicked() {
                        self.show_import = true;
                    }
                });
            });
    }

    fn show_folder_tree(&mut self, ui: &mut egui::Ui, parent: PathBuf) {
        let folders: Vec<PathBuf> = if let Some(v) = &self.vault {
            v.list_folders()
                .into_iter()
                .filter(|f| {
                    if parent == PathBuf::new() {
                        f.components().count() == 1
                    } else {
                        f.parent().unwrap_or(Path::new("")) == parent.as_path()
                    }
                })
                .collect()
        } else {
            vec![]
        };

        let mut pending_folder_retype: Option<(PathBuf, NoteType)> = None;
        for folder in folders {
            let name = folder
                .file_name()
                .and_then(|s| s.to_str())
                .unwrap_or("?")
                .to_string();
            let folder_clone = folder.clone();

            let header = egui::CollapsingHeader::new(format!("📁 {}", name))
                .id_salt(format!("folder_{}", folder.to_string_lossy()))
                .default_open(true)
                .show(ui, |ui| {
                    self.show_folder_tree(ui, folder_clone.clone());
                    self.show_notes_in_folder(ui, &folder_clone);
                });

            header.header_response.context_menu(|ui| {
                if ui.button("📄+ Nova nota aqui").clicked() {
                    if let Some(v) = &mut self.vault {
                        let rel = folder.clone();
                        match v.create_note(Some(&rel), "", NoteType::default()) {
                            Ok(note) => {
                                self.active_note = Some(note);
                                self.editing = true;
                                self.dirty = false;
                            }
                            Err(e) => self.error_msg = Some(e),
                        }
                    }
                    ui.close_menu();
                }
                ui.menu_button("🏷 Categoria (pasta toda)", |ui| {
                    for t in NoteType::all() {
                        if ui.button(format!("{} {}", t.icon(), t.label())).clicked() {
                            pending_folder_retype = Some((folder.clone(), t));
                            ui.close_menu();
                        }
                    }
                });
                if ui.button("🗑 Deletar pasta").clicked() {
                    self.confirm_action = Some(ConfirmAction::DeleteFolder(folder.clone()));
                    ui.close_menu();
                }
            });
        }

        if let Some((folder, t)) = pending_folder_retype {
            let res = self
                .vault
                .as_mut()
                .map(|v| v.set_folder_note_type(&folder, t));
            match res {
                Some(Ok(n)) => {
                    if let Some(active) = &mut self.active_note {
                        if active.rel_path.starts_with(&folder)
                            && active.path.extension().and_then(|s| s.to_str()) == Some("md")
                        {
                            active.frontmatter.note_type = t;
                        }
                    }
                    self.self_write_until =
                        std::time::Instant::now() + std::time::Duration::from_millis(400);
                    self.toast_success(format!("{n} nota(s) → {}", t.label()));
                }
                Some(Err(e)) => self.toast_error(e),
                None => {}
            }
        }
    }

    fn show_notes_in_folder(&mut self, ui: &mut egui::Ui, folder: &Path) {
        let query_lower = self.query.to_lowercase();
        let type_filter = self.type_filter;
        let active_id = self.active_note.as_ref().map(|n| n.frontmatter.id.clone());

        let notes: Vec<(String, String, NoteType)> = if let Some(v) = &self.vault {
            v.notes
                .iter()
                .filter(|n| {
                    let parent = n.rel_path.parent().unwrap_or(Path::new(""));
                    let in_folder = parent == folder;
                    let matches_q = query_lower.is_empty()
                        || n.title.to_lowercase().contains(&query_lower)
                        || n.content.to_lowercase().contains(&query_lower);
                    let matches_t = type_filter.is_none_or(|t| n.frontmatter.note_type == t);
                    in_folder && matches_q && matches_t
                })
                .map(|n| {
                    (
                        n.frontmatter.id.clone(),
                        format!("{} {}", n.frontmatter.note_type.icon(), n.title),
                        n.frontmatter.note_type,
                    )
                })
                .collect()
        } else {
            vec![]
        };

        let move_targets: Vec<PathBuf> = self
            .vault
            .as_ref()
            .map(|v| v.list_folders())
            .unwrap_or_default();

        let mut pending_select: Option<String> = None;
        let mut pending_delete: Option<String> = None;
        let mut pending_retype: Option<(String, NoteType)> = None;
        let mut pending_move: Option<(String, Option<PathBuf>)> = None;

        for (id, label, current_type) in notes {
            let is_active = active_id.as_deref() == Some(&id);
            let resp = ui.selectable_label(is_active, &label);
            if resp.clicked() {
                pending_select = Some(id.clone());
            }
            resp.context_menu(|ui| {
                if ui.button("📖 Abrir").clicked() {
                    pending_select = Some(id.clone());
                    ui.close_menu();
                }
                ui.menu_button("🏷 Categoria", |ui| {
                    for t in NoteType::all() {
                        let marker = if t == current_type { "● " } else { "   " };
                        if ui
                            .button(format!("{}{} {}", marker, t.icon(), t.label()))
                            .clicked()
                        {
                            pending_retype = Some((id.clone(), t));
                            ui.close_menu();
                        }
                    }
                });
                ui.menu_button("📁 Mover para", |ui| {
                    if ui.button("⌂ (raiz)").clicked() {
                        pending_move = Some((id.clone(), None));
                        ui.close_menu();
                    }
                    for f in &move_targets {
                        if ui.button(format!("📁 {}", f.to_string_lossy())).clicked() {
                            pending_move = Some((id.clone(), Some(f.clone())));
                            ui.close_menu();
                        }
                    }
                });
                ui.separator();
                if ui.button("🗑 Deletar").clicked() {
                    pending_delete = Some(id.clone());
                    ui.close_menu();
                }
            });
        }

        if let Some(id) = pending_select {
            self.select_note(&id);
        }
        if let Some((id, t)) = pending_retype {
            let res = self.vault.as_mut().map(|v| v.set_note_type(&id, t));
            match res {
                Some(Ok(())) => {
                    if let Some(active) = &mut self.active_note {
                        if active.frontmatter.id == id {
                            active.frontmatter.note_type = t;
                        }
                    }
                    self.self_write_until =
                        std::time::Instant::now() + std::time::Duration::from_millis(400);
                    self.toast_success(format!("Categoria → {}", t.label()));
                }
                Some(Err(e)) => self.toast_error(e),
                None => {}
            }
        }
        if let Some((id, dest)) = pending_move {
            let res = self
                .vault
                .as_mut()
                .map(|v| v.move_note_by_id(&id, dest.as_deref()));
            match res {
                Some(Ok(())) => {
                    if let Some(active) = &mut self.active_note {
                        if active.frontmatter.id == id {
                            if let Some(v) = &self.vault {
                                if let Some(fresh) =
                                    v.notes.iter().find(|n| n.frontmatter.id == id).cloned()
                                {
                                    *active = fresh;
                                }
                            }
                        }
                    }
                    self.self_write_until =
                        std::time::Instant::now() + std::time::Duration::from_millis(400);
                    self.toast_success("Nota movida");
                }
                Some(Err(e)) => self.toast_error(e),
                None => {}
            }
        }
        if let Some(id) = pending_delete {
            self.confirm_action = Some(ConfirmAction::DeleteNote(id));
        }
    }
}
