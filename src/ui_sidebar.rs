use crate::app::OmniNoteApp;
use crate::types::{ConfirmAction, NoteType};
use egui::RichText;
use std::path::{Path, PathBuf};

/// Drag-and-drop payload: id of the note being dragged.
#[derive(Clone, Debug)]
pub struct NoteIdPayload(pub String);

impl OmniNoteApp {
    pub fn show_sidebar(&mut self, ctx: &egui::Context) {
        use crate::theme;
        egui::SidePanel::left("sidebar")
            .resizable(false)
            .exact_width(280.0)
            .frame(
                egui::Frame::none()
                    .fill(theme::BG)
                    .inner_margin(egui::Margin {
                        left: 28.0,
                        right: 28.0,
                        top: 32.0,
                        bottom: 24.0,
                    })
                    .stroke(egui::Stroke::new(1.0, theme::BORDER)),
            )
            .show(ctx, |ui| {
                ui.spacing_mut().item_spacing.y = 4.0;
                ui.spacing_mut().button_padding = egui::vec2(0.0, 2.0);

                // Header — orange square + brand + date right-aligned mono
                ui.horizontal(|ui| {
                    let (rect, _) = ui.allocate_exact_size(
                        egui::vec2(14.0, 14.0),
                        egui::Sense::hover(),
                    );
                    ui.painter().rect_filled(rect, 0.0, theme::ACCENT);
                    ui.add_space(8.0);
                    ui.label(
                        RichText::new("OmniNote")
                            .strong()
                            .size(16.0)
                            .color(theme::TEXT),
                    );
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        let date = chrono::Utc::now().format("%d.%m").to_string();
                        ui.label(
                            RichText::new(date)
                                .monospace()
                                .size(10.0)
                                .color(theme::DIMMER),
                        );
                    });
                });
                ui.add_space(28.0);

                // Search section
                theme::section_label(ui, "SEARCH");
                let search = ui.add(
                    egui::TextEdit::singleline(&mut self.query)
                        .hint_text("")
                        .frame(false)
                        .desired_width(f32::INFINITY),
                );
                theme::hairline(ui);
                let search_sc =
                    egui::KeyboardShortcut::new(egui::Modifiers::COMMAND, egui::Key::K);
                if ctx.input_mut(|i| i.consume_shortcut(&search_sc)) {
                    search.request_focus();
                }
                ui.add_space(24.0);

                // Type filters as numbered list
                theme::section_label(ui, "VIEWS");
                let total_count = self.vault.as_ref().map(|v| v.notes.len()).unwrap_or(0);
                self.swiss_view_item(ui, "01", "Todas", self.type_filter.is_none(), total_count, |s| {
                    s.type_filter = None;
                });
                for (i, t) in NoteType::all().iter().enumerate() {
                    let n = format!("{:02}", i + 2);
                    let selected = self.type_filter == Some(*t);
                    let count = self
                        .vault
                        .as_ref()
                        .map(|v| v.notes.iter().filter(|n| n.frontmatter.note_type == *t).count())
                        .unwrap_or(0);
                    let tt = *t;
                    self.swiss_view_item(ui, &n, t.label(), selected, count, |s| {
                        s.type_filter = if selected { None } else { Some(tt) };
                    });
                }
                ui.add_space(24.0);

                // Folders + notes tree
                theme::section_label(ui, "FOLDERS");
                egui::ScrollArea::vertical()
                    .id_salt("sidebar_scroll")
                    .max_height(ui.available_height() - 90.0)
                    .show(ui, |ui| {
                        self.show_folder_tree(ui, PathBuf::new());
                        self.show_notes_in_folder(ui, &PathBuf::new());
                    });

                // Footer — accent NEW button + mono Cmd+N hint + small icons
                ui.add_space(8.0);
                theme::hairline(ui);
                ui.add_space(12.0);
                ui.horizontal(|ui| {
                    let new_btn = egui::Button::new(
                        RichText::new("NEW")
                            .strong()
                            .size(12.0)
                            .color(theme::ACCENT_INK),
                    )
                    .fill(theme::ACCENT)
                    .stroke(egui::Stroke::NONE)
                    .min_size(egui::vec2(64.0, 32.0));
                    if ui.add(new_btn).on_hover_text("Nova nota").clicked() {
                        self.show_new = true;
                    }
                    ui.add_space(8.0);
                    ui.label(
                        RichText::new("⌘N")
                            .monospace()
                            .size(10.0)
                            .color(theme::DIMMER),
                    );
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        if ui.small_button("⚙").on_hover_text("Configurações (⌘,)").clicked() {
                            self.show_settings = true;
                        }
                        if ui.small_button("📂").on_hover_text("Trocar vault").clicked() {
                            self.pick_vault();
                        }
                        if ui.small_button("📥").on_hover_text("Importar").clicked() {
                            self.show_import = true;
                        }
                        if ui.small_button("📁").on_hover_text("Nova pasta").clicked() {
                            if let Some(v) = &mut self.vault {
                                let _ = v.create_folder(None, "Nova pasta");
                            }
                        }
                    });
                });
            });
    }

    /// Swiss-style nav item: `NN  Label                   count`
    fn swiss_view_item(
        &mut self,
        ui: &mut egui::Ui,
        n: &str,
        label: &str,
        active: bool,
        count: usize,
        mut on_click: impl FnMut(&mut Self),
    ) {
        use crate::theme;
        let row = ui.horizontal(|ui| {
            let (line_rect, _) = ui.allocate_exact_size(
                egui::vec2(ui.available_width(), 1.0),
                egui::Sense::hover(),
            );
            ui.painter().rect_filled(line_rect, 0.0, theme::BORDER);
        });
        let _ = row;
        let resp = ui
            .scope(|ui| {
                ui.horizontal(|ui| {
                    ui.spacing_mut().item_spacing.x = 8.0;
                    let n_color = if active { theme::ACCENT } else { theme::DIMMER };
                    ui.add_sized(
                        [28.0, 22.0],
                        egui::Label::new(
                            RichText::new(n).monospace().size(10.0).color(n_color),
                        ),
                    );
                    let label_color = if active { theme::TEXT } else { theme::DIM };
                    let label_text = if active {
                        RichText::new(label).size(13.0).color(label_color).strong()
                    } else {
                        RichText::new(label).size(13.0).color(label_color)
                    };
                    ui.label(label_text);
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        if count > 0 {
                            ui.label(
                                RichText::new(format!("{:02}", count))
                                    .monospace()
                                    .size(10.0)
                                    .color(theme::DIMMER),
                            );
                        } else {
                            ui.label(
                                RichText::new("—")
                                    .monospace()
                                    .size(10.0)
                                    .color(theme::DIMMER),
                            );
                        }
                    });
                });
            })
            .response
            .interact(egui::Sense::click());
        if resp.clicked() {
            on_click(self);
        }
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

        for folder in folders {
            let name = folder
                .file_name()
                .and_then(|s| s.to_str())
                .unwrap_or("?")
                .to_string();
            let folder_clone = folder.clone();

            // Folder = drop zone for moving notes (v0.7)
            let dnd_response = ui.dnd_drop_zone::<NoteIdPayload, _>(egui::Frame::default(), |ui| {
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
                    if ui.button("🗑 Deletar pasta").clicked() {
                        self.confirm_action = Some(ConfirmAction::DeleteFolder(folder.clone()));
                        ui.close_menu();
                    }
                });
            });

            // Handle drop on this folder
            if let Some(payload) = dnd_response.1 {
                let id = payload.0.clone();
                if let Some(v) = &mut self.vault {
                    match v.move_note_by_id(&id, Some(&folder)) {
                        Ok(()) => {
                            // Update active_note path if it was the moved note
                            if let Some(active) = &mut self.active_note {
                                if active.frontmatter.id == id {
                                    if let Some(fresh) =
                                        v.notes.iter().find(|n| n.frontmatter.id == id).cloned()
                                    {
                                        *active = fresh;
                                    }
                                }
                            }
                            // Self-write window so watcher doesn't bounce
                            self.self_write_until =
                                std::time::Instant::now() + std::time::Duration::from_millis(400);
                        }
                        Err(e) => self.error_msg = Some(e),
                    }
                }
            }
        }
    }

    fn show_notes_in_folder(&mut self, ui: &mut egui::Ui, folder: &Path) {
        let query_lower = self.query.to_lowercase();
        let type_filter = self.type_filter;
        let active_id = self.active_note.as_ref().map(|n| n.frontmatter.id.clone());

        let notes: Vec<(String, String)> = if let Some(v) = &self.vault {
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
                    )
                })
                .collect()
        } else {
            vec![]
        };

        let mut pending_select: Option<String> = None;
        let mut pending_delete: Option<String> = None;

        for (id, label) in notes {
            let is_active = active_id.as_deref() == Some(&id);
            let drag_id = egui::Id::new(format!("note_drag_{}", id));

            ui.horizontal(|ui| {
                ui.spacing_mut().item_spacing.x = 2.0;

                // Drag handle — small icon, only this is the drag source
                ui.dnd_drag_source(drag_id, NoteIdPayload(id.clone()), |ui| {
                    ui.label(
                        egui::RichText::new("⋮⋮")
                            .size(10.0)
                            .weak()
                            .monospace(),
                    )
                    .on_hover_cursor(egui::CursorIcon::Grab)
                    .on_hover_text("Arraste pra mover entre pastas");
                });

                // Normal selectable label — clicks + selection + context_menu work normally
                let label_resp = ui.selectable_label(is_active, &label);
                if label_resp.clicked() {
                    pending_select = Some(id.clone());
                }
                label_resp.context_menu(|ui| {
                    if ui.button("✎ Editar").clicked() {
                        pending_select = Some(id.clone());
                        ui.close_menu();
                    }
                    if ui.button("🗑 Deletar").clicked() {
                        pending_delete = Some(id.clone());
                        ui.close_menu();
                    }
                });
            });
        }

        if let Some(id) = pending_select {
            self.select_note(&id);
        }
        if let Some(id) = pending_delete {
            self.confirm_action = Some(ConfirmAction::DeleteNote(id));
        }
    }
}
