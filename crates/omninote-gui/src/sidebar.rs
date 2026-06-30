//! Left panel: vault header, search filter, `NoteType` filter chips, the folder
//! tree + note list (drag source for notes, drop zones for folders → move), and a
//! footer of actions. Note data is snapshotted up front so the render body can call
//! `&mut self` helpers freely.

use std::path::PathBuf;

use omninote_core::types::NoteType;

use crate::app::OmniNoteApp;

struct Row {
    id: String,
    title: String,
    ty: NoteType,
    folder: Option<String>,
}

impl OmniNoteApp {
    pub fn show_sidebar(&mut self, ctx: &egui::Context) {
        // Snapshot — releases the borrow on `self.vault` for the render body.
        let has_vault = self.vault.is_some();
        let vault_name = self
            .vault
            .as_ref()
            .and_then(|v| v.root.file_name().map(|s| s.to_string_lossy().to_string()))
            .unwrap_or_else(|| "—".to_string());
        let rows: Vec<Row> = self
            .vault
            .as_ref()
            .map(|v| {
                v.notes
                    .iter()
                    .map(|n| Row {
                        id: n.frontmatter.id.clone(),
                        title: n.title.clone(),
                        ty: n.frontmatter.note_type,
                        folder: n
                            .rel_path
                            .parent()
                            .filter(|p| !p.as_os_str().is_empty())
                            .map(|p| p.to_string_lossy().to_string()),
                    })
                    .collect()
            })
            .unwrap_or_default();
        let mut folders: Vec<String> = self
            .vault
            .as_ref()
            .map(|v| {
                v.list_folders()
                    .iter()
                    .map(|p| p.to_string_lossy().to_string())
                    .collect()
            })
            .unwrap_or_default();
        folders.sort();

        let theme = self.theme;

        egui::SidePanel::left("sidebar")
            .resizable(true)
            .default_width(264.0)
            .width_range(200.0..=420.0)
            .frame(egui::Frame::none().fill(theme.panel).inner_margin(10.0))
            .show(ctx, |ui| {
                // Wordmark.
                ui.horizontal(|ui| {
                    ui.label(
                        egui::RichText::new("OMNINOTE")
                            .heading()
                            .color(theme.accent),
                    );
                });
                ui.label(
                    egui::RichText::new(format!("vault · {vault_name}"))
                        .small()
                        .color(theme.faint),
                );
                ui.add_space(6.0);

                // Toolbar.
                ui.horizontal(|ui| {
                    if ui
                        .button("⚙")
                        .on_hover_text("Configurações (Cmd/Ctrl+,)")
                        .clicked()
                    {
                        self.show_settings = true;
                    }
                    let theme_icon = if theme.dark { "☀" } else { "🌙" };
                    if ui
                        .button(theme_icon)
                        .on_hover_text("Alternar tema (Cmd/Ctrl+Shift+D)")
                        .clicked()
                    {
                        self.toggle_theme();
                    }
                    if ui.button("📂").on_hover_text("Abrir vault").clicked() {
                        if let Some(path) = rfd::FileDialog::new().pick_folder() {
                            self.open_vault(path, ctx);
                        }
                    }
                    if ui
                        .button("🔍")
                        .on_hover_text("Buscar (Cmd/Ctrl+K)")
                        .clicked()
                    {
                        self.palette_open = true;
                        self.focus_palette = true;
                    }
                });
                ui.add_space(6.0);

                // Search filter (Cmd/Ctrl+K focuses the palette; this is a live filter).
                let resp = ui.add(
                    egui::TextEdit::singleline(&mut self.filter)
                        .hint_text("filtrar notas…")
                        .desired_width(f32::INFINITY),
                );
                if std::mem::take(&mut self.focus_filter) {
                    resp.request_focus();
                }
                ui.add_space(6.0);

                // Type filter chips.
                self.type_chips(ui);
                ui.separator();

                if !has_vault {
                    ui.add_space(20.0);
                    ui.vertical_centered(|ui| {
                        ui.label(egui::RichText::new("Nenhum vault aberto").color(theme.dim));
                        if ui.button("Abrir vault…").clicked() {
                            if let Some(path) = rfd::FileDialog::new().pick_folder() {
                                self.open_vault(path, ctx);
                            }
                        }
                    });
                    return;
                }

                egui::ScrollArea::vertical()
                    .auto_shrink([false, false])
                    .show(ui, |ui| {
                        self.render_tree(ui, &rows, &folders);
                    });

                // Footer.
                ui.separator();
                ui.horizontal(|ui| {
                    if ui
                        .button("+ nota")
                        .on_hover_text("Nova nota (Cmd/Ctrl+N)")
                        .clicked()
                    {
                        self.new_in_folder = None;
                        self.show_new = true;
                    }
                    if ui.button("+ pasta").clicked() {
                        self.new_folder_name.clear();
                        self.show_new_folder = true;
                    }
                    if ui.button("importar").clicked() {
                        self.show_import = true;
                    }
                });
            });
    }

