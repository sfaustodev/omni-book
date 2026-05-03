use crate::app::OmniNoteApp;
use crate::types::{ConfirmAction, NoteType};
use egui::RichText;
use std::path::{Path, PathBuf};

/// Drag-and-drop payload: id of the note being dragged.
#[derive(Clone, Debug)]
pub struct NoteIdPayload(pub String);

/// Returns a short human-readable age string from an ISO-8601 date string.
fn time_ago(iso: &str) -> String {
    let Ok(dt) = chrono::DateTime::parse_from_rfc3339(iso) else {
        return "—".to_string();
    };
    let secs = (chrono::Utc::now() - dt.to_utc()).num_seconds().max(0);
    if secs < 120 {
        "JUST NOW".to_string()
    } else if secs < 3600 {
        format!("{} MIN AGO", secs / 60)
    } else if secs < 7200 {
        "1 HR AGO".to_string()
    } else if secs < 86400 {
        format!("{} HR AGO", secs / 3600)
    } else if secs < 172_800 {
        "YESTERDAY".to_string()
    } else {
        format!("{}D", secs / 86400)
    }
}

impl OmniNoteApp {
    pub fn show_sidebar(&mut self, ctx: &egui::Context) {
        use crate::theme;
        egui::SidePanel::left("sidebar")
            .resizable(false)
            .exact_width(360.0)
            .frame(
                egui::Frame::none()
                    .fill(theme::BG)
                    .inner_margin(egui::Margin {
                        left: 20.0,
                        right: 20.0,
                        top: 24.0,
                        bottom: 20.0,
                    })
                    .stroke(egui::Stroke::new(1.0, theme::BORDER)),
            )
            .show(ctx, |ui| {
                ui.spacing_mut().item_spacing.y = 0.0;
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
                        let date = chrono::Utc::now().format("%y.%m").to_string();
                        ui.label(
                            RichText::new(date)
                                .monospace()
                                .size(10.0)
                                .color(theme::DIMMER),
                        );
                    });
                });
                ui.add_space(16.0);

                // Search — orange bordered box with "busca" label + bracket placeholder
                ui.label(
                    RichText::new("BUSCA —")
                        .monospace()
                        .size(9.0)
                        .color(theme::ACCENT),
                );
                ui.add_space(8.0);
                egui::Frame::none()
                    .fill(theme::ACCENT_BG)
                    .stroke(egui::Stroke::new(1.0, theme::ACCENT))
                    .inner_margin(egui::Margin::symmetric(12.0, 8.0))
                    .show(ui, |ui| {
                        ui.horizontal(|ui| {
                            ui.spacing_mut().item_spacing.x = 10.0;
                            ui.label(
                                RichText::new("busca")
                                    .monospace()
                                    .size(13.0)
                                    .strong()
                                    .color(theme::ACCENT),
                            );
                            let search = ui.add(
                                egui::TextEdit::singleline(&mut self.query)
                                    .hint_text("[                      ]")
                                    .frame(false)
                                    .desired_width(f32::INFINITY),
                            );
                            let search_sc = egui::KeyboardShortcut::new(
                                egui::Modifiers::COMMAND,
                                egui::Key::K,
                            );
                            if ctx.input_mut(|i| i.consume_shortcut(&search_sc)) {
                                search.request_focus();
                            }
                            ui.with_layout(
                                egui::Layout::right_to_left(egui::Align::Center),
                                |ui| {
                                    ui.label(
                                        RichText::new("⌘K")
                                            .monospace()
                                            .size(10.0)
                                            .color(theme::ACCENT_SOFT),
                                    );
                                },
                            );
                        });
                    });
                ui.add_space(18.0);

                // Views — type filters
                ui.label(
                    RichText::new("VIEWS —")
                        .monospace()
                        .size(9.0)
                        .color(theme::DIMMER),
                );
                ui.add_space(4.0);
                let total_count = self.vault.as_ref().map(|v| v.notes.len()).unwrap_or(0);
                self.swiss_view_item(ui, "01", "All", self.type_filter.is_none(), total_count, |s| {
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
                ui.add_space(20.0);

                // Note list — flat, sorted by updated desc
                self.show_note_list(ui);

                // Footer — accent NEW NOTE button + icons
                ui.add_space(8.0);
                {
                    let (rule_rect, _) = ui.allocate_exact_size(
                        egui::vec2(ui.available_width(), 1.0),
                        egui::Sense::hover(),
                    );
                    ui.painter().rect_filled(rule_rect, 0.0, theme::BORDER);
                }
                ui.add_space(12.0);
                ui.horizontal(|ui| {
                    let new_btn = egui::Button::new(
                        RichText::new("+ NOTE")
                            .strong()
                            .size(12.0)
                            .color(theme::ACCENT_INK),
                    )
                    .fill(theme::ACCENT)
                    .stroke(egui::Stroke::NONE)
                    .min_size(egui::vec2(72.0, 32.0));
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

    /// Flat note list sorted by most-recently updated, with numbered rows.
    fn show_note_list(&mut self, ui: &mut egui::Ui) {
        use crate::theme;

        let query_lower = self.query.to_lowercase();
        let type_filter = self.type_filter;
        let active_id = self.active_note.as_ref().map(|n| n.frontmatter.id.clone());

        // Collect + filter + sort notes
        let mut notes: Vec<(String, String, String, String)> = if let Some(v) = &self.vault {
            v.notes
                .iter()
                .filter(|n| {
                    let matches_q = query_lower.is_empty()
                        || n.title.to_lowercase().contains(&query_lower)
                        || n.content.to_lowercase().contains(&query_lower);
                    let matches_t = type_filter.is_none_or(|t| n.frontmatter.note_type == t);
                    matches_q && matches_t
                })
                .map(|n| {
                    (
                        n.frontmatter.id.clone(),
                        n.title.clone(),
                        n.frontmatter.created.clone(),
                        n.frontmatter.note_type.label().to_uppercase(),
                    )
                })
                .collect()
        } else {
            vec![]
        };
        // Sort newest first
        notes.sort_by(|a, b| b.2.cmp(&a.2));

        let count = notes.len();

        // Section header
        ui.horizontal(|ui| {
            ui.label(
                RichText::new(format!("NOTES — {:02}", count))
                    .monospace()
                    .size(9.0)
                    .color(theme::DIMMER),
            );
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                ui.label(
                    RichText::new("↓ EDITED")
                        .monospace()
                        .size(9.0)
                        .color(theme::DIMMER),
                );
            });
        });
        ui.add_space(4.0);

        let max_h = (ui.available_height() - 80.0).max(100.0);
        egui::ScrollArea::vertical()
            .id_salt("notes_list_scroll")
            .max_height(max_h)
            .show(ui, |ui| {
                ui.spacing_mut().item_spacing.y = 0.0;
                let mut pending_select: Option<String> = None;
                let mut pending_delete: Option<String> = None;

                for (idx, (id, title, updated, type_label)) in notes.iter().enumerate() {
                    let is_active = active_id.as_deref() == Some(id.as_str());
                    let n_str = format!("{:03}", idx + 1);
                    let age = time_ago(updated);

                    // Row background + left accent border for active
                    let row_fill = if is_active { theme::ACCENT_BG } else { egui::Color32::TRANSPARENT };
                    let row_stroke_color = if is_active { theme::ACCENT } else { theme::BORDER };

                    let resp = egui::Frame::none()
                        .fill(row_fill)
                        .inner_margin(egui::Margin {
                            left: if is_active { 10.0 } else { 12.0 },
                            right: 10.0,
                            top: 10.0,
                            bottom: 10.0,
                        })
                        .show(ui, |ui| {
                            // Top border line
                            let (line_rect, _) = ui.allocate_exact_size(
                                egui::vec2(ui.available_width(), 1.0),
                                egui::Sense::hover(),
                            );
                            ui.painter().rect_filled(line_rect, 0.0, row_stroke_color);
                            ui.add_space(8.0);

                            ui.horizontal(|ui| {
                                ui.spacing_mut().item_spacing.x = 10.0;
                                // Number col (28px)
                                let n_color = if is_active { theme::ACCENT } else { theme::DIMMER };
                                ui.add_sized(
                                    [28.0, 0.0],
                                    egui::Label::new(
                                        RichText::new(&n_str)
                                            .monospace()
                                            .size(10.0)
                                            .color(n_color),
                                    ),
                                );

                                // Content col
                                ui.vertical(|ui| {
                                    let title_color = if is_active { theme::TEXT } else { theme::DIM };
                                    let title_rt = if is_active {
                                        RichText::new(title).size(13.0).color(title_color).strong()
                                    } else {
                                        RichText::new(title).size(13.0).color(title_color)
                                    };
                                    ui.label(title_rt);
                                    ui.add_space(4.0);
                                    ui.horizontal(|ui| {
                                        ui.spacing_mut().item_spacing.x = 8.0;
                                        let meta_color =
                                            if is_active { theme::ACCENT_SOFT } else { theme::DIMMER };
                                        ui.label(
                                            RichText::new(&age)
                                                .monospace()
                                                .size(9.0)
                                                .color(meta_color),
                                        );
                                        let tag_color =
                                            if is_active { theme::ACCENT } else { theme::DIMMER };
                                        let tag_border =
                                            if is_active { theme::ACCENT } else { theme::BORDER };
                                        // Type tag chip
                                        egui::Frame::none()
                                            .fill(if is_active { theme::ACCENT_BG } else { egui::Color32::TRANSPARENT })
                                            .stroke(egui::Stroke::new(1.0, tag_border))
                                            .inner_margin(egui::Margin::symmetric(5.0, 1.0))
                                            .show(ui, |ui| {
                                                ui.label(
                                                    RichText::new(type_label)
                                                        .monospace()
                                                        .size(9.0)
                                                        .color(tag_color),
                                                );
                                            });
                                    });
                                });

                                // Active dot indicator
                                if is_active {
                                    ui.with_layout(
                                        egui::Layout::right_to_left(egui::Align::Min),
                                        |ui| {
                                            let (dot_rect, _) = ui.allocate_exact_size(
                                                egui::vec2(5.0, 5.0),
                                                egui::Sense::hover(),
                                            );
                                            ui.painter().circle_filled(
                                                dot_rect.center(),
                                                2.5,
                                                theme::ACCENT,
                                            );
                                        },
                                    );
                                }
                            });
                        })
                        .response
                        .interact(egui::Sense::click());

                    // Draw left accent border for active row on top of frame
                    if is_active {
                        let row_rect = resp.rect;
                        let border_rect = egui::Rect::from_min_max(
                            row_rect.min,
                            egui::pos2(row_rect.min.x + 2.0, row_rect.max.y),
                        );
                        ui.painter().rect_filled(border_rect, 0.0, theme::ACCENT);
                    }

                    if resp.clicked() {
                        pending_select = Some(id.clone());
                    }
                    resp.context_menu(|ui| {
                        if ui.button("✎ Editar").clicked() {
                            pending_select = Some(id.clone());
                            ui.close_menu();
                        }
                        if ui.button("🗑 Deletar").clicked() {
                            pending_delete = Some(id.clone());
                            ui.close_menu();
                        }
                    });
                }

                if let Some(id) = pending_select {
                    self.select_note(&id);
                }
                if let Some(id) = pending_delete {
                    self.confirm_action = Some(ConfirmAction::DeleteNote(id));
                }
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
        {
            let (line_rect, _) = ui.allocate_exact_size(
                egui::vec2(ui.available_width(), 1.0),
                egui::Sense::hover(),
            );
            ui.painter().rect_filled(line_rect, 0.0, theme::BORDER);
        }
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

    // Folder tree is still used by context menus indirectly via vault.list_folders(),
    // kept here as a private helper for future drag-drop or folder modal.
    #[allow(dead_code)]
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

            if let Some(payload) = dnd_response.1 {
                let id = payload.0.clone();
                if let Some(v) = &mut self.vault {
                    match v.move_note_by_id(&id, Some(&folder)) {
                        Ok(()) => {
                            if let Some(active) = &mut self.active_note {
                                if active.frontmatter.id == id {
                                    if let Some(fresh) =
                                        v.notes.iter().find(|n| n.frontmatter.id == id).cloned()
                                    {
                                        *active = fresh;
                                    }
                                }
                            }
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