    fn type_chips(&mut self, ui: &mut egui::Ui) {
        let theme = self.theme;
        ui.horizontal_wrapped(|ui| {
            let all = self.type_filter.is_none();
            if ui.selectable_label(all, "Todos").clicked() {
                self.type_filter = None;
            }
            for ty in NoteType::all() {
                let on = self.type_filter == Some(ty);
                let txt =
                    egui::RichText::new(format!("{} {}", ty.icon(), ty.label())).color(if on {
                        theme.note_type_color(ty)
                    } else {
                        theme.dim
                    });
                if ui.selectable_label(on, txt).clicked() {
                    self.type_filter = if on { None } else { Some(ty) };
                }
            }
        });
    }

    fn row_visible(&self, row: &Row) -> bool {
        if let Some(t) = self.type_filter {
            if row.ty != t {
                return false;
            }
        }
        let q = self.filter.trim().to_lowercase();
        q.is_empty() || row.title.to_lowercase().contains(&q)
    }

    fn render_tree(&mut self, ui: &mut egui::Ui, rows: &[Row], folders: &[String]) {
        let theme = self.theme;
        let filtering = !self.filter.trim().is_empty();

        // When filtering, flatten to a simple matching list.
        if filtering {
            let visible: Vec<&Row> = rows.iter().filter(|r| self.row_visible(r)).collect();
            for row in visible {
                self.note_row(ui, row, 0);
            }
            return;
        }

        // Root drop zone + root notes.
        let (_, dropped) = ui.dnd_drop_zone::<String, _>(egui::Frame::none(), |ui| {
            ui.label(egui::RichText::new("/ (raiz)").small().color(theme.faint));
        });
        if let Some(id) = dropped {
            self.move_note(&id, None);
        }
        let roots: Vec<&Row> = rows
            .iter()
            .filter(|r| r.folder.is_none() && self.row_visible(r))
            .collect();
        for row in roots {
            self.note_row(ui, row, 1);
        }

        // Folders.
        for folder in folders {
            let depth = folder.matches('/').count();
            let name = folder.rsplit('/').next().unwrap_or(folder);
            let collapsed = self.collapsed.contains(folder);

            let frame = egui::Frame::none().inner_margin(egui::Margin::symmetric(2.0, 1.0));
            let (header, dropped) = ui.dnd_drop_zone::<String, _>(frame, |ui| {
                ui.horizontal(|ui| {
                    ui.add_space(depth as f32 * 12.0);
                    let arrow = if collapsed { "▸" } else { "▾" };
                    ui.selectable_label(
                        false,
                        egui::RichText::new(format!("{arrow} 📁 {name}")).color(theme.dim),
                    )
                    .clicked()
                })
            });
            if header.inner.inner {
                if collapsed {
                    self.collapsed.remove(folder);
                } else {
                    self.collapsed.insert(folder.clone());
                }
            }
            if let Some(id) = dropped {
                self.move_note(&id, Some(PathBuf::from(folder)));
            }

            if !collapsed {
                let kids: Vec<&Row> = rows
                    .iter()
                    .filter(|r| r.folder.as_deref() == Some(folder) && self.row_visible(r))
                    .collect();
                for row in kids {
                    self.note_row(ui, row, depth + 2);
                }
            }
        }
    }

    /// One draggable, clickable note row.
    fn note_row(&mut self, ui: &mut egui::Ui, row: &Row, indent: usize) {
        let theme = self.theme;
        let active = self.active_id.as_deref() == Some(row.id.as_str());
        let id = egui::Id::new(("note", &row.id));

        let clicked = ui
            .dnd_drag_source(id, row.id.clone(), |ui| {
                ui.horizontal(|ui| {
                    ui.add_space(indent as f32 * 12.0);
                    ui.label(
                        egui::RichText::new(row.ty.icon()).color(theme.note_type_color(row.ty)),
                    );
                    let mut text = egui::RichText::new(&row.title);
                    if active {
                        text = text.color(theme.accent).strong();
                    }
                    ui.selectable_label(active, text).clicked()
                })
                .inner
            })
            .inner;
        if clicked {
            self.select_note(&row.id);
        }
    }
}
